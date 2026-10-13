use super::*;
use ratatui::{Terminal, backend::TestBackend};

fn draw(engine: &mut Engine, width: u16) {
    let mut terminal = Terminal::new(TestBackend::new(width, 30)).unwrap();
    terminal
        .draw(|frame| engine.draw(frame, frame.area()).unwrap())
        .unwrap();
    engine.after_draw();
}

fn selected(engine: &Engine, side: Side, first: u32, last: u32) {
    assert_eq!(
        engine.selection(),
        Some(SourceRange {
            review: engine.review_id(),
            file: 0,
            side,
            first,
            last,
        })
    );
}

#[test]
fn line_selection_uses_original_side_across_unified_replacements_and_reflow() {
    let mut patch = "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1,4 +1,5 @@\n shared\t日本語\r\n-old two\r\n+new two\r\n-old three\r\n+new three\r\n+inserted\r\n tail".as_bytes().to_vec();
    patch.extend_from_slice(b"\xff\n\\ No newline at end of file\ndiff --git a/b b/b\n--- a/b\n+++ b/b\n@@ -1 +1 @@\n-other\n+next\n");
    let (mut engine, _dir) = super::search::review(&patch);
    engine
        .app
        .as_mut()
        .unwrap()
        .set_sidebar_preference(SidebarPreference::Hide);
    engine.app.as_mut().unwrap().layout = Layout::Unified;
    draw(&mut engine, 100);
    assert_eq!(
        engine.execute(Command::SelectLines).unwrap(),
        InputEffect::Ignored
    );
    engine.execute(Command::FirstSource).unwrap();
    engine.execute(Command::SwitchSide).unwrap();
    engine.execute(Command::SelectLines).unwrap();
    selected(&engine, Side::Old, 1, 1);
    for last in [2, 3] {
        engine.execute(Command::Down).unwrap();
        selected(&engine, Side::Old, 1, last);
    }
    for _ in 0..3 {
        engine.execute(Command::Down).unwrap();
        assert_eq!(engine.selected_source().unwrap().unwrap().side, Side::New);
        selected(&engine, Side::Old, 1, 3);
    }
    engine.execute(Command::Down).unwrap();
    selected(&engine, Side::Old, 1, 4);
    let mut bytes = Vec::new();
    engine
        .source_read(engine.selection().unwrap())
        .unwrap()
        .write_to(&mut bytes, &|| Ok(()))
        .unwrap();
    let mut expected = "shared\t日本語\r\nold two\r\nold three\r\ntail"
        .as_bytes()
        .to_vec();
    expected.push(0xff);
    assert_eq!(bytes, expected);
    engine.execute(Command::SelectLines).unwrap();
    assert!(!engine.selection_extending());
    for (command, width) in [
        (Command::ToggleWrap, 45),
        (Command::ToggleLayout, 160),
        (Command::ToggleTheme, 120),
        (Command::ToggleLayout, 60),
    ] {
        engine.execute(command).unwrap();
        draw(&mut engine, width);
        selected(&engine, Side::Old, 1, 4);
    }
    engine.execute(Command::Up).unwrap();
    selected(&engine, Side::Old, 1, 4);
    engine.execute(Command::NextFile).unwrap();
    assert!(engine.selection().is_none());
    assert!(engine.selected_source().unwrap().is_none());
    engine.close().unwrap();
}

#[test]
fn padding_and_unavailable_gaps_never_become_exported_source() {
    let patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1,3 @@\n-old\n+new\n+extra\n+another\n@@ -8 +10 @@\n tail\n";
    let (mut engine, _dir) = super::search::review(patch);
    draw(&mut engine, 160);
    engine.execute(Command::FirstSource).unwrap();
    engine.execute(Command::SelectLines).unwrap();
    selected(&engine, Side::Old, 1, 1);
    engine.execute(Command::Down).unwrap();
    assert!(engine.selected_source().unwrap().is_none());
    selected(&engine, Side::Old, 1, 1);
    engine.execute(Command::ClearSelection).unwrap();
    assert_eq!(
        engine.execute(Command::SelectLines).unwrap(),
        InputEffect::Ignored
    );
    engine.execute(Command::LastSource).unwrap();
    engine.execute(Command::SwitchSide).unwrap();
    engine.execute(Command::SelectLines).unwrap();
    selected(&engine, Side::Old, 8, 8);
    engine.execute(Command::FirstSource).unwrap();
    selected(&engine, Side::Old, 1, 8);
    let mut bytes = Vec::new();
    assert!(
        engine
            .source_read(engine.selection().unwrap())
            .unwrap()
            .write_to(&mut bytes, &|| Ok(()))
            .is_err()
    );
    assert!(bytes.is_empty());
    engine.execute(Command::SwitchSide).unwrap();
    assert!(engine.selection().is_none());
    engine.close().unwrap();
}

#[test]
fn selection_releases_on_source_withdrawal_or_replacement_without_owning_sources() {
    let (mut engine, dir, pairs) = checked_review(b"prefix\n");
    install(&mut engine, &pairs);
    draw(&mut engine, 160);
    engine.execute(Command::FirstSource).unwrap();
    engine.execute(Command::SwitchSide).unwrap();
    let file = &engine.app.as_ref().unwrap().doc.files[0];
    let owner = Arc::downgrade(file);
    engine.execute(Command::SelectLines).unwrap();
    selected(&engine, Side::New, 2, 2);
    engine
        .app
        .as_mut()
        .unwrap()
        .set_source(1, SourceState::Unavailable("other file withdrawn".into()))
        .unwrap();
    selected(&engine, Side::New, 2, 2);
    engine
        .app
        .as_mut()
        .unwrap()
        .set_source(
            0,
            SourceState::Unavailable("selected file withdrawn".into()),
        )
        .unwrap();
    assert!(engine.selection().is_none());
    engine.execute(Command::SelectLines).unwrap();
    let previous = engine.selection().unwrap();
    engine
        .replace(Input::Patch(dir.path().join("review.patch")))
        .unwrap();
    assert!(engine.selection().is_none());
    assert_ne!(engine.review_id(), previous.review);
    assert!(engine.source_read(previous).is_err());
    engine.close().unwrap();
    drop(pairs);
    assert!(owner.upgrade().is_none());
}
