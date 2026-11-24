//! What moves the view sideways. Only `Command::Left` and `Command::Right` (and reveals) do: a
//! reflow keeps the horizontal scroll, wherever the last click was, and turning wrapping off
//! keeps the top row. A host without horizontal scrolling must never be left showing the ends
//! of the lines.

use super::rows::{draw, text};
use super::*;
use crate::app::Position;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

const SIZE: (u16, u16) = (190, 24);
const NARROW: (u16, u16) = (130, 24);

/// The three ways a long line is shared: split evenly, split 30:70, and unified.
const LAYOUTS: [(Layout, u16); 3] = [
    (Layout::Split, 500),
    (Layout::Split, 300),
    (Layout::Unified, 500),
];

/// A line far wider than any pane here, starting with `tag` and a colon.
fn line(tag: &str) -> String {
    format!("{tag}:{}", "abcdefghij".repeat(24))
}

/// Thirty context lines, three removed-only rows, a context line, three added-only rows and
/// two more context lines, the last without a newline. No removed line pairs with an added
/// one, so both layouts have the same rows.
fn patch() -> String {
    let mut patch =
        String::from("diff --git a/long.txt b/long.txt\n--- a/long.txt\n+++ b/long.txt\n");
    patch.push_str("@@ -1,36 +1,36 @@\n");
    for n in 1..=30 {
        patch.push_str(&format!(" {}\n", line(&format!("c{n:02}"))));
    }
    for n in 1..=3 {
        patch.push_str(&format!("-{}\n", line(&format!("r{n:02}"))));
    }
    patch.push_str(&format!(" {}\n", line("c31")));
    for n in 1..=3 {
        patch.push_str(&format!("+{}\n", line(&format!("a{n:02}"))));
    }
    patch.push_str(&format!(" {}\n", line("c32")));
    patch.push_str(&format!(" {}\n\\ No newline at end of file\n", line("c33")));
    patch
}

fn open((layout, ratio): (Layout, u16), wrap: bool) -> (Engine, tempfile::TempDir) {
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
    engine
        .set_split_ratio(SplitRatio::from_permille(ratio))
        .unwrap();
    draw(&mut engine, SIZE);
    // Seven wheel steps of three rows: the top is the 20th context line, and the rows below
    // reach the end of the file.
    for _ in 0..7 {
        mouse(&mut engine, MouseEventKind::ScrollDown, 1, 5);
    }
    draw(&mut engine, SIZE);
    (engine, directory)
}

fn mouse(engine: &mut Engine, kind: MouseEventKind, column: u16, row: u16) {
    let event = MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    };
    engine.handle_event(Event::Mouse(event), true).unwrap();
}

fn click(engine: &mut Engine, column: u16, row: u16) {
    mouse(engine, MouseEventKind::Down(MouseButton::Left), column, row);
}

fn app(engine: &Engine) -> &App {
    engine.app.as_ref().unwrap()
}

/// What the top row shows, by its line rather than its index, and its wrapped part.
fn top(engine: &Engine) -> (String, usize) {
    let app = app(engine);
    let row = app.row(app.top.row).unwrap();
    let shown = match row.kind {
        RowKind::Code { left, right } => format!("{:?}", right.or(left)),
        kind => format!("{kind:?} at {}", app.top.row),
    };
    (shown, app.top.part)
}

/// The screen row, below the sticky heading, of the first row from `from` down that `wanted`
/// accepts.
fn screen_row(engine: &Engine, from: u16, wanted: impl Fn(&DiffRow) -> bool) -> u16 {
    (from..SIZE.1)
        .find(|&y| engine.row_at(1, y).is_some_and(|row| wanted(&row)))
        .expect("the row is on screen")
}

fn new_line(line: u32) -> impl Fn(&DiffRow) -> bool {
    move |row| row.kind == DiffRowKind::Code && row.new_line == Some(line) && row.part == 0
}

fn old_line(line: u32) -> impl Fn(&DiffRow) -> bool {
    move |row| row.kind == DiffRowKind::Code && row.old_line == Some(line) && row.new_line.is_none()
}

/// The column just inside the right edge of the old side, and of the new side or the
/// unified row.
fn far_right(engine: &Engine, size: (u16, u16)) -> (u16, u16) {
    let new = size.0 - 2;
    (engine.split_line().map_or(new, |line| line - 2), new)
}

/// Draws at `size` and checks that nothing scrolled sideways: the horizontal scroll is 0 and
/// every line drawn starts at its first character, its tag and a colon. The top row is `top`.
fn assert_unscrolled(engine: &mut Engine, size: (u16, u16), top_row: &(String, usize), what: &str) {
    let buffer = draw(engine, size);
    assert_eq!(app(engine).horizontal, 0, "{what}: the horizontal scroll");
    assert_eq!(&top(engine), top_row, "{what}: the top row");
    let split = engine.split_line().is_some();
    let mut lines = 0;
    for y in 1..size.1 {
        let Some(row) = engine.row_at(1, y) else {
            continue;
        };
        if row.kind != DiffRowKind::Code || row.part != 0 {
            continue;
        }
        let expected = if split {
            usize::from(row.old_line.is_some()) + usize::from(row.new_line.is_some())
        } else {
            1
        };
        let shown = text(&buffer, y);
        assert_eq!(
            shown.matches(":abcdefghij").count(),
            expected,
            "{what}: row {y} starts each line at its first character: {shown:?}"
        );
        lines += 1;
    }
    assert!(lines > 10, "{what}: lines drawn");
}

type Step = (&'static str, fn(&mut Engine, &mut (u16, u16)));

/// Everything that reflows the diff without moving the cursor, as a host does it.
const STEPS: [Step; 9] = [
    ("a narrower pane", |_, size| *size = NARROW),
    ("tab width", |engine, _| {
        engine.set_tab_width(TabWidth::new(16)).unwrap()
    }),
    ("whitespace hidden", |engine, _| {
        engine.set_whitespace(Whitespace::IgnoreAll).unwrap()
    }),
    ("split ratio", |engine, _| {
        let other = if engine.split_ratio() == SplitRatio::EVEN {
            300
        } else {
            500
        };
        engine
            .set_split_ratio(SplitRatio::from_permille(other))
            .unwrap()
    }),
    ("layout", |engine, _| {
        let other = if engine.options().layout == Layout::Unified {
            Layout::Split
        } else {
            Layout::Unified
        };
        engine.set_layout(other).unwrap()
    }),
    ("line numbers", |engine, _| {
        engine.execute(Command::ToggleNumbers).unwrap();
    }),
    ("layout back", |engine, _| {
        engine.execute(Command::ToggleLayout).unwrap();
    }),
    ("a pane wide enough for every line", |_, size| {
        *size = (1000, 24)
    }),
    ("narrow again", |_, size| *size = (121, 24)),
];

/// Each step alone after `prepare`, then all of them in turn, keep the view unscrolled.
fn check_steps(
    layout: (Layout, u16),
    wrap: bool,
    what: &str,
    prepare: &dyn Fn(&mut Engine) -> (u16, u16),
) {
    for (name, step) in STEPS {
        let (mut engine, _directory) = open(layout, wrap);
        let mut size = prepare(&mut engine);
        let before = top(&engine);
        assert_unscrolled(&mut engine, size, &before, &format!("{what}, clicked"));
        step(&mut engine, &mut size);
        assert_unscrolled(&mut engine, size, &before, &format!("{what}, then {name}"));
    }
    let (mut engine, _directory) = open(layout, wrap);
    let mut size = prepare(&mut engine);
    let before = top(&engine);
    for (name, step) in STEPS {
        step(&mut engine, &mut size);
        assert_unscrolled(
            &mut engine,
            size,
            &before,
            &format!("{what}, in turn {name}"),
        );
    }
}

#[test]
fn a_click_anywhere_never_makes_a_reflow_scroll_sideways() {
    // New lines: c01-c30 1-30, c31 31, a01-a03 32-34, c32 35, c33 36. Old lines: c01-c30
    // 1-30, r01-r03 31-33, c31 34.
    type Click = (&'static str, fn(&mut Engine));
    let clicks: [Click; 7] = [
        ("far right on a context line", |engine| {
            let y = screen_row(engine, 1, new_line(25));
            let x = far_right(engine, SIZE).1;
            click(engine, x, y);
        }),
        (
            "far right in the empty old half of an added row",
            |engine| {
                let y = screen_row(engine, 1, new_line(33));
                let x = far_right(engine, SIZE).0;
                click(engine, x, y);
            },
        ),
        (
            "far right in the empty new half of a removed row",
            |engine| {
                let y = screen_row(engine, 1, old_line(32));
                let x = far_right(engine, SIZE).1;
                click(engine, x, y);
            },
        ),
        (
            "far right on the old side of the no-newline row",
            |engine| {
                let y = screen_row(engine, 1, |row| row.kind == DiffRowKind::NoNewline);
                let x = far_right(engine, SIZE).0;
                click(engine, x, y);
            },
        ),
        (
            "far right on the new side of the no-newline row",
            |engine| {
                let y = screen_row(engine, 1, |row| row.kind == DiffRowKind::NoNewline);
                let x = far_right(engine, SIZE).1;
                click(engine, x, y);
            },
        ),
        (
            "an old-side selection dragged onto added rows, then cleared",
            |engine| {
                let (old, _) = far_right(engine, SIZE);
                let y = screen_row(engine, 1, new_line(31));
                click(engine, old, y);
                engine.execute(Command::SelectLines).unwrap();
                for line in [32, 33, 34] {
                    let y = screen_row(engine, 1, new_line(line));
                    click(engine, old, y);
                }
                engine.execute(Command::ClearSelection).unwrap();
            },
        ),
        ("far right on the old side of a removed row", |engine| {
            let y = screen_row(engine, 1, old_line(31));
            let x = far_right(engine, SIZE).0;
            click(engine, x, y);
        }),
    ];
    for layout in LAYOUTS {
        for (what, act) in clicks {
            let what = format!("{layout:?} {what}");
            check_steps(layout, false, &what, &|engine| {
                act(engine);
                SIZE
            });
        }
    }
}

/// Turns wrapping off with the cursor on a wrapped part that `prepare` put it on, and checks
/// that the top row and the cursor's row stay, every line starts at its first character, and
/// nothing after that scrolls sideways either.
fn check_unwrap(layout: (Layout, u16), what: &str, prepare: fn(&mut Engine)) {
    let (mut engine, _directory) = open(layout, true);
    prepare(&mut engine);
    draw(&mut engine, SIZE);
    let cursor = app(&engine).cursor;
    assert!(cursor.part > 0, "{what}: the cursor is on a wrapped part");
    assert!(
        cursor.row >= app(&engine).top.row + 2,
        "{what}: the cursor is well below the top"
    );
    let (shown, _) = top(&engine);
    engine.execute(Command::ToggleWrap).unwrap();
    assert_unscrolled(&mut engine, SIZE, &(shown, 0), &format!("{what}, wrap off"));
    assert_eq!(
        app(&engine).cursor,
        Position {
            row: cursor.row,
            part: 0
        },
        "{what}: the cursor stays on its row"
    );
    check_steps(layout, true, what, &|engine| {
        prepare(engine);
        draw(engine, SIZE);
        engine.execute(Command::ToggleWrap).unwrap();
        SIZE
    });
}

#[test]
fn turning_wrapping_off_keeps_the_top_row_and_never_scrolls_sideways() {
    type Prepare = (&'static str, fn(&mut Engine));
    let prepares: [Prepare; 3] = [
        (
            "a click far right on the new side of a wrapped part",
            |engine| {
                let y = screen_row(engine, 8, |row| {
                    row.kind == DiffRowKind::Code && row.part > 0
                });
                let x = far_right(engine, SIZE).1;
                click(engine, x, y);
            },
        ),
        (
            "a click far right on the old side of a wrapped part",
            |engine| {
                let y = screen_row(engine, 8, |row| {
                    row.kind == DiffRowKind::Code && row.part > 0
                });
                let x = far_right(engine, SIZE).0;
                click(engine, x, y);
            },
        ),
        ("Down onto a wrapped part", |engine| {
            for _ in 0..14 {
                engine.execute(Command::Down).unwrap();
            }
            while engine.app.as_ref().unwrap().cursor.part == 0 {
                engine.execute(Command::Down).unwrap();
            }
        }),
    ];
    for layout in LAYOUTS {
        for (what, prepare) in prepares {
            check_unwrap(layout, &format!("{layout:?} {what}"), prepare);
        }
    }
}

#[test]
fn turning_wrapping_on_keeps_the_clicked_text_on_the_cursor_row() {
    for layout in LAYOUTS {
        let (mut engine, _directory) = open(layout, false);
        for _ in 0..10 {
            engine.execute(Command::Right).unwrap();
        }
        let y = screen_row(&engine, 1, new_line(25));
        let x = far_right(&engine, SIZE).1;
        click(&mut engine, x, y);
        engine.execute(Command::ToggleWrap).unwrap();
        draw(&mut engine, SIZE);
        let app = app(&engine);
        assert!(
            app.cursor.part > 0,
            "{layout:?}: the part with the clicked text"
        );
        assert_eq!(
            app.horizontal, 40,
            "{layout:?}: kept for when wrapping is off"
        );
    }
}

#[test]
fn a_scroll_the_person_made_is_kept_or_lowered_and_never_raised() {
    for layout in LAYOUTS {
        let (mut engine, _directory) = open(layout, false);
        for _ in 0..10 {
            engine.execute(Command::Right).unwrap();
        }
        assert_eq!(app(&engine).horizontal, 40, "{layout:?}");
        // A click far right, where the old engine scrolled to on the next reflow.
        let y = screen_row(&engine, 1, new_line(33));
        let x = far_right(&engine, SIZE).0;
        click(&mut engine, x, y);
        let before = top(&engine);
        let mut size = SIZE;
        let mut expected = 40;
        for (name, step) in STEPS {
            step(&mut engine, &mut size);
            draw(&mut engine, size);
            let app = app(&engine);
            let what = format!("{layout:?} after {name}");
            assert_eq!(top(&engine), before, "{what}");
            // Kept, and lowered only as far as the lines shown still reach.
            expected = expected.min(crate::render::horizontal_limit(app).unwrap());
            assert_eq!(app.horizontal, expected, "{what}");
            if name == "a narrower pane" {
                assert_eq!(app.horizontal, 40, "{what}: kept");
            }
        }
        assert_eq!(expected, 0, "{layout:?}: lowered once every line fit");
        // Left and Right still scroll.
        engine.execute(Command::Right).unwrap();
        assert_eq!(app(&engine).horizontal, 4, "{layout:?}");
    }
}

#[test]
fn a_host_that_asks_keeps_the_clicked_text_in_view_across_a_reflow() {
    for layout in LAYOUTS {
        let (mut engine, _directory) = open(layout, false);
        assert!(!engine.keeps_text_in_view(), "off by default");
        engine.set_keep_text_in_view(true);
        let y = screen_row(&engine, 1, new_line(25));
        let x = far_right(&engine, SIZE).1;
        click(&mut engine, x, y);
        let before = top(&engine);
        draw(&mut engine, NARROW);
        assert!(
            app(&engine).horizontal > 0,
            "{layout:?}: scrolled to the clicked text"
        );
        assert_eq!(top(&engine), before, "{layout:?}");
        // Kept across a new review, as the other settings are.
        engine
            .replace(Input::Patch(_directory.path().join("review.patch")))
            .unwrap();
        assert!(
            engine.keeps_text_in_view(),
            "{layout:?}: kept across replace"
        );
    }
}
