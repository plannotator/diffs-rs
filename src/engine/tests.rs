use super::*;
use std::{fs, io::Write, time::Instant};

use crate::{
    reading::SourceRange,
    source::{CheckedPair, SourceStore, check_pair},
};

mod domain;
mod search;
mod search_delivery;
mod selection;

fn checked_review(prefix: &[u8]) -> (Engine, tempfile::TempDir, Vec<Arc<CheckedPair>>) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("review.patch");
    fs::write(&path, b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -2 +2 @@\n-before\n+after\ndiff --git a/b b/b\n--- a/b\n+++ b/b\n@@ -2 +2 @@\n-before\n+after\n").unwrap();
    let (wake, ready) = mpsc::sync_channel(1);
    let mut engine = Engine::open(Input::Patch(path), Options::default(), wake).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while engine.status().loading {
        engine.poll().unwrap();
        assert!(Instant::now() < deadline, "input worker did not finish");
        if engine.status().loading {
            let _ = ready.recv_timeout(Duration::from_millis(20));
        }
    }
    assert!(engine.status().error.is_none());
    let old = directory.path().join("old");
    let new = directory.path().join("new");
    for (path, value) in [
        (&old, b"before\n".as_slice()),
        (&new, b"after\n".as_slice()),
    ] {
        let mut file = File::create(path).unwrap();
        file.write_all(prefix).unwrap();
        file.write_all(value).unwrap();
        file.write_all(b"suffix\n").unwrap();
    }
    let app = engine.app.as_ref().unwrap();
    let mut store = SourceStore::sharing(&app.doc.files[0].spool).unwrap();
    let old = store.capture(File::open(old).unwrap(), &|| Ok(())).unwrap();
    let new = store.capture(File::open(new).unwrap(), &|| Ok(())).unwrap();
    let pairs = app
        .doc
        .files
        .iter()
        .map(|file| {
            Arc::new(
                check_pair(file.clone(), Some(old.clone()), Some(new.clone()), &|| {
                    Ok(())
                })
                .unwrap(),
            )
        })
        .collect();
    (engine, directory, pairs)
}

fn range(engine: &Engine, file: usize, first: u32, last: u32) -> SourceRange {
    SourceRange {
        review: engine.review_id(),
        file,
        side: Side::New,
        first,
        last,
    }
}

fn install(engine: &mut Engine, pairs: &[Arc<CheckedPair>]) {
    let app = engine.app.as_mut().unwrap();
    app.sources = Some(pairs.iter().map(|_| SourceState::Pending).collect());
    for (file, pair) in pairs.iter().enumerate() {
        app.set_source(file, SourceState::Checked(pair.clone()))
            .unwrap();
    }
}

#[test]
fn withdrawing_checked_source_revokes_only_its_full_source_jobs() {
    let (mut engine, _directory, pairs) = checked_review(b"prefix\n");
    let patch_range = range(&engine, 0, 2, 2);
    let patch = engine.source_read(patch_range).unwrap();
    install(&mut engine, &pairs);
    let first_range = range(&engine, 0, 1, 3);
    let second_range = range(&engine, 1, 1, 3);
    let first = engine.source_read(first_range).unwrap();
    let second = engine.source_read(second_range).unwrap();

    engine
        .app
        .as_mut()
        .unwrap()
        .set_source(0, SourceState::Checked(pairs[0].clone()))
        .unwrap();
    let mut output = Vec::new();
    first.write_to(&mut output, &|| Ok(())).unwrap();
    assert_eq!(output, b"prefix\nafter\nsuffix\n");

    engine
        .app
        .as_mut()
        .unwrap()
        .set_source(0, SourceState::Unavailable("source withdrawn".into()))
        .unwrap();
    let mut output = tempfile::tempfile().unwrap();
    let error = first.write_to(&mut output, &|| Ok(())).unwrap_err();
    assert!(error.to_string().contains("no longer available"));
    assert_eq!(output.metadata().unwrap().len(), 0);
    assert!(
        engine
            .source_read(first_range)
            .unwrap()
            .write_to(&mut output, &|| Ok(()))
            .is_err()
    );
    assert_eq!(output.metadata().unwrap().len(), 0);
    let mut output = Vec::new();
    second.write_to(&mut output, &|| Ok(())).unwrap();
    assert_eq!(output, b"prefix\nafter\nsuffix\n");
    output.clear();
    patch.write_to(&mut output, &|| Ok(())).unwrap();
    assert_eq!(output, b"after\n");

    engine
        .app
        .as_mut()
        .unwrap()
        .set_source(0, SourceState::Checked(pairs[0].clone()))
        .unwrap();
    assert!(first.write_to(&mut Vec::new(), &|| Ok(())).is_err());
    let mut output = Vec::new();
    engine
        .source_read(first_range)
        .unwrap()
        .write_to(&mut output, &|| Ok(()))
        .unwrap();
    assert_eq!(output, b"prefix\nafter\nsuffix\n");
    engine.close().unwrap();
}

#[test]
fn source_withdrawal_interrupts_an_in_flight_full_source_copy() {
    let mut prefix = vec![b'x'; 256 * 1024];
    prefix.push(b'\n');
    let (mut engine, _directory, pairs) = checked_review(&prefix);
    install(&mut engine, &pairs);
    let selected = range(&engine, 0, 1, 1);
    let read = engine.source_read(selected).unwrap();
    let (written, observed) = mpsc::sync_channel(1);
    let (resume, resumed) = mpsc::sync_channel(1);
    std::thread::scope(|scope| {
        let worker = scope.spawn(move || {
            let mut output = tempfile::tempfile().unwrap();
            let file = output.try_clone().unwrap();
            let waited = std::cell::Cell::new(false);
            let result = read.write_to(&mut output, &|| {
                if file.metadata()?.len() != 0 && !waited.replace(true) {
                    written.send(())?;
                    resumed.recv_timeout(Duration::from_secs(5))?;
                }
                Ok(())
            });
            (result, output.metadata().unwrap().len())
        });
        observed.recv_timeout(Duration::from_secs(5)).unwrap();
        engine
            .app
            .as_mut()
            .unwrap()
            .set_source(0, SourceState::Unavailable("source withdrawn".into()))
            .unwrap();
        resume.send(()).unwrap();
        let (result, length) = worker.join().unwrap();
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("no longer available")
        );
        assert!(length > 0 && length <= 2 * 64 * 1024, "{length}");
    });
    engine.close().unwrap();
}
