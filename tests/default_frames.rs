//! Every frame the engine draws at its default settings, compared with digests recorded from
//! the engine before tab width, inline-change modes and whitespace hiding existed.
//!
//! Each case opens a patch from the corpus at one layout, wrap setting and size with default
//! options otherwise, waits for its colors, and then pages and scrolls through it. A frame's
//! digest covers every cell's symbol, colors and modifiers. A changed default shows up here as a
//! changed digest, with the frame printed.
//!
//! `DIFFS_RECORD_DEFAULT_FRAMES=1` rewrites `fixtures/options/default-frames.txt`. Only do that
//! from the commit the digests are meant to describe.

use std::{
    fmt::Write as _,
    fs,
    path::Path,
    sync::mpsc,
    time::{Duration, Instant},
};

use diffs_rs::engine::{Command, Engine, Input, Layout, Options};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

const PATCHES: [&str; 5] = [
    "fixtures/options/whitespace.patch",
    "fixtures/review.patch",
    "fixtures/position/review.patch",
    "fixtures/git-output/log.patch",
    "fixtures/git-output/format-patch.patch",
];
const LAYOUTS: [Layout; 3] = [Layout::Split, Layout::Unified, Layout::Auto];
const SIZES: [(u16, u16); 4] = [(40, 10), (64, 16), (100, 24), (170, 40)];
const GOLDEN: &str = "fixtures/options/default-frames.txt";

fn digest(buffer: &Buffer) -> u64 {
    // FNV-1a over each cell's text and style.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut cell_text = String::new();
    for cell in &buffer.content {
        cell_text.clear();
        write!(
            cell_text,
            "{}\u{1}{:?}\u{1}{:?}\u{1}{:?}\u{1}{:?}\u{2}",
            cell.symbol(),
            cell.fg,
            cell.bg,
            cell.modifier,
            cell.underline_color
        )
        .unwrap();
        for byte in cell_text.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

fn screen(buffer: &Buffer) -> String {
    let mut text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}

struct Case {
    engine: Engine,
    terminal: Terminal<TestBackend>,
    ready: mpsc::Receiver<()>,
}

impl Case {
    fn open(patch: &Path, layout: Layout, wrap: bool, (width, height): (u16, u16)) -> Self {
        let (wake, ready) = mpsc::sync_channel(1);
        let options = Options {
            layout,
            wrap,
            ..Options::default()
        };
        let engine = Engine::open(Input::Patch(patch.into()), options, wake).unwrap();
        let terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut case = Self {
            engine,
            terminal,
            ready,
        };
        let deadline = Instant::now() + Duration::from_secs(20);
        while case.engine.status().loading {
            case.engine.poll().unwrap();
            assert!(
                Instant::now() < deadline,
                "{} did not load",
                patch.display()
            );
            if case.engine.status().loading {
                let _ = case.ready.recv_timeout(Duration::from_millis(20));
            }
        }
        assert!(case.engine.status().error.is_none());
        case
    }

    /// Draws until the visible rows are colored and nothing else is pending.
    fn settled(&mut self) -> Buffer {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let mut drawn = Ok(());
            let engine = &mut self.engine;
            self.terminal
                .draw(|frame| drawn = engine.draw(frame, frame.area()))
                .unwrap();
            drawn.unwrap();
            self.engine.after_draw();
            let status = self.engine.status();
            if !status.coloring && !status.loading && !status.preparing_source {
                let buffer = self.terminal.backend().buffer().clone();
                // One more poll and frame must not change anything.
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
}

/// The steps each case takes after its first frame.
const STEPS: [(Command, usize); 9] = [
    (Command::PageDown, 12),
    (Command::Right, 3),
    (Command::PageUp, 4),
    (Command::Down, 5),
    (Command::Left, 1),
    (Command::SwitchSide, 1),
    (Command::ToggleNumbers, 1),
    (Command::ToggleTheme, 1),
    (Command::LastSource, 1),
];

type Frame = (String, u64, String);

fn cases() -> Vec<(String, Vec<Frame>)> {
    let mut cases = Vec::new();
    for patch in PATCHES {
        for layout in LAYOUTS {
            for wrap in [false, true] {
                for size in SIZES {
                    // Relative to the package, where Cargo runs tests, so that the title bar
                    // shows the same path on every machine.
                    let mut case = Case::open(Path::new(patch), layout, wrap, size);
                    let name = format!("{patch} {layout:?} wrap={wrap} {}x{}", size.0, size.1);
                    let buffer = case.settled();
                    let mut frames = vec![("start".to_string(), digest(&buffer), screen(&buffer))];
                    for (command, times) in STEPS {
                        for n in 0..times {
                            case.engine.execute(command).unwrap();
                            let buffer = case.settled();
                            frames.push((
                                format!("{command:?} {n}"),
                                digest(&buffer),
                                screen(&buffer),
                            ));
                        }
                    }
                    case.engine.close().unwrap();
                    cases.push((name, frames));
                }
            }
        }
    }
    cases
}

#[test]
fn default_settings_draw_every_frame_as_before() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cases = cases();
    let recorded: String = cases
        .iter()
        .map(|(name, frames)| {
            let digests: Vec<_> = frames.iter().map(|(_, d, _)| format!("{d:016x}")).collect();
            format!("{name}: {}\n", digests.join(" "))
        })
        .collect();
    if std::env::var_os("DIFFS_RECORD_DEFAULT_FRAMES").is_some() {
        fs::write(root.join(GOLDEN), &recorded).unwrap();
        return;
    }
    let expected = fs::read_to_string(root.join(GOLDEN)).unwrap();
    let expected: Vec<_> = expected.lines().collect();
    assert_eq!(expected.len(), cases.len(), "the number of cases changed");
    for ((name, frames), line) in cases.iter().zip(expected) {
        let (expected_name, digests) = line.split_once(": ").unwrap();
        assert_eq!(name, expected_name);
        let digests: Vec<_> = digests.split(' ').collect();
        assert_eq!(
            digests.len(),
            frames.len(),
            "{name}: the number of frames changed"
        );
        for ((step, digest, screen), expected) in frames.iter().zip(digests) {
            assert_eq!(
                format!("{digest:016x}"),
                expected,
                "{name}: frame {step} changed:\n{screen}"
            );
        }
    }
}
