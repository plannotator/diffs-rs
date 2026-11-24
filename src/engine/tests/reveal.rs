//! `Engine::select_range` and `Engine::reveal` several times between two frames: a line already
//! on screen never moves the view, and a line off screen still scrolls into view.

use super::rows::{LAYOUTS, check, draw};
use super::*;

/// Six files of forty lines, each with replaced, removed and added lines among context.
fn long_patch() -> String {
    let mut patch = String::new();
    for file in 0..6 {
        let (mut old, mut new, mut body) = (0, 0, String::new());
        for n in 1..=40 {
            match n % 9 {
                3 => {
                    body.push_str(&format!("-old {file} {n}\n+new {file} {n}\n"));
                    (old, new) = (old + 1, new + 1);
                }
                6 => {
                    body.push_str(&format!("-gone {file} {n}\n"));
                    old += 1;
                }
                7 => {
                    body.push_str(&format!("+added {file} {n}\n line {file} {n}\n"));
                    (old, new) = (old + 1, new + 2);
                }
                _ => {
                    body.push_str(&format!(" line {file} {n}\n"));
                    (old, new) = (old + 1, new + 1);
                }
            }
        }
        patch.push_str(&format!(
            "diff --git a/f{file}.txt b/f{file}.txt\n--- a/f{file}.txt\n+++ b/f{file}.txt\n@@ -1,{old} +1,{new} @@\n{body}"
        ));
    }
    patch
}

/// The last new line of every file of `long_patch`.
const LAST: u32 = 40;

fn open(layout: Layout, wrap: bool) -> (Engine, tempfile::TempDir) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("long.patch");
    fs::write(&path, long_patch()).unwrap();
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

fn top(engine: &Engine) -> crate::app::Position {
    engine.app.as_ref().unwrap().top
}

/// Two lines of one file on `side`, both drawn away from the edges of the frame: the first
/// and the last such line of the file with the most.
fn two_drawn_lines(engine: &Engine, height: u16, side: Side) -> (usize, u32, u32) {
    let mut by_file: std::collections::BTreeMap<usize, Vec<u32>> = Default::default();
    for y in 3..height - 3 {
        let Some(row) = engine.row_at(10, y) else {
            continue;
        };
        let line = if side == Side::Old {
            row.old_line
        } else {
            row.new_line
        };
        if row.kind == DiffRowKind::Code
            && row.part == 0
            && let Some(line) = line
        {
            by_file.entry(row.file).or_default().push(line);
        }
    }
    let (file, lines) = by_file
        .into_iter()
        .max_by_key(|(_, lines)| lines.len())
        .expect("lines on screen");
    assert!(lines.len() >= 2, "two lines of one file on screen");
    (file, lines[0], *lines.last().unwrap())
}

#[test]
fn reveals_of_lines_on_screen_between_frames_never_move_the_view() {
    let size = (121, 30);
    for (layout, wrap) in LAYOUTS {
        let sides: &[Side] = if layout == Layout::Split {
            &[Side::New, Side::Old]
        } else {
            &[Side::New]
        };
        for &side in sides {
            let what = format!("{layout:?} {wrap} {side:?}");
            let (mut engine, _directory) = open(layout, wrap);
            draw(&mut engine, size);
            for _ in 0..3 {
                engine.execute(Command::PageDown).unwrap();
                draw(&mut engine, size);
            }
            let buffer = draw(&mut engine, size);
            check(&engine, &buffer, &what);
            let before = top(&engine);
            assert!(before.row > 0, "{what}: scrolled into the middle");
            let (file, first, last) = two_drawn_lines(&engine, size.1, side);
            let range = SourceRange {
                review: engine.review_id(),
                file,
                side,
                first,
                last,
            };
            // No frame between any of these.
            for ends in [[first, last], [last, first]] {
                engine.select_range(range).unwrap();
                assert_eq!(top(&engine), before, "{what}: select_range");
                for line in ends {
                    engine.reveal(file, side, line).unwrap();
                    assert_eq!(top(&engine), before, "{what}: reveal {line} after {ends:?}");
                }
                engine.reveal(file, side, ends[0]).unwrap();
                engine.select_range(range).unwrap();
                assert_eq!(top(&engine), before, "{what}: select_range again");
            }
            assert_eq!(engine.selection(), Some(range), "{what}");
            let buffer = draw(&mut engine, size);
            check(&engine, &buffer, &what);
            assert_eq!(top(&engine), before, "{what}: after the frame");
            let cursor = engine.cursor_row().unwrap();
            let shown = if side == Side::Old {
                cursor.old_line
            } else {
                cursor.new_line
            };
            assert_eq!((cursor.file, shown), (file, Some(last)), "{what}");

            // A line below the screen still scrolls into view, as does one above it, with
            // a reveal of a line on screen in between and no frame.
            engine.reveal(5, Side::New, LAST).unwrap();
            let below = top(&engine);
            assert!(below > before, "{what}: scrolled down to the last line");
            engine.reveal(5, Side::New, LAST).unwrap();
            assert_eq!(
                top(&engine),
                below,
                "{what}: the last line is on screen now"
            );
            let buffer = draw(&mut engine, size);
            check(&engine, &buffer, &format!("{what} at the end"));
            assert_eq!(engine.cursor_row().unwrap().new_line, Some(LAST), "{what}");
            engine.reveal(0, Side::New, 1).unwrap();
            assert!(
                top(&engine) < below,
                "{what}: scrolled up to the first line"
            );
            let buffer = draw(&mut engine, size);
            check(&engine, &buffer, &format!("{what} at the start"));
            let cursor = engine.cursor_row().unwrap();
            assert_eq!((cursor.file, cursor.new_line), (0, Some(1)), "{what}");
        }
    }
}
