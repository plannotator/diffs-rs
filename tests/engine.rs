use std::{
    fs,
    sync::mpsc,
    time::{Duration, Instant},
};

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use native_diff::engine::{
    Command, Engine, Input, InputEffect, Layout, Options, Side, SidebarPreference,
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
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("review.patch");
        fs::write(&path, patch("after")).unwrap();
        let (wake, ready) = mpsc::sync_channel(1);
        let engine = Engine::open(
            Input::Patch(path),
            Options {
                layout: Layout::Split,
                sidebar: SidebarPreference::Hide,
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
    use native_diff::input::LocalInput;
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
