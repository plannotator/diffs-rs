//! `Engine::cursor_row` and `Engine::row_at` against the frame drawn: on every kind of row, in
//! both layouts with and without wrapping, at several sizes, after every kind of move.

use super::*;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

const WIDE: &str = "a very wide line that runs on and on past the edge of any pane this test draws, \
                    so that wrapping it takes several screen rows in either layout and more";

/// A binary file first, a mode change, a rename, a deleted file, a new one, a symlink replaced
/// by a file, two hunks with gaps and a very wide line, and a last line with no newline.
fn mixed() -> String {
    let mut patch = String::new();
    patch.push_str("diff --git a/aa.bin b/aa.bin\nindex 1111111..2222222 100644\nBinary files a/aa.bin and b/aa.bin differ\n");
    patch.push_str("diff --git a/bb.sh b/bb.sh\nold mode 100644\nnew mode 100755\n");
    patch.push_str("diff --git a/cc_old.txt b/cc_new.txt\nsimilarity index 100%\nrename from cc_old.txt\nrename to cc_new.txt\n");
    patch.push_str("diff --git a/dd.txt b/dd.txt\ndeleted file mode 100644\nindex 3333333..0000000\n--- a/dd.txt\n+++ /dev/null\n@@ -1,2 +0,0 @@\n-gone one\n-gone two\n");
    patch.push_str("diff --git a/ee.txt b/ee.txt\nnew file mode 100644\nindex 0000000..4444444\n--- /dev/null\n+++ b/ee.txt\n@@ -0,0 +1,2 @@\n+fresh one\n+fresh two\n");
    patch.push_str("diff --git a/ff.txt b/ff.txt\nindex 5555555..6666666 100644\n--- a/ff.txt\n+++ b/ff.txt\n@@ -9,7 +9,7 @@ line 8\n line 9\n line 10\n line 11\n-line 12\n");
    patch.push_str(&format!("+{WIDE}\n line 13\n line 14\n line 15\n"));
    patch.push_str("@@ -30,7 +30,8 @@ line 29\n line 30\n line 31\n line 32\n-line 33\n+line 33a\n+line 33b\n line 34\n line 35\n line 36\n");
    patch.push_str("diff --git a/gg.txt b/gg.txt\ndeleted file mode 120000\nindex 7777777..0000000\n--- a/gg.txt\n+++ /dev/null\n@@ -1 +0,0 @@\n-target\n\\ No newline at end of file\n");
    patch.push_str("diff --git a/gg.txt b/gg.txt\nnew file mode 100644\nindex 0000000..8888888\n--- /dev/null\n+++ b/gg.txt\n@@ -0,0 +1 @@\n+now a file\n");
    patch.push_str("diff --git a/hh.txt b/hh.txt\nindex 9999999..aaaaaaa 100644\n--- a/hh.txt\n+++ b/hh.txt\n@@ -1,2 +1,2 @@\n first\n-last\n\\ No newline at end of file\n+changed\n\\ No newline at end of file\n");
    patch
}

fn open(layout: Layout, wrap: bool) -> (Engine, tempfile::TempDir) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("review.patch");
    fs::write(&path, mixed()).unwrap();
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
    assert_eq!(engine.status().files, 9);
    (engine, directory)
}

fn draw(engine: &mut Engine, (width, height): (u16, u16)) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| engine.draw(frame, frame.area()).unwrap())
        .unwrap();
    engine.after_draw();
    terminal.backend().buffer().clone()
}

fn text(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

/// The file whose heading `text` is, by its path and its counts.
fn heading(engine: &Engine, text: &str) -> Option<usize> {
    (0..engine.status().files).find(|&index| {
        let f = engine.file(index).unwrap();
        let path = if f.old_path != f.path {
            format!(" {} → {}", f.old_path, f.path)
        } else {
            format!(" {}", f.path)
        };
        let summary = format!(" {}   +{} −{} ", f.status, f.additions, f.deletions);
        text.starts_with(&path) && text.ends_with(&summary)
    })
}

/// What a row of the frame shows, read from its text alone.
fn kind_of(text: &str) -> DiffRowKind {
    if text.starts_with(" @@") {
        DiffRowKind::Hunk(usize::MAX)
    } else if text.contains("Context not included in patch")
        || text.contains("unchanged lines · Show")
    {
        DiffRowKind::Gap
    } else if text.contains("no textual hunks") {
        DiffRowKind::Metadata
    } else if text.contains("No newline at end of file") {
        DiffRowKind::NoNewline
    } else {
        DiffRowKind::Code
    }
}

/// Every screen row of the frame against `row_at`, and the cursor's row against
/// `cursor_row`. Returns what `row_at` said for each screen row.
fn check(engine: &Engine, buffer: &Buffer, what: &str) -> Vec<Option<DiffRow>> {
    let height = buffer.area.height;
    let mut file: Option<usize> = None;
    // The hunk whose heading was drawn last above, when one was.
    let mut hunk: Option<Option<usize>> = None;
    let mut found = Vec::new();
    for y in 0..height {
        let line = text(buffer, y);
        let at = engine.row_at(buffer.area.width / 4, y);
        let head = heading(engine, &line);
        if let Some(index) = head {
            // The first heading drawn may be one kept over the rows of its file.
            if let Some(before) = file {
                assert_eq!(index, before + 1, "{what}: headings in order at row {y}");
            }
            file = Some(index);
            hunk = Some(None);
        }
        match &at {
            None => {
                assert!(
                    line.trim().is_empty() || file.is_none(),
                    "{what}: row {y} {line:?} has no row"
                );
                found.push(None);
                continue;
            }
            Some(row) => {
                let file =
                    file.unwrap_or_else(|| panic!("{what}: row {y} {line:?} before any heading"));
                assert_eq!(row.file, file, "{what}: the file of row {y} {line:?}");
                assert_eq!(
                    row.path,
                    &*engine.file(file).unwrap().path,
                    "{what}: row {y}"
                );
                assert_eq!(row.review, engine.review_id());
                let expected = match (head, y) {
                    (Some(_), 0) if row.kind == DiffRowKind::StickyHeader => {
                        DiffRowKind::StickyHeader
                    }
                    (Some(_), _) => DiffRowKind::Header,
                    (None, _) => kind_of(&line),
                };
                let kind = match row.kind {
                    DiffRowKind::Hunk(_) => DiffRowKind::Hunk(usize::MAX),
                    kind => kind,
                };
                assert_eq!(kind, expected, "{what}: the kind of row {y} {line:?}");
                if let DiffRowKind::Hunk(h) = row.kind {
                    let header = &engine.file(file).unwrap().hunks[h].header;
                    assert!(
                        line.starts_with(&format!(" {}", &header[..header.len().min(20)])),
                        "{what}: {line:?}"
                    );
                }
                match row.kind {
                    DiffRowKind::Hunk(index) => {
                        assert_eq!(row.hunk, Some(index), "{what}: row {y}");
                        hunk = Some(Some(index));
                    }
                    DiffRowKind::Gap => {
                        assert_eq!(row.hunk, None, "{what}: row {y}");
                        hunk = Some(None);
                    }
                    DiffRowKind::Code | DiffRowKind::NoNewline => {
                        assert!(row.hunk.is_some(), "{what}: row {y} is in a hunk");
                        if let Some(above) = hunk {
                            assert_eq!(row.hunk, above, "{what}: the hunk of row {y} {line:?}");
                        }
                    }
                    DiffRowKind::StickyHeader => {
                        // Kept over the file's rows: the hunk heading may be above the screen.
                        assert_eq!(row.hunk, None, "{what}: row {y}");
                        hunk = None;
                    }
                    _ => assert_eq!(row.hunk, None, "{what}: row {y}"),
                }
                if row.kind != DiffRowKind::Code {
                    assert_eq!(
                        (row.old_line, row.new_line),
                        (None, None),
                        "{what}: row {y}"
                    );
                }
            }
        }
        found.push(at);
    }
    // The cursor is on a row the frame drew.
    let app = engine.app.as_ref().unwrap();
    let cursor = engine.cursor_row().expect("the cursor's row");
    assert_eq!(cursor.file, app.row(app.cursor.row).unwrap().file);
    assert_eq!(cursor.part, app.cursor.part);
    if let Some(&(position, at)) = app.visible.get(app.cursor_screen_row)
        && position == app.cursor
    {
        let drawn = found[usize::from(at - 1)].clone().expect("drawn");
        assert_eq!(
            DiffRow {
                side: cursor.side,
                ..drawn
            },
            cursor,
            "{what}: the cursor's row"
        );
    }
    // On a line, the same file and lines `selected_source` reports.
    if let Some(location) = engine.selected_source().unwrap() {
        assert_eq!(cursor.file, location.file, "{what}");
        assert_eq!(cursor.side, location.side, "{what}");
        let line = |row: &DiffRow| {
            if row.side == Side::Old {
                row.old_line
            } else {
                row.new_line
            }
        };
        let expected = if location.side == Side::Old {
            location.old_line
        } else {
            location.new_line
        };
        assert_eq!(line(&cursor), expected, "{what}");
        if !app.split() {
            assert_eq!(
                (cursor.old_line, cursor.new_line),
                (location.old_line, location.new_line),
                "{what}"
            );
        }
    }
    found
}

fn run(engine: &mut Engine, size: (u16, u16), command: Command, what: &str) -> bool {
    let moved = engine.execute(command).unwrap() != InputEffect::Ignored;
    let buffer = draw(engine, size);
    check(engine, &buffer, what);
    moved
}

const LAYOUTS: [(Layout, bool); 4] = [
    (Layout::Unified, false),
    (Layout::Unified, true),
    (Layout::Split, false),
    (Layout::Split, true),
];
const SIZES: [(u16, u16); 3] = [(70, 12), (120, 24), (170, 60)];

#[test]
fn every_row_down_and_up_names_its_file_and_kind() {
    for (layout, wrap) in LAYOUTS {
        for size in SIZES {
            let (mut engine, _directory) = open(layout, wrap);
            let buffer = draw(&mut engine, size);
            check(&engine, &buffer, "first frame");
            let mut kinds = std::collections::HashSet::new();
            let mut files = std::collections::HashSet::new();
            for n in 0..400 {
                let row = engine.cursor_row().unwrap();
                kinds.insert(std::mem::discriminant(&row.kind));
                files.insert(row.file);
                if !run(
                    &mut engine,
                    size,
                    Command::Down,
                    &format!("{layout:?} {wrap} {size:?} down {n}"),
                ) {
                    break;
                }
            }
            assert_eq!(files.len(), 9, "{layout:?} {wrap} {size:?}: every file");
            // Every kind but the sticky heading, which the cursor is never on.
            assert_eq!(kinds.len(), 6, "{layout:?} {wrap} {size:?}: {kinds:?}");
            for n in 0..400 {
                if !run(
                    &mut engine,
                    size,
                    Command::Up,
                    &format!("{layout:?} {wrap} {size:?} up {n}"),
                ) {
                    break;
                }
            }
            assert_eq!(engine.cursor_row().unwrap().file, 0);
        }
    }
}

#[test]
fn pages_jumps_wheel_and_clicks_keep_both_answers_right() {
    for (layout, wrap) in LAYOUTS {
        for size in SIZES {
            let what = format!("{layout:?} {wrap} {size:?}");
            let (mut engine, _directory) = open(layout, wrap);
            draw(&mut engine, size);
            for command in [Command::PageDown; 12]
                .into_iter()
                .chain([Command::PageUp; 12])
            {
                run(&mut engine, size, command, &format!("{what} {command:?}"));
            }
            for command in [Command::NextHunk; 8]
                .into_iter()
                .chain([Command::PreviousHunk; 8])
                .chain([Command::NextFile; 9])
                .chain([Command::PreviousFile; 9])
                .chain([Command::LastSource, Command::FirstSource])
            {
                run(&mut engine, size, command, &format!("{what} {command:?}"));
                run(
                    &mut engine,
                    size,
                    Command::Down,
                    &format!("{what} {command:?} then down"),
                );
            }
            for (kind, times) in [
                (MouseEventKind::ScrollDown, 20),
                (MouseEventKind::ScrollUp, 20),
            ] {
                for n in 0..times {
                    let event = MouseEvent {
                        kind,
                        column: 10,
                        row: 3,
                        modifiers: KeyModifiers::NONE,
                    };
                    engine.handle_event(Event::Mouse(event), true).unwrap();
                    let buffer = draw(&mut engine, size);
                    check(&engine, &buffer, &format!("{what} {kind:?} {n}"));
                }
            }
            // A click on every screen row, the sticky heading included, a few screens down:
            // the cursor goes to the row `row_at` named there, and nothing moves on the
            // sticky heading, which takes no click.
            let mut sticky = 0;
            for page in 0..4 {
                for y in 0..size.1 {
                    let buffer = draw(&mut engine, size);
                    let found = check(&engine, &buffer, &format!("{what} page {page}"));
                    let column = size.0 * 3 / 4;
                    let at = engine.row_at(column, y);
                    let before = engine.cursor_row();
                    let event = MouseEvent {
                        kind: MouseEventKind::Down(MouseButton::Left),
                        column,
                        row: y,
                        modifiers: KeyModifiers::NONE,
                    };
                    engine.handle_event(Event::Mouse(event), true).unwrap();
                    let after = engine.cursor_row();
                    match at {
                        Some(DiffRow {
                            kind: DiffRowKind::StickyHeader,
                            ..
                        })
                        | None => {
                            sticky += usize::from(at.is_some());
                            assert_eq!(after, before, "{what}: a click on row {y} moves nothing")
                        }
                        Some(at) => {
                            let after = after.unwrap();
                            assert_eq!(
                                DiffRow {
                                    side: after.side,
                                    ..at
                                },
                                after,
                                "{what}: click on row {y}"
                            );
                            assert_eq!(
                                found[usize::from(y)].as_ref().map(|r| r.file),
                                Some(after.file)
                            );
                        }
                    }
                    let buffer = draw(&mut engine, size);
                    check(
                        &engine,
                        &buffer,
                        &format!("{what} after the click on row {y}"),
                    );
                }
                run(
                    &mut engine,
                    size,
                    Command::PageDown,
                    &format!("{what} page {page}"),
                );
            }
            // Short screens scroll under a sticky heading at some point.
            assert!(
                size.1 > 24 || sticky > 0,
                "{what}: a sticky heading was clicked"
            );
        }
    }
}

#[test]
fn a_resize_a_layout_change_and_two_events_before_a_frame_keep_both_answers_right() {
    for wrap in [false, true] {
        let (mut engine, _directory) = open(Layout::Auto, wrap);
        let sizes = [(170, 30), (90, 14), (130, 20), (60, 10), (200, 50)];
        draw(&mut engine, sizes[0]);
        for step in 0..60 {
            let size = sizes[step % sizes.len()];
            let what = format!("wrap {wrap} step {step} {size:?}");
            // Each pair of events lands before the next frame.
            let (first, second) = match step % 4 {
                0 => (Some(Command::ToggleLayout), Command::Down),
                1 => (None, Command::PageDown),
                2 => (Some(Command::Down), Command::Down),
                _ => (Some(Command::NextHunk), Command::Up),
            };
            engine
                .handle_event(Event::Resize(size.0, size.1), true)
                .unwrap();
            if let Some(first) = first {
                engine.execute(first).unwrap();
            }
            assert!(engine.cursor_row().is_some(), "{what}: before the frame");
            engine.execute(second).unwrap();
            let before = engine.cursor_row().unwrap();
            let buffer = draw(&mut engine, size);
            check(&engine, &buffer, &what);
            // Drawing lays the rows out for the size, so it may move the cursor; it never
            // moves it to another file.
            assert_eq!(
                engine.cursor_row().unwrap().file,
                before.file,
                "{what}: the frame kept the file"
            );
            let event = MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 5,
                row: 2,
                modifiers: KeyModifiers::NONE,
            };
            engine.handle_event(Event::Mouse(event), true).unwrap();
            engine.execute(Command::Down).unwrap();
            let buffer = draw(&mut engine, size);
            check(&engine, &buffer, &format!("{what} wheel then down"));
        }
    }
}

#[test]
fn asking_moves_nothing() {
    let (mut engine, _directory) = open(Layout::Split, true);
    let size = (120, 16);
    draw(&mut engine, size);
    engine.execute(Command::NextFile).unwrap();
    engine.execute(Command::NextFile).unwrap();
    engine.execute(Command::Down).unwrap();
    let before = draw(&mut engine, size);
    let (cursor, top) = {
        let app = engine.app.as_ref().unwrap();
        (app.cursor, app.top)
    };
    for _ in 0..3 {
        let _ = engine.cursor_row();
        for y in 0..size.1 {
            let _ = engine.row_at(10, y);
            let _ = engine.row_at(100, y);
        }
    }
    let app = engine.app.as_ref().unwrap();
    assert_eq!((app.cursor, app.top), (cursor, top));
    assert_eq!(draw(&mut engine, size), before);
    assert_eq!(engine.row_at(500, 2), None, "outside the diff");
}

/// Two files with checked sources: `a.txt`, two one-line hunks with gaps before, between and
/// after them and no newline at its end, and `b.txt`, one hunk whose own lines end with no
/// newline. Its hunks' new-side lines are 5, 15 and 1-2.
fn checked(layout: Layout) -> (Engine, tempfile::TempDir) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("review.patch");
    let mut patch = String::from("diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n");
    patch.push_str("@@ -5 +5 @@\n-l5\n+L5\n@@ -15 +15 @@\n-l15\n+L15\n");
    patch.push_str("diff --git a/b.txt b/b.txt\n--- a/b.txt\n+++ b/b.txt\n");
    patch.push_str("@@ -1,2 +1,2 @@\n one\n-two\n\\ No newline at end of file\n+TWO\n\\ No newline at end of file\n");
    fs::write(&path, patch).unwrap();
    let (wake, ready) = mpsc::sync_channel(1);
    let options = Options {
        layout,
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
    let lines = |change: &[(u32, &str)]| {
        (1..=20)
            .map(|n| {
                change
                    .iter()
                    .find(|(at, _)| *at == n)
                    .map_or_else(|| format!("l{n}"), |(_, text)| (*text).to_owned())
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let texts = [
        (lines(&[]), lines(&[(5, "L5"), (15, "L15")])),
        ("one\ntwo".to_owned(), "one\nTWO".to_owned()),
    ];
    let app = engine.app.as_ref().unwrap();
    let mut store = SourceStore::sharing(&app.doc.files[0].spool).unwrap();
    let pairs: Vec<_> = app
        .doc
        .files
        .iter()
        .zip(&texts)
        .map(|(file, (old, new))| {
            let old = store.capture(old.as_bytes(), &|| Ok(())).unwrap();
            let new = store.capture(new.as_bytes(), &|| Ok(())).unwrap();
            Arc::new(check_pair(file.clone(), Some(old), Some(new), &|| Ok(())).unwrap())
        })
        .collect();
    install(&mut engine, &pairs);
    (engine, directory)
}

/// The hunk of `checked()`'s line, by its numbers: each hunk's file, old and new lines.
fn hunk_of(row: &DiffRow) -> Option<usize> {
    let spans = [(0, 5..=5, 5..=5), (0, 15..=15, 15..=15), (1, 1..=2, 1..=2)];
    spans
        .iter()
        .filter(|(file, ..)| *file == row.file)
        .position(|(_, old, new)| {
            row.old_line.is_some_and(|line| old.contains(&line))
                || row.new_line.is_some_and(|line| new.contains(&line))
        })
}

/// Every row of the frame of `checked()` against its hunk. Returns the first gap's screen
/// row, and notes revealed lines and the "No newline" marks drawn.
fn scan(
    engine: &Engine,
    buffer: &Buffer,
    what: &str,
    revealed: &mut std::collections::BTreeSet<u32>,
    marks: &mut [bool; 2],
) -> Option<u16> {
    let mut gap = None;
    for y in 0..buffer.area.height {
        for column in [buffer.area.width / 4, buffer.area.width * 3 / 4] {
            let Some(row) = engine.row_at(column, y) else {
                continue;
            };
            let line = text(buffer, y);
            match row.kind {
                DiffRowKind::Code => {
                    let expected = hunk_of(&row);
                    assert_eq!(row.hunk, expected, "{what}: row {y} {line:?} {row:?}");
                    if expected.is_none() {
                        revealed.insert(row.new_line.or(row.old_line).unwrap());
                    }
                }
                DiffRowKind::NoNewline => {
                    // a.txt's mark comes with its last gap's lines; b.txt's is its
                    // hunk's own.
                    let expected = (row.file == 1).then_some(0);
                    assert_eq!(row.hunk, expected, "{what}: row {y} {line:?}");
                    marks[row.file] = true;
                }
                DiffRowKind::Gap => {
                    assert_eq!(row.hunk, None);
                    assert!(line.contains("Show"), "{what}: an expandable gap {line:?}");
                    gap.get_or_insert(y);
                }
                _ => {}
            }
        }
    }
    gap
}

#[test]
fn revealed_context_and_its_no_newline_mark_are_in_no_hunk() {
    for layout in [Layout::Unified, Layout::Split] {
        let (mut engine, _directory) = checked(layout);
        let size = (120, 70);
        draw(&mut engine, size);
        assert_eq!(
            engine.app.as_ref().unwrap().split(),
            layout == Layout::Split
        );
        let mut revealed = std::collections::BTreeSet::new();
        let mut marks = [false, false];
        for click in 0..30 {
            let buffer = draw(&mut engine, size);
            let what = format!("{layout:?} click {click}");
            let gap = scan(&engine, &buffer, &what, &mut revealed, &mut marks);
            let Some(y) = gap else { break };
            // A click on a gap shows its lines, and the cursor is then on one of them.
            let event = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: size.0 / 4,
                row: y,
                modifiers: KeyModifiers::NONE,
            };
            engine.handle_event(Event::Mouse(event), true).unwrap();
            let cursor = engine.cursor_row().unwrap();
            if cursor.kind == DiffRowKind::Code {
                assert_eq!(
                    cursor.hunk,
                    hunk_of(&cursor),
                    "{what}: the cursor {cursor:?}"
                );
            }
        }
        // Back at the top, under no sticky heading: the lines a heading hid after a click.
        engine.execute(Command::FirstSource).unwrap();
        for _ in 0..3 {
            let buffer = draw(&mut engine, size);
            scan(
                &engine,
                &buffer,
                &format!("{layout:?} top"),
                &mut revealed,
                &mut marks,
            );
            engine.execute(Command::PageDown).unwrap();
        }
        // Lines from before the first hunk, between the two and after the last.
        let expected: std::collections::BTreeSet<u32> =
            (1..=20).filter(|n| ![5, 15].contains(n)).collect();
        assert_eq!(revealed, expected, "{layout:?}: every gap was shown");
        assert_eq!(marks, [true, true], "{layout:?}: both marks were drawn");
        // The cursor on each row, walking down: the same answer.
        engine.execute(Command::FirstSource).unwrap();
        for _ in 0..80 {
            let cursor = engine.cursor_row().unwrap();
            match cursor.kind {
                DiffRowKind::Code => assert_eq!(cursor.hunk, hunk_of(&cursor), "{cursor:?}"),
                DiffRowKind::NoNewline => {
                    assert_eq!(cursor.hunk, (cursor.file == 1).then_some(0), "{cursor:?}")
                }
                _ => {}
            }
            if engine.execute(Command::Down).unwrap() == InputEffect::Ignored {
                break;
            }
        }
    }
}
