use super::*;
use crate::{reading::SearchDirection, view::LineId};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

fn draw(engine: &mut Engine, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    let mut outcome = Ok(());
    terminal
        .draw(|frame| outcome = engine.draw(frame, frame.area()))
        .unwrap();
    outcome.unwrap();
    engine.after_draw();
    terminal.backend().buffer().clone()
}

fn selected_old_text(engine: &Engine, cells: &Buffer) -> String {
    let app = engine.app.as_ref().unwrap();
    let columns = if app.split() {
        4..(cells.area.width - 1) / 2
    } else {
        6..cells.area.width
    };
    app.visible
        .iter()
        .filter(|(position, _)| position.row == app.cursor.row)
        .map(|(_, y)| {
            columns
                .clone()
                .map(|x| cells[(x, *y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

#[test]
fn exact_search_destinations_survive_reflow_without_retaining_source_owners() {
    let mut old = vec![b' '; 4096];
    old.extend_from_slice(b"\xff\t\x1b");
    old.extend_from_slice("日本語 e\u{301} needle OLD".as_bytes());
    let offset = old.len() - "needle OLD".len();
    let mut patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-".to_vec();
    patch.extend_from_slice(&old);
    patch.extend_from_slice(b"\n+needle new\n");
    let (mut engine, _directory) = super::search::review(&patch);
    engine
        .app
        .as_mut()
        .unwrap()
        .set_sidebar_preference(SidebarPreference::Hide);
    draw(&mut engine, 160, 30);
    let owner = Arc::downgrade(&engine.app.as_ref().unwrap().doc.files[0]);
    let read = engine.search_read("needle".into()).unwrap();
    let hit = read
        .find(None, SearchDirection::Forward, &|| Ok(()))
        .unwrap()
        .unwrap();
    assert_eq!(hit.source_bytes(), offset as u64..offset as u64 + 6);
    assert_eq!(
        hit.display,
        4096 + "\\xff\t\\u{1b}日本語 e\u{301} ".len()
            ..4096 + "\\xff\t\\u{1b}日本語 e\u{301} needle".len()
    );
    assert_eq!(hit.location().side, Side::Old);
    assert_eq!(hit.location().old_line, Some(1));
    engine.show_search_match(&hit).unwrap();
    for (width, command) in [
        (160, None),
        (80, Some(Command::ToggleWrap)),
        (110, Some(Command::ToggleLayout)),
        (150, None),
        (160, Some(Command::ToggleWrap)),
    ] {
        if let Some(command) = command {
            engine.execute(command).unwrap();
        }
        let cells = draw(&mut engine, width, 30);
        assert!(
            selected_old_text(&engine, &cells).contains("needle"),
            "selected old occurrence not visible at width {width}"
        );
        assert_eq!(engine.selected_source().unwrap(), Some(hit.location()));
    }
    let next = read
        .find(Some(&hit), SearchDirection::Forward, &|| Ok(()))
        .unwrap()
        .unwrap();
    assert_eq!(next.location().side, Side::New);
    assert_eq!(next.source_bytes(), 0..6);
    engine.show_search_match(&next).unwrap();
    draw(&mut engine, 100, 30);
    assert_eq!(engine.selected_source().unwrap(), Some(next.location()));
    let before = engine.selected_source().unwrap();
    let other = engine.search_read("absent".into()).unwrap();
    assert!(
        other
            .find(Some(&next), SearchDirection::Forward, &|| Ok(()))
            .is_err()
    );
    assert!(
        other
            .find(None, SearchDirection::Forward, &|| Ok(()))
            .unwrap()
            .is_none()
    );
    assert_eq!(engine.selected_source().unwrap(), before);
    drop(other);
    drop(read);
    engine.close().unwrap();
    assert!(
        owner.upgrade().is_none(),
        "matches must not retain source files"
    );
    assert!(engine.show_search_match(&hit).is_err());
}

#[test]
fn search_starts_at_the_current_source_or_after_a_structural_or_padding_row() {
    let (mut engine, _directory) = super::search::review(b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1,3 @@\n-old needle\n+new one needle\n+new two needle\n+new three needle\ndiff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-before needle\n+after needle\n");
    engine
        .app
        .as_mut()
        .unwrap()
        .set_sidebar_preference(SidebarPreference::Hide);
    engine
        .app
        .as_mut()
        .unwrap()
        .reflow(Layout::Split, 160, 30)
        .unwrap();
    draw(&mut engine, 160, 30);
    let first = engine
        .search_read("needle".into())
        .unwrap()
        .find(None, SearchDirection::Forward, &|| Ok(()))
        .unwrap()
        .unwrap();
    assert_eq!(first.hit.source, LineId::Patch(0));
    engine.show_search_match(&first).unwrap();
    engine.execute(Command::Down).unwrap();
    assert!(
        engine.selected_source().unwrap().is_none(),
        "short old side must be padding: cursor={:?}, row={:?}, old={}, split={}",
        engine.app.as_ref().unwrap().cursor,
        engine
            .app
            .as_ref()
            .unwrap()
            .row(engine.app.as_ref().unwrap().cursor.row),
        engine.app.as_ref().unwrap().old_side,
        engine.app.as_ref().unwrap().split(),
    );
    let after_padding = engine
        .search_read("needle".into())
        .unwrap()
        .find(None, SearchDirection::Forward, &|| Ok(()))
        .unwrap()
        .unwrap();
    assert_eq!(after_padding.hit.source, LineId::Patch(3));
    engine.execute(Command::NextFile).unwrap();
    let second_file = engine
        .search_read("needle".into())
        .unwrap()
        .find(None, SearchDirection::Forward, &|| Ok(()))
        .unwrap()
        .unwrap();
    assert_eq!(second_file.location().file, 1);
    assert_eq!(second_file.location().side, Side::Old);
    engine.show_search_match(&second_file).unwrap();
    let current = engine
        .search_read("needle".into())
        .unwrap()
        .find(None, SearchDirection::Forward, &|| Ok(()))
        .unwrap()
        .unwrap();
    assert_eq!(current.hit.source, second_file.hit.source);
    assert!(!current.wrapped());
    engine.close().unwrap();
}

#[test]
fn search_matches_are_revoked_by_context_changes_and_review_replacement() {
    let (mut engine, directory, pairs) = checked_review(b"prefix needle\n");
    install(&mut engine, &pairs);
    {
        let app = engine.app.as_mut().unwrap();
        app.view.expand(&app.doc, 0, 0).unwrap();
    }
    let read = engine.search_read("needle".into()).unwrap();
    let hit = read
        .find(None, SearchDirection::Forward, &|| Ok(()))
        .unwrap()
        .unwrap();
    assert_eq!(hit.location().new_line, Some(1));
    engine.show_search_match(&hit).unwrap();
    let before = engine.selected_source().unwrap();
    {
        let app = engine.app.as_mut().unwrap();
        app.view.expand(&app.doc, 1, 0).unwrap();
    }
    assert!(engine.show_search_match(&hit).is_err());
    assert_eq!(engine.selected_source().unwrap(), before);
    engine
        .replace(Input::Patch(directory.path().join("review.patch")))
        .unwrap();
    assert!(engine.show_search_match(&hit).is_err());
    engine.close().unwrap();
}
