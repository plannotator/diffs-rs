//! The end of the diff: no reflow and no wrapping change leaves empty screen rows under the
//! last row while the diff is longer than the pane, the cursor stays on screen, and turning
//! wrapping off and straight back on returns to the view it left.

use super::rows::draw;
use super::*;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

fn line(tag: &str) -> String {
    format!("{tag}:{}", "abcdefghij".repeat(24))
}

/// `a.txt`: seventy long context lines with removed and added lines among them, the last
/// without a newline. `b.txt`: one hunk whose every change is whitespace only, which hiding
/// whitespace draws as its heading alone, so the diff gets shorter at its end.
fn patch() -> String {
    let mut patch = String::from("diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n");
    patch.push_str("@@ -1,73 +1,73 @@\n");
    for n in 1..=70 {
        if n == 40 {
            for r in 1..=3 {
                patch.push_str(&format!("-{}\n", line(&format!("r{r}"))));
                patch.push_str(&format!("+{}\n", line(&format!("a{r}"))));
            }
        }
        patch.push_str(&format!(" {}\n", line(&format!("c{n:02}"))));
    }
    patch.push_str("\\ No newline at end of file\n");
    patch.push_str("diff --git a/b.txt b/b.txt\n--- a/b.txt\n+++ b/b.txt\n@@ -1,7 +1,7 @@\n");
    patch.push_str(" first\n");
    for n in 1..=5 {
        patch.push_str(&format!("-    spaced {n}\n"));
    }
    for n in 1..=5 {
        patch.push_str(&format!("+spaced {n}\n"));
    }
    patch.push_str(" last\n");
    patch
}

fn open(layout: Layout, wrap: bool) -> (Engine, tempfile::TempDir) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("review.patch");
    fs::write(&path, patch()).unwrap();
    let (wake, ready) = mpsc::sync_channel(1);
    let options = Options {
        layout,
        wrap,
        chrome: false,
        sidebar: SidebarPreference::Hide,
        ..Options::default()
    };
    let mut engine = Engine::open(Input::Patch(path), options, wake).unwrap();
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

fn app(engine: &Engine) -> &App {
    engine.app.as_ref().unwrap()
}

/// Draws at `size` and checks the bottom screen row shows a row (the diff is far longer than
/// any pane here) and the cursor is drawn.
fn assert_full(engine: &mut Engine, size: (u16, u16), what: &str) {
    draw(engine, size);
    let bottom = size.1 - 1;
    assert!(
        engine.row_at(1, bottom).is_some(),
        "{what}: the bottom row is empty, top {:?}",
        app(engine).top
    );
    let app = app(engine);
    assert!(
        app.visible.iter().any(|(p, _)| *p == app.cursor),
        "{what}: the cursor {:?} is off screen",
        app.cursor
    );
}

fn click_no_newline(engine: &mut Engine, size: (u16, u16)) {
    let y = (0..size.1)
        .find(|&y| {
            engine
                .row_at(1, y)
                .is_some_and(|row| row.kind == DiffRowKind::NoNewline)
        })
        .expect("the mark is on screen");
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: size.0 - 2,
        row: y,
        modifiers: KeyModifiers::NONE,
    };
    engine.handle_event(Event::Mouse(event), true).unwrap();
    assert_eq!(
        engine.cursor_row().unwrap().kind,
        DiffRowKind::NoNewline,
        "on the mark"
    );
}

type Reflow = (&'static str, fn(&mut Engine, &mut (u16, u16)));

const REFLOWS: [Reflow; 8] = [
    ("wrapping off", |engine, _| {
        if engine.options().wrap {
            engine.execute(Command::ToggleWrap).unwrap();
        }
    }),
    ("a taller pane", |_, size| size.1 = 45),
    ("a wider pane", |_, size| size.0 = 260),
    ("unified", |engine, _| {
        engine.set_layout(Layout::Unified).unwrap()
    }),
    ("whitespace hidden", |engine, _| {
        engine.set_whitespace(Whitespace::IgnoreAll).unwrap()
    }),
    ("tab width", |engine, _| {
        engine.set_tab_width(TabWidth::new(16)).unwrap()
    }),
    ("line numbers", |engine, _| {
        engine.execute(Command::ToggleNumbers).unwrap();
    }),
    ("split ratio", |engine, _| {
        engine
            .set_split_ratio(SplitRatio::from_permille(300))
            .unwrap()
    }),
];

#[test]
fn no_reflow_at_the_end_leaves_empty_rows_under_the_last_one() {
    for layout in [Layout::Split, Layout::Unified] {
        for wrap in [true, false] {
            for on_mark in [false, true] {
                for (name, reflow) in REFLOWS {
                    let what = format!("{layout:?} wrap {wrap} on the mark {on_mark}: {name}");
                    let (mut engine, _directory) = open(layout, wrap);
                    let mut size = (190, 30);
                    draw(&mut engine, size);
                    engine.execute(Command::LastSource).unwrap();
                    assert_full(&mut engine, size, &format!("{what}, at the end"));
                    if on_mark {
                        click_no_newline(&mut engine, size);
                    }
                    reflow(&mut engine, &mut size);
                    assert_full(&mut engine, size, &what);
                    if on_mark {
                        assert_eq!(
                            engine.cursor_row().unwrap().kind,
                            DiffRowKind::NoNewline,
                            "{what}: still on the mark"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn wrapping_off_and_straight_back_on_returns_to_the_view_it_left() {
    for layout in [Layout::Split, Layout::Unified] {
        for at_end in [false, true] {
            let what = format!("{layout:?} at the end {at_end}");
            let (mut engine, _directory) = open(layout, true);
            let size = (190, 30);
            draw(&mut engine, size);
            if at_end {
                engine.execute(Command::LastSource).unwrap();
            } else {
                for _ in 0..30 {
                    engine.execute(Command::Down).unwrap();
                }
            }
            for _ in 0..3 {
                engine.execute(Command::Up).unwrap();
            }
            draw(&mut engine, size);
            let before = (app(&engine).top, app(&engine).cursor);
            engine.execute(Command::ToggleWrap).unwrap();
            draw(&mut engine, size);
            engine.execute(Command::ToggleWrap).unwrap();
            draw(&mut engine, size);
            assert_eq!((app(&engine).top, app(&engine).cursor), before, "{what}");
            // Anything in between, here a move, and wrapping on keeps the cursor's line on its
            // screen row as before.
            engine.execute(Command::ToggleWrap).unwrap();
            engine.execute(Command::Up).unwrap();
            draw(&mut engine, size);
            engine.execute(Command::ToggleWrap).unwrap();
            assert_full(&mut engine, size, &format!("{what}, moved in between"));
        }
    }
}
