use std::sync::mpsc;

use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect};

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
    let mut doc = storage::Document::default();
    storage::read(patch.as_bytes(), |file| doc.append(file)).unwrap();
    let (wake, _) = mpsc::sync_channel(1);
    let mut app = App::new(doc, label.into(), Layout::Split, Appearance::Dark, wake).unwrap();
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
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| {
            super::draw_with_controls(frame, app, frame.area(), None, controls).unwrap();
        })
        .unwrap();
    rows(terminal.backend().buffer())
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
        assert!(title.starts_with(" native-d"), "width {width}: {header:?}");
    }
    // A short label stays complete.
    let mut app = review(&rename(1), "review.patch");
    let header = draw(&mut app, 80, 24, None).remove(0);
    assert!(
        header.starts_with(" native-diff  /  review.patch "),
        "{header:?}"
    );
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
