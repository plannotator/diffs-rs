use std::{
    fs,
    io::Read,
    sync::{Arc, mpsc},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use diffs_rs::{
    app::{App, Layout, Position, SidebarPreference},
    input::{Cancellation, LocalInput, Review, SourceState},
    model::RowKind,
    render,
    source::CheckedPair,
    storage,
    theme::Appearance,
    view::LineId,
};
use ratatui::{Terminal, backend::TestBackend};

#[path = "support/readiness.rs"]
mod readiness;

#[test]
fn source_readiness_keeps_the_exact_endpoint_visible_in_each_reply_order() {
    let dir = readiness::repository(true);
    let cancel = Cancellation::default();
    let mut review = Review::open(
        LocalInput::Git {
            repo: dir.path().to_owned(),
            base: "HEAD^".into(),
            head: "HEAD".into(),
        },
        &cancel,
        |_| Ok(()),
    )
    .unwrap();
    let checked: Vec<_> = (0..12)
        .map(|file| review.prepare(file, &cancel).unwrap())
        .collect();
    for layout in [Layout::Split, Layout::Unified] {
        for wrap in [false, true] {
            for (order, draw_each) in [
                ([11, 10, 9], true),
                ([9, 10, 11], true),
                ([11, 10, 9], false),
                ([9, 10, 11], false),
            ] {
                let (wake, _) = mpsc::sync_channel(1);
                let mut app = App::new(
                    storage::Document {
                        files: review.doc().files.clone(),
                    },
                    "readiness".into(),
                    layout,
                    Appearance::Dark,
                    wake,
                )
                .unwrap();
                app.loading = false;
                app.sources = Some(vec![SourceState::Pending; 12]);
                app.set_sidebar_preference(SidebarPreference::Hide);
                app.wrap = wrap;
                draw(&mut app);
                app.jump(true).unwrap();
                draw(&mut app);
                if wrap {
                    app.move_cursor(2).unwrap();
                }
                app.old_side = true;
                draw(&mut app);
                let original = app.row(app.cursor.row).unwrap().kind;
                let RowKind::Code {
                    left: Some(LineId::Patch(id)),
                    ..
                } = original
                else {
                    panic!("missing endpoint");
                };
                assert_eq!(app.doc.info(id).old, Some(763));
                let part = app.cursor.part;
                let byte = render::source_anchor_byte(&app, id.into(), part, true, None).unwrap();
                let screen_row = app
                    .visible
                    .iter()
                    .find(|(p, _)| *p == app.cursor)
                    .unwrap()
                    .1;
                let assert_anchor = |app: &App| {
                    assert_eq!(app.row(app.cursor.row).unwrap().kind, original);
                    assert_eq!(app.cursor.part, part);
                    assert_eq!(
                        render::source_anchor_byte(app, id.into(), part, true, None).unwrap(),
                        byte
                    );
                    assert!(app.old_side);
                    assert_eq!(
                        app.visible
                            .iter()
                            .find(|(p, _)| *p == app.cursor)
                            .map(|(_, y)| *y),
                        Some(screen_row),
                        "layout={layout:?} wrap={wrap} order={order:?} draw_each={draw_each}"
                    );
                };
                for file in order {
                    app.set_source(file, checked[file].clone()).unwrap();
                    if draw_each {
                        draw(&mut app);
                        assert_anchor(&app);
                    }
                }
                draw(&mut app);
                assert_anchor(&app);
            }
        }
    }
}

fn comparison(old: &[u8], new: &[u8]) -> Arc<CheckedPair> {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("old.ts");
    let b = dir.path().join("new.ts");
    fs::write(&a, old).unwrap();
    fs::write(&b, new).unwrap();
    let mut review = Review::open(
        LocalInput::Files { old: a, new: b },
        &Cancellation::default(),
        |_| Ok(()),
    )
    .unwrap();
    let SourceState::Checked(pair) = review.prepare(0, &Cancellation::default()).unwrap() else {
        panic!("missing checked source")
    };
    pair
}

fn app(pair: &Arc<CheckedPair>, layout: Layout) -> App {
    let (wake, _) = mpsc::sync_channel(1);
    let mut app = App::new(
        storage::Document {
            files: vec![pair.file().clone()],
        },
        "context".into(),
        layout,
        Appearance::Dark,
        wake,
    )
    .unwrap();
    app.loading = false;
    app.set_sidebar_preference(SidebarPreference::Hide);
    app.sources = Some(vec![SourceState::Pending]);
    draw(&mut app);
    app.set_source(0, SourceState::Checked(pair.clone()))
        .unwrap();
    draw(&mut app);
    app
}

fn draw(app: &mut App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(160, 50)).unwrap();
    terminal
        .draw(|frame| render::draw(frame, app, None).unwrap())
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

fn enter(app: &mut App, row: usize) {
    app.cursor = Position { row, part: 0 };
    app.top = app.cursor;
    draw(app);
    app.handle_event(Event::Key(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::NONE,
    )))
    .unwrap();
}

fn sources(app: &App, pair: &CheckedPair, old_side: bool) -> Vec<(u32, Vec<u8>)> {
    (0..app.row_len())
        .filter_map(|index| {
            let RowKind::Code { left, right } = app.row(index)?.kind else {
                return None;
            };
            let id = if old_side { left } else { right }?;
            let (number, bytes) = match id {
                LineId::Patch(id) => {
                    let info = pair.file().info(id);
                    (
                        (if old_side { info.old } else { info.new })?,
                        pair.file().bytes(id).unwrap(),
                    )
                }
                LineId::Context { old, new, .. } => {
                    let number = if old_side { old.get() } else { new.get() };
                    let source = if old_side {
                        pair.old()
                    } else {
                        pair.new_source()
                    }
                    .unwrap();
                    let mut bytes = Vec::new();
                    source
                        .range(number - 1..number)
                        .unwrap()
                        .read_to_end(&mut bytes)
                        .unwrap();
                    (number, bytes)
                }
            };
            Some((number, bytes))
        })
        .collect()
}

#[test]
fn expansion_exhausts_exact_sources_without_duplicates_or_changed_patch_records() {
    for newline in [false, true] {
        let lines: Vec<_> = (1..=163)
            .map(|n| format!("return {n}; // row {n} 日本語\t café"))
            .collect();
        let mut changed = lines.clone();
        changed.splice(
            43..44,
            [
                "return 430;".into(),
                "return 431;".into(),
                "return 432;".into(),
            ],
        );
        changed[106] = "return 1060;".into();
        let old = lines.join("\n") + if newline { "\n" } else { "" };
        let new = changed.join("\n") + if newline { "\n" } else { "" };
        let pair = comparison(old.as_bytes(), new.as_bytes());
        assert_eq!(pair.file().hunks.len(), 2);
        let mut canonical = Vec::new();
        pair.file().write_canonical(&mut canonical).unwrap();
        for layout in [Layout::Split, Layout::Unified] {
            let mut app = app(&pair, layout);
            let mut actions = 0;
            loop {
                for (side, expected) in [(true, &old), (false, &new)] {
                    let actual = sources(&app, &pair, side);
                    assert!(actual.windows(2).all(|pair| pair[0].0 < pair[1].0));
                    let expected: Vec<_> = expected.split_inclusive('\n').collect();
                    for (number, bytes) in actual {
                        assert_eq!(bytes, expected[number as usize - 1].as_bytes());
                    }
                }
                let Some(gap) =
                    (0..app.row_len()).find(|index| app.row(*index).unwrap().kind == RowKind::Gap)
                else {
                    break;
                };
                let before = sources(&app, &pair, true).len();
                enter(&mut app, gap);
                let after = sources(&app, &pair, true).len();
                assert!((1..=20).contains(&(after - before)));
                assert!(matches!(
                    app.row(app.cursor.row).unwrap().kind,
                    RowKind::Code {
                        left: Some(LineId::Context { .. }),
                        ..
                    }
                ));
                actions += 1;
                assert!(actions < 12);
            }
            assert_eq!(actions, 8);
            for (side, expected) in [(true, &old), (false, &new)] {
                let actual = sources(&app, &pair, side);
                assert_eq!(
                    actual.iter().map(|(_, bytes)| bytes.len()).sum::<usize>(),
                    expected.len()
                );
                assert_eq!(
                    actual
                        .into_iter()
                        .flat_map(|(_, bytes)| bytes)
                        .collect::<Vec<_>>(),
                    expected.as_bytes()
                );
            }
            let markers = (0..app.row_len())
                .filter(|index| matches!(app.row(*index).unwrap().kind, RowKind::NoNewline { .. }))
                .count();
            assert_eq!(markers, usize::from(!newline));
            app.jump(false).unwrap();
            assert!(matches!(
                app.row(app.cursor.row).unwrap().kind,
                RowKind::Code {
                    left: Some(LineId::Context { old, new, .. }),
                    ..
                } if old.get() == 1 && new.get() == 1
            ));
            app.jump(true).unwrap();
            assert!(matches!(
                app.row(app.cursor.row).unwrap().kind,
                RowKind::Code {
                    right: Some(LineId::Context { old, new, .. }),
                    ..
                } if old.get() == 163 && new.get() == 165
            ));
            app.jump(false).unwrap();
            for expected in 0..2 {
                app.jump_section(true, false);
                assert_eq!(
                    app.row(app.cursor.row).unwrap().kind,
                    RowKind::Hunk(expected)
                );
            }
            app.jump_section(false, false);
            assert_eq!(app.row(app.cursor.row).unwrap().kind, RowKind::Hunk(0));
            let mut after = Vec::new();
            pair.file().write_canonical(&mut after).unwrap();
            assert_eq!(canonical, after);
        }
    }
}

#[test]
fn source_readiness_and_failure_preserve_patch_identity_and_reject_stale_context() {
    let old = (1..=110)
        .map(|n| format!("return {n};\n"))
        .collect::<String>();
    let new = old.replace("return 45;", "return 450;");
    let pair = comparison(old.as_bytes(), new.as_bytes());
    let mut app = app(&pair, Layout::Split);
    app.jump(false).unwrap();
    let original = app.row(app.cursor.row).unwrap().kind;
    app.set_source(0, SourceState::Pending).unwrap();
    assert_eq!(app.row(app.cursor.row).unwrap().kind, original);
    let before = app.row_len();
    let gap = (0..before)
        .find(|index| app.row(*index).unwrap().kind == RowKind::Gap)
        .unwrap();
    enter(&mut app, gap);
    assert_eq!(app.row_len(), before);
    app.jump(false).unwrap();
    app.set_source(0, SourceState::Checked(pair.clone()))
        .unwrap();
    assert_eq!(app.row(app.cursor.row).unwrap().kind, original);
    let gap = (0..app.row_len())
        .find(|index| app.row(*index).unwrap().kind == RowKind::Gap)
        .unwrap();
    enter(&mut app, gap);
    draw(&mut app);
    app.request_highlights();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let stale = loop {
        if let Ok(reply) = app.worker.try_reply() {
            let reply = reply.unwrap();
            if matches!(reply.id, LineId::Context { .. }) {
                break reply;
            }
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    app.set_source(0, SourceState::Unavailable("source read failed".into()))
        .unwrap();
    assert!(!app.accept_highlight(stale).unwrap());
    assert_eq!(app.row_len(), pair.file().rows(true).len());
    let text = draw(&mut app);
    assert!(text.contains("Source unavailable") && text.contains("patch view retained"));
    app.jump(false).unwrap();
    assert_eq!(app.row(app.cursor.row).unwrap().kind, original);
}

#[test]
fn many_file_readiness_expansion_and_later_publication_keep_original_destinations() {
    use diffs_rs::source::{SourceStore, check_pair};
    let old = (1..=61)
        .map(|n| format!("return {n};\n"))
        .collect::<String>();
    let new = old.replace("return 31;", "return 310;");
    let mut patch = String::new();
    for file in 0..256 {
        patch.push_str(&format!("diff --git a/dir/{file:04}.ts b/dir/{file:04}.ts\n--- a/dir/{file:04}.ts\n+++ b/dir/{file:04}.ts\n@@ -28,7 +28,7 @@\n return 28;\n return 29;\n return 30;\n-return 31;\n+return 310;\n return 32;\n return 33;\n return 34;\n"));
    }
    let mut doc = storage::Document::default();
    let spool = storage::read(patch.as_bytes(), |file| doc.append(file)).unwrap();
    let mut store = SourceStore::sharing(&spool).unwrap();
    let a = store.capture(old.as_bytes(), &|| Ok(())).unwrap();
    let b = store.capture(new.as_bytes(), &|| Ok(())).unwrap();
    let pairs: Vec<_> = doc
        .files
        .iter()
        .map(|file| {
            Arc::new(
                check_pair(file.clone(), Some(a.clone()), Some(b.clone()), &|| Ok(())).unwrap(),
            )
        })
        .collect();
    let later = doc.files.split_off(128);
    let (wake, _) = mpsc::sync_channel(1);
    let mut app = App::new(
        doc,
        "many files".into(),
        Layout::Split,
        Appearance::Dark,
        wake,
    )
    .unwrap();
    app.sources = Some(vec![SourceState::Pending; 128]);
    draw(&mut app);
    app.jump(true).unwrap();
    let original = app.row(app.cursor.row).unwrap();
    for file in (0..128).rev() {
        app.set_source(file, SourceState::Checked(pairs[file].clone()))
            .unwrap();
    }
    assert_eq!(app.row(app.cursor.row).unwrap().file, original.file);
    assert_eq!(app.row(app.cursor.row).unwrap().kind, original.kind);
    for file in later {
        app.doc.append(file).unwrap();
        app.sources.as_mut().unwrap().push(SourceState::Pending);
    }
    for file in (128..256).rev() {
        app.set_source(file, SourceState::Checked(pairs[file].clone()))
            .unwrap();
    }
    for file in [255, 0, 128, 1, 254] {
        let gap = (0..app.row_len())
            .find(|&row| {
                app.row(row)
                    .is_some_and(|row| row.file == file && row.kind == RowKind::Gap)
            })
            .unwrap();
        enter(&mut app, gap);
        assert_eq!(
            app.row(app.cursor.row).unwrap().kind,
            RowKind::Code {
                left: LineId::context(file, 8, 8),
                right: LineId::context(file, 8, 8),
            }
        );
    }
    for layout in [Layout::Unified, Layout::Split] {
        app.reflow(layout, 160, 50).unwrap();
        for file in 0..256 {
            let header = (0..app.row_len())
                .find(|&row| {
                    app.row(row)
                        .is_some_and(|row| row.file == file && row.kind == RowKind::Header)
                })
                .unwrap();
            app.cursor = Position {
                row: header,
                part: 0,
            };
            app.jump_section(true, false);
            assert_eq!(
                (
                    app.row(app.cursor.row).unwrap().file,
                    app.row(app.cursor.row).unwrap().kind
                ),
                (file, RowKind::Hunk(0))
            );
            for id in app.doc.files[file].source_base..app.doc.files[file].source_end() {
                let positions: Vec<_> = (header..app.row_len()).map_while(|row| app.row(row).filter(|row| row.file == file)).filter(|row| matches!(row.kind, RowKind::Code { left, right } if left == Some(LineId::Patch(id)) || right == Some(LineId::Patch(id)))).collect();
                assert_eq!(positions.len(), 1);
            }
            app.jump_section(true, true);
            if file < 255 {
                assert_eq!(app.row(app.cursor.row).unwrap().file, file + 1);
            }
        }
    }
}

#[test]
fn footer_source_status_describes_the_cursor_file() {
    let mut patch = String::new();
    for name in ["first", "second"] {
        patch.push_str(&format!(
            "diff --git a/{name}.rs b/{name}.rs\n--- a/{name}.rs\n+++ b/{name}.rs\n@@ -1 +1 @@\n-old {name}\n+new {name}\n"
        ));
    }
    let mut doc = storage::Document::default();
    storage::read(patch.as_bytes(), |file| doc.append(file)).unwrap();
    let (wake, _) = mpsc::sync_channel(1);
    let mut app = App::new(doc, "status".into(), Layout::Split, Appearance::Dark, wake).unwrap();
    app.loading = false;
    app.set_sidebar_preference(SidebarPreference::Hide);
    app.sources = Some(vec![
        SourceState::Metadata,
        SourceState::Unavailable("second source missing".into()),
    ]);
    let footer = |text: &str| text.chars().skip(160 * 49).collect::<String>();

    // Both files are visible; the cursor's file decides the status.
    let text = draw(&mut app);
    assert!(text.contains("new first") && text.contains("new second"));
    assert_eq!(app.row(app.cursor.row).unwrap().file, 0);
    let status = footer(&text);
    assert!(status.contains("Metadata only"), "{status}");
    assert!(!status.contains("Source unavailable"), "{status}");

    app.jump_section(true, true);
    let text = draw(&mut app);
    assert_eq!(app.row(app.cursor.row).unwrap().file, 1);
    assert!(
        footer(&text).contains("Source unavailable: second source missing"),
        "{}",
        footer(&text)
    );

    app.jump_section(false, true);
    let text = draw(&mut app);
    assert!(footer(&text).contains("Metadata only"), "{}", footer(&text));
}
