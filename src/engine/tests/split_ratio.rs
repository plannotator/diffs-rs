//! `Engine::set_split_ratio` and the line between the sides: drawn where `split_line` says,
//! clicks and `row_at` on the side the line divides, `cursor_row` right at every ratio, wrapping
//! and horizontal scrolling measured per side, and nothing changed at the even ratio.

use super::rows::{LAYOUTS, check, draw, open, run, text};
use super::*;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

const RATIOS: [u16; 5] = [100, 300, 500, 618, 900];
const SIZES: [(u16, u16); 3] = [(70, 12), (121, 24), (170, 40)];

fn click(engine: &mut Engine, column: u16, row: u16) {
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    };
    engine.handle_event(Event::Mouse(event), true).unwrap();
}

/// Rows that show a split line of code: the cell at the line is the separator.
fn line_rows(buffer: &ratatui::buffer::Buffer, column: u16) -> Vec<u16> {
    (0..buffer.area.height)
        .filter(|&y| buffer[(column, y)].symbol() == "│")
        .collect()
}

#[test]
fn the_even_ratio_draws_every_frame_as_before_and_an_unset_ratio_is_even() {
    for (layout, wrap) in LAYOUTS {
        for size in SIZES {
            let (mut plain, _a) = open(layout, wrap);
            let (mut even, _b) = open(layout, wrap);
            assert_eq!(even.split_ratio(), SplitRatio::EVEN);
            even.set_split_ratio(SplitRatio::EVEN).unwrap();
            for step in 0..30 {
                let a = draw(&mut plain, size);
                let b = draw(&mut even, size);
                assert_eq!(a, b, "{layout:?} {wrap} {size:?} step {step}");
                plain.execute(Command::PageDown).unwrap();
                even.execute(Command::PageDown).unwrap();
            }
        }
    }
}

#[test]
fn rows_and_the_cursor_are_named_right_at_uneven_ratios() {
    for (layout, wrap) in LAYOUTS {
        for size in SIZES {
            for permille in RATIOS {
                let what = format!("{layout:?} {wrap} {size:?} {permille}");
                let (mut engine, _directory) = open(layout, wrap);
                engine
                    .set_split_ratio(SplitRatio::from_permille(permille))
                    .unwrap();
                let buffer = draw(&mut engine, size);
                check(&engine, &buffer, &format!("{what} first frame"));
                for n in 0..200 {
                    if !run(
                        &mut engine,
                        size,
                        Command::Down,
                        &format!("{what} down {n}"),
                    ) {
                        break;
                    }
                }
                for n in 0..12 {
                    run(
                        &mut engine,
                        size,
                        Command::PageUp,
                        &format!("{what} up {n}"),
                    );
                }
            }
        }
    }
}

#[test]
fn the_line_is_drawn_where_split_line_says_and_divides_clicks_and_row_at() {
    for wrap in [false, true] {
        for size in [(121, 30), (170, 40)] {
            for permille in RATIOS {
                let what = format!("{wrap} {size:?} {permille}");
                let (mut engine, _directory) = open(Layout::Split, wrap);
                engine
                    .set_split_ratio(SplitRatio::from_permille(permille))
                    .unwrap();
                engine.execute(Command::NextFile).unwrap();
                engine.execute(Command::NextFile).unwrap();
                engine.execute(Command::NextFile).unwrap();
                engine.execute(Command::NextFile).unwrap();
                engine.execute(Command::NextFile).unwrap();
                let buffer = draw(&mut engine, size);
                let line = engine.split_line().expect("split");
                let (old, new) = split::sides(size.0, SplitRatio::from_permille(permille));
                assert_eq!(usize::from(line), old, "{what}");
                let rows = line_rows(&buffer, line);
                assert!(rows.len() > 3, "{what}: the line is drawn on code rows");
                for &y in &rows {
                    // Nothing else on the row looks like the line.
                    let other = (0..size.0)
                        .filter(|&x| x != line && buffer[(x, y)].symbol() == "│")
                        .count();
                    assert_eq!(other, 0, "{what}: row {y} {:?}", text(&buffer, y));
                }
                let y = rows[rows.len() / 2];
                for column in 0..size.0 {
                    let expected = if usize::from(column) == old {
                        old < new
                    } else {
                        usize::from(column) < old
                    };
                    let side = if expected { Side::Old } else { Side::New };
                    let at = engine.row_at(column, y).expect("a row");
                    assert_eq!(at.side, side, "{what}: row_at column {column}");
                    click(&mut engine, column, y);
                    assert_eq!(
                        engine.cursor_row().unwrap().side,
                        side,
                        "{what}: a click on column {column}"
                    );
                    let buffer = draw(&mut engine, size);
                    check(&engine, &buffer, &format!("{what} click {column}"));
                }
            }
        }
    }
}

#[test]
fn the_ratio_at_a_column_puts_the_line_there_and_stays_within_the_limits() {
    let (mut engine, _directory) = open(Layout::Split, false);
    assert_eq!(engine.split_line(), None, "before the first frame");
    assert_eq!(engine.split_ratio_at(60), None);
    let size = (160, 30);
    draw(&mut engine, size);
    let least = SplitRatio::MIN_SIDE_WIDTH;
    for column in 0..size.0 {
        let ratio = engine.split_ratio_at(column).unwrap();
        engine.set_split_ratio(ratio).unwrap();
        draw(&mut engine, size);
        let expected = column.clamp(least, size.0 - 1 - least);
        assert_eq!(engine.split_line(), Some(expected), "column {column}");
    }
    let middle = (size.0 - 1) / 2;
    assert_eq!(engine.split_ratio_at(middle), Some(SplitRatio::EVEN));
    // A ratio past what the pane can draw is kept as set and drawn at the limit.
    engine.set_split_ratio(SplitRatio::MAX).unwrap();
    draw(&mut engine, (60, 30));
    assert_eq!(engine.split_ratio(), SplitRatio::MAX);
    assert_eq!(engine.split_line(), Some(59 - least));
    draw(&mut engine, (40, 30));
    assert_eq!(engine.split_line(), Some(19), "too narrow: even");
    assert_eq!(engine.split_ratio_at(5), None, "and the line cannot move");
    draw(&mut engine, (201, 30));
    assert_eq!(engine.split_line(), Some(180));
}

#[test]
fn unified_ignores_the_ratio_and_split_takes_it_back() {
    for wrap in [false, true] {
        let (mut plain, _a) = open(Layout::Unified, wrap);
        let (mut set, _b) = open(Layout::Unified, wrap);
        set.set_split_ratio(SplitRatio::from_permille(250)).unwrap();
        for step in 0..10 {
            assert_eq!(
                draw(&mut plain, (160, 30)),
                draw(&mut set, (160, 30)),
                "{wrap} {step}"
            );
            assert_eq!(set.split_line(), None);
            plain.execute(Command::PageDown).unwrap();
            set.execute(Command::PageDown).unwrap();
        }
        set.execute(Command::ToggleLayout).unwrap();
        draw(&mut set, (160, 30));
        assert_eq!(
            set.split_line(),
            Some(split::sides(160, SplitRatio::from_permille(250)).0 as u16)
        );
    }
}

#[test]
fn the_cursors_line_keeps_its_screen_row_when_the_ratio_changes() {
    for wrap in [false, true] {
        let size = (160, 40);
        let (mut engine, _directory) = open(Layout::Split, wrap);
        draw(&mut engine, size);
        for _ in 0..5 {
            engine.execute(Command::NextFile).unwrap();
        }
        for _ in 0..6 {
            engine.execute(Command::Down).unwrap();
        }
        draw(&mut engine, size);
        let screen_row = |engine: &Engine| engine.app.as_ref().unwrap().cursor_screen_row;
        let source = |engine: &Engine| engine.selected_source().unwrap();
        let (row, at) = (screen_row(&engine), source(&engine));
        assert!(at.is_some(), "{wrap}: on a line");
        for permille in [100, 900, 333, 500, 777] {
            engine
                .set_split_ratio(SplitRatio::from_permille(permille))
                .unwrap();
            let buffer = draw(&mut engine, size);
            check(&engine, &buffer, &format!("{wrap} {permille}"));
            assert_eq!(source(&engine), at, "{wrap} {permille}: the same line");
            assert_eq!(screen_row(&engine), row, "{wrap} {permille}: on its row");
        }
        // Many changes before one frame end where one change would.
        let (mut burst, _b) = open(Layout::Split, wrap);
        let (mut once, _c) = open(Layout::Split, wrap);
        for engine in [&mut burst, &mut once] {
            draw(engine, size);
            engine.execute(Command::PageDown).unwrap();
            draw(engine, size);
        }
        for permille in (100..=900).step_by(7) {
            burst
                .set_split_ratio(SplitRatio::from_permille(permille))
                .unwrap();
        }
        once.set_split_ratio(SplitRatio::from_permille(898))
            .unwrap();
        burst
            .set_split_ratio(SplitRatio::from_permille(898))
            .unwrap();
        assert_eq!(draw(&mut burst, size), draw(&mut once, size), "{wrap}");
    }
}

#[test]
fn each_side_scrolls_to_its_own_widest_line() {
    let wide_old = format!("old {}", "o".repeat(150));
    let wide_new = format!("new {}", "n".repeat(150));
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("wide.patch");
    fs::write(
        &path,
        format!(
            "diff --git a/w.txt b/w.txt\n--- a/w.txt\n+++ b/w.txt\n@@ -1,2 +1,2 @@\n-{wide_old}END\n+{wide_new}FIN\n same\n"
        ),
    )
    .unwrap();
    for permille in [150, 500, 850] {
        let (wake, ready) = mpsc::sync_channel(1);
        let options = Options {
            layout: Layout::Split,
            chrome: false,
            sidebar: SidebarPreference::Hide,
            ..Options::default()
        };
        let mut engine = Engine::open(Input::Patch(path.clone()), options, wake).unwrap();
        while engine.status().loading {
            engine.poll().unwrap();
            let _ = ready.recv_timeout(Duration::from_millis(20));
        }
        engine
            .set_split_ratio(SplitRatio::from_permille(permille))
            .unwrap();
        let size = (160, 12);
        draw(&mut engine, size);
        engine.execute(Command::Down).unwrap();
        for _ in 0..80 {
            engine.execute(Command::Right).unwrap();
        }
        let buffer = draw(&mut engine, size);
        let line = engine.split_line().unwrap();
        let row = (0..size.1)
            .find(|&y| buffer[(line, y)].symbol() == "│")
            .expect("the changed row");
        let left: String = (0..line).map(|x| buffer[(x, row)].symbol()).collect();
        let right: String = (line + 1..size.0)
            .map(|x| buffer[(x, row)].symbol())
            .collect();
        // The narrower side reaches its line's end; the wider one shows as far as the shared
        // offset takes it, never past its own end.
        let narrow_is_old = permille < 500;
        let (narrow, wide) = if narrow_is_old {
            (&left, &right)
        } else {
            (&right, &left)
        };
        let end = if narrow_is_old { "END" } else { "FIN" };
        assert!(
            narrow.trim_end().ends_with(end),
            "{permille}: {narrow:?} | {wide:?}"
        );
        if permille == 500 {
            assert!(left.trim_end().ends_with("END") && right.trim_end().ends_with("FIN"));
        }
        let buffer = draw(&mut engine, size);
        check(&engine, &buffer, &format!("scrolled {permille}"));
    }
}

#[test]
fn the_ratio_is_kept_from_before_the_first_file_and_across_replace() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("one.patch");
    fs::write(
        &path,
        "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-before\n+after\n",
    )
    .unwrap();
    let (wake, ready) = mpsc::sync_channel(1);
    let options = Options {
        layout: Layout::Split,
        chrome: false,
        sidebar: SidebarPreference::Hide,
        ..Options::default()
    };
    let mut engine = Engine::open(Input::Patch(path.clone()), options, wake).unwrap();
    let ratio = SplitRatio::from_permille(300);
    engine.set_split_ratio(ratio).unwrap();
    let wait = |engine: &mut Engine| {
        while engine.status().loading {
            engine.poll().unwrap();
            let _ = ready.recv_timeout(Duration::from_millis(20));
        }
    };
    wait(&mut engine);
    draw(&mut engine, (121, 12));
    assert_eq!(engine.split_line(), Some(36));
    engine.replace(Input::Patch(path)).unwrap();
    assert_eq!(engine.split_ratio(), ratio);
    wait(&mut engine);
    draw(&mut engine, (121, 12));
    assert_eq!(engine.split_line(), Some(36));
}

#[test]
fn set_layout_shows_each_layout_and_auto_follows_the_width() {
    let (mut engine, _directory) = open(Layout::Split, false);
    draw(&mut engine, (160, 30));
    assert!(engine.split_line().is_some());
    engine.set_layout(Layout::Unified).unwrap();
    assert!(engine.needs_draw());
    let buffer = draw(&mut engine, (160, 30));
    check(&engine, &buffer, "unified");
    assert_eq!(engine.options().layout, Layout::Unified);
    assert_eq!(engine.split_line(), None);
    engine.set_layout(Layout::Auto).unwrap();
    let buffer = draw(&mut engine, (160, 30));
    check(&engine, &buffer, "auto wide");
    assert!(engine.split_line().is_some());
    let buffer = draw(&mut engine, (100, 30));
    check(&engine, &buffer, "auto narrow");
    assert_eq!(engine.split_line(), None);
    assert_eq!(engine.options().layout, Layout::Auto);
}
