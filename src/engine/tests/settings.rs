//! `Engine::set_tab_width`, `Engine::set_inline_changes` and `Engine::set_whitespace`: what each
//! draws, `row_at` and `cursor_row` on every row after each change, in both layouts with and
//! without wrapping and at an uneven split ratio, the cursor and a selection across changes,
//! bursts of changes and panes too small to draw.

use super::rows::{LAYOUTS, check, draw, text};
use super::*;
use crate::{
    highlight::{ChangeSide, changed_spans_by},
    reading::SearchDirection,
};
use ratatui::{buffer::Buffer, style::Modifier};

const CORPUS: &str = "fixtures/options/whitespace.patch";
const SIZES: [(u16, u16); 3] = [(64, 14), (121, 30), (170, 60)];
const RATIOS: [u16; 2] = [500, 300];
const WHITESPACE: [Whitespace; 3] = [
    Whitespace::Show,
    Whitespace::IgnoreAll,
    Whitespace::IgnoreChange,
];
const INLINE: [InlineChanges; 4] = [
    InlineChanges::Word,
    InlineChanges::WordAlt,
    InlineChanges::Char,
    InlineChanges::None,
];

struct Review {
    engine: Engine,
    ready: mpsc::Receiver<()>,
    _directory: Option<tempfile::TempDir>,
}

fn open_with(
    input: Input,
    layout: Layout,
    wrap: bool,
    directory: Option<tempfile::TempDir>,
) -> Review {
    open_set(input, layout, wrap, directory, |_| {})
}

/// Opens `input`, calling `set` on the engine before any file has arrived.
fn open_set(
    input: Input,
    layout: Layout,
    wrap: bool,
    directory: Option<tempfile::TempDir>,
    set: impl FnOnce(&mut Engine),
) -> Review {
    let (wake, ready) = mpsc::sync_channel(1);
    let options = Options {
        layout,
        wrap,
        chrome: false,
        sidebar: SidebarPreference::Hide,
        ..Options::default()
    };
    let mut engine = Engine::open(input, options, wake).unwrap();
    assert!(engine.app.is_none());
    set(&mut engine);
    let mut review = Review {
        engine,
        ready,
        _directory: directory,
    };
    review.load();
    review
}

fn open(layout: Layout, wrap: bool) -> Review {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CORPUS);
    open_with(Input::Patch(path), layout, wrap, None)
}

impl Review {
    fn load(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while self.engine.status().loading {
            self.engine.poll().unwrap();
            assert!(Instant::now() < deadline, "the review did not load");
            if self.engine.status().loading {
                let _ = self.ready.recv_timeout(Duration::from_millis(20));
            }
        }
        assert!(self.engine.status().error.is_none());
    }

    /// Draws until the visible rows are colored.
    fn settle(&mut self, size: (u16, u16)) -> Buffer {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let buffer = draw(&mut self.engine, size);
            let status = self.engine.status();
            if !status.coloring && !status.preparing_source {
                self.engine.poll().unwrap();
                if !self.engine.needs_draw() {
                    return buffer;
                }
                continue;
            }
            assert!(Instant::now() < deadline, "colors did not arrive");
            let _ = self.ready.recv_timeout(Duration::from_millis(20));
            self.engine.poll().unwrap();
        }
    }

    fn app(&self) -> &App {
        self.engine.app.as_ref().unwrap()
    }

    /// Moves the cursor down to the first row whose line on `side` is `line` in `file`.
    fn cursor_to(&mut self, size: (u16, u16), file: usize, side: Side, line: u32) {
        self.engine.execute(Command::FirstSource).unwrap();
        for _ in 0..400 {
            draw(&mut self.engine, size);
            let row = self.engine.cursor_row().unwrap();
            let on = if side == Side::Old {
                row.old_line
            } else {
                row.new_line
            };
            if row.file == file && on == Some(line) && row.part == 0 {
                // Off the bottom row, which reflowing avoids.
                let mut down = 0;
                while down < 4
                    && self.engine.execute(Command::Down).unwrap() != InputEffect::Ignored
                {
                    draw(&mut self.engine, size);
                    down += 1;
                }
                for _ in 0..down {
                    self.engine.execute(Command::Up).unwrap();
                    draw(&mut self.engine, size);
                }
                assert_eq!(self.engine.cursor_row().unwrap(), row);
                if self.app().split() && row.side != side {
                    self.engine.execute(Command::SwitchSide).unwrap();
                }
                draw(&mut self.engine, size);
                return;
            }
            if self.engine.execute(Command::Down).unwrap() == InputEffect::Ignored {
                break;
            }
        }
        panic!("no row shows {side:?} line {line} of file {file}");
    }

    /// Walks the cursor down through every row and back up, checking each frame.
    fn walk(&mut self, size: (u16, u16), what: &str) {
        self.walk_by(size, what, check_rows);
    }

    fn walk_by(&mut self, size: (u16, u16), what: &str, check: fn(&Engine, &Buffer, &str)) {
        for (command, name) in [(Command::Down, "down"), (Command::Up, "up")] {
            for n in 0..600 {
                let moved = self.engine.execute(command).unwrap() != InputEffect::Ignored;
                let buffer = draw(&mut self.engine, size);
                check(&self.engine, &buffer, &format!("{what} {name} {n}"));
                if !moved {
                    break;
                }
            }
        }
    }

    /// The cursor is on screen row `before` again, as reflowing keeps it: it may move by one
    /// next to the sticky heading, and up when the rows above it no longer fill the screen.
    fn assert_kept(&self, before: u16, what: &str) {
        let now = self.cursor_screen_row();
        if self.app().top == crate::app::Position::default() {
            assert!(now <= before + 1, "{what}: {now} from {before}");
        } else {
            assert!(now.abs_diff(before) <= 1, "{what}: {now} from {before}");
        }
    }

    /// The screen row of the cursor in the last frame.
    fn cursor_screen_row(&self) -> u16 {
        let app = self.app();
        app.visible
            .iter()
            .find(|(position, _)| *position == app.cursor)
            .map(|(_, y)| *y)
            .expect("the cursor is drawn")
    }
}

fn check_rows(engine: &Engine, buffer: &Buffer, what: &str) {
    check(engine, buffer, what);
}

/// `check` for a review of two files, whose heading is not a path: every drawn row has a
/// row, and the cursor's row is the one drawn under it.
fn check_local(engine: &Engine, buffer: &Buffer, what: &str) {
    let mut drawn = Vec::new();
    for y in 0..buffer.area.height {
        let line = text(buffer, y);
        let at = engine.row_at(buffer.area.width / 4, y);
        if line.trim().is_empty() {
            drawn.push(at);
            continue;
        }
        let row = at
            .clone()
            .unwrap_or_else(|| panic!("{what}: row {y} {line:?}"));
        assert_eq!(row.file, 0, "{what}");
        if line.contains("whitespace only") {
            assert!(matches!(row.kind, DiffRowKind::Hunk(_)), "{what}: row {y}");
        }
        if row.kind == DiffRowKind::Code {
            assert!(
                row.old_line.is_some() || row.new_line.is_some(),
                "{what}: row {y}"
            );
        }
        drawn.push(at);
    }
    let app = engine.app.as_ref().unwrap();
    let cursor = engine.cursor_row().expect("the cursor's row");
    if let Some(&(position, at)) = app.visible.get(app.cursor_screen_row)
        && position == app.cursor
    {
        let row = drawn[usize::from(at - 1)].clone().expect("drawn");
        assert_eq!(
            DiffRow {
                side: cursor.side,
                ..row
            },
            cursor,
            "{what}: the cursor's row"
        );
    }
}

const MIXED: usize = 4;
const LONG: usize = 3;
const UNICODE: usize = 5;

// ---- Tab width -------------------------------------------------------------------------

/// The column where `needle` starts on row `y`, counted in cells.
fn column_of(buffer: &Buffer, y: u16, from: u16, needle: &str) -> Option<u16> {
    let cells: Vec<&str> = (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect();
    let needle: Vec<String> = needle.chars().map(String::from).collect();
    (usize::from(from)..cells.len().saturating_sub(needle.len()))
        .find(|&x| needle.iter().enumerate().all(|(n, c)| cells[x + n] == c))
        .map(|x| x as u16)
}

#[test]
fn tab_width_moves_every_tab_stop_on_both_sides() {
    for layout in [Layout::Split, Layout::Unified] {
        for width in [1u8, 2, 3, 4, 5, 8, 16] {
            let mut review = open(layout, false);
            review.engine.set_tab_width(TabWidth::new(width)).unwrap();
            assert_eq!(review.engine.tab_width().get(), width);
            let size = (200, 60);
            review.cursor_to(size, LONG, Side::New, 1);
            let buffer = draw(&mut review.engine, size);
            // `col0` ends at column 4; the tab after it ends at the next multiple of the width.
            let stop = 4 + usize::from(width) - 4 % usize::from(width);
            let mut seen = 0;
            for y in 0..buffer.area.height {
                for start in ["col0", "COL0"] {
                    if let Some(x) = column_of(&buffer, y, 0, start) {
                        let next = column_of(&buffer, y, x, "col1").unwrap();
                        assert_eq!(usize::from(next - x), stop, "{layout:?} tab {width}");
                        if layout == Layout::Split {
                            // The other side of the same row measures the same way.
                            let other = if start == "col0" { "COL0" } else { "col0" };
                            let x = column_of(&buffer, y, 0, other).unwrap();
                            let next = column_of(&buffer, y, x, "col1").unwrap();
                            assert_eq!(usize::from(next - x), stop);
                        }
                        seen += 1;
                    }
                }
            }
            assert_eq!(seen, 2, "{layout:?} tab {width}: both lines of long.tsv");
            // Scrolling reaches the end of the line at every width and no further.
            let mut steps = 0;
            while review.engine.execute(Command::Right).unwrap() != InputEffect::Ignored {
                let app = review.app();
                if app.horizontal >= crate::render::horizontal_limit(app).unwrap() {
                    break;
                }
                steps += 1;
                assert!(steps < 1000);
            }
            let buffer = draw(&mut review.engine, size);
            let shown: String = (0..buffer.area.height).map(|y| text(&buffer, y)).collect();
            assert!(
                shown.contains("col59"),
                "{layout:?} tab {width}: the end of the line"
            );
            check(
                &review.engine,
                &buffer,
                &format!("{layout:?} tab {width} scrolled"),
            );
        }
    }
}

#[test]
fn tab_width_keeps_emphasis_and_search_matches_on_their_characters() {
    for layout in [Layout::Split, Layout::Unified] {
        for wrap in [false, true] {
            for width in [1u8, 4, 7, 16] {
                let what = format!("{layout:?} {wrap} tab {width}");
                let mut review = open(layout, wrap);
                review.engine.set_tab_width(TabWidth::new(width)).unwrap();
                // Wide enough that `COL7` is on screen at 16 cells a tab on a split side.
                let size = (260, 60);
                review.cursor_to(size, LONG, Side::New, 1);
                let buffer = review.settle(size);
                let colors = review.app().colors.clone();
                // The changed word `COL7` has the added-word background, and the tab before it
                // the added-line background.
                let y = (0..buffer.area.height)
                    .find(|&y| column_of(&buffer, y, 0, "COL7").is_some())
                    .unwrap_or_else(|| panic!("{what}: COL7 is drawn"));
                let x = column_of(&buffer, y, 0, "COL7").unwrap();
                for n in 0..4 {
                    assert_eq!(buffer[(x + n, y)].bg, colors.add_word, "{what}");
                }
                assert_eq!(
                    buffer[(x - 1, y)].bg,
                    colors.add_bg,
                    "{what}: the tab before"
                );
                // A search match is underlined on its own characters.
                let search = review.engine.search_read("col9".into()).unwrap();
                let found = search
                    .find(None, SearchDirection::Forward, &|| Ok(()))
                    .unwrap()
                    .unwrap();
                review.engine.show_search_match(&found).unwrap();
                let buffer = review.settle(size);
                let underlined: Vec<(u16, u16)> = (0..buffer.area.height)
                    .flat_map(|y| (0..buffer.area.width).map(move |x| (x, y)))
                    .filter(|&(x, y)| {
                        buffer[(x, y)]
                            .modifier
                            .contains(Modifier::BOLD | Modifier::UNDERLINED)
                    })
                    .collect();
                let symbols: String = underlined.iter().map(|&p| buffer[p].symbol()).collect();
                assert_eq!(symbols, "col9", "{what}");
                check(&review.engine, &buffer, &what);
            }
        }
    }
}

#[test]
fn tab_width_keeps_rows_and_the_cursor_right() {
    for (layout, wrap) in LAYOUTS {
        for size in SIZES {
            for permille in RATIOS {
                let what = format!("{layout:?} {wrap} {size:?} {permille}");
                let mut review = open(layout, wrap);
                review
                    .engine
                    .set_split_ratio(SplitRatio::from_permille(permille))
                    .unwrap();
                draw(&mut review.engine, size);
                review.cursor_to(size, MIXED, Side::New, 8);
                let before = review.cursor_screen_row();
                for width in [16u8, 1, 8, 3] {
                    review.engine.set_tab_width(TabWidth::new(width)).unwrap();
                    let buffer = draw(&mut review.engine, size);
                    check(&review.engine, &buffer, &format!("{what} tab {width}"));
                    let row = review.engine.cursor_row().unwrap();
                    assert_eq!((row.file, row.new_line), (MIXED, Some(8)), "{what}");
                    review.assert_kept(before, &format!("{what} tab {width}"));
                }
                review.walk(size, &what);
            }
        }
    }
}

#[test]
fn tab_width_is_kept_between_its_limits_before_files_and_across_replace() {
    assert_eq!(TabWidth::new(0), TabWidth::MIN);
    assert_eq!(TabWidth::new(17), TabWidth::MAX);
    assert_eq!(TabWidth::new(u8::MAX).get(), 16);
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CORPUS);
    let (wake, _ready) = mpsc::sync_channel(1);
    let mut engine = Engine::open(Input::Patch(path.clone()), Options::default(), wake).unwrap();
    assert_eq!(engine.tab_width(), TabWidth::DEFAULT);
    assert_eq!(engine.inline_changes(), InlineChanges::Word);
    assert_eq!(engine.whitespace(), Whitespace::Show);
    engine.set_tab_width(TabWidth::new(8)).unwrap();
    engine.set_inline_changes(InlineChanges::Char).unwrap();
    engine.set_whitespace(Whitespace::IgnoreAll).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while engine.status().loading {
        engine.poll().unwrap();
        assert!(Instant::now() < deadline);
    }
    let app = engine.app.as_ref().unwrap();
    assert_eq!(app.geometry.tab_width(), 8);
    assert_eq!(app.inline_changes, InlineChanges::Char);
    assert_eq!(app.whitespace, Whitespace::IgnoreAll);
    engine.replace(Input::Patch(path)).unwrap();
    assert_eq!(engine.tab_width().get(), 8);
    assert_eq!(engine.inline_changes(), InlineChanges::Char);
    assert_eq!(engine.whitespace(), Whitespace::IgnoreAll);
    engine.close().unwrap();
}

// ---- Inline changes --------------------------------------------------------------------

#[test]
fn inline_modes_emphasize_what_each_compares() {
    // Lines of unicode.txt with no tab and no wide character, so a cell is a character.
    let pairs = [
        ("let café = \"naïve\";", "let cafe = \"naive\";"),
        ("old_value + 14;", "new_value + 14;"),
        ("foo(bar, baz);", "foo(bar,  qux);"),
    ];
    for layout in [Layout::Split, Layout::Unified] {
        for wrap in [false, true] {
            for mode in INLINE {
                let what = format!("{layout:?} {wrap} {mode:?}");
                let mut review = open(layout, wrap);
                review.engine.set_inline_changes(mode).unwrap();
                assert_eq!(review.engine.inline_changes(), mode);
                let size = (170, 60);
                review.cursor_to(size, UNICODE, Side::New, 5);
                let buffer = review.settle(size);
                let colors = review.app().colors.clone();
                for (old, new) in pairs {
                    for (text_of, side, word) in [
                        (old, ChangeSide::Old, colors.del_word),
                        (new, ChangeSide::New, colors.add_word),
                    ] {
                        let spans = changed_spans_by(old, new, side, mode);
                        let (y, x) = (0..buffer.area.height)
                            .find_map(|y| column_of(&buffer, y, 0, text_of).map(|x| (y, x)))
                            .unwrap_or_else(|| panic!("{what}: {text_of:?} is drawn"));
                        for (cell, (byte, character)) in (x..).zip(text_of.char_indices()) {
                            let emphasized = spans.iter().any(|span| span.contains(&byte));
                            assert_eq!(
                                buffer[(cell, y)].bg == word,
                                emphasized,
                                "{what}: {text_of:?} at {character:?}"
                            );
                        }
                    }
                }
                if mode == InlineChanges::None {
                    let any = buffer
                        .content
                        .iter()
                        .any(|cell| cell.bg == colors.add_word || cell.bg == colors.del_word);
                    assert!(!any, "{what}: nothing emphasized");
                }
                check(&review.engine, &buffer, &what);
            }
        }
    }
}

#[test]
fn a_burst_of_inline_mode_changes_ends_as_a_review_opened_with_the_last() {
    for (layout, wrap) in LAYOUTS {
        let size = (121, 40);
        let mut review = open(layout, wrap);
        review.cursor_to(size, UNICODE, Side::New, 2);
        review.settle(size);
        let mut last = InlineChanges::Word;
        for n in 0..23 {
            last = INLINE[(n * 7 + 3) % INLINE.len()];
            review.engine.set_inline_changes(last).unwrap();
            if n % 3 == 0 {
                draw(&mut review.engine, size);
                review.engine.poll().unwrap();
            }
        }
        let burst = review.settle(size);
        let mut fresh = open(layout, wrap);
        fresh.engine.set_inline_changes(last).unwrap();
        fresh.cursor_to(size, UNICODE, Side::New, 2);
        let expected = fresh.settle(size);
        assert_eq!(burst, expected, "{layout:?} {wrap} {last:?}");
    }
}

// ---- Whitespace ------------------------------------------------------------------------

fn screen(buffer: &Buffer) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| text(buffer, y).trim_end().to_string())
        .collect()
}

#[test]
fn hidden_whitespace_draws_pairs_unchanged_and_whitespace_only_hunks_as_headings() {
    let size = (120, 120);
    for (whitespace, value_paired, spaced_hidden) in [
        (Whitespace::Show, false, false),
        (Whitespace::IgnoreAll, true, true),
        (Whitespace::IgnoreChange, false, false),
    ] {
        let what = format!("{whitespace:?}");
        let mut unified = open(Layout::Unified, false);
        unified.engine.set_whitespace(whitespace).unwrap();
        assert_eq!(unified.engine.whitespace(), whitespace);
        let buffer = unified.settle(size);
        let rows = screen(&buffer);
        let has = |line: &str| rows.iter().any(|row| row == line);
        // indent.rs only changes indentation.
        assert_eq!(
            has(" @@ -1,6 +1,6 @@  ·  whitespace only"),
            whitespace != Whitespace::Show,
            "{what}"
        );
        // A tab against spaces, and trailing whitespace, are hidden by -b and -w.
        assert_eq!(
            has("   6   6    trailing"),
            whitespace != Whitespace::Show,
            "{what}"
        );
        assert_eq!(
            has("   8   8          indented    columns here"),
            whitespace != Whitespace::Show,
            "{what}"
        );
        // `compute(a,b)` against `compute(a, b)` only by -w.
        assert_eq!(
            has("   5   5  value = compute(a, b);"),
            value_paired,
            "{what}"
        );
        assert_eq!(
            has(" @@ -23,8 +23,8 @@ line 22  ·  whitespace only"),
            spaced_hidden,
            "{what}"
        );
        // Real changes and unpaired blank lines stay.
        assert!(has("   7    − real old change"), "{what}");
        assert!(has("       7+ real new change"), "{what}");
        assert!(has("       2+"), "{what}: a blank line added");
        // The interleaved block pairs `  one` with `one` and keeps `two` → `TWO`.
        assert_eq!(
            has("   2   2  one"),
            whitespace == Whitespace::IgnoreAll,
            "{what}"
        );
        assert!(has("   3    − two"), "{what}");
        // Paired rows look unchanged: the context background and no sign.
        let colors = unified.app().colors.clone();
        if let Some(y) = rows.iter().position(|row| row == "   6   6    trailing") {
            let y = y as u16;
            assert_eq!(buffer[(20, y)].bg, colors.context, "{what}");
            assert_eq!(buffer[(9, y)].symbol(), " ", "{what}");
        }
        check(&unified.engine, &buffer, &what);
        // File counts are the patch's.
        assert_eq!(unified.engine.file(MIXED).unwrap().additions, 9);

        let mut split = open(Layout::Split, false);
        split.engine.set_whitespace(whitespace).unwrap();
        let buffer = split.settle(size);
        let rows = screen(&buffer);
        let pair = rows
            .iter()
            .position(|row| row.starts_with("   6") && row.contains("trailing"))
            .unwrap_or_else(|| panic!("{what}: the trailing pair"));
        let y = pair as u16;
        let paired = whitespace != Whitespace::Show;
        // Each side shows its own text; a pair has neither sign.
        assert_eq!(buffer[(4, y)].symbol() == "−", !paired, "{what}");
        assert_eq!(buffer[(10, y)].bg == colors.context, paired, "{what}");
        check(&split.engine, &buffer, &what);
    }
}

#[test]
fn rows_and_the_cursor_stay_right_after_every_whitespace_change() {
    for (layout, wrap) in LAYOUTS {
        for size in SIZES {
            for permille in RATIOS {
                let mut review = open(layout, wrap);
                review
                    .engine
                    .set_split_ratio(SplitRatio::from_permille(permille))
                    .unwrap();
                draw(&mut review.engine, size);
                for whitespace in [
                    Whitespace::IgnoreAll,
                    Whitespace::IgnoreChange,
                    Whitespace::Show,
                    Whitespace::IgnoreChange,
                ] {
                    let what = format!("{layout:?} {wrap} {size:?} {permille} {whitespace:?}");
                    review.engine.set_whitespace(whitespace).unwrap();
                    let buffer = draw(&mut review.engine, size);
                    check(&review.engine, &buffer, &what);
                    review.walk(size, &what);
                }
            }
        }
    }
}

#[test]
fn the_cursor_on_a_hidden_line_goes_to_its_heading_and_comes_back() {
    for (layout, wrap) in LAYOUTS {
        for size in SIZES {
            let what = format!("{layout:?} {wrap} {size:?}");
            let mut review = open(layout, wrap);
            // `spaced  out`, old line 26 of mixed.txt, in a hunk of whitespace changes only.
            review.cursor_to(size, MIXED, Side::Old, 26);
            let screen_row = review.cursor_screen_row();
            review.engine.set_whitespace(Whitespace::IgnoreAll).unwrap();
            let buffer = draw(&mut review.engine, size);
            check(&review.engine, &buffer, &what);
            let row = review.engine.cursor_row().unwrap();
            assert_eq!(
                (row.file, row.kind),
                (MIXED, DiffRowKind::Hunk(1)),
                "{what}"
            );
            assert!(review.engine.selected_source().unwrap().is_none());
            review.assert_kept(screen_row, &what);
            // The line is not displayed, so a host cannot reveal or select it.
            assert!(review.engine.reveal(MIXED, Side::Old, 26).is_err());
            let range = SourceRange {
                review: review.engine.review_id(),
                file: MIXED,
                side: Side::Old,
                first: 26,
                last: 27,
            };
            assert!(review.engine.select_range(range).is_err());
            // Showing whitespace again puts the cursor back on the line.
            review.engine.set_whitespace(Whitespace::Show).unwrap();
            let buffer = draw(&mut review.engine, size);
            check(&review.engine, &buffer, &what);
            let row = review.engine.cursor_row().unwrap();
            assert_eq!((row.file, row.old_line), (MIXED, Some(26)), "{what}");
            review.assert_kept(screen_row, &what);
            review.engine.reveal(MIXED, Side::Old, 26).unwrap();
        }
    }
}

#[test]
fn the_cursor_on_a_paired_line_stays_on_the_row_that_shows_it() {
    for (layout, wrap) in LAYOUTS {
        for size in SIZES {
            for side in [Side::Old, Side::New] {
                let what = format!("{layout:?} {wrap} {size:?} {side:?}");
                let mut review = open(layout, wrap);
                // `value\t= compute(a,b);` → `value = compute(a, b);`, lines 5 of mixed.txt.
                review.cursor_to(size, MIXED, side, 5);
                let screen_row = review.cursor_screen_row();
                review.engine.set_whitespace(Whitespace::IgnoreAll).unwrap();
                let buffer = draw(&mut review.engine, size);
                check(&review.engine, &buffer, &what);
                let row = review.engine.cursor_row().unwrap();
                assert_eq!(row.kind, DiffRowKind::Code, "{what}");
                assert_eq!((row.old_line, row.new_line), (Some(5), Some(5)), "{what}");
                let location = review.engine.selected_source().unwrap().unwrap();
                assert_eq!(
                    (location.old_line, location.new_line),
                    (Some(5), Some(5)),
                    "{what}: a pair has both numbers in either layout"
                );
                review.assert_kept(screen_row, &what);
                review
                    .engine
                    .set_whitespace(Whitespace::IgnoreChange)
                    .unwrap();
                let buffer = draw(&mut review.engine, size);
                check(&review.engine, &buffer, &what);
                let row = review.engine.cursor_row().unwrap();
                let line = if side == Side::Old {
                    row.old_line
                } else {
                    row.new_line
                };
                assert_eq!((row.file, line), (MIXED, Some(5)), "{what}");
            }
        }
    }
}

#[test]
fn the_cursor_on_a_heading_gap_or_note_stays_on_it() {
    for (layout, wrap) in LAYOUTS {
        let size = (121, 30);
        let mut review = open(layout, wrap);
        draw(&mut review.engine, size);
        // Every row with no line, in turn.
        let mut landmarks = Vec::new();
        review.engine.execute(Command::FirstSource).unwrap();
        for _ in 0..400 {
            draw(&mut review.engine, size);
            let row = review.engine.cursor_row().unwrap();
            if !matches!(row.kind, DiffRowKind::Code | DiffRowKind::NoNewline) {
                landmarks.push(row);
            }
            if review.engine.execute(Command::Down).unwrap() == InputEffect::Ignored {
                break;
            }
        }
        assert!(landmarks.len() > 10);
        for landmark in landmarks {
            let what = format!("{layout:?} {wrap} {landmark:?}");
            review.engine.execute(Command::FirstSource).unwrap();
            loop {
                draw(&mut review.engine, size);
                if review.engine.cursor_row().unwrap() == landmark {
                    break;
                }
                assert_ne!(
                    review.engine.execute(Command::Down).unwrap(),
                    InputEffect::Ignored,
                    "{what}"
                );
            }
            for whitespace in [
                Whitespace::IgnoreAll,
                Whitespace::IgnoreChange,
                Whitespace::Show,
            ] {
                review.engine.set_whitespace(whitespace).unwrap();
                let buffer = draw(&mut review.engine, size);
                check(&review.engine, &buffer, &what);
                let row = review.engine.cursor_row().unwrap();
                assert_eq!(
                    (row.file, row.kind),
                    (landmark.file, landmark.kind),
                    "{what}"
                );
            }
        }
    }
}

#[test]
fn a_selection_is_kept_and_still_extends_across_whitespace_changes() {
    for (layout, wrap) in LAYOUTS {
        let size = (121, 30);
        let what = format!("{layout:?} {wrap}");
        let mut review = open(layout, wrap);
        review.cursor_to(size, MIXED, Side::New, 3);
        review.engine.execute(Command::SelectLines).unwrap();
        for _ in 0..3 {
            review.engine.execute(Command::Down).unwrap();
        }
        let before = review.engine.selection().unwrap();
        assert!(review.engine.selection_extending());
        for whitespace in [
            Whitespace::IgnoreAll,
            Whitespace::Show,
            Whitespace::IgnoreChange,
        ] {
            review.engine.set_whitespace(whitespace).unwrap();
            let buffer = draw(&mut review.engine, size);
            check(&review.engine, &buffer, &format!("{what} {whitespace:?}"));
            assert_eq!(
                review.engine.selection(),
                Some(before),
                "{what} {whitespace:?}"
            );
            assert!(review.engine.selection_extending());
        }
        review.engine.set_whitespace(Whitespace::IgnoreAll).unwrap();
        for _ in 0..4 {
            review.engine.execute(Command::Down).unwrap();
            let buffer = draw(&mut review.engine, size);
            check(&review.engine, &buffer, &what);
        }
        let after = review.engine.selection().unwrap();
        assert_eq!(
            (after.file, after.side, after.first),
            (before.file, before.side, before.first)
        );
        assert!(after.last > before.last, "{what}: the selection extended");
        review.engine.execute(Command::SelectLines).unwrap();
        assert!(!review.engine.selection_extending());
        let read = review.engine.source_read(after).unwrap();
        let mut copied = Vec::new();
        read.write_to(&mut copied, &|| Ok(())).unwrap();
        assert!(
            String::from_utf8(copied)
                .unwrap()
                .starts_with("line 3\nline 4\nvalue = compute(a, b);\n")
        );
    }
}

#[test]
fn bursts_of_changes_and_tiny_panes_never_lose_the_cursor() {
    let sizes = [(40, 8), (64, 14), (30, 5), (41, 9), (121, 30)];
    let commands = [
        Command::Down,
        Command::PageDown,
        Command::Up,
        Command::NextHunk,
        Command::Right,
        Command::SwitchSide,
        Command::PreviousFile,
        Command::LastSource,
        Command::PageUp,
        Command::NextFile,
        Command::FirstSource,
        Command::ToggleWrap,
        Command::ToggleLayout,
    ];
    for (layout, wrap) in LAYOUTS {
        let mut review = open(layout, wrap);
        let mut seed: u32 = 0x9e37_79b9 ^ (layout as u32) << 3 ^ u32::from(wrap);
        let mut next = |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as usize % n
        };
        for step in 0..400 {
            let size = sizes[next(sizes.len())];
            match next(6) {
                0 => review.engine.set_whitespace(WHITESPACE[next(3)]).unwrap(),
                1 => review
                    .engine
                    .set_tab_width(TabWidth::new(next(18) as u8))
                    .unwrap(),
                2 => review.engine.set_inline_changes(INLINE[next(4)]).unwrap(),
                3 => review
                    .engine
                    .set_split_ratio(SplitRatio::from_permille(100 + next(801) as u16))
                    .unwrap(),
                _ => {
                    review
                        .engine
                        .execute(commands[next(commands.len())])
                        .unwrap();
                }
            }
            if next(3) == 0 {
                // Several changes before a frame.
                continue;
            }
            let buffer = draw(&mut review.engine, size);
            review.engine.poll().unwrap();
            if size.0 >= 40 && size.1 >= 8 {
                check(
                    &review.engine,
                    &buffer,
                    &format!("{layout:?} {wrap} step {step}"),
                );
            } else {
                assert!(review.engine.cursor_row().is_some());
            }
        }
        // A large pane afterwards draws a consistent frame.
        let buffer = review.settle((170, 60));
        check(
            &review.engine,
            &buffer,
            &format!("{layout:?} {wrap} after the burst"),
        );
        review.walk((170, 60), &format!("{layout:?} {wrap} after the burst"));
    }
}

#[test]
fn search_sees_the_lines_displayed_and_an_old_search_is_out_of_date() {
    let mut review = open(Layout::Unified, false);
    draw(&mut review.engine, (121, 30));
    let find = |engine: &mut Engine, query: &str| {
        engine
            .search_read(query.into())
            .unwrap()
            .find(None, SearchDirection::Forward, &|| Ok(()))
            .unwrap()
    };
    let before = review.engine.search_read("spaced".into()).unwrap();
    assert!(find(&mut review.engine, "spaced").is_some());
    review.engine.set_whitespace(Whitespace::IgnoreAll).unwrap();
    let error = before
        .find(None, SearchDirection::Forward, &|| Ok(()))
        .unwrap_err();
    assert!(
        error.to_string().contains("Search scope changed"),
        "{error}"
    );
    // The hunk with `spaced  out` is hidden, and `x=1` with it.
    assert!(find(&mut review.engine, "spaced").is_none());
    assert!(find(&mut review.engine, "x=1").is_none());
    // A pair shows its new text, which is found.
    let found = find(&mut review.engine, "compute(a, b)").unwrap();
    assert_eq!(found.location().new_line, Some(5));
    assert_eq!(
        found.location().old_line,
        Some(5),
        "a pair has both numbers"
    );
    review.engine.show_search_match(&found).unwrap();
    let buffer = draw(&mut review.engine, (121, 30));
    check(&review.engine, &buffer, "after the match");
    assert!(find(&mut review.engine, "real old change").is_some());
    review
        .engine
        .set_whitespace(Whitespace::IgnoreChange)
        .unwrap();
    assert!(find(&mut review.engine, "x=1").is_some());
}

#[test]
fn files_arriving_while_whitespace_is_hidden_are_shown_hidden() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CORPUS);
    let mut early = open_set(Input::Patch(path), Layout::Auto, false, None, |engine| {
        engine.set_whitespace(Whitespace::IgnoreAll).unwrap();
    });
    let mut late = open(Layout::Auto, false);
    late.engine.set_whitespace(Whitespace::IgnoreAll).unwrap();
    for size in SIZES {
        assert_eq!(early.settle(size), late.settle(size), "{size:?}");
    }
    assert_eq!(early.app().doc.files.len(), late.app().doc.files.len());
    assert_eq!(early.app().row_len(), late.app().row_len());
}

/// A file pair whose diff has two hunks with whitespace-only changes and gaps around them.
fn local_review(layout: Layout, wrap: bool) -> Review {
    let directory = tempfile::tempdir().unwrap();
    let old: String = (1..=80).map(|n| format!("line {n}\n")).collect();
    let mut new: Vec<String> = (1..=80).map(|n| format!("line {n}\n")).collect();
    new[9] = "  line 10\n".into();
    new[39] = "line 40 changed\n".into();
    new[40] = "line\t41\n".into();
    new[69] = "line 70   \n".into();
    let old_path = directory.path().join("old.txt");
    let new_path = directory.path().join("new.txt");
    fs::write(&old_path, old).unwrap();
    fs::write(&new_path, new.concat()).unwrap();
    let input = Input::Local(LocalInput::Files {
        old: old_path,
        new: new_path,
    });
    let mut review = open_with(input, layout, wrap, Some(directory));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        draw(&mut review.engine, (121, 30));
        review.engine.poll().unwrap();
        let ready = matches!(
            review.app().sources.as_ref().and_then(|s| s.first()),
            Some(SourceState::Checked(_))
        );
        if ready {
            break;
        }
        assert!(Instant::now() < deadline, "the source was not checked");
        let _ = review.ready.recv_timeout(Duration::from_millis(20));
    }
    review
}

#[test]
fn hidden_whitespace_keeps_revealed_context_of_checked_sources() {
    for (layout, wrap) in LAYOUTS {
        let size = (121, 40);
        let what = format!("{layout:?} {wrap}");
        let mut review = local_review(layout, wrap);
        // Reveal context in the first gap after the first hunk.
        review.engine.execute(Command::FirstSource).unwrap();
        for _ in 0..40 {
            draw(&mut review.engine, size);
            if review.engine.cursor_row().unwrap().kind == DiffRowKind::Gap {
                break;
            }
            review.engine.execute(Command::Down).unwrap();
        }
        assert_eq!(review.engine.cursor_row().unwrap().kind, DiffRowKind::Gap);
        review.engine.execute(Command::Activate).unwrap();
        // The whole review, from its first line, on a screen tall enough for all of it.
        let shown = |review: &mut Review| -> Vec<String> {
            review.engine.execute(Command::FirstSource).unwrap();
            for _ in 0..50 {
                if review.engine.execute(Command::Up).unwrap() == InputEffect::Ignored {
                    break;
                }
            }
            screen(&review.settle((121, 200)))
        };
        let before = shown(&mut review);
        for whitespace in [
            Whitespace::IgnoreAll,
            Whitespace::IgnoreChange,
            Whitespace::Show,
        ] {
            review.engine.set_whitespace(whitespace).unwrap();
            let buffer = draw(&mut review.engine, size);
            check_local(&review.engine, &buffer, &format!("{what} {whitespace:?}"));
            review.walk_by(size, &format!("{what} {whitespace:?}"), check_local);
            let rows = shown(&mut review);
            // Context revealed before stays revealed.
            let revealed =
                |rows: &[String]| rows.iter().filter(|row| row.contains("line 14")).count();
            assert_eq!(revealed(&rows), revealed(&before), "{what} {whitespace:?}");
            let hidden = rows
                .iter()
                .filter(|row| row.contains("whitespace only"))
                .count();
            match whitespace {
                Whitespace::Show => assert_eq!(rows, before, "{what}"),
                // The hunks at lines 10 and 70; line 40 is a real change.
                Whitespace::IgnoreAll => assert_eq!(hidden, 2, "{what}"),
                // `  line 10` adds whitespace where there was none.
                _ => assert_eq!(hidden, 1, "{what}"),
            }
        }
        // Context can still be revealed while whitespace is hidden.
        review.engine.set_whitespace(Whitespace::IgnoreAll).unwrap();
        review.engine.execute(Command::LastSource).unwrap();
        for _ in 0..80 {
            draw(&mut review.engine, size);
            if review.engine.cursor_row().unwrap().kind == DiffRowKind::Gap {
                review.engine.execute(Command::Activate).unwrap();
                let buffer = draw(&mut review.engine, size);
                check_local(
                    &review.engine,
                    &buffer,
                    &format!("{what} revealed while hidden"),
                );
                break;
            }
            review.engine.execute(Command::Up).unwrap();
        }
        review.walk_by(size, &format!("{what} revealed while hidden"), check_local);
    }
}

#[test]
fn vertical_tab_and_form_feed_are_not_whitespace_to_git() {
    // `git diff -b` and `git diff -w` both show these as two removed and two added lines.
    let patch = b"diff --git a/vf.txt b/vf.txt\n--- a/vf.txt\n+++ b/vf.txt\n@@ -1,4 +1,4 @@\n keep\n-x\x0by\n-p\x0cq\n+x y\n+p q\n end\n";
    for layout in [Layout::Unified, Layout::Split] {
        for whitespace in [Whitespace::IgnoreChange, Whitespace::IgnoreAll] {
            let what = format!("{layout:?} {whitespace:?}");
            let (mut engine, _directory) = super::search::review(patch);
            engine.set_layout(layout).unwrap();
            engine.set_whitespace(whitespace).unwrap();
            let buffer = draw(&mut engine, (100, 20));
            let rows = screen(&buffer);
            assert!(
                !rows.iter().any(|row| row.contains("whitespace only")),
                "{what}: {rows:#?}"
            );
            // Rows of code, not the file's counts or the hunk's heading.
            let changed = rows
                .iter()
                .filter(|row| row.trim_start().starts_with(|c: char| c.is_ascii_digit()))
                .filter(|row| row.contains("− ") || row.contains("+ "))
                .count();
            let expected = if layout == Layout::Split { 2 } else { 4 };
            assert_eq!(changed, expected, "{what}: {rows:#?}");
            let app = engine.app.as_ref().unwrap();
            assert!(!app.doc.files[0].whitespace_only(app.doc.files[0].source_base + 1));
        }
    }
}

#[test]
fn unified_wrapped_pairs_take_the_height_of_the_line_drawn() {
    // A pair whose old line wraps further than its new one, in hunks that also have a real
    // change: trailing spaces, and tabs that are wide at 16 cells.
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("wrap.patch");
    let spaces = " ".repeat(150);
    let patch = format!(
        "diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -1,4 +1,4 @@\n before\n-value = 1;{spaces}\n-real old\n+value = 1;\n+real new\n after\ndiff --git a/b.rs b/b.rs\n--- a/b.rs\n+++ b/b.rs\n@@ -1,4 +1,4 @@\n fn f() {{\n-\t\t\t\tlet a = 1;\n-\told\n+    let a = 1;\n+    new\n }}\n"
    );
    fs::write(&path, patch).unwrap();
    let size = (60, 30);
    for whitespace in [Whitespace::IgnoreAll, Whitespace::IgnoreChange] {
        for (layout, wrap) in LAYOUTS {
            let what = format!("{layout:?} {wrap} {whitespace:?}");
            let mut review = open_with(Input::Patch(path.clone()), layout, wrap, None);
            review.engine.set_tab_width(TabWidth::new(16)).unwrap();
            review.engine.set_whitespace(whitespace).unwrap();
            let buffer = review.settle(size);
            check(&review.engine, &buffer, &what);
            let rows = screen(&buffer);
            let pairs: Vec<usize> = rows
                .iter()
                .enumerate()
                .filter(|(_, row)| row.contains("value = 1;") || row.contains("let a = 1;"))
                .map(|(y, _)| y)
                .collect();
            assert_eq!(pairs.len(), 2, "{what}: {rows:#?}");
            for y in pairs {
                let row = review.engine.row_at(10, y as u16).unwrap();
                assert!(row.whitespace_only, "{what}: row {y}");
                assert_eq!((row.old_line, row.new_line), (Some(2), Some(2)), "{what}");
                if layout == Layout::Unified {
                    // The line drawn fits on one row, and the next line is right below it.
                    let next = review.engine.row_at(10, y as u16 + 1).unwrap();
                    assert_eq!((next.kind, next.part), (DiffRowKind::Code, 0), "{what}");
                    assert!(!rows[y + 1].trim().is_empty(), "{what}: {rows:#?}");
                    assert!(!next.whitespace_only, "{what}");
                }
            }
            // No blank row among the review's rows.
            let last = rows.iter().rposition(|row| !row.trim().is_empty()).unwrap();
            assert!(
                rows[..=last].iter().all(|row| !row.trim().is_empty()),
                "{what}: {rows:#?}"
            );
            review.walk(size, &what);
        }
    }
}

#[test]
fn the_engine_says_which_rows_and_hunks_it_hides_as_whitespace() {
    for (whitespace, indent, spaced) in [
        (Whitespace::Show, false, false),
        (Whitespace::IgnoreChange, true, false),
        (Whitespace::IgnoreAll, true, true),
    ] {
        let what = format!("{whitespace:?}");
        let mut review = open(Layout::Unified, false);
        review.engine.set_whitespace(whitespace).unwrap();
        // indent.rs, file 2, only re-indents; mixed.txt's second hunk only adds spaces.
        assert_eq!(
            review.engine.hunk_is_whitespace_only(2, 0),
            indent,
            "{what}"
        );
        assert_eq!(
            review.engine.hunk_is_whitespace_only(MIXED, 1),
            spaced,
            "{what}"
        );
        assert!(!review.engine.hunk_is_whitespace_only(MIXED, 0), "{what}");
        assert!(!review.engine.hunk_is_whitespace_only(MIXED, 9), "{what}");
        assert!(!review.engine.hunk_is_whitespace_only(99, 0), "{what}");
        let buffer = review.settle((120, 120));
        check(&review.engine, &buffer, &what);
        let mut flagged_hunks = 0;
        let mut flagged_lines = 0;
        for y in 0..buffer.area.height {
            let Some(row) = review.engine.row_at(10, y) else {
                continue;
            };
            let line = text(&buffer, y);
            match row.kind {
                DiffRowKind::Hunk(h) => {
                    assert_eq!(
                        row.whitespace_only,
                        review.engine.hunk_is_whitespace_only(row.file, h),
                        "{what}: row {y}"
                    );
                    assert_eq!(row.whitespace_only, line.contains("whitespace only"));
                    flagged_hunks += usize::from(row.whitespace_only);
                }
                DiffRowKind::Code => {
                    // A pair is drawn as unchanged: both numbers and no sign.
                    if row.whitespace_only {
                        assert!(row.old_line.is_some() && row.new_line.is_some());
                        assert!(!line.contains('−') && !line[..12].contains('+'), "{line:?}");
                    }
                    flagged_lines += usize::from(row.whitespace_only);
                }
                _ => assert!(!row.whitespace_only, "{what}: row {y}"),
            }
        }
        if whitespace == Whitespace::Show {
            assert_eq!((flagged_hunks, flagged_lines), (0, 0));
        } else {
            assert!(flagged_hunks > 0 && flagged_lines > 0, "{what}");
        }
    }
}
