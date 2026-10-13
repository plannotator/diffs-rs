use std::{fs, sync::mpsc, time::Duration};

use super::*;
use crate::{
    input::{Cancellation, LocalInput, Review, SourceState},
    source::{SourceStore, check_pair},
};

fn reply(worker: &Worker, wake: &mpsc::Receiver<()>) -> Reply {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        match worker.try_reply() {
            Ok(reply) => return reply.unwrap(),
            Err(mpsc::TryRecvError::Empty) => {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                assert!(!remaining.is_zero());
                wake.recv_timeout(remaining).unwrap();
            }
            Err(error) => panic!("{error}"),
        }
    }
}

#[test]
fn either_source_read_failure_is_reported_and_replacement_recovers() {
    let dir = tempfile::tempdir().unwrap();
    let prefix = "return 1; // hidden source\n".repeat(5000);
    let old = format!("{prefix}return 2;\n");
    let new = format!("{prefix}return 3;\n");
    let a = dir.path().join("old.ts");
    let b = dir.path().join("new.ts");
    fs::write(&a, &old).unwrap();
    fs::write(&b, &new).unwrap();
    let mut review = Review::open(
        LocalInput::Files { old: a, new: b },
        &Cancellation::default(),
        |_| Ok(()),
    )
    .unwrap();
    let SourceState::Checked(healthy) = review.prepare(0, &Cancellation::default()).unwrap() else {
        panic!("Source not checked");
    };
    let id = LineId::context(0, 4800, 4800).unwrap();
    for old_fault in [true, false] {
        let file = healthy.file();
        let (mut fault, path) = SourceStore::named_sharing(&file.spool).unwrap();
        let mut intact = SourceStore::sharing(&file.spool).unwrap();
        let (a, b) = if old_fault {
            (
                fault.capture(old.as_bytes(), &|| Ok(())).unwrap(),
                intact.capture(new.as_bytes(), &|| Ok(())).unwrap(),
            )
        } else {
            (
                intact.capture(old.as_bytes(), &|| Ok(())).unwrap(),
                fault.capture(new.as_bytes(), &|| Ok(())).unwrap(),
            )
        };
        let broken = Arc::new(check_pair(file.clone(), Some(a), Some(b), &|| Ok(())).unwrap());
        fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(10_000)
            .unwrap();
        let (wake, notified) = mpsc::sync_channel(1);
        let worker = start(wake);
        for (pair, failed) in [(broken, true), (healthy.clone(), false)] {
            worker.request(Request {
                epoch: 0,
                appearance: Appearance::Dark,
                ids: vec![id],
                files: vec![FileSource::Complete(pair.clone())],
            });
            let reply = reply(&worker, &notified);
            assert_eq!(reply.id, id);
            assert_eq!(reply.origin, FileSource::Complete(pair).origin());
            assert_eq!(reply.source_error.is_some(), failed);
            if failed {
                assert!(reply.style.tokens.is_empty() && reply.style.old_tokens.is_none());
            } else {
                for side in [true, false] {
                    assert_eq!(reply.style.tokens_for(side)[0].range, 0..6);
                    assert_eq!(
                        reply.style.tokens_for(side)[0].color,
                        Color::Rgb(255, 103, 141)
                    );
                }
            }
        }
    }
}
