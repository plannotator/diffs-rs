use std::{
    fs,
    sync::mpsc,
    time::{Duration, Instant},
};

use super::*;
use crate::{
    highlight::{self, FileSource, Request},
    theme::Appearance,
};

#[test]
fn private_patch_index_failures_remain_fatal_in_preparation_and_full_source_coloring() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.ts");
    let new = dir.path().join("new.ts");
    fs::write(&old, b"return 1;\n").unwrap();
    fs::write(&new, b"return 2;\n").unwrap();
    for prepared in [false, true] {
        let mut review = Review::open(
            LocalInput::Files {
                old: old.clone(),
                new: new.clone(),
            },
            &Cancellation::default(),
            |_| Ok(()),
        )
        .unwrap();
        let (_index, path) =
            storage::tests::copy_index(Arc::get_mut(&mut review.doc.files[0]).unwrap());
        let pair = if prepared {
            let SourceState::Checked(pair) = review.prepare(0, &Cancellation::default()).unwrap()
            else {
                panic!("Source not checked");
            };
            Some(pair)
        } else {
            None
        };
        fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(0)
            .unwrap();
        if let Some(pair) = pair {
            let (wake, notified) = mpsc::sync_channel(1);
            let mut worker = highlight::start(wake);
            worker.request(Request {
                epoch: 0,
                appearance: Appearance::Dark,
                ids: vec![0usize],
                files: vec![FileSource::Complete(pair)],
            });
            let deadline = Instant::now() + Duration::from_secs(20);
            let error = loop {
                match worker.try_reply() {
                    Ok(reply) => {
                        break reply
                            .err()
                            .expect("Index failure was downgraded to a style");
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        let remaining = deadline.saturating_duration_since(Instant::now());
                        assert!(!remaining.is_zero());
                        notified.recv_timeout(remaining).unwrap();
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            assert!(error.is::<crate::source_spans::ReadFailure>());
            worker.close().unwrap();
        } else {
            let error = review.prepare(0, &Cancellation::default()).unwrap_err();
            assert!(error.is::<crate::source_spans::ReadFailure>());
            assert!(matches!(review.sources()[0], SourceState::Pending));
        }
    }
}
