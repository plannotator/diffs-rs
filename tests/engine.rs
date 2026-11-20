use std::{
    fs,
    sync::mpsc,
    time::{Duration, Instant},
};

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use diffs_rs::{
    engine::{
        Badge, BadgeColor, Command, Engine, FileMark, Input, InputEffect, Layout, Marker, Options,
        Side, SidebarPreference,
    },
    reading::SourceRange,
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Paragraph},
};

const LONG: &str = "let long = \"tabs\t日本語 café 👩‍💻 and enough ordinary words to wrap onto several rows in the small split pane\";";

fn patch(value: &str) -> String {
    format!(
        "diff --git a/src/review.rs b/src/review.rs\n--- a/src/review.rs\n+++ b/src/review.rs\n@@ -1,6 +1,7 @@\n fn start() {{}}\n-let before = 1;\n+let {value} = 2;\n+let extra = true;\n \n-let short = 0;\n+{LONG}\n \n fn end() {{}}\ndiff --git a/script.sh b/script.sh\nold mode 100644\nnew mode 100755\n"
    )
}

struct Host {
    engine: Engine,
    terminal: Terminal<TestBackend>,
    ready: mpsc::Receiver<()>,
    area: Rect,
    directory: tempfile::TempDir,
}

impl Host {
    fn new() -> Self {
        Self::open(&patch("after"), true)
    }

    fn open(patch: &str, chrome: bool) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("review.patch");
        fs::write(&path, patch).unwrap();
        let (wake, ready) = mpsc::sync_channel(1);
        let engine = Engine::open(
            Input::Patch(path),
            Options {
                layout: Layout::Split,
                sidebar: SidebarPreference::Hide,
                chrome,
                ..Options::default()
            },
            wake,
        )
        .unwrap();
        Self {
            engine,
            terminal: Terminal::new(TestBackend::new(130, 32)).unwrap(),
            ready,
            area: Rect::new(23, 3, 100, 25),
            directory,
        }
    }

    fn draw(&mut self) -> Buffer {
        let mut result = Ok(());
        let area = self.area;
        self.terminal
            .draw(|frame| {
                let bounds = frame.area();
                frame.render_widget(Block::new().style(Style::new().bg(Color::Magenta)), bounds);
                for y in bounds.y..bounds.bottom() {
                    frame.render_widget(
                        Paragraph::new("#".repeat(bounds.width as usize)),
                        Rect::new(0, y, bounds.width, 1),
                    );
                }
                result = self.engine.draw(frame, area);
            })
            .unwrap();
        result.unwrap();
        self.engine.after_draw();
        let buffer = self.terminal.backend().buffer().clone();
        let pane = area.intersection(buffer.area);
        for y in buffer.area.y..buffer.area.bottom() {
            for x in buffer.area.x..buffer.area.right() {
                if !pane.contains((x, y).into()) {
                    assert_eq!(buffer[(x, y)].symbol(), "#", "outside pane at {x},{y}");
                    assert_eq!(
                        buffer[(x, y)].bg,
                        Color::Magenta,
                        "outside pane style at {x},{y}"
                    );
                }
            }
        }
        buffer
    }

    fn settle(&mut self) -> Buffer {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let _ = self.ready.try_recv();
            self.engine.poll().unwrap();
            let buffer = self.draw();
            let status = self.engine.status();
            if !status.loading && !status.coloring && !status.preparing_source {
                return buffer;
            }
            assert!(
                Instant::now() < deadline,
                "engine did not settle without input"
            );
            let _ = self.ready.recv_timeout(Duration::from_millis(20));
        }
    }

    fn click(&mut self, x: u16, y: u16) -> InputEffect {
        self.engine
            .handle_event(
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: x,
                    row: y,
                    modifiers: KeyModifiers::NONE,
                }),
                false,
            )
            .unwrap()
    }
}

fn line(buffer: &Buffer, y: u16, from: u16, to: u16) -> String {
    (from..to).map(|x| buffer[(x, y)].symbol()).collect()
}

fn find(buffer: &Buffer, area: Rect, text: &str) -> u16 {
    (area.y..area.bottom())
        .find(|y| line(buffer, *y, area.x, area.right()).contains(text))
        .unwrap_or_else(|| panic!("missing {text:?} in {buffer:?}"))
}

#[test]
fn host_rectangle_focus_and_original_source_coordinates_survive_reflow() {
    let mut host = Host::new();
    let buffer = host.settle();
    assert_eq!(host.engine.status().files, 2);
    let area = host.area;
    let row = find(&buffer, area, "let after = 2;");
    host.click(area.x + 60, row);
    let selected = host.engine.selected_source().unwrap().unwrap();
    assert_eq!(
        (
            selected.file,
            selected.side,
            selected.old_line,
            selected.new_line
        ),
        (0, Side::New, None, Some(2))
    );
    let ignored = host
        .engine
        .handle_event(
            Event::Key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE)),
            false,
        )
        .unwrap();
    assert_eq!(ignored, InputEffect::Ignored);
    assert_eq!(host.engine.selected_source().unwrap(), Some(selected));
    assert_eq!(host.click(0, row), InputEffect::Ignored);
    assert_eq!(host.engine.selected_source().unwrap(), Some(selected));

    host.area = Rect::new(7, 2, 100, 26);
    host.engine
        .handle_event(Event::Resize(130, 32), true)
        .unwrap();
    let buffer = host.settle();
    assert_eq!(host.engine.selected_source().unwrap(), Some(selected));
    let row = find(&buffer, host.area, "let before = 1;");
    host.click(host.area.x + 8, row);
    let old = host.engine.selected_source().unwrap().unwrap();
    assert_eq!(
        (old.side, old.old_line, old.new_line),
        (Side::Old, Some(2), None)
    );

    host.engine.execute(Command::ToggleWrap).unwrap();
    host.engine.execute(Command::ToggleLayout).unwrap();
    host.settle();
    assert_eq!(host.engine.selected_source().unwrap(), Some(old));
    host.engine.execute(Command::Help).unwrap();
    host.settle();
    host.engine.execute(Command::Back).unwrap();
    for area in [
        Rect::new(120, 27, 30, 20),
        Rect::new(12, 4, 2, 2),
        Rect::new(250, 100, 0, 0),
        Rect::new(7, 2, 100, 26),
    ] {
        host.area = area;
        host.settle();
    }
    assert_eq!(host.engine.selected_source().unwrap(), Some(old));
    host.engine.close().unwrap();
    host.draw();
    assert!(host.engine.status().closed);
    assert_eq!(host.engine.selected_source().unwrap(), None);
}

#[test]
fn alignment_padding_headers_and_metadata_never_invent_source_locations() {
    let mut host = Host::new();
    host.settle();
    host.engine.execute(Command::ToggleWrap).unwrap();
    let buffer = host.settle();
    let area = host.area;
    let extra = find(&buffer, area, "let extra = true;");
    host.click(area.x + 8, extra);
    assert_eq!(host.engine.selected_source().unwrap(), None);
    host.click(area.x + 60, extra);
    let source = host.engine.selected_source().unwrap().unwrap();
    assert_eq!((source.side, source.new_line), (Side::New, Some(3)));
    let short = find(&buffer, area, "let short = 0;");
    host.click(area.x + 8, short + 1);
    assert_eq!(host.engine.selected_source().unwrap(), None);
    host.click(area.x + 60, short + 1);
    let source = host.engine.selected_source().unwrap().unwrap();
    assert_eq!((source.side, source.new_line), (Side::New, Some(5)));
    host.click(area.x + 8, find(&buffer, area, "src/review.rs"));
    assert_eq!(host.engine.selected_source().unwrap(), None);
    host.click(area.x + 8, find(&buffer, area, "script.sh"));
    assert_eq!(host.engine.selected_source().unwrap(), None);
}

#[test]
fn replace_after_failure_and_close_uses_fresh_identity_and_source() {
    let mut host = Host::new();
    host.settle();
    let original = host.engine.review_id();
    host.engine.execute(Command::ToggleTheme).unwrap();
    let appearance = host.engine.options().appearance;
    host.engine
        .replace(Input::Patch(host.directory.path().join("missing.patch")))
        .unwrap();
    let failed = host.engine.review_id();
    assert_ne!(original, failed);
    let buffer = host.settle();
    assert!(host.engine.status().error.unwrap().contains("Cannot open"));
    find(&buffer, host.area, "Cannot open patch");
    let path = host.directory.path().join("review.patch");
    for value in ["replaced", "again"] {
        fs::write(&path, patch(value)).unwrap();
        host.engine.replace(Input::Patch(path.clone())).unwrap();
        assert_ne!(host.engine.review_id(), failed);
        assert_eq!(host.engine.options().appearance, appearance);
        let buffer = host.settle();
        let row = find(&buffer, host.area, &format!("let {value} = 2;"));
        host.click(host.area.x + 60, row);
        assert_eq!(
            host.engine.selected_source().unwrap().unwrap().review,
            host.engine.review_id()
        );
        assert!(host.engine.status().error.is_none());
        host.engine.close().unwrap();
        host.engine.close().unwrap();
        assert!(host.engine.status().closed);
        assert_eq!(host.engine.status().files, 0);
    }
}

#[test]
fn checked_context_and_sidebar_keep_original_coordinates_inside_a_host_pane() {
    use diffs_rs::input::LocalInput;
    let mut host = Host::new();
    host.settle();
    let old = host.directory.path().join("before.rs");
    let new = host.directory.path().join("after.rs");
    let original: String = (1..=80)
        .map(|n| format!("let item_{n:03} = {n};\n"))
        .collect();
    fs::write(&old, &original).unwrap();
    fs::write(&new, original.replace("item_044 = 44", "item_044 = 444")).unwrap();
    let input = || {
        Input::Local(LocalInput::Files {
            old: old.clone(),
            new: new.clone(),
        })
    };
    host.engine.replace(input()).unwrap();
    let buffer = host.settle();
    find(&buffer, host.area, "complete-context coloring");
    host.engine.execute(Command::Files).unwrap();
    let buffer = host.settle();
    let gap = find(&buffer, host.area, "40 unchanged lines");
    host.click(host.area.right() - 12, gap);
    let buffer = host.settle();
    let row = find(&buffer, host.area, "item_025");
    host.click(host.area.right() - 8, row);
    let selected = host.engine.selected_source().unwrap().unwrap();
    assert_eq!(
        (
            selected.file,
            selected.side,
            selected.old_line,
            selected.new_line
        ),
        (0, Side::New, Some(25), Some(25))
    );
    host.engine.execute(Command::ToggleWrap).unwrap();
    host.area = Rect::new(4, 1, 122, 30);
    host.settle();
    assert_eq!(host.engine.selected_source().unwrap(), Some(selected));
    fs::write(&new, original.replace("item_044 = 44", "item_044 = 888")).unwrap();
    host.engine.replace(input()).unwrap();
    let buffer = host.settle();
    assert_ne!(host.engine.review_id(), selected.review);
    find(&buffer, host.area, "item_044 = 888;");
    find(&buffer, host.area, "40 unchanged lines");
    assert!(host.engine.status().error.is_none());
    host.engine.close().unwrap();
}

fn added(path: &str, count: u32) -> String {
    let lines: String = (1..=count).map(|n| format!("+line_{n:03}\n")).collect();
    format!(
        "diff --git a/{path} b/{path}\nnew file mode 100644\n--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{count} @@\n{lines}"
    )
}

/// Two hunks with an unsupplied gap between them, then a second file.
fn two_hunks() -> String {
    let context: String = (100..160).map(|n| format!(" ctx_{n:03}\n")).collect();
    format!(
        "diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -1,3 +1,3 @@\n ctx_001\n-old_002\n+new_002\n ctx_003\n@@ -100,60 +100,60 @@\n{context}diff --git a/b.txt b/b.txt\n--- a/b.txt\n+++ b/b.txt\n@@ -1 +1 @@\n-b_old\n+b_new\n"
    )
}

fn rows(buffer: &Buffer, area: Rect) -> Vec<String> {
    (area.y..area.bottom())
        .map(|y| line(buffer, y, area.x, area.right()))
        .collect()
}

fn range(engine: &Engine, file: usize, side: Side, first: u32, last: u32) -> SourceRange {
    SourceRange {
        review: engine.review_id(),
        file,
        side,
        first,
        last,
    }
}

fn located(engine: &Engine) -> (usize, Side, Option<u32>, Option<u32>) {
    let source = engine.selected_source().unwrap().unwrap();
    (source.file, source.side, source.old_line, source.new_line)
}

#[test]
fn without_chrome_the_diff_fills_the_rectangle_and_the_host_owns_help() {
    let mut chromed = Host::open(&added("big.txt", 80), true);
    let with = rows(&chromed.settle(), chromed.area);
    assert!(with[0].starts_with(" diffs  /"), "{:?}", with[0]);
    assert!(with[24].contains("q quit"), "{:?}", with[24]);
    assert!(with[23].contains("line_021"), "{:?}", with[23]);
    chromed.engine.close().unwrap();

    let mut host = Host::open(&added("big.txt", 80), false);
    assert!(!host.engine.options().chrome);
    let area = host.area;
    let without = rows(&host.settle(), area);
    assert!(without[0].starts_with(" big.txt"), "{:?}", without[0]);
    assert!(without[1].contains("@@ -0,0 +1,80 @@"), "{:?}", without[1]);
    assert!(without[24].contains("line_023"), "{:?}", without[24]);
    for row in &without {
        assert!(!row.contains("diffs  /") && !row.contains("q quit") && !row.contains("? help"));
    }

    // Help belongs to the host: neither the command nor the default key opens it.
    assert_eq!(
        host.engine.execute(Command::Help).unwrap(),
        InputEffect::Ignored
    );
    let key = Event::Key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
    assert_eq!(
        host.engine.handle_event(key, true).unwrap(),
        InputEffect::Ignored
    );
    assert!(
        !rows(&host.draw(), area)
            .iter()
            .any(|row| row.contains("Reading controls"))
    );

    // Clicks map to the rows actually drawn, starting at the rectangle's first row.
    host.click(area.x + 60, area.y + 2);
    assert_eq!(located(&host.engine), (0, Side::New, None, Some(1)));
    // The cursor stays inside the rectangle while moving past its last row.
    for _ in 0..40 {
        host.engine.execute(Command::Down).unwrap();
        let buffer = host.draw();
        let (_, _, _, new) = located(&host.engine);
        let row = find(&buffer, area, &format!("line_{:03}", new.unwrap()));
        assert!(row < area.bottom());
    }
    host.engine.execute(Command::PageDown).unwrap();
    let buffer = host.draw();
    find(
        &buffer,
        area,
        &format!("line_{:03}", located(&host.engine).3.unwrap()),
    );

    // Without its two rows of chrome, the engine needs eight rows instead of ten.
    host.area = Rect::new(area.x, area.y, area.width, 8);
    let small = rows(&host.draw(), host.area);
    assert!(small.iter().any(|row| row.contains("line_")), "{small:?}");
    host.area = Rect::new(area.x, area.y, area.width, 7);
    let tiny = rows(&host.draw(), host.area);
    assert!(tiny[0].contains("Enlarge pane"), "{tiny:?}");
    assert!(!tiny.iter().any(|row| row.contains("q quit")));

    // Placeholders name no keys either.
    host.area = area;
    host.engine
        .replace(Input::Patch(host.directory.path().join("missing.patch")))
        .unwrap();
    assert!(!host.engine.options().chrome);
    let failed = rows(&host.settle(), area);
    assert!(failed.iter().any(|row| row.contains("Cannot open patch")));
    assert!(!failed.iter().any(|row| row.contains("q quit")));
    assert!(host.engine.status().error.is_some());
    host.engine.close().unwrap();
}

fn marks(buffer: &Buffer, area: Rect) -> usize {
    (area.y..area.bottom())
        .flat_map(|y| (area.x..area.right()).map(move |x| (x, y)))
        .filter(|&cell| buffer[cell].symbol() == "▌")
        .count()
}

#[test]
fn markers_draw_in_the_gutter_on_their_side_and_replace_each_other() {
    let mut host = Host::new();
    host.settle();
    let area = host.area;
    // The split pane puts the old gutter at the left edge and the new one after the divider.
    let (old, new) = (area.x, area.x + 50);
    let pending = Style::new().fg(Color::Yellow);
    let sent = Style::new().fg(Color::Green);
    let engine = &host.engine;
    let markers = [
        Marker {
            range: range(engine, 0, Side::New, 2, 3),
            style: pending,
        },
        Marker {
            range: range(engine, 0, Side::Old, 4, 4),
            style: sent,
        },
        // Lines outside the diff and files outside the review draw nothing.
        Marker {
            range: range(engine, 0, Side::New, 40, 50),
            style: sent,
        },
        Marker {
            range: range(engine, 7, Side::New, 1, 1),
            style: sent,
        },
    ];
    host.engine.set_markers(&markers).unwrap();
    assert!(host.engine.needs_draw());
    let buffer = host.settle();
    let after = find(&buffer, area, "let after = 2;");
    let extra = find(&buffer, area, "let extra = true;");
    let short = find(&buffer, area, "let short = 0;");
    for row in [after, extra] {
        assert_eq!(buffer[(new, row)].symbol(), "▌");
        assert_eq!(buffer[(new, row)].fg, Color::Yellow);
    }
    assert_eq!(buffer[(old, after)].symbol(), " ");
    assert_eq!(buffer[(old, short)].symbol(), "▌");
    assert_eq!(buffer[(old, short)].fg, Color::Green);
    assert_eq!(buffer[(new, short)].symbol(), " ");
    assert_eq!(marks(&buffer, area), 3);

    // The cursor indicator takes the cell on its own row.
    host.click(area.x + 60, after);
    let buffer = host.settle();
    assert_eq!(buffer[(new, after)].symbol(), "▎");
    assert_eq!(marks(&buffer, area), 2);

    // Unified rows carry both numbers, so markers on either side apply.
    host.engine.execute(Command::ToggleLayout).unwrap();
    let engine = &host.engine;
    let markers = [
        Marker {
            range: range(engine, 0, Side::Old, 1, 1),
            style: pending,
        },
        Marker {
            range: range(engine, 0, Side::Old, 2, 2),
            style: sent,
        },
    ];
    host.engine.set_markers(&markers).unwrap();
    let buffer = host.settle();
    for (text, color) in [
        ("fn start() {}", Color::Yellow),
        ("let before = 1;", Color::Green),
    ] {
        let row = find(&buffer, area, text);
        assert_eq!(buffer[(area.x, row)].symbol(), "▌", "{text}");
        assert_eq!(buffer[(area.x, row)].fg, color, "{text}");
    }
    assert_eq!(marks(&buffer, area), 2);

    // An empty set clears every marker; invalid ranges are rejected and change nothing.
    let review = host.engine.review_id();
    let invalid = |first, last| Marker {
        range: SourceRange {
            review,
            file: 0,
            side: Side::New,
            first,
            last,
        },
        style: sent,
    };
    assert!(host.engine.set_markers(&[invalid(0, 1)]).is_err());
    assert!(host.engine.set_markers(&[invalid(3, 2)]).is_err());
    assert_eq!(marks(&host.settle(), area), 2);
    host.engine.set_markers(&[]).unwrap();
    assert_eq!(marks(&host.settle(), area), 0);

    // Markers belong to one review. Replacement drops them and rejects stale ones; a new
    // set can arrive before the first file does.
    let stale = markers[0];
    let path = host.directory.path().join("review.patch");
    host.engine.replace(Input::Patch(path)).unwrap();
    assert!(host.engine.set_markers(&[stale]).is_err());
    let fresh = Marker {
        range: range(&host.engine, 0, Side::Old, 1, 1),
        ..stale
    };
    host.engine.set_markers(&[fresh]).unwrap();
    let buffer = host.settle();
    let row = find(&buffer, area, "fn start() {}");
    assert_eq!(buffer[(area.x, row)].symbol(), "▌");
    assert_eq!(marks(&buffer, area), 1);
    host.engine.close().unwrap();
    assert!(host.engine.set_markers(&[]).is_err());
}

#[test]
fn reveal_and_select_range_act_like_the_user_and_reject_lines_outside_the_diff() {
    let mut host = Host::open(&two_hunks(), true);
    let buffer = host.settle();
    let area = host.area;
    assert!(
        !rows(&buffer, area)
            .iter()
            .any(|row| row.contains("ctx_150"))
    );

    host.engine.reveal(0, Side::New, 150).unwrap();
    assert_eq!(located(&host.engine), (0, Side::New, Some(150), Some(150)));
    let buffer = host.settle();
    let row = find(&buffer, area, "ctx_150");
    assert_eq!(buffer[(area.x + 50, row)].symbol(), "▎");
    host.engine.reveal(0, Side::Old, 2).unwrap();
    assert_eq!(located(&host.engine), (0, Side::Old, Some(2), None));
    find(&host.settle(), area, "old_002");

    // Lines in the unsupplied gap, past the end, on a side without them, or in a missing
    // file are errors, and the cursor stays where it was.
    for (file, side, line) in [
        (0, Side::New, 50),
        (0, Side::New, 0),
        (0, Side::New, 161),
        (1, Side::Old, 2),
        (2, Side::New, 1),
    ] {
        let error = host
            .engine
            .reveal(file, side, line)
            .unwrap_err()
            .to_string();
        assert!(error.contains("not in the"), "{error}");
        assert_eq!(located(&host.engine), (0, Side::Old, Some(2), None));
    }

    // A host selection reads back exactly like a finished user selection.
    let selected = range(&host.engine, 0, Side::New, 140, 150);
    host.engine.select_range(selected).unwrap();
    assert_eq!(host.engine.selection(), Some(selected));
    assert!(!host.engine.selection_extending());
    assert_eq!(located(&host.engine), (0, Side::New, Some(150), Some(150)));
    let buffer = host.settle();
    assert_eq!(
        buffer[(area.x + 50, find(&buffer, area, "ctx_145"))].symbol(),
        "┃"
    );
    let mut bytes = Vec::new();
    host.engine
        .source_read(selected)
        .unwrap()
        .write_to(&mut bytes, &|| Ok(()))
        .unwrap();
    let expected: String = (140..=150).map(|n| format!("ctx_{n:03}\n")).collect();
    assert_eq!(String::from_utf8(bytes).unwrap(), expected);
    host.engine.execute(Command::Down).unwrap();
    assert_eq!(host.engine.selection(), Some(selected));

    // Endpoints must be displayed; the lines between them may cross a gap, as a user
    // selection may, and reading it then fails.
    for (first, last) in [(50, 150), (140, 50), (150, 140), (0, 3)] {
        let invalid = range(&host.engine, 0, Side::New, first, last);
        assert!(host.engine.select_range(invalid).is_err());
        assert_eq!(host.engine.selection(), Some(selected));
    }
    let gap = range(&host.engine, 0, Side::New, 2, 120);
    host.engine.select_range(gap).unwrap();
    assert!(
        host.engine
            .source_read(gap)
            .unwrap()
            .write_to(&mut Vec::new(), &|| Ok(()))
            .is_err()
    );

    // Another file and the old side work too; revealing elsewhere clears the selection.
    let other = range(&host.engine, 1, Side::Old, 1, 1);
    host.engine.select_range(other).unwrap();
    assert_eq!(located(&host.engine), (1, Side::Old, Some(1), None));
    find(&host.settle(), area, "b_old");
    host.engine.reveal(0, Side::New, 3).unwrap();
    assert_eq!(host.engine.selection(), None);

    // Ranges from another review are rejected.
    let (wake, _ready) = mpsc::sync_channel(1);
    let mut second = Engine::open(
        Input::Patch(host.directory.path().join("review.patch")),
        Options::default(),
        wake,
    )
    .unwrap();
    let foreign = range(&second, 0, Side::New, 140, 150);
    second.close().unwrap();
    assert!(host.engine.select_range(foreign).is_err());
    assert_eq!(host.engine.selection(), None);

    host.engine.close().unwrap();
    assert!(host.engine.reveal(0, Side::New, 3).is_err());
    assert!(host.engine.select_range(other).is_err());
}

#[test]
fn reveal_reaches_checked_context_only_once_it_is_revealed() {
    use diffs_rs::input::LocalInput;
    let mut host = Host::new();
    host.settle();
    let old = host.directory.path().join("before.rs");
    let new = host.directory.path().join("after.rs");
    let original: String = (1..=80)
        .map(|n| format!("let item_{n:03} = {n};\n"))
        .collect();
    fs::write(&old, &original).unwrap();
    fs::write(&new, original.replace("item_044 = 44", "item_044 = 444")).unwrap();
    host.engine
        .replace(Input::Local(LocalInput::Files { old, new }))
        .unwrap();
    let buffer = host.settle();
    find(&buffer, host.area, "complete-context coloring");
    assert!(host.engine.reveal(0, Side::New, 25).is_err());
    let gap = find(&buffer, host.area, "40 unchanged lines");
    host.click(host.area.right() - 12, gap);
    host.settle();
    host.engine.reveal(0, Side::New, 25).unwrap();
    assert_eq!(located(&host.engine), (0, Side::New, Some(25), Some(25)));
    find(&host.settle(), host.area, "item_025");
    assert!(host.engine.reveal(0, Side::Old, 10).is_err());

    let selected = range(&host.engine, 0, Side::Old, 21, 44);
    host.engine.select_range(selected).unwrap();
    assert_eq!(host.engine.selection(), Some(selected));
    let mut bytes = Vec::new();
    host.engine
        .source_read(selected)
        .unwrap()
        .write_to(&mut bytes, &|| Ok(()))
        .unwrap();
    let expected: String = original
        .lines()
        .skip(20)
        .take(24)
        .map(|line| format!("{line}\n"))
        .collect();
    assert_eq!(String::from_utf8(bytes).unwrap(), expected);
    host.engine.close().unwrap();
}

/// The sidebar row naming `name`, up to the divider.
fn tree_row(buffer: &Buffer, area: Rect, name: &str) -> String {
    (area.y..area.bottom())
        .map(|y| line(buffer, y, area.x, area.x + 33))
        .find(|row| row.contains(name))
        .unwrap_or_else(|| panic!("missing {name:?} in {buffer:?}"))
}

#[test]
fn file_marks_badge_and_dim_sidebar_rows_without_moving_the_diff() {
    let mut host = Host::new();
    let viewed = |path| FileMark {
        path,
        badge: Some(Badge {
            text: "✓",
            color: BadgeColor::Added,
        }),
        dimmed: true,
    };
    // Marks can be set before the first file arrives.
    host.engine.set_file_marks(&[viewed("script.sh")]).unwrap();
    host.settle();
    host.engine.execute(Command::Files).unwrap();
    host.engine.execute(Command::Back).unwrap();
    let before = host.settle();
    let area = host.area;
    assert_eq!(
        tree_row(&before, area, "script.sh"),
        "  script.sh          M +0 −0 ✓  │"
    );
    assert_eq!(
        tree_row(&before, area, "review.rs"),
        " *  review.rs        M +3 −2    │"
    );

    // Replacing the set redraws the sidebar only; the diff keeps its layout.
    host.engine
        .set_file_marks(&[viewed("src/review.rs"), viewed("script.sh")])
        .unwrap();
    assert!(host.engine.needs_draw());
    let after = host.settle();
    assert_eq!(
        tree_row(&after, area, "src/"),
        "  ▾ src/             1 +3 −2 ✓  │"
    );
    for y in area.top()..area.bottom() {
        assert_eq!(
            line(&after, y, area.x + 33, area.right()),
            line(&before, y, area.x + 33, area.right())
        );
    }

    // An invalid badge rejects the whole set and keeps the previous one.
    let wide = FileMark {
        badge: Some(Badge {
            text: "abc",
            color: BadgeColor::Accent,
        }),
        ..viewed("script.sh")
    };
    assert!(host.engine.set_file_marks(&[wide]).is_err());
    assert!(tree_row(&host.settle(), area, "script.sh").ends_with("✓  │"));

    // Replacement drops the marks and the badge column; close rejects new ones.
    let path = host.directory.path().join("review.patch");
    host.engine.replace(Input::Patch(path)).unwrap();
    let buffer = host.settle();
    assert_eq!(
        tree_row(&buffer, area, "script.sh"),
        "  script.sh             M +0 −0 │"
    );
    host.engine.close().unwrap();
    assert!(host.engine.set_file_marks(&[]).is_err());
}

#[test]
fn terminal_background_draws_plain_text_and_context_in_the_terminal_colors() {
    let mut host = Host::new();
    let theme_text = Color::Rgb(0xfa, 0xfa, 0xfa);
    let buffer = host.settle();
    let area = host.area;
    let cells = |buffer: &Buffer, y: u16| -> Vec<(Color, Color)> {
        (area.x..area.right())
            .map(|x| (buffer[(x, y)].fg, buffer[(x, y)].bg))
            .collect()
    };
    let row = find(&buffer, area, "fn start() {}");
    assert!(cells(&buffer, row).iter().any(|(fg, _)| *fg == theme_text));
    host.engine.set_terminal_background(true).unwrap();
    assert!(host.engine.needs_draw());
    for replaced in [false, true] {
        let buffer = host.settle();
        // A colored context row: no background anywhere, and the theme's text color is gone.
        let row = find(&buffer, area, "fn start() {}");
        let row = cells(&buffer, row);
        assert!(row.iter().all(|(_, bg)| *bg == Color::Reset), "{replaced}");
        assert!(row.iter().all(|(fg, _)| *fg != theme_text), "{replaced}");
        assert!(row.iter().any(|(fg, _)| *fg == Color::Reset), "{replaced}");
        // Syntax colors stay: some cell carries an explicit foreground.
        assert!(
            row.iter().any(|(fg, _)| matches!(fg, Color::Rgb(..))),
            "{replaced}"
        );
        // An added line keeps its tint.
        let added = find(&buffer, area, "let extra = true;");
        assert!(
            cells(&buffer, added)
                .iter()
                .any(|(_, bg)| matches!(bg, Color::Rgb(..))),
            "{replaced}"
        );
        // The setting survives replacement.
        let path = host.directory.path().join("review.patch");
        host.engine.replace(Input::Patch(path)).unwrap();
    }
}
