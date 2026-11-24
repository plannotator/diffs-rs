//! Host insets: where they hang, what `inset_areas`, `row_at` and `cursor_row` report, how the
//! cursor, selections and scrolling go over them, and that neither setting them nor any reflow
//! with them present moves what the person is reading.

use super::rows::{WIDE, draw, text};
use super::*;
use crate::app::Position;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;

const SIZE: (u16, u16) = (120, 26);
/// Unified, split evenly and split 30:70.
const LAYOUTS: [(Layout, u16); 3] = [
    (Layout::Unified, 500),
    (Layout::Split, 500),
    (Layout::Split, 300),
];

fn long(tag: &str) -> String {
    format!("{tag}:{}", "abcdefghij".repeat(24))
}

/// `a.txt`: two hunks, a removed pair, an added line and a very wide line. `b.txt`: a hunk
/// whose every change is whitespace only. `c.txt`: sixty long lines, the 15th replaced.
///
/// Lines of `a.txt`, old / new: c01..c08 1..8 / 1..8; r09, r10 9, 10 / -; a09..a11 - / 9..11;
/// c11..c20 11..20 / 12..21, the wide line at 14 / 15; d40..d44 40..44 / 41..45; r45 45 / -;
/// a45 - / 46; d46..d49 46..49 / 47..50.
fn patch() -> String {
    let mut p = String::from("diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n");
    p.push_str("@@ -1,20 +1,21 @@\n");
    for n in 1..=8 {
        p.push_str(&format!(" c{n:02}\n"));
    }
    p.push_str("-r09\n-r10\n+a09\n+a10\n+a11\n");
    for n in 11..=20 {
        if n == 14 {
            p.push_str(&format!(" {WIDE}\n"));
        } else {
            p.push_str(&format!(" c{n:02}\n"));
        }
    }
    p.push_str("@@ -40,10 +41,10 @@\n");
    for n in 40..=44 {
        p.push_str(&format!(" d{n}\n"));
    }
    p.push_str("-r45\n+a45\n");
    for n in 46..=49 {
        p.push_str(&format!(" d{n}\n"));
    }
    p.push_str("diff --git a/b.txt b/b.txt\n--- a/b.txt\n+++ b/b.txt\n@@ -1,4 +1,4 @@\n");
    p.push_str(" first\n-    spaced 1\n-    spaced 2\n+spaced 1\n+spaced 2\n last\n");
    p.push_str("diff --git a/c.txt b/c.txt\n--- a/c.txt\n+++ b/c.txt\n@@ -1,60 +1,60 @@\n");
    for n in 1..=60 {
        if n == 15 {
            p.push_str(&format!("-{}\n+{}\n", long("x15"), long("y15")));
        } else {
            p.push_str(&format!(" {}\n", long(&format!("e{n:02}"))));
        }
    }
    p
}

const A: usize = 0;
const B: usize = 1;
const C: usize = 2;

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
    (engine, directory)
}

fn on_line(engine: &Engine, id: u64, file: usize, side: Side, line: u32, height: u16) -> Inset {
    Inset {
        review: engine.review_id(),
        id,
        file,
        anchor: InsetAnchor::Line { side, line },
        height,
    }
}

fn on_heading(engine: &Engine, id: u64, file: usize, height: u16) -> Inset {
    Inset {
        review: engine.review_id(),
        id,
        file,
        anchor: InsetAnchor::FileHeading,
        height,
    }
}

fn app(engine: &Engine) -> &App {
    engine.app.as_ref().unwrap()
}

fn mouse(engine: &mut Engine, kind: MouseEventKind, column: u16, row: u16) -> InputEffect {
    let event = MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    };
    engine.handle_event(Event::Mouse(event), true).unwrap()
}

/// A cell on the new side of a split row, and the middle of a unified one.
fn new_column(engine: &Engine, size: (u16, u16)) -> u16 {
    if app(engine).split() {
        size.0 * 3 / 4
    } else {
        size.0 / 2
    }
}

/// The screen rows of the row that shows `line` on `side` of `file`, top to bottom.
fn rows_of(engine: &Engine, buffer: &Buffer, file: usize, side: Side, line: u32) -> Vec<u16> {
    (0..buffer.area.height)
        .filter(|&y| {
            engine.row_at(2, y).is_some_and(|row| {
                row.file == file
                    && row.kind == DiffRowKind::Code
                    && match side {
                        Side::Old => row.old_line,
                        Side::New => row.new_line,
                    } == Some(line)
            })
        })
        .collect()
}

fn area(engine: &Engine, id: u64) -> Option<InsetArea> {
    engine.inset_areas().iter().copied().find(|a| a.id == id)
}

/// The cursor's one-based screen row in the last frame.
fn cursor_y(engine: &Engine) -> u16 {
    let app = app(engine);
    app.visible
        .iter()
        .find(|(p, _)| *p == app.cursor)
        .map(|(_, y)| *y)
        .expect("the cursor is on screen")
}

/// The row drawn first under the sticky heading, or at the top when there is none.
fn top_row(engine: &Engine) -> DiffRow {
    let first = engine.row_at(2, 0).unwrap();
    if first.kind == DiffRowKind::StickyHeader {
        engine.row_at(2, 1).unwrap()
    } else {
        first
    }
}

/// What the person reads: the cursor's row, its screen row and the horizontal scroll.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Seen {
    row: DiffRow,
    y: u16,
    horizontal: usize,
}

fn seen(engine: &Engine) -> Seen {
    Seen {
        row: engine.cursor_row().unwrap(),
        y: cursor_y(engine),
        horizontal: app(engine).horizontal,
    }
}

/// The cursor's line on the new side, wherever on its row the cursor is.
fn cursor_line(engine: &Engine) -> u32 {
    let row = engine.cursor_row().unwrap();
    assert_eq!(row.kind, DiffRowKind::Code, "on code: {row:?}");
    row.new_line.unwrap()
}

/// Moves down until the cursor is on the inset `id`, returning the moves taken.
fn down_to(engine: &mut Engine, id: u64, what: &str) -> usize {
    for n in 1..=40 {
        engine.execute(Command::Down).unwrap();
        if engine.cursor_row().unwrap().kind == (DiffRowKind::Inset { id }) {
            return n;
        }
    }
    panic!("{what}: never reached inset {id}");
}

#[test]
fn insets_hang_under_the_rows_they_name() {
    for layout in LAYOUTS {
        for wrap in [false, true] {
            let what = format!("{layout:?} wrap {wrap}");
            let (mut engine, _directory) = open(layout, wrap);
            // Tall enough for the whole review.
            let size = (120, 1200);
            let plain = draw(&mut engine, size);
            let shown = |buffer: &Buffer| {
                (0..buffer.area.height)
                    .filter(|&y| !text(buffer, y).trim().is_empty())
                    .count()
            };
            let rows_before = shown(&plain);
            let set = [
                on_line(&engine, 1, A, Side::New, 3, 2),
                // The last line of the first hunk, and of the file.
                on_line(&engine, 2, A, Side::New, 21, 1),
                on_line(&engine, 3, A, Side::New, 50, 3),
                // A removed line, on the old side.
                on_line(&engine, 4, A, Side::Old, 10, 2),
                on_heading(&engine, 5, A, 2),
                // Stacked under line 3, after inset 1.
                on_line(&engine, 6, A, Side::New, 3, 1),
                // The wide line, which wraps.
                on_line(&engine, 7, A, Side::New, 15, 2),
                // An added line with no removed line beside it.
                on_line(&engine, 8, A, Side::New, 11, 1),
                // The last line of the review.
                on_line(&engine, 9, C, Side::New, 60, 2),
            ];
            engine.set_insets(&set).unwrap();
            assert_eq!(engine.insets(), set);
            assert!(engine.hidden_insets().is_empty(), "{what}");
            let buffer = draw(&mut engine, size);
            let heading = (0..size.1)
                .find(|&y| {
                    engine
                        .row_at(2, y)
                        .is_some_and(|r| r.file == A && r.kind == DiffRowKind::Header)
                })
                .unwrap();
            let groups: [(u16, &[u64]); 8] = [
                (
                    *rows_of(&engine, &buffer, A, Side::New, 3).last().unwrap(),
                    &[1, 6],
                ),
                (
                    *rows_of(&engine, &buffer, A, Side::New, 21).last().unwrap(),
                    &[2],
                ),
                (
                    *rows_of(&engine, &buffer, A, Side::New, 50).last().unwrap(),
                    &[3],
                ),
                (
                    *rows_of(&engine, &buffer, A, Side::Old, 10).last().unwrap(),
                    &[4],
                ),
                (heading, &[5]),
                (
                    *rows_of(&engine, &buffer, A, Side::New, 15).last().unwrap(),
                    &[7],
                ),
                (
                    *rows_of(&engine, &buffer, A, Side::New, 11).last().unwrap(),
                    &[8],
                ),
                (
                    *rows_of(&engine, &buffer, C, Side::New, 60).last().unwrap(),
                    &[9],
                ),
            ];
            if wrap {
                assert!(
                    rows_of(&engine, &buffer, A, Side::New, 15).len() > 1,
                    "{what}"
                );
            }
            for (last, ids) in groups {
                let mut y = last + 1;
                for &id in ids {
                    let inset = set.iter().find(|i| i.id == id).unwrap();
                    let found = area(&engine, id).unwrap();
                    assert_eq!(
                        (
                            found.area.y,
                            found.area.height,
                            found.clipped_top,
                            found.cursor
                        ),
                        (y, inset.height, 0, false),
                        "{what}: inset {id}"
                    );
                    assert_eq!(found.height, inset.height);
                    for part in 0..inset.height {
                        let row = engine.row_at(2, y).unwrap();
                        assert_eq!(row.kind, DiffRowKind::Inset { id }, "{what}: row {y}");
                        assert_eq!(row.part, usize::from(part), "{what}: row {y}");
                        assert_eq!(row.file, inset.file);
                        let expected = match inset.anchor {
                            InsetAnchor::Line {
                                side: Side::Old,
                                line,
                            } => (Some(line), None),
                            InsetAnchor::Line { line, .. } => (None, Some(line)),
                            _ => (None, None),
                        };
                        assert_eq!((row.old_line, row.new_line), expected, "{what}: {id}");
                        assert_eq!(
                            row.hunk,
                            match id {
                                5 => None,
                                3 => Some(1),
                                9 => Some(0),
                                _ => Some(0),
                            },
                            "{what}: inset {id}"
                        );
                        // Nothing but the background and the line between the sides.
                        let line = text(&buffer, y);
                        assert_eq!(
                            line.trim(),
                            if app(&engine).split() { "│" } else { "" },
                            "{what}: row {y} {line:?}"
                        );
                        y += 1;
                    }
                }
                // The row under the stack is not an inset.
                assert!(
                    !matches!(
                        engine.row_at(2, y).map(|r| r.kind),
                        Some(DiffRowKind::Inset { .. })
                    ),
                    "{what}: under the insets at {last}"
                );
            }
            // The row under line 3's insets is line 4.
            assert_eq!(
                rows_of(&engine, &buffer, A, Side::New, 4)[0],
                *rows_of(&engine, &buffer, A, Side::New, 3).last().unwrap() + 4,
                "{what}"
            );
            // Where the host draws: right of the gutter, to the right edge.
            for found in engine.inset_areas() {
                assert_eq!(found.area.right(), size.0, "{what}");
                assert_eq!(found.area, found.left.union(found.right), "{what}");
                if app(&engine).split() {
                    let line = engine.split_line().unwrap();
                    assert_eq!(found.left.right(), line, "{what}");
                    assert!(found.right.x > line + 1, "{what}");
                    assert_eq!(found.right.right(), size.0, "{what}");
                    assert_eq!(
                        Some(found.left.width),
                        engine.inset_width(A, Side::Old),
                        "{what}"
                    );
                    assert_eq!(
                        Some(found.right.width),
                        engine.inset_width(A, Side::New),
                        "{what}"
                    );
                } else {
                    assert_eq!((found.left, found.right), (found.area, found.area));
                    assert!(found.area.x > 0, "{what}: the gutter is the engine's");
                    assert_eq!(
                        Some(found.area.width),
                        engine.inset_width(A, Side::New),
                        "{what}"
                    );
                    assert_eq!(
                        engine.inset_width(A, Side::Old),
                        engine.inset_width(A, Side::New)
                    );
                }
            }
            // Exactly the inset rows were added: none blank at the end or anywhere else.
            let added: usize = set.iter().map(|i| usize::from(i.height)).sum();
            let blank_inset_rows: usize = engine
                .inset_areas()
                .iter()
                .map(|a| usize::from(a.area.height))
                .sum();
            assert_eq!(blank_inset_rows, added, "{what}");
            let split_inset_rows = if app(&engine).split() { 0 } else { added };
            assert_eq!(
                shown(&buffer) + split_inset_rows,
                rows_before + added,
                "{what}"
            );
            // Cleared, the frame is the plain one again.
            engine.set_insets(&[]).unwrap();
            assert_eq!(draw(&mut engine, size), plain, "{what}");
            assert!(engine.inset_areas().is_empty());
        }
    }
}

#[test]
fn areas_cut_by_the_top_and_bottom_report_the_rows_on_screen() {
    for layout in LAYOUTS {
        for wrap in [false, true] {
            let what = format!("{layout:?} wrap {wrap}");
            let (mut engine, _directory) = open(layout, wrap);
            let size = (120, 10);
            draw(&mut engine, size);
            let inset = on_line(&engine, 1, C, Side::New, 4, 7);
            engine.set_insets(&[inset]).unwrap();
            engine.reveal(B, Side::New, 1).unwrap();
            engine.execute(Command::NextFile).unwrap();
            assert_eq!(engine.cursor_row().unwrap().kind, DiffRowKind::Header);
            draw(&mut engine, size);
            // Scrolled three rows at a time from file c's heading, the inset moves up three
            // rows a step and is reported as far as it shows.
            let mut start: Option<i32> = None;
            let mut was_sticky = false;
            let mut cut_at_bottom = false;
            let mut cut_at_top = false;
            for step in 0..60 {
                let found = area(&engine, 1);
                let sticky = engine.row_at(2, 0).unwrap().kind == DiffRowKind::StickyHeader;
                let first = i32::from(sticky);
                // The heading kept over the top row pushes the rows under it down one.
                start = start.map(|s| s + first - i32::from(was_sticky));
                was_sticky = sticky;
                if let Some(found) = found {
                    let virtual_start = i32::from(found.area.y) - i32::from(found.clipped_top);
                    if let Some(start) = start {
                        assert_eq!(virtual_start, start, "{what}: step {step}");
                    }
                    let top = virtual_start.max(first);
                    let bottom = (virtual_start + 7).min(i32::from(size.1));
                    assert_eq!(i32::from(found.area.y), top, "{what}: step {step}");
                    assert_eq!(
                        i32::from(found.area.height),
                        bottom - top,
                        "{what}: step {step}"
                    );
                    assert_eq!(found.clipped_top as i32, top - virtual_start);
                    cut_at_bottom |= bottom - top < 7 && found.clipped_top == 0;
                    cut_at_top |= found.clipped_top > 0;
                    for y in found.area.top()..found.area.bottom() {
                        let row = engine.row_at(2, y).unwrap();
                        assert_eq!(row.kind, DiffRowKind::Inset { id: 1 });
                        assert_eq!(
                            row.part,
                            usize::from(found.clipped_top + y - found.area.y),
                            "{what}"
                        );
                    }
                    start = Some(virtual_start - 3);
                } else if start.is_some_and(|s| s + 7 <= first) {
                    break;
                }
                mouse(&mut engine, MouseEventKind::ScrollDown, 5, 5);
                draw(&mut engine, size);
            }
            assert!(
                cut_at_bottom && cut_at_top,
                "{what}: {cut_at_bottom} {cut_at_top}"
            );
        }
    }
}

#[test]
fn the_cursor_stops_once_on_each_inset() {
    for layout in LAYOUTS {
        for wrap in [false, true] {
            let what = format!("{layout:?} wrap {wrap}");
            let (mut engine, _directory) = open(layout, wrap);
            draw(&mut engine, SIZE);
            let set = [
                on_line(&engine, 1, C, Side::New, 5, 3),
                on_line(&engine, 2, C, Side::New, 5, 2),
                on_line(&engine, 3, C, Side::New, 6, 4),
                on_line(&engine, 9, C, Side::New, 60, 3),
            ];
            engine.set_insets(&set).unwrap();
            engine.reveal(C, Side::New, 5).unwrap();
            draw(&mut engine, SIZE);
            down_to(&mut engine, 1, &what);
            assert_eq!(engine.cursor_row().unwrap().part, 0, "{what}");
            assert_eq!(engine.selected_source().unwrap(), None);
            draw(&mut engine, SIZE);
            let found = area(&engine, 1).unwrap();
            assert!(found.cursor, "{what}");
            assert!(area(&engine, 2).is_none_or(|area| !area.cursor));
            // The cursor's mark runs down the gutter of the inset's side.
            let buffer = draw(&mut engine, SIZE);
            let gutter = if app(&engine).split() {
                engine.split_line().unwrap() + 1
            } else {
                0
            };
            for y in found.area.top()..found.area.bottom() {
                assert_eq!(buffer[(gutter, y)].symbol(), "▎", "{what}: row {y}");
            }
            engine.execute(Command::Down).unwrap();
            assert_eq!(
                engine.cursor_row().unwrap().kind,
                DiffRowKind::Inset { id: 2 }
            );
            engine.execute(Command::Down).unwrap();
            assert_eq!(cursor_line(&engine), 6, "{what}");
            assert_eq!(engine.cursor_row().unwrap().part, 0, "{what}");
            down_to(&mut engine, 3, &what);
            // Up from below stops on each inset's nearest row, then on the lines.
            engine.execute(Command::Up).unwrap();
            assert_eq!(cursor_line(&engine), 6, "{what}");
            for _ in 0..10 {
                if engine.cursor_row().unwrap().part == 0 {
                    break;
                }
                engine.execute(Command::Up).unwrap();
            }
            engine.execute(Command::Up).unwrap();
            let row = engine.cursor_row().unwrap();
            assert_eq!(
                (row.kind, row.part),
                (DiffRowKind::Inset { id: 2 }, 1),
                "{what}"
            );
            engine.execute(Command::Up).unwrap();
            let row = engine.cursor_row().unwrap();
            assert_eq!(
                (row.kind, row.part),
                (DiffRowKind::Inset { id: 1 }, 2),
                "{what}"
            );
            engine.execute(Command::Up).unwrap();
            assert_eq!(cursor_line(&engine), 5, "{what}");

            // A page counts every screen row, the insets' included.
            draw(&mut engine, SIZE);
            let start = app(&engine).cursor;
            let page = usize::from(app(&engine).height - 4);
            let expected = (0..page).fold(start, |p, _| app(&engine).next(p).unwrap());
            engine.execute(Command::PageDown).unwrap();
            assert_eq!(app(&engine).cursor, expected, "{what}");
            draw(&mut engine, SIZE);
            cursor_y(&engine);
            engine.execute(Command::PageUp).unwrap();
            assert_eq!(app(&engine).cursor, start, "{what}");

            // The last line shows the insets under it; the first line is the first.
            engine.execute(Command::LastSource).unwrap();
            let buffer = draw(&mut engine, SIZE);
            assert_eq!(cursor_line(&engine), 60, "{what}");
            let bottom = engine.row_at(2, SIZE.1 - 1).unwrap();
            assert_eq!(
                (bottom.kind, bottom.part),
                (DiffRowKind::Inset { id: 9 }, 2),
                "{what}"
            );
            assert!(!rows_of(&engine, &buffer, C, Side::New, 60).is_empty());
            down_to(&mut engine, 9, &what);
            assert_eq!(
                engine.execute(Command::Down).unwrap(),
                InputEffect::Highlight,
                "{what}"
            );
            assert_eq!(
                engine.cursor_row().unwrap().kind,
                DiffRowKind::Inset { id: 9 },
                "{what}: nothing below the last inset"
            );
            engine.execute(Command::FirstSource).unwrap();
            let row = engine.cursor_row().unwrap();
            assert_eq!(
                (row.file, row.kind, row.new_line),
                (A, DiffRowKind::Code, Some(1))
            );

            // Insets that are not stops are passed over and never clicked onto.
            engine.set_inset_stops(false).unwrap();
            assert!(!engine.inset_stops());
            engine.reveal(C, Side::New, 5).unwrap();
            for _ in 0..12 {
                engine.execute(Command::Down).unwrap();
                assert!(
                    !matches!(engine.cursor_row().unwrap().kind, DiffRowKind::Inset { .. }),
                    "{what}"
                );
            }
            assert!(cursor_line(&engine) >= 6, "{what}");
            for _ in 0..12 {
                engine.execute(Command::Up).unwrap();
                assert!(
                    !matches!(engine.cursor_row().unwrap().kind, DiffRowKind::Inset { .. }),
                    "{what}"
                );
            }
            draw(&mut engine, SIZE);
            let before = seen(&engine);
            let click = MouseEventKind::Down(MouseButton::Left);
            let y = area(&engine, 1).or(area(&engine, 2)).or(area(&engine, 3));
            let y = y.expect("an inset on screen").area.y;
            assert_eq!(
                {
                    let column = new_column(&engine, SIZE);
                    mouse(&mut engine, click, column, y)
                },
                InputEffect::Ignored
            );
            draw(&mut engine, SIZE);
            assert_eq!(seen(&engine), before, "{what}");
            // Short of the end of the diff, where the last inset can be all there is on screen.
            for _ in 0..8 {
                mouse(&mut engine, MouseEventKind::ScrollDown, 5, 5);
                assert!(
                    !matches!(engine.cursor_row().unwrap().kind, DiffRowKind::Inset { .. }),
                    "{what}"
                );
            }
            // Turned off while on an inset, the cursor goes to the line above.
            engine.set_inset_stops(true).unwrap();
            engine.reveal(C, Side::New, 6).unwrap();
            down_to(&mut engine, 3, &what);
            draw(&mut engine, SIZE);
            let line_y = cursor_y(&engine) - 1;
            engine.set_inset_stops(false).unwrap();
            draw(&mut engine, SIZE);
            assert_eq!(cursor_line(&engine), 6, "{what}");
            assert_eq!(cursor_y(&engine), line_y, "{what}");
            // A click onto an inset that is a stop puts the cursor on it.
            engine.set_inset_stops(true).unwrap();
            engine.reveal(C, Side::New, 6).unwrap();
            let buffer = draw(&mut engine, SIZE);
            let y = rows_of(&engine, &buffer, C, Side::New, 6).last().unwrap() + 2;
            {
                let column = new_column(&engine, SIZE);
                mouse(&mut engine, click, column, y)
            };
            let row = engine.cursor_row().unwrap();
            assert_eq!(
                (row.kind, row.part),
                (DiffRowKind::Inset { id: 3 }, 1),
                "{what}"
            );
        }
    }
}

#[test]
fn a_selection_over_an_inset_takes_only_its_lines() {
    for layout in LAYOUTS {
        let what = format!("{layout:?}");
        let (mut engine, _directory) = open(layout, false);
        draw(&mut engine, SIZE);
        engine
            .set_insets(&[on_line(&engine, 1, C, Side::New, 5, 2)])
            .unwrap();
        engine
            .set_markers(&[Marker {
                range: SourceRange {
                    review: engine.review_id(),
                    file: C,
                    side: Side::New,
                    first: 1,
                    last: 60,
                },
                style: ratatui::style::Style::new().fg(ratatui::style::Color::Yellow),
            }])
            .unwrap();
        engine.reveal(C, Side::New, 4).unwrap();
        if app(&engine).split() && engine.cursor_row().unwrap().side != Side::New {
            engine.execute(Command::SwitchSide).unwrap();
        }
        engine.execute(Command::SelectLines).unwrap();
        engine.execute(Command::Down).unwrap();
        engine.execute(Command::Down).unwrap();
        assert_eq!(
            engine.cursor_row().unwrap().kind,
            DiffRowKind::Inset { id: 1 }
        );
        let range = engine.selection().unwrap();
        assert_eq!((range.first, range.last), (4, 5), "{what}");
        engine.execute(Command::Down).unwrap();
        engine.execute(Command::SelectLines).unwrap();
        let range = engine.selection().unwrap();
        assert_eq!(
            (range.file, range.side, range.first, range.last),
            (C, Side::New, 4, 6)
        );
        let buffer = draw(&mut engine, SIZE);
        let found = area(&engine, 1).unwrap();
        for y in found.area.top()..found.area.bottom() {
            for x in 0..SIZE.0 {
                let symbol = buffer[(x, y)].symbol();
                assert!(
                    symbol == " " || symbol == "│",
                    "{what}: no selection or marker on an inset: {symbol:?} at {x},{y}"
                );
            }
        }
        let mut copied = Vec::new();
        engine
            .source_read(range)
            .unwrap()
            .write_to(&mut copied, &|| Ok(()))
            .unwrap();
        let expected: String = [4, 5, 6]
            .iter()
            .map(|n| format!("{}\n", long(&format!("e{n:02}"))))
            .collect();
        assert_eq!(String::from_utf8(copied).unwrap(), expected, "{what}");
    }
}

/// Puts the cursor on code in file c: near the top of the pane, in its middle, or on the last
/// page, by scrolling and a click, as a person would.
#[derive(Clone, Copy, Debug)]
enum Where {
    NearTop,
    Middle,
    LastPage,
}

fn place(engine: &mut Engine, size: (u16, u16), at: Where) {
    let column = new_column(engine, size);
    match at {
        Where::NearTop | Where::Middle => {
            engine.reveal(B, Side::New, 1).unwrap();
            engine.execute(Command::NextFile).unwrap();
            draw(engine, size);
            while top_row(engine).new_line.is_none_or(|line| line < 10) {
                mouse(engine, MouseEventKind::ScrollDown, 5, 5);
                draw(engine, size);
            }
            let y = if matches!(at, Where::NearTop) {
                2
            } else {
                size.1 / 2
            };
            mouse(engine, MouseEventKind::Down(MouseButton::Left), column, y);
        }
        Where::LastPage => {
            engine.execute(Command::LastSource).unwrap();
            draw(engine, size);
            mouse(
                engine,
                MouseEventKind::Down(MouseButton::Left),
                column,
                size.1 - 5,
            );
        }
    }
    draw(engine, size);
    assert_eq!(engine.cursor_row().unwrap().kind, DiffRowKind::Code);
}

/// Sets `insets` and checks the cursor's row, its screen row and the horizontal scroll stay.
fn set_keeping(engine: &mut Engine, size: (u16, u16), insets: &[Inset], what: &str) {
    let before = seen(engine);
    engine.set_insets(insets).unwrap();
    // `cursor_row` is right before the next frame.
    assert_eq!(engine.cursor_row().unwrap(), before.row, "{what}");
    assert_eq!(seen_now(engine, size), before, "{what}");
}

fn seen_now(engine: &mut Engine, size: (u16, u16)) -> Seen {
    draw(engine, size);
    seen(engine)
}

#[test]
fn setting_insets_never_moves_what_is_read() {
    for layout in LAYOUTS {
        for wrap in [false, true] {
            for at in [Where::NearTop, Where::Middle, Where::LastPage] {
                let what = format!("{layout:?} wrap {wrap} {at:?}");
                let (mut engine, _directory) = open(layout, wrap);
                draw(&mut engine, SIZE);
                place(&mut engine, SIZE, at);
                if !wrap {
                    for _ in 0..3 {
                        engine.execute(Command::Right).unwrap();
                    }
                    draw(&mut engine, SIZE);
                    assert_eq!(app(&engine).horizontal, 12, "{what}");
                }
                let line = cursor_line(&engine);
                let (above, below) = (line - 2, line + 2);
                let review = engine.review_id();
                let i = move |id, line, height| Inset {
                    review,
                    id,
                    file: C,
                    anchor: InsetAnchor::Line {
                        side: Side::New,
                        line,
                    },
                    height,
                };
                let heading = on_heading(&engine, 20, C, 2);
                let steps: Vec<(&str, Vec<Inset>)> = vec![
                    (
                        "above and below",
                        vec![i(1, above, 2), i(2, below.min(60), 2)],
                    ),
                    (
                        "another above",
                        vec![i(1, above, 2), i(2, below.min(60), 2), i(3, above - 1, 3)],
                    ),
                    (
                        "on the cursor's line",
                        vec![
                            i(1, above, 2),
                            i(2, below.min(60), 2),
                            i(3, above - 1, 3),
                            i(4, line, 2),
                        ],
                    ),
                    (
                        "the one above taller",
                        vec![
                            i(1, above, 9),
                            i(2, below.min(60), 2),
                            i(3, above - 1, 3),
                            i(4, line, 2),
                        ],
                    ),
                    (
                        "the one above shorter",
                        vec![
                            i(1, above, 1),
                            i(2, below.min(60), 2),
                            i(3, above - 1, 3),
                            i(4, line, 2),
                        ],
                    ),
                    (
                        "the one below taller",
                        vec![
                            i(1, above, 1),
                            i(2, below.min(60), 12),
                            i(3, above - 1, 3),
                            i(4, line, 2),
                        ],
                    ),
                    (
                        "the heading and far above",
                        vec![
                            i(1, above, 1),
                            i(2, below.min(60), 12),
                            i(3, above - 1, 3),
                            i(4, line, 2),
                            heading,
                            on_line(&engine, 21, A, Side::New, 3, 30),
                        ],
                    ),
                    (
                        "the ones above removed",
                        vec![i(2, below.min(60), 12), i(4, line, 2), heading],
                    ),
                    ("replaced", vec![i(7, above, 4), i(8, line, 1)]),
                    ("cleared", vec![]),
                ];
                for (step, set) in &steps {
                    set_keeping(&mut engine, SIZE, set, &format!("{what}: {step}"));
                }

                // On an inset: it keeps its screen row while it stays, and when it goes the
                // line it was under keeps its own.
                engine.set_insets(&[i(1, above, 2), i(4, line, 4)]).unwrap();
                draw(&mut engine, SIZE);
                let found = area(&engine, 4).unwrap();
                let y = found.area.y + 1;
                if y >= SIZE.1 {
                    continue;
                }
                {
                    let column = new_column(&engine, SIZE);
                    mouse(
                        &mut engine,
                        MouseEventKind::Down(MouseButton::Left),
                        column,
                        y,
                    )
                };
                draw(&mut engine, SIZE);
                let row = engine.cursor_row().unwrap();
                assert_eq!(
                    (row.kind, row.part),
                    (DiffRowKind::Inset { id: 4 }, 1),
                    "{what}"
                );
                let steps: Vec<(&str, Vec<Inset>)> = vec![
                    ("taller", vec![i(1, above, 2), i(4, line, 7)]),
                    ("above taller", vec![i(1, above, 6), i(4, line, 7)]),
                    (
                        "another under the line, before it",
                        vec![i(5, line, 2), i(1, above, 6), i(4, line, 7)],
                    ),
                ];
                for (step, set) in &steps {
                    set_keeping(
                        &mut engine,
                        SIZE,
                        set,
                        &format!("{what}: on an inset, {step}"),
                    );
                }
                // Cut to one row, the cursor is on that row, on the same screen row.
                let before = seen(&engine);
                engine
                    .set_insets(&[i(5, line, 2), i(1, above, 6), i(4, line, 1)])
                    .unwrap();
                let now = seen_now(&mut engine, SIZE);
                assert_eq!(
                    (now.y, now.horizontal),
                    (before.y, before.horizontal),
                    "{what}"
                );
                assert_eq!(
                    (now.row.kind, now.row.part),
                    (DiffRowKind::Inset { id: 4 }, 0),
                    "{what}"
                );
                // Removed: back to the line, which does not move.
                let buffer = draw(&mut engine, SIZE);
                let line_y = *rows_of(&engine, &buffer, C, Side::New, line)
                    .last()
                    .unwrap();
                engine.set_insets(&[i(1, above, 6)]).unwrap();
                let now = seen_now(&mut engine, SIZE);
                assert_eq!(cursor_line(&engine), line, "{what}");
                assert_eq!(
                    (now.y, now.horizontal),
                    (line_y + 1, before.horizontal),
                    "{what}"
                );
            }
        }
    }
}

#[test]
fn off_screen_the_top_row_stays_at_the_top() {
    for layout in LAYOUTS {
        for wrap in [false, true] {
            let what = format!("{layout:?} wrap {wrap}");
            let (mut engine, _directory) = open(layout, wrap);
            draw(&mut engine, SIZE);
            engine.reveal(C, Side::New, 3).unwrap();
            draw(&mut engine, SIZE);
            // The view far below the cursor, as a host scrolling the pane itself leaves it.
            let line = engine.app.as_ref().unwrap().view.source_row(
                &app(&engine).doc,
                app(&engine).displayed_source(C, false, 30).unwrap(),
                app(&engine).split(),
            );
            engine.app.as_mut().unwrap().top = Position {
                row: line.unwrap(),
                part: 0,
            };
            draw(&mut engine, SIZE);
            let top = top_row(&engine);
            assert_eq!(top.new_line, Some(30), "{what}");
            for (step, set) in [
                ("above", vec![on_line(&engine, 1, C, Side::New, 28, 3)]),
                (
                    "on the top line",
                    vec![on_line(&engine, 2, C, Side::New, 30, 3)],
                ),
                ("below", vec![on_line(&engine, 3, C, Side::New, 33, 3)]),
                (
                    "on the cursor's line",
                    vec![on_line(&engine, 4, C, Side::New, 3, 3)],
                ),
            ] {
                engine.set_insets(&set).unwrap();
                draw(&mut engine, SIZE);
                assert_eq!(top_row(&engine), top, "{what}: {step}");
            }
            // With the top on an inset: kept while the inset is, then the row below it.
            engine
                .set_insets(&[on_line(&engine, 2, C, Side::New, 30, 4)])
                .unwrap();
            let top = app(&engine).top;
            let text = crate::render::text_height(app(&engine), top.row).unwrap();
            engine.app.as_mut().unwrap().top.part = text + 1;
            draw(&mut engine, SIZE);
            let on = top_row(&engine);
            assert_eq!(
                (on.kind, on.part),
                (DiffRowKind::Inset { id: 2 }, 1),
                "{what}"
            );
            engine
                .set_insets(&[
                    on_line(&engine, 5, C, Side::New, 29, 4),
                    on_line(&engine, 2, C, Side::New, 30, 4),
                ])
                .unwrap();
            draw(&mut engine, SIZE);
            assert_eq!(top_row(&engine), on, "{what}");
            engine.set_insets(&[]).unwrap();
            draw(&mut engine, SIZE);
            let top = top_row(&engine);
            assert_eq!(
                (top.kind, top.new_line),
                (DiffRowKind::Code, Some(31)),
                "{what}"
            );
        }
    }
}

#[test]
fn at_the_start_of_the_diff_the_cursor_moves_up_only_by_what_was_removed() {
    let (mut engine, _directory) = open(LAYOUTS[0], false);
    draw(&mut engine, SIZE);
    engine
        .set_insets(&[on_line(&engine, 1, A, Side::New, 1, 5)])
        .unwrap();
    engine.reveal(A, Side::New, 3).unwrap();
    draw(&mut engine, SIZE);
    // Heading, hunk heading, line 1, five inset rows, line 2, line 3.
    assert_eq!(cursor_y(&engine), 10);
    engine.set_insets(&[]).unwrap();
    draw(&mut engine, SIZE);
    assert_eq!(cursor_y(&engine), 5);
    assert_eq!(cursor_line(&engine), 3);
    assert_eq!(app(&engine).top, Position::default());
}

type Reflow = (&'static str, fn(&mut Engine, &mut (u16, u16)));

/// Every kind of reflow, each undone by the next where it can be.
const REFLOWS: [Reflow; 14] = [
    ("a wider pane", |_, size| size.0 = 150),
    ("a narrower pane", |_, size| size.0 = 120),
    ("a taller pane", |_, size| size.1 = 34),
    ("a shorter pane", |_, size| size.1 = 26),
    ("the other layout", |engine, _| {
        engine.execute(Command::ToggleLayout).unwrap();
    }),
    ("the layout back", |engine, _| {
        engine.execute(Command::ToggleLayout).unwrap();
    }),
    ("a split ratio", |engine, _| {
        engine
            .set_split_ratio(SplitRatio::from_permille(700))
            .unwrap();
    }),
    ("a tab width", |engine, _| {
        engine.set_tab_width(TabWidth::new(8)).unwrap();
    }),
    ("whitespace hidden", |engine, _| {
        engine.set_whitespace(Whitespace::IgnoreAll).unwrap();
    }),
    ("whitespace shown", |engine, _| {
        engine.set_whitespace(Whitespace::Show).unwrap();
    }),
    ("no line numbers", |engine, _| {
        engine.execute(Command::ToggleNumbers).unwrap();
    }),
    ("line numbers", |engine, _| {
        engine.execute(Command::ToggleNumbers).unwrap();
    }),
    ("wrapping toggled", |engine, _| {
        engine.execute(Command::ToggleWrap).unwrap();
    }),
    ("wrapping toggled back", |engine, _| {
        engine.execute(Command::ToggleWrap).unwrap();
    }),
];

#[test]
fn every_reflow_with_insets_keeps_what_is_read() {
    for layout in LAYOUTS {
        for wrap in [false, true] {
            for at in [Where::NearTop, Where::Middle] {
                for on_inset in [false, true] {
                    let what = format!("{layout:?} wrap {wrap} {at:?} on an inset {on_inset}");
                    let (mut engine, _directory) = open(layout, wrap);
                    let mut size = SIZE;
                    draw(&mut engine, size);
                    place(&mut engine, size, at);
                    let line = cursor_line(&engine);
                    let set = [
                        on_line(&engine, 1, C, Side::New, line - 2, 3),
                        on_line(&engine, 2, C, Side::New, line + 2, 3),
                        on_line(&engine, 3, C, Side::New, line, 2),
                        on_heading(&engine, 4, B, 2),
                        on_line(&engine, 5, A, Side::New, 3, 2),
                    ];
                    set_keeping(&mut engine, size, &set, &what);
                    if on_inset {
                        down_to(&mut engine, 3, &what);
                        draw(&mut engine, size);
                    }
                    if !engine.options().wrap {
                        engine.execute(Command::Right).unwrap();
                        draw(&mut engine, size);
                    }
                    // The view before wrapping went off, which turning it back on returns to.
                    let mut wrapped: Option<Seen> = None;
                    for (reflow, apply) in REFLOWS {
                        let what = format!("{what}: {reflow}");
                        let wrapping = reflow.starts_with("wrapping");
                        let wrapping_off = wrapping && engine.options().wrap;
                        let mut before = seen(&engine);
                        let top = top_row(&engine);
                        apply(&mut engine, &mut size);
                        let now = seen_now(&mut engine, size);
                        if wrapping
                            && !wrapping_off
                            && let Some(wrapped) = wrapped.take()
                        {
                            before = wrapped;
                        }
                        if wrapping_off {
                            wrapped = Some(before.clone());
                            // Turning wrapping off keeps the top row at the top instead.
                            let mut expected = top.clone();
                            if expected.kind == DiffRowKind::Code {
                                expected.part = 0;
                            }
                            assert_eq!(top_row(&engine), expected, "{what}");
                        } else {
                            assert_eq!(now.y, before.y, "{what}");
                        }
                        assert_eq!(now.row.kind, before.row.kind, "{what}");
                        assert_eq!(now.row.new_line, before.row.new_line, "{what}");
                        if on_inset {
                            assert_eq!(now.row.part, before.row.part, "{what}");
                        }
                        if !engine.options().wrap && before.horizontal != 0 {
                            assert_eq!(now.horizontal, before.horizontal, "{what}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn insets_in_hunks_hidden_for_whitespace_come_back_with_them() {
    for layout in LAYOUTS {
        let what = format!("{layout:?}");
        let (mut engine, _directory) = open(layout, false);
        let size = (120, 200);
        draw(&mut engine, size);
        let set = [
            on_line(&engine, 1, B, Side::New, 2, 2),
            on_line(&engine, 2, B, Side::Old, 3, 1),
            on_line(&engine, 3, B, Side::New, 1, 1),
            on_heading(&engine, 4, B, 1),
            // Lines that are not in the diff, and a file that is not.
            on_line(&engine, 5, A, Side::New, 30, 1),
            on_line(&engine, 6, A, Side::Old, 999, 1),
            on_heading(&engine, 7, 9, 1),
            on_line(&engine, 8, 9, Side::New, 1, 1),
        ];
        engine.set_insets(&set).unwrap();
        draw(&mut engine, size);
        assert_eq!(engine.hidden_insets(), [5, 6, 7, 8], "{what}");
        let shown =
            |engine: &Engine| -> Vec<u64> { engine.inset_areas().iter().map(|a| a.id).collect() };
        // Unified the removed line 3 is above the added line 2; split they share rows.
        let order: &[u64] = if app(&engine).split() {
            &[4, 3, 1, 2]
        } else {
            &[4, 3, 2, 1]
        };
        assert_eq!(shown(&engine), order, "{what}");
        engine.set_whitespace(Whitespace::IgnoreAll).unwrap();
        draw(&mut engine, size);
        assert!(engine.hunk_is_whitespace_only(B, 0));
        assert_eq!(engine.hidden_insets(), [1, 2, 3, 5, 6, 7, 8], "{what}");
        assert_eq!(shown(&engine), [4], "{what}");
        // The hunk's heading keeps no rows for them.
        let heading = (0..size.1)
            .find(|&y| {
                engine
                    .row_at(2, y)
                    .is_some_and(|r| r.kind == DiffRowKind::Hunk(0) && r.file == B)
            })
            .unwrap();
        assert_eq!(engine.row_at(2, heading + 1).unwrap().file, C, "{what}");
        engine.set_whitespace(Whitespace::Show).unwrap();
        draw(&mut engine, size);
        assert_eq!(engine.hidden_insets(), [5, 6, 7, 8], "{what}");
        assert_eq!(shown(&engine), order, "{what}");
    }
}

#[test]
fn an_inset_taller_than_the_pane_scrolls_and_lets_the_cursor_through() {
    for layout in LAYOUTS {
        for wrap in [false, true] {
            let what = format!("{layout:?} wrap {wrap}");
            let (mut engine, _directory) = open(layout, wrap);
            let size = (120, 14);
            draw(&mut engine, size);
            engine
                .set_insets(&[on_line(&engine, 1, C, Side::New, 5, 40)])
                .unwrap();
            engine.reveal(C, Side::New, 5).unwrap();
            down_to(&mut engine, 1, &what);
            draw(&mut engine, size);
            // Down leaves it at once, and the next line is on screen.
            engine.execute(Command::Down).unwrap();
            draw(&mut engine, size);
            assert_eq!(cursor_line(&engine), 6, "{what}");
            cursor_y(&engine);
            engine.execute(Command::Up).unwrap();
            let row = engine.cursor_row().unwrap();
            assert_eq!(
                (row.kind, row.part),
                (DiffRowKind::Inset { id: 1 }, 39),
                "{what}"
            );
            draw(&mut engine, size);
            cursor_y(&engine);
            // Paging goes through it a page at a time.
            engine.execute(Command::Up).unwrap();
            assert_eq!(engine.cursor_row().unwrap().new_line, Some(5));
            let mut parts = Vec::new();
            for _ in 0..10 {
                engine.execute(Command::PageDown).unwrap();
                draw(&mut engine, size);
                cursor_y(&engine);
                let row = engine.cursor_row().unwrap();
                if row.kind != (DiffRowKind::Inset { id: 1 }) {
                    break;
                }
                parts.push(row.part);
            }
            assert!(
                parts.len() >= 3 && parts.windows(2).all(|w| w[0] < w[1]),
                "{what}: {parts:?}"
            );
            assert!(cursor_line(&engine) > 5, "{what}");
            // The wheel scrolls through it row by row.
            engine.reveal(C, Side::New, 5).unwrap();
            draw(&mut engine, size);
            let mut cut = Vec::new();
            for _ in 0..30 {
                mouse(&mut engine, MouseEventKind::ScrollDown, 5, 5);
                draw(&mut engine, size);
                match area(&engine, 1) {
                    Some(found) => cut.push(found.clipped_top),
                    None => break,
                }
            }
            assert!(
                cut.windows(2).all(|w| w[1] == w[0] + 3 || w[0] == 0),
                "{what}: {cut:?}"
            );
            assert!(cut.iter().any(|&c| c > 30), "{what}: {cut:?}");
        }
    }
}

#[test]
fn the_end_of_the_diff_counts_its_insets() {
    for layout in LAYOUTS {
        for wrap in [false, true] {
            let what = format!("{layout:?} wrap {wrap}");
            let (mut engine, _directory) = open(layout, wrap);
            let mut size = SIZE;
            draw(&mut engine, size);
            engine
                .set_insets(&[on_line(&engine, 1, C, Side::New, 60, 4)])
                .unwrap();
            engine.execute(Command::LastSource).unwrap();
            let bottom = |engine: &mut Engine, size: (u16, u16)| {
                draw(engine, size);
                engine.row_at(2, size.1 - 1).map(|r| (r.kind, r.part))
            };
            assert_eq!(
                bottom(&mut engine, size),
                Some((DiffRowKind::Inset { id: 1 }, 3))
            );
            // A taller pane and wrapping changes leave no empty rows under the last one.
            size.1 = 34;
            assert_eq!(
                bottom(&mut engine, size),
                Some((DiffRowKind::Inset { id: 1 }, 3)),
                "{what}"
            );
            engine.execute(Command::ToggleWrap).unwrap();
            assert_eq!(
                bottom(&mut engine, size),
                Some((DiffRowKind::Inset { id: 1 }, 3)),
                "{what}"
            );
        }
    }
}

#[test]
fn no_insets_draw_as_before() {
    for layout in LAYOUTS {
        for wrap in [false, true] {
            let what = format!("{layout:?} wrap {wrap}");
            let (mut engine, _directory) = open(layout, wrap);
            draw(&mut engine, SIZE);
            place(&mut engine, SIZE, Where::Middle);
            let before = draw(&mut engine, SIZE);
            engine.set_insets(&[]).unwrap();
            assert_eq!(draw(&mut engine, SIZE), before, "{what}");
            // Set and removed again, the view is as it was.
            let line = cursor_line(&engine);
            engine
                .set_insets(&[
                    on_line(&engine, 1, C, Side::New, line - 3, 4),
                    on_line(&engine, 2, C, Side::New, line, 2),
                    on_line(&engine, 3, C, Side::New, line + 1, 4),
                ])
                .unwrap();
            draw(&mut engine, SIZE);
            engine.set_insets(&[]).unwrap();
            assert_eq!(draw(&mut engine, SIZE), before, "{what}");
            // Insets of height 0 keep no rows and are not hidden.
            engine
                .set_insets(&[on_line(&engine, 1, C, Side::New, line, 0)])
                .unwrap();
            assert_eq!(draw(&mut engine, SIZE), before, "{what}");
            assert!(engine.hidden_insets().is_empty());
            assert!(engine.inset_areas().is_empty());
        }
    }
}

#[test]
fn insets_set_early_wait_for_their_files_and_go_with_the_review() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("review.patch");
    fs::write(&path, patch()).unwrap();
    let (wake, ready) = mpsc::sync_channel(1);
    let options = Options {
        layout: Layout::Unified,
        chrome: false,
        sidebar: SidebarPreference::Hide,
        ..Options::default()
    };
    let mut engine = Engine::open(Input::Patch(path.clone()), options, wake).unwrap();
    assert!(engine.app.is_none());
    let early = [on_line(&engine, 1, A, Side::New, 1, 2)];
    engine.set_insets(&early).unwrap();
    engine.set_inset_stops(false).unwrap();
    assert_eq!(engine.hidden_insets(), [1]);
    assert_eq!(engine.inset_width(A, Side::New), None);
    let other = Inset {
        review: ReviewId(u64::MAX),
        ..early[0]
    };
    assert!(engine.set_insets(&[other]).is_err());
    assert!(engine.set_insets(&[early[0], early[0]]).is_err());
    assert_eq!(engine.insets(), early);
    let deadline = Instant::now() + Duration::from_secs(10);
    while engine.status().loading {
        engine.poll().unwrap();
        assert!(Instant::now() < deadline);
        let _ = ready.recv_timeout(Duration::from_millis(20));
    }
    draw(&mut engine, SIZE);
    assert_eq!(engine.insets(), early);
    assert!(engine.hidden_insets().is_empty());
    let found = area(&engine, 1).unwrap();
    assert_eq!((found.area.y, found.area.height), (3, 2));
    assert!(engine.inset_width(A, Side::New).is_some());
    engine.replace(Input::Patch(path)).unwrap();
    assert!(engine.insets().is_empty());
    assert!(!engine.inset_stops(), "the setting stays");
    engine.close().unwrap();
    assert!(engine.set_insets(&[]).is_err());
}

#[test]
fn revealed_context_shows_its_insets_and_keeps_the_cursor_row() {
    let directory = tempfile::tempdir().unwrap();
    let old: String = (1..=80).map(|n| format!("line {n}\n")).collect();
    let mut new: Vec<String> = (1..=80).map(|n| format!("line {n}\n")).collect();
    new[9] = "line 10 changed\n".into();
    new[39] = "line 40 changed\n".into();
    new[69] = "line 70 changed\n".into();
    let old_path = directory.path().join("old.txt");
    let new_path = directory.path().join("new.txt");
    fs::write(&old_path, old).unwrap();
    fs::write(&new_path, new.concat()).unwrap();
    for layout in [Layout::Unified, Layout::Split] {
        let (wake, ready) = mpsc::sync_channel(1);
        let options = Options {
            layout,
            chrome: false,
            sidebar: SidebarPreference::Hide,
            ..Options::default()
        };
        let input = Input::Local(LocalInput::Files {
            old: old_path.clone(),
            new: new_path.clone(),
        });
        let mut engine = Engine::open(input, options, wake).unwrap();
        let size = (120, 30);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            draw(&mut engine, size);
            engine.poll().unwrap();
            if matches!(
                engine
                    .app
                    .as_ref()
                    .and_then(|app| app.sources.as_ref())
                    .and_then(|s| s.first()),
                Some(SourceState::Checked(_))
            ) && !engine.status().loading
            {
                break;
            }
            assert!(Instant::now() < deadline, "the source was not checked");
            let _ = ready.recv_timeout(Duration::from_millis(20));
        }
        // The first hunk shows lines 7 to 13; 15 and 25 are in the gap after it.
        let set = [
            on_line(&engine, 1, 0, Side::New, 8, 3),
            on_line(&engine, 2, 0, Side::New, 15, 2),
            on_line(&engine, 3, 0, Side::New, 25, 2),
        ];
        engine.set_insets(&set).unwrap();
        assert_eq!(engine.hidden_insets(), [2, 3]);
        engine.execute(Command::FirstSource).unwrap();
        for _ in 0..60 {
            draw(&mut engine, size);
            if engine.cursor_row().unwrap().kind == DiffRowKind::Gap {
                break;
            }
            engine.execute(Command::Down).unwrap();
        }
        assert_eq!(engine.cursor_row().unwrap().kind, DiffRowKind::Gap);
        draw(&mut engine, size);
        let y = cursor_y(&engine);
        // Twenty lines are revealed and the cursor goes to the last, on the gap's screen row.
        engine.execute(Command::Activate).unwrap();
        draw(&mut engine, size);
        assert_eq!(cursor_y(&engine), y, "{layout:?}");
        assert_eq!(cursor_line(&engine), 33, "{layout:?}");
        assert!(engine.hidden_insets().is_empty(), "{layout:?}");
        // The revealed line's inset hangs under it.
        let buffer = draw(&mut engine, size);
        if let Some(found) = area(&engine, 3) {
            let line = *rows_of(&engine, &buffer, 0, Side::New, 25).last().unwrap();
            assert_eq!(found.area.y, line + 1);
        }
        engine.close().unwrap();
    }
}
