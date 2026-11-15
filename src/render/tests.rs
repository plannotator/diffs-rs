use std::sync::mpsc;

use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect, style::Color};

use crate::{
    app::{App, Layout},
    engine::ReadingControls,
    storage,
    theme::Appearance,
};

/// The standalone viewer's reading controls, chosen by width as `reading_ui` does.
fn standalone(width: u16) -> ReadingControls<'static> {
    ReadingControls {
        hint: if width < 160 {
            "/ search  V select"
        } else {
            "/ search  F3/Shift-F3 next/prev  V select  y copy  Ctrl-S save"
        },
        help: "  READING · original diff text\n  /              Literal search",
        status: None,
    }
}

fn review(patch: &str, label: &str) -> App {
    review_in(patch, label, Appearance::Dark)
}

fn review_in(patch: &str, label: &str, appearance: Appearance) -> App {
    let mut doc = storage::Document::default();
    storage::read(patch.as_bytes(), |file| doc.append(file)).unwrap();
    let (wake, _) = mpsc::sync_channel(1);
    let mut app = App::new(doc, label.into(), Layout::Split, appearance, wake).unwrap();
    app.loading = false;
    app
}

/// A metadata-only review, so the footer shows the longest default status.
fn rename(count: usize) -> String {
    (0..count)
        .map(|n| {
            format!(
                "diff --git a/old{n}.txt b/new{n}.txt\nsimilarity index 100%\nrename from old{n}.txt\nrename to new{n}.txt\n"
            )
        })
        .collect()
}

fn draw(
    app: &mut App,
    width: u16,
    height: u16,
    controls: Option<&ReadingControls<'_>>,
) -> Vec<String> {
    rows(&draw_buffer(app, width, height, controls))
}

fn draw_buffer(
    app: &mut App,
    width: u16,
    height: u16,
    controls: Option<&ReadingControls<'_>>,
) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| {
            super::draw_with_controls(frame, app, frame.area(), None, controls).unwrap();
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

fn rows(buffer: &Buffer) -> Vec<String> {
    let Rect { width, height, .. } = buffer.area;
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect()
}

const STATUS: &str = "Patch view · syntax context may be incomplete";

#[test]
fn header_label_keeps_a_gap_before_the_file_count() {
    let mut app = review(&rename(5), &"a-very-long-review-label/".repeat(8));
    for width in [40, 50, 60, 80, 100, 120, 140, 200] {
        let header = draw(&mut app, width, 24, None).remove(0);
        let right = format!(
            "5 files  ·  {}  ·  dark ",
            if app.split() { "SPLIT" } else { "UNIFIED" }
        );
        assert!(header.ends_with(&right), "width {width}: {header:?}");
        let title = header.strip_suffix(&right).unwrap();
        assert!(title.ends_with("  "), "width {width}: {header:?}");
        assert!(title.trim_end().ends_with('…'), "width {width}: {header:?}");
        assert!(title.starts_with(" diffs"), "width {width}: {header:?}");
    }
    // A short label stays complete.
    let mut app = review(&rename(1), "review.patch");
    let header = draw(&mut app, 80, 24, None).remove(0);
    assert!(header.starts_with(" diffs  /  review.patch "), "{header:?}");
    assert!(!header.contains('…'), "{header:?}");
}

#[test]
fn footer_status_stays_readable_beside_the_hint() {
    let mut app = review(&rename(1), "review.patch");
    let widths = [
        63, 80, 83, 99, 100, 110, 120, 130, 140, 150, 159, 160, 170, 200, 240,
    ];
    for width in widths {
        for controls in [None, Some(standalone(width))] {
            let footer = draw(&mut app, width, 24, controls.as_ref()).pop().unwrap();
            let hint = controls.as_ref().map_or("? help  q quit", |c| c.hint);
            assert!(
                footer.contains("? help  q quit") && footer.trim_end().ends_with(hint),
                "width {width}: {footer:?}"
            );
            // The default status and the standalone hints fit together from 83 columns.
            if controls.is_none() || width >= 83 {
                assert!(
                    footer.starts_with(&format!(" {STATUS} ")),
                    "width {width}, controls {}: {footer:?}",
                    controls.is_some()
                );
            }
        }
    }
    // Wide terminals keep every view hint.
    let footer = draw(&mut app, 240, 24, Some(&standalone(240)))
        .pop()
        .unwrap();
    assert!(
        footer
            .contains(" f files  u layout  w wrap  t theme  ? help  q quit  / search  F3/Shift-F3"),
        "{footer:?}"
    );
    let footer = draw(&mut app, 130, 24, Some(&standalone(130)))
        .pop()
        .unwrap();
    assert!(
        footer.contains(" f files  u layout  w wrap  t theme  ? help  q quit  / search  V select "),
        "{footer:?}"
    );
    // Between 100 and 119 columns the view toggles give way to the status.
    let footer = draw(&mut app, 110, 24, Some(&standalone(110)))
        .pop()
        .unwrap();
    assert!(
        footer.ends_with(" f files  ? help  q quit  / search  V select "),
        "{footer:?}"
    );
}

#[test]
fn footer_truncates_a_long_status_with_an_ellipsis() {
    let mut app = review(&rename(1), "review.patch");
    for width in [40, 50, 62, 82] {
        for controls in [None, Some(standalone(width))] {
            // Optional engine hints give way first; help, quit, and caller hints remain.
            let footer = draw(&mut app, width, 24, controls.as_ref()).pop().unwrap();
            let suffix = match &controls {
                Some(controls) => format!(" ? help  q quit  {} ", controls.hint),
                None => " ? help  q quit ".into(),
            };
            if width == 82 && controls.is_none() {
                assert!(footer.starts_with(&format!(" {STATUS} ")), "{footer:?}");
                continue;
            }
            assert!(footer.ends_with(&suffix), "{footer:?}");
            let status = footer.strip_suffix(&suffix).unwrap();
            assert!(status.starts_with(" P"), "{footer:?}");
            assert!(status.ends_with("… "), "{footer:?}");
        }
    }
}

#[test]
fn short_help_lines_keep_their_leading_space() {
    let mut app = review(&rename(1), "review.patch");
    app.help = true;
    let controls = standalone(80);
    let screen = draw(&mut app, 80, 12, Some(&controls)).join("\n");
    let line = screen
        .lines()
        .find(|line| line.contains(controls.hint))
        .unwrap();
    assert!(line.contains(&format!("│ {}", controls.hint)), "{screen}");
    assert!(screen.contains("│ j/k ↑↓ move"), "{screen}");
}

#[test]
fn small_terminal_notice_wraps_instead_of_clipping() {
    let mut app = review(&rename(1), "review.patch");
    for (width, height) in [(39, 10), (30, 8), (24, 9)] {
        let screen = draw(&mut app, width, height, None);
        let text = screen
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            text.contains("Enlarge terminal to at least 40 columns × 10 rows q quit"),
            "{width}x{height}: {screen:#?}"
        );
    }
}

/// An added, a copied, a deleted, a modified, and a renamed file under `src/`.
const KINDS: &str = "diff --git a/src/added.rs b/src/added.rs
new file mode 100644
index 0000000..1111111
--- /dev/null
+++ b/src/added.rs
@@ -0,0 +1,2 @@
+fn added() {}
+fn more() {}
diff --git a/src/modified.rs b/src/modified.rs
index 2222222..3333333 100644
--- a/src/modified.rs
+++ b/src/modified.rs
@@ -1 +1 @@
-fn old() {}
+fn new() {}
diff --git a/src/deleted.rs b/src/deleted.rs
deleted file mode 100644
index 4444444..0000000
--- a/src/deleted.rs
+++ /dev/null
@@ -1,2 +0,0 @@
-fn gone() {}
-fn also() {}
diff --git a/old.rs b/src/renamed.rs
similarity index 100%
rename from old.rs
rename to src/renamed.rs
diff --git a/src/modified.rs b/src/copied.rs
similarity index 100%
copy from src/modified.rs
copy to src/copied.rs
";

/// The foreground and background of each cell of `text` on the first row of `area` that
/// contains `row` and then `text`.
fn styled(buffer: &Buffer, area: Rect, row: &str, text: &str) -> Vec<(Color, Color)> {
    for y in area.top()..area.bottom() {
        let line = (area.left()..area.right())
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>();
        let Some(start) = line.find(row) else {
            continue;
        };
        let Some(found) = line[start..].find(text) else {
            continue;
        };
        let column = line[..start + found].chars().count() as u16;
        return (0..text.chars().count() as u16)
            .map(|n| {
                let cell = &buffer[(area.x + column + n, y)];
                (cell.fg, cell.bg)
            })
            .collect();
    }
    panic!("no {row:?} row with {text:?}:\n{}", rows(buffer).join("\n"));
}

/// The foreground of each cell of `text`, in the row containing `row`.
fn fg(buffer: &Buffer, area: Rect, row: &str, text: &str) -> Vec<Color> {
    styled(buffer, area, row, text)
        .into_iter()
        .map(|(fg, _)| fg)
        .collect()
}

#[test]
fn sidebar_colors_status_letters_and_counts_by_kind() {
    for (appearance, modified, renamed) in [
        (
            Appearance::Dark,
            Color::Rgb(0x00, 0x9f, 0xff),
            Color::Rgb(0xe1, 0x30, 0xac),
        ),
        (
            Appearance::Light,
            Color::Rgb(0x1a, 0x85, 0xd4),
            Color::Rgb(0xbd, 0x2e, 0x90),
        ),
    ] {
        let mut app = review_in(KINDS, "kinds.patch", appearance);
        let buffer = draw_buffer(&mut app, 160, 24, None);
        let tree = app.sidebar.area;
        let c = app.colors.clone();
        assert_eq!(
            (c.modified, c.renamed),
            (modified, renamed),
            "{appearance:?}"
        );
        let (add, del, dim) = (c.added, c.deleted, c.dim);
        for (row, summary, expected) in [
            ("added.rs", "A +2 −0", [add, dim, add, add, dim, dim, dim]),
            (
                "modified.rs",
                "M +1 −1",
                [modified, dim, add, add, dim, del, del],
            ),
            ("deleted.rs", "D +0 −2", [del, dim, dim, dim, dim, del, del]),
            (
                "renamed.rs",
                "R +0 −0",
                [renamed, dim, dim, dim, dim, dim, dim],
            ),
            (
                "copied.rs",
                "C +0 −0",
                [renamed, dim, dim, dim, dim, dim, dim],
            ),
            // A folder's file count stays muted; its aggregate counts are colored.
            ("src/", "5 +3 −3", [dim, dim, add, add, dim, del, del]),
        ] {
            let cells = styled(&buffer, tree, row, summary);
            assert_eq!(
                cells.iter().map(|(fg, _)| *fg).collect::<Vec<_>>(),
                expected,
                "{appearance:?} {row}"
            );
            // Colors change only the text: the row keeps the sidebar background.
            assert!(cells.iter().all(|(_, bg)| *bg == c.chrome), "{row}");
        }
        // The text and layout are unchanged.
        let text = rows(&buffer);
        let folder = text[2].chars().take(33).collect::<String>();
        assert_eq!(folder, "  ▾ src/                5 +3 −3 │");
        let file = text[3].chars().take(33).collect::<String>();
        assert_eq!(file, " *  added.rs            A +2 −0 │");
        // The sticky file header colors its status and counts the same way.
        let diff = app.sidebar.diff;
        assert_eq!(fg(&buffer, diff, "src/added.rs", "Added"), [add; 5]);
        assert_eq!(
            fg(&buffer, diff, "src/added.rs", "+2 −0"),
            [add, add, dim, dim, dim]
        );
    }
}

#[test]
fn sidebar_cursor_row_keeps_kind_colors_on_its_highlight() {
    let mut app = review(KINDS, "kinds.patch");
    draw_buffer(&mut app, 160, 24, None);
    // Focusing Files puts the keyboard cursor on the current file, `added.rs`.
    app.files_command().unwrap();
    let buffer = draw_buffer(&mut app, 160, 24, None);
    let c = app.colors.clone();
    let cells = styled(&buffer, app.sidebar.area, ">*", "A +2 −0");
    assert_eq!(
        cells,
        [c.added, c.dim, c.added, c.added, c.dim, c.dim, c.dim]
            .map(|fg| (fg, c.context))
            .to_vec()
    );
    let name = styled(&buffer, app.sidebar.area, ">*", "added.rs");
    assert!(name.iter().all(|(fg, bg)| (*fg, *bg) == (c.fg, c.context)));
    // Other rows keep the sidebar background.
    let other = styled(&buffer, app.sidebar.area, "modified.rs", "M +1 −1");
    assert!(other.iter().all(|(_, bg)| *bg == c.chrome));
}

#[test]
fn file_headers_color_each_status_kind() {
    let mut app = review(KINDS, "kinds.patch");
    // Every file header fits on one screen.
    let buffer = draw_buffer(&mut app, 160, 40, None);
    let c = app.colors.clone();
    let diff = app.sidebar.diff;
    for (path, status, color) in [
        ("src/added.rs", "Added", c.added),
        ("src/modified.rs", "Modified", c.modified),
        ("src/deleted.rs", "Deleted", c.deleted),
        ("old.rs → src/renamed.rs", "Renamed", c.renamed),
        ("src/modified.rs → src/copied.rs", "Copied", c.renamed),
    ] {
        assert_eq!(
            fg(&buffer, diff, path, status),
            vec![color; status.len()],
            "{path}"
        );
    }
    assert_eq!(
        fg(&buffer, diff, "src/modified.rs   ", "+1 −1"),
        [c.added, c.added, c.dim, c.deleted, c.deleted]
    );
}

#[test]
fn truncated_summaries_keep_a_muted_ellipsis() {
    let (wake, _) = mpsc::sync_channel(1);
    let app = App::new(
        storage::Document::default(),
        "empty".into(),
        Layout::Split,
        Appearance::Dark,
        wake,
    )
    .unwrap();
    let c = &app.colors;
    let mut buffer = Buffer::empty(Rect::new(0, 0, 16, 1));
    let text = super::ellipsize("2188 +1009257 −4024", 16);
    assert_eq!(text, "2188 +1009257 −…");
    super::label(&mut buffer, 0, 0, 16, &text, c.dim, c.chrome, false);
    let summary = super::Summary {
        at: 0,
        lead: 4,
        lead_fg: c.dim,
        gap: 1,
        additions: 1_009_257,
        deletions: 4024,
    };
    super::color_summary(&mut buffer, 0, 0, 15, summary, c);
    let colors = (0..16).map(|x| buffer[(x, 0)].fg).collect::<Vec<_>>();
    let mut expected = vec![c.dim; 5];
    expected.extend([c.added; 8]);
    expected.extend([c.dim, c.deleted, c.dim]);
    assert_eq!(colors, expected);
}
