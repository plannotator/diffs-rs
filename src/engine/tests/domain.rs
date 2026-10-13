use super::*;
use crate::{app::Position, view::LineId};
use std::io::Read;

fn ids(domain: &crate::reading::domain::Domain, file: usize) -> Vec<LineId> {
    let order = &domain.files[file].order;
    (0..order.len())
        .filter_map(|row| order.source(row))
        .collect()
}

#[test]
fn search_scope_owns_only_exposed_context_and_reflow_keeps_it_valid() {
    let (mut engine, _directory, pairs) = checked_review(b"prefix\n");
    let patch = engine.search_domain().unwrap();
    assert_eq!(patch.review, engine.review_id());
    assert_eq!(patch.files.len(), 2);
    assert_eq!(ids(&patch, 0), [LineId::Patch(0), LineId::Patch(1)]);
    assert_eq!(ids(&patch, 1), [LineId::Patch(2), LineId::Patch(3)]);
    assert!(patch.files.iter().all(|file| file.context.is_none()));

    install(&mut engine, &pairs);
    assert!(patch.check().is_err());
    let hidden = engine.search_domain().unwrap();
    assert!(hidden.files.iter().all(|file| file.context.is_none()));
    let retained = Arc::strong_count(pairs[0].new_source().unwrap());
    {
        let app = engine.app.as_mut().unwrap();
        app.set_sidebar_preference(SidebarPreference::Hide);
        app.prepare_view(Rect::new(0, 0, 120, 30)).unwrap();
        let row = (0..app.row_len())
            .find(|&row| app.view.gap(&app.doc, row, app.split()).is_some())
            .unwrap();
        app.cursor = Position { row, part: 0 };
        assert!(app.expand_context().unwrap());
    }
    assert!(hidden.check().is_err());
    let exposed = engine.search_domain().unwrap();
    let expected = [
        LineId::context(0, 1, 1).unwrap(),
        LineId::Patch(0),
        LineId::Patch(1),
    ];
    assert_eq!(ids(&exposed, 0), expected);
    assert_eq!(ids(&exposed, 1), [LineId::Patch(2), LineId::Patch(3)]);
    assert!(exposed.files[1].context.is_none());
    assert_eq!(
        Arc::strong_count(pairs[0].new_source().unwrap()),
        retained + 1
    );
    let mut original = Vec::new();
    exposed.files[0]
        .context
        .as_ref()
        .unwrap()
        .uncached_range(0..1)
        .unwrap()
        .read_to_end(&mut original)
        .unwrap();
    assert_eq!(original, b"prefix\n");
    for command in [
        Command::ToggleLayout,
        Command::ToggleWrap,
        Command::ToggleTheme,
        Command::ToggleNumbers,
    ] {
        engine.execute(command).unwrap();
        exposed.check().unwrap();
        assert_eq!(ids(&exposed, 0), expected);
    }
    for width in [80, 180, 100, 160] {
        engine
            .app
            .as_mut()
            .unwrap()
            .prepare_view(Rect::new(3, 2, width, 30))
            .unwrap();
        exposed.check().unwrap();
    }
    engine
        .app
        .as_mut()
        .unwrap()
        .set_sidebar_preference(SidebarPreference::Show);
    engine
        .app
        .as_mut()
        .unwrap()
        .prepare_view(Rect::new(0, 0, 160, 30))
        .unwrap();
    exposed.check().unwrap();

    engine
        .app
        .as_mut()
        .unwrap()
        .set_source(0, SourceState::Unavailable("source withdrawn".into()))
        .unwrap();
    assert!(exposed.check().is_err());
    let withdrawn = engine.search_domain().unwrap();
    assert!(withdrawn.files[0].context.is_none());
    assert_eq!(ids(&withdrawn, 0), [LineId::Patch(0), LineId::Patch(1)]);
    engine
        .app
        .as_mut()
        .unwrap()
        .set_source(0, SourceState::Checked(pairs[0].clone()))
        .unwrap();
    assert!(withdrawn.check().is_err());
    assert!(exposed.check().is_err());
    let restored = engine.search_domain().unwrap();
    assert!(restored.files[0].context.is_none());
    assert_eq!(ids(&restored, 0), [LineId::Patch(0), LineId::Patch(1)]);
    restored.check().unwrap();
    drop(exposed);
    assert_eq!(Arc::strong_count(pairs[0].new_source().unwrap()), retained);
    engine.close().unwrap();
    assert!(restored.check().is_err());
}

#[test]
fn search_scope_rejects_live_partial_and_failed_inputs_but_accepts_completed_inputs() {
    for malformed in [false, true] {
        let (read_end, write_end) = nix::unistd::pipe().unwrap();
        let mut producer = File::from(write_end);
        let (wake, ready) = mpsc::sync_channel(1);
        let mut engine = Engine::open(
            Input::Stream {
                file: read_end.into(),
                label: "source-scope".into(),
            },
            Options::default(),
            wake,
        )
        .unwrap();
        assert!(engine.search_domain().is_err());
        producer.write_all(b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+first\ndiff --git a/b b/b\n").unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while engine.status().files == 0 {
            engine.poll().unwrap();
            assert!(engine.status().error.is_none());
            assert!(
                Instant::now() < deadline,
                "first streamed file did not arrive"
            );
            let _ = ready.recv_timeout(Duration::from_millis(20));
        }
        assert!(engine.status().loading);
        assert_eq!(engine.status().files, 1);
        assert!(engine.search_domain().is_err());
        let remainder = if malformed {
            b"--- a/b\n+++ b/b\n@@ -1,5 +1,5 @@\n-old\n+second\n".as_slice()
        } else {
            b"--- a/b\n+++ b/b\n@@ -1 +1 @@\n-old\n+second\n".as_slice()
        };
        producer.write_all(remainder).unwrap();
        drop(producer);
        let deadline = Instant::now() + Duration::from_secs(5);
        while engine.status().loading {
            engine.poll().unwrap();
            assert!(Instant::now() < deadline, "streamed input did not finish");
            let _ = ready.recv_timeout(Duration::from_millis(20));
        }
        if malformed {
            assert!(engine.status().error.is_some());
            assert_eq!(engine.status().files, 1);
            assert!(engine.search_domain().is_err());
        } else {
            assert!(engine.status().error.is_none());
            let domain = engine.search_domain().unwrap();
            assert_eq!(domain.files.len(), 2);
            assert_eq!(ids(&domain, 1), [LineId::Patch(2), LineId::Patch(3)]);
            let store = Arc::downgrade(&domain.files[0].order.file.spool);
            domain.check().unwrap();
            engine.close().unwrap();
            assert!(domain.check().is_err());
            assert!(store.upgrade().is_some());
            drop(domain);
            assert!(store.upgrade().is_none());
        }
        engine.close().unwrap();
    }
}

#[test]
fn replacing_or_dropping_the_engine_revokes_its_search_scope() {
    let (mut engine, directory, pairs) = checked_review(b"prefix\n");
    drop(pairs);
    let before = engine.search_domain().unwrap();
    let path = directory.path().join("next.patch");
    fs::write(
        &path,
        b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+replacement\n",
    )
    .unwrap();
    engine.replace(Input::Patch(path)).unwrap();
    assert!(before.check().is_err());
    assert_ne!(before.review, engine.review_id());
    let deadline = Instant::now() + Duration::from_secs(5);
    while engine.status().loading {
        engine.poll().unwrap();
        assert!(Instant::now() < deadline, "replacement did not finish");
        std::thread::sleep(Duration::from_millis(5));
    }
    let after = engine.search_domain().unwrap();
    assert_eq!(after.review, engine.review_id());
    assert_eq!(after.files.len(), 1);
    assert_eq!(
        after.files[0].order.file.bytes(1).unwrap(),
        b"replacement\n"
    );
    after.check().unwrap();
    drop(engine);
    assert!(after.check().is_err());
    assert!(before.check().is_err());
}
