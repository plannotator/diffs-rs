use super::*;
use diffs_rs::{
    engine::{Engine, Input, Options, Side},
    reading::SourceRange,
};
use std::{
    cell::Cell,
    os::unix::fs::symlink,
    sync::mpsc,
    time::{Duration, Instant},
};

fn review(patch: &[u8]) -> (Engine, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("review.patch");
    fs::write(&path, patch).unwrap();
    let (wake, ready) = mpsc::sync_channel(1);
    let mut engine = Engine::open(Input::Patch(path), Options::default(), wake).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while engine.status().loading {
        engine.poll().unwrap();
        assert!(Instant::now() < deadline);
        if engine.status().loading {
            let _ = ready.recv_timeout(Duration::from_millis(20));
        }
    }
    assert!(engine.status().error.is_none());
    (engine, dir)
}

fn source(engine: &mut Engine, side: Side, first: u32, last: u32) -> Arc<SourceRead> {
    Arc::new(
        engine
            .source_read(SourceRange {
                review: engine.review_id(),
                file: 0,
                side,
                first,
                last,
            })
            .unwrap(),
    )
}

fn staging(parent: &std::path::Path) -> Vec<PathBuf> {
    fs::read_dir(parent)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".diffs-save-")
        })
        .collect()
}

const PATCH: &[u8] = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1,2 +1,2 @@\n-\told\r\n-last\xff\n\\ No newline at end of file\n+\tnew \xe6\x97\xa5\r\n+tail\n\\ No newline at end of file\n";

#[test]
fn private_staging_and_no_clobber_publication_preserve_original_bytes_on_both_sides() {
    let (mut engine, dir) = review(PATCH);
    for (side, expected, name) in [
        (Side::Old, b"\told\r\nlast\xff".as_slice(), "old q bytes"),
        (
            Side::New,
            b"\tnew \xe6\x97\xa5\r\ntail".as_slice(),
            "new \u{65e5} bytes",
        ),
    ] {
        let path = dir.path().join(name);
        let prepared =
            Prepared::prepare(source(&mut engine, side, 1, 2), path.clone(), &|| Ok(())).unwrap();
        assert!(!path.exists());
        assert_eq!(fs::read(&prepared.path).unwrap(), expected);
        assert_eq!(
            fs::metadata(prepared.directory.path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&prepared.path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let saved = prepared.publish(engine.review_id()).unwrap();
        assert_eq!(saved.destination, path);
        assert_eq!(saved.bytes as usize, expected.len());
        saved.cleanup.unwrap();
        assert_eq!(fs::read(&path).unwrap(), expected);
        assert!(staging(dir.path()).is_empty());
    }
    engine.close().unwrap();
}

#[test]
fn existing_files_links_and_publication_races_are_never_overwritten() {
    let (mut engine, dir) = review(PATCH);
    let read = source(&mut engine, Side::Old, 1, 2);
    let target = dir.path().join("target");
    fs::write(&target, b"unchanged").unwrap();
    let link = dir.path().join("link");
    symlink(&target, &link).unwrap();
    let missing = dir.path().join("missing");
    let broken = dir.path().join("broken-link");
    symlink(&missing, &broken).unwrap();
    for path in [&target, &link, &broken] {
        let error = Prepared::prepare(read.clone(), path.to_owned(), &|| Ok(()))
            .err()
            .unwrap();
        assert!(error.to_string().contains("already exists"));
        assert!(staging(dir.path()).is_empty());
    }
    for link_race in [false, true] {
        let path = dir
            .path()
            .join(if link_race { "late-link" } else { "late-file" });
        let prepared = Prepared::prepare(read.clone(), path.clone(), &|| Ok(())).unwrap();
        if link_race {
            symlink(&target, &path).unwrap();
        } else {
            fs::write(&path, b"later owner").unwrap();
        }
        assert!(prepared.publish(engine.review_id()).is_err());
        assert_eq!(
            fs::read(&path).unwrap(),
            if link_race {
                b"unchanged".as_slice()
            } else {
                b"later owner"
            }
        );
        assert!(staging(dir.path()).is_empty());
    }
    assert_eq!(fs::read(&target).unwrap(), b"unchanged");
    assert_eq!(fs::read_link(&link).unwrap(), target);
    assert_eq!(fs::read_link(&broken).unwrap(), missing);
    engine.close().unwrap();
}

#[test]
fn stale_reviews_and_unavailable_gaps_leave_no_destination_or_staging() {
    let (mut engine, dir) = review(PATCH);
    let path = dir.path().join("closed");
    let prepared = Prepared::prepare(source(&mut engine, Side::Old, 1, 2), path.clone(), &|| {
        Ok(())
    })
    .unwrap();
    let id = engine.review_id();
    engine.close().unwrap();
    assert!(
        prepared
            .publish(id)
            .err()
            .unwrap()
            .to_string()
            .contains("closed review")
    );
    assert!(!path.exists());
    assert!(staging(dir.path()).is_empty());

    let (mut engine, dir) = review(b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n@@ -9 +9 @@\n-last\n+later\n");
    let path = dir.path().join("gap");
    assert!(
        Prepared::prepare(source(&mut engine, Side::Old, 1, 9), path.clone(), &|| Ok(
            ()
        ))
        .is_err()
    );
    assert!(!path.exists());
    assert!(staging(dir.path()).is_empty());
    let missing_parent = dir.path().join("missing/child");
    assert!(
        Prepared::prepare(
            source(&mut engine, Side::Old, 1, 1),
            missing_parent,
            &|| Ok(())
        )
        .is_err()
    );
    let prepared = Prepared::prepare(source(&mut engine, Side::Old, 1, 1), path.clone(), &|| {
        Ok(())
    })
    .unwrap();
    prepared
        .publish(engine.review_id())
        .unwrap()
        .cleanup
        .unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"old\n");
    engine.close().unwrap();
}

#[test]
fn cancellation_after_actual_partial_output_removes_all_private_data() {
    let mut patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-".to_vec();
    patch.extend(std::iter::repeat_n(b'x', 256 * 1024));
    patch.extend_from_slice(b"\n+new\n");
    let (mut engine, dir) = review(&patch);
    let path = dir.path().join("canceled");
    let partial = Cell::new(0);
    let check = || {
        for directory in staging(dir.path()) {
            for entry in fs::read_dir(directory)? {
                let bytes = entry?.metadata()?.len();
                if bytes > 0 {
                    partial.set(bytes);
                    anyhow::bail!("Canceled after writing source");
                }
            }
        }
        Ok(())
    };
    let error = Prepared::prepare(source(&mut engine, Side::Old, 1, 1), path.clone(), &check)
        .err()
        .unwrap();
    assert!(error.to_string().contains("Canceled after writing"));
    assert!(partial.get() > 0 && partial.get() < 256 * 1024);
    assert!(!path.exists());
    assert!(staging(dir.path()).is_empty());
    engine.close().unwrap();
}

#[test]
fn failed_cleanup_stays_typed_when_source_validation_also_fails() {
    let (mut engine, dir) = review(PATCH);
    let path = dir.path().join("never-published");
    let prepared = Prepared::prepare(source(&mut engine, Side::Old, 1, 2), path.clone(), &|| {
        Ok(())
    })
    .unwrap();
    let staging_path = prepared.directory.path().to_owned();
    let held = dir.path().join("moved-private-data");
    fs::rename(&staging_path, &held).unwrap();
    fs::write(&staging_path, b"test replacement").unwrap();
    engine.close().unwrap();
    let error = prepared.publish(engine.review_id()).err().unwrap();
    assert!(error.is::<CleanupError>(), "{error:#}");
    assert!(format!("{error:#}").contains("closed review"));
    assert!(format!("{error:#}").contains(&staging_path.display().to_string()));
    assert!(!path.exists());
    assert_eq!(fs::read(&staging_path).unwrap(), b"test replacement");
    // The test owns both displaced paths; the save operation must not guess where they moved.
    fs::remove_file(staging_path).unwrap();
    fs::remove_dir_all(held).unwrap();
}
