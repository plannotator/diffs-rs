use super::*;
use crate::{
    input::{Cancellation, Cancelled},
    reading::{literal::Direction, search::Position},
    view::LineId,
};

pub(super) fn review(bytes: &[u8]) -> (Engine, tempfile::TempDir) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("review.patch");
    fs::write(&path, bytes).unwrap();
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
    (engine, directory)
}

#[test]
fn literal_search_follows_unified_sources_and_wraps_in_both_directions() {
    let patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1,4 +1,4 @@\n context needle\n-\xffneedle\n+new needle\n-old two needle\n+new two needle\n tail needle\n\\ No newline at end of file\ndiff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old needle\n+new needle\n";
    let (mut engine, _directory) = review(patch);
    let domain = engine.search_domain().unwrap();
    let cache_before = domain.files[0].order.file.spool.cached_bytes();
    let expected = [
        (0, 0, 8),
        (0, 1, 1),
        (0, 3, 8),
        (0, 2, 4),
        (0, 4, 8),
        (0, 5, 5),
        (1, 6, 4),
        (1, 7, 4),
    ];
    let mut position = Position::default();
    let mut first = None;
    for &(file, source, offset) in &expected {
        let hit = domain
            .find("needle", position, Direction::Forward, &|| Ok(()))
            .unwrap()
            .unwrap();
        assert_eq!(hit.review, engine.review_id());
        assert_eq!(
            (hit.position.file, hit.source, hit.bytes.clone()),
            (file, LineId::Patch(source), offset..offset + 6)
        );
        assert!(!hit.wrapped);
        first.get_or_insert_with(|| hit.clone());
        position = Position {
            byte: hit.bytes.start + 1,
            ..hit.position
        };
    }
    let wrapped = domain
        .find("needle", position, Direction::Forward, &|| Ok(()))
        .unwrap()
        .unwrap();
    assert!(wrapped.wrapped);
    assert_eq!(wrapped.position, first.as_ref().unwrap().position);
    position = wrapped.position;
    for (step, &(file, source, offset)) in expected.iter().rev().enumerate() {
        let hit = domain
            .find("needle", position, Direction::Backward, &|| Ok(()))
            .unwrap()
            .unwrap();
        assert_eq!(
            (hit.position.file, hit.source, hit.bytes.clone()),
            (file, LineId::Patch(source), offset..offset + 6)
        );
        assert_eq!(hit.wrapped, step == 0);
        position = hit.position;
    }
    assert!(
        domain
            .find("\\xff", Position::default(), Direction::Forward, &|| Ok(()))
            .unwrap()
            .is_none()
    );
    assert!(
        domain
            .find("absent", Position::default(), Direction::Backward, &|| Ok(
                ()
            ))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        domain.files[0].order.file.spool.cached_bytes(),
        cache_before
    );
    let invalid = Position {
        byte: 1_000,
        ..first.unwrap().position
    };
    for direction in [Direction::Forward, Direction::Backward] {
        assert!(
            domain
                .find("needle", invalid, direction, &|| Ok(()))
                .is_err()
        );
    }
    engine.close().unwrap();
    assert!(
        domain
            .find(
                "needle",
                Position::default(),
                Direction::Forward,
                &|| Ok(())
            )
            .is_err()
    );
}

#[test]
fn overlapping_unicode_and_chunk_boundary_hits_keep_raw_offsets() {
    let mut patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-".to_vec();
    let mut old = vec![b'.'; 3 * 64 * 1024];
    let query = "\t日本語 e\u{301}";
    let offset = 2 * 64 * 1024 - 1;
    old[offset..offset + query.len()].copy_from_slice(query.as_bytes());
    patch.extend_from_slice(&old);
    patch.extend_from_slice(b"\n+ababa\n");
    let (mut engine, _directory) = review(&patch);
    let domain = engine.search_domain().unwrap();
    for direction in [Direction::Forward, Direction::Backward] {
        let hit = domain
            .find(query, Position::default(), direction, &|| Ok(()))
            .unwrap()
            .unwrap();
        assert_eq!(hit.source, LineId::Patch(0));
        assert_eq!(hit.bytes, offset as u64..(offset + query.len()) as u64);
    }
    let first = domain
        .find("aba", Position::default(), Direction::Forward, &|| Ok(()))
        .unwrap()
        .unwrap();
    assert_eq!(first.bytes, 0..3);
    let next = domain
        .find(
            "aba",
            Position {
                byte: 1,
                ..first.position
            },
            Direction::Forward,
            &|| Ok(()),
        )
        .unwrap()
        .unwrap();
    assert_eq!(next.bytes, 2..5);
    let previous = domain
        .find("aba", next.position, Direction::Backward, &|| Ok(()))
        .unwrap()
        .unwrap();
    assert_eq!(previous.bytes, 0..3);
    assert!(!previous.wrapped);
    let cancel = Cancellation::default();
    let checks = std::cell::Cell::new(0);
    let result = domain.find("missing", Position::default(), Direction::Forward, &|| {
        checks.set(checks.get() + 1);
        if checks.get() == 10 {
            cancel.stop();
        }
        cancel.check()
    });
    assert!(result.unwrap_err().is::<Cancelled>());
    assert_eq!(checks.get(), 10);
    engine.close().unwrap();
}

#[test]
fn search_includes_only_exposed_context_and_does_not_revive_withdrawn_domains() {
    let (mut engine, _directory, pairs) = checked_review(b"prefix needle\n");
    install(&mut engine, &pairs);
    let hidden = engine.search_domain().unwrap();
    assert!(
        hidden
            .find(
                "needle",
                Position::default(),
                Direction::Forward,
                &|| Ok(())
            )
            .unwrap()
            .is_none()
    );
    {
        let app = engine.app.as_mut().unwrap();
        app.view.expand(&app.doc, 0, 0).unwrap();
    }
    assert!(
        hidden
            .find(
                "needle",
                Position::default(),
                Direction::Forward,
                &|| Ok(())
            )
            .is_err()
    );
    let exposed = engine.search_domain().unwrap();
    let hit = exposed
        .find(
            "needle",
            Position::default(),
            Direction::Forward,
            &|| Ok(()),
        )
        .unwrap()
        .unwrap();
    assert_eq!(hit.source, LineId::context(0, 1, 1).unwrap());
    assert_eq!(hit.bytes, 7..13);
    let next = exposed
        .find(
            "needle",
            Position {
                byte: 8,
                ..hit.position
            },
            Direction::Forward,
            &|| Ok(()),
        )
        .unwrap()
        .unwrap();
    assert_eq!(next.source, hit.source);
    assert!(next.wrapped);
    engine
        .app
        .as_mut()
        .unwrap()
        .set_source(0, SourceState::Unavailable("withdrawn".into()))
        .unwrap();
    engine
        .app
        .as_mut()
        .unwrap()
        .set_source(0, SourceState::Checked(pairs[0].clone()))
        .unwrap();
    assert!(
        exposed
            .find(
                "needle",
                Position::default(),
                Direction::Forward,
                &|| Ok(())
            )
            .is_err()
    );
    engine.close().unwrap();
}

#[test]
fn closing_the_review_interrupts_an_owned_search_thread_before_publication() {
    let (mut engine, _directory) =
        review(b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-before\n+needle\n");
    let domain = engine.search_domain().unwrap();
    let owner = Arc::downgrade(&domain.files[0].order.file);
    let (entered, observed) = mpsc::sync_channel(1);
    let (resume, resumed) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let waited = std::cell::Cell::new(false);
        domain.find("needle", Position::default(), Direction::Forward, &|| {
            if !waited.replace(true) {
                entered.send(())?;
                resumed.recv_timeout(Duration::from_secs(5))?;
            }
            Ok(())
        })
    });
    observed.recv_timeout(Duration::from_secs(5)).unwrap();
    engine.close().unwrap();
    resume.send(()).unwrap();
    assert!(
        worker
            .join()
            .unwrap()
            .unwrap_err()
            .to_string()
            .contains("closed review")
    );
    assert!(owner.upgrade().is_none());
}
