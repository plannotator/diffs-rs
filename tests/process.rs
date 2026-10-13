use std::{
    fs,
    io::{Read, Write},
    process::Command,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use serde_json::Value;

fn viewer_binary() -> std::path::PathBuf {
    std::env::var_os("NATIVE_DIFF_CONTRACT_BINARY")
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_native-diff").into())
        .into()
}

#[path = "process/render_contract.rs"]
mod render_contract;

#[path = "process/geometry_contract.rs"]
mod geometry_contract;

#[path = "process/horizontal_contract.rs"]
mod horizontal_contract;

#[path = "process/git_paths_contract.rs"]
mod git_paths_contract;

#[path = "process/git_output_contract.rs"]
mod git_output_contract;

#[path = "process/syntax_contract.rs"]
mod syntax_contract;

#[path = "process/delivery_contract.rs"]
mod delivery_contract;

#[path = "process/sidebar_contract.rs"]
mod sidebar_contract;

#[path = "process/storage_contract.rs"]
mod storage_contract;

#[path = "process/local_source_contract.rs"]
mod local_source_contract;

#[path = "process/full_source_color_contract.rs"]
mod full_source_color_contract;

#[path = "process/context_expansion_contract.rs"]
mod context_expansion_contract;

#[path = "process/engine_host_contract.rs"]
mod engine_host_contract;

#[path = "process/search_contract.rs"]
mod search_contract;

#[path = "process/selection_contract.rs"]
mod selection_contract;

#[path = "process/save_contract.rs"]
mod save_contract;

#[path = "process/copy_contract.rs"]
mod copy_contract;

fn generated_patch(old: &[u8], new: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
    generated_patch_in_language(old, new, "rs")
}

fn generated_patch_in_language(
    old: &[u8],
    new: &[u8],
    extension: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let old_path = format!("old.{extension}");
    let new_path = format!("new.{extension}");
    fs::write(dir.path().join(&old_path), old).unwrap();
    fs::write(dir.path().join(&new_path), new).unwrap();
    let output = Command::new("git")
        .args([
            "diff",
            "--no-index",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--unified=100000",
            &old_path,
            &new_path,
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let path = dir.path().join("input.patch");
    fs::write(&path, output.stdout).unwrap();
    (dir, path)
}

#[test]
fn source_bytes_survive_git_patch_parsing() {
    let mut long_old = format!("\t{}", "é日本語 ".repeat(12_000)).into_bytes();
    long_old.extend_from_slice(b"\xff\r\n\nold tail");
    let mut long_new = format!("\t{}", "e\u{301}日本語 ".repeat(12_000)).into_bytes();
    long_new.extend_from_slice(b"\xfe\r\n\nnew tail");
    for (old, new) in [
        (
            b"fn run() {\n    retries(3);\n}\n".as_slice(),
            b"fn run() {\n\n    retries(5);\n}\n".as_slice(),
        ),
        (b"\told\r\nlast".as_slice(), b"\tnew\r\nlast!".as_slice()),
        (b"old\r".as_slice(), b"new\r".as_slice()),
        (
            "const S: &str = \"café 日本語 👩‍💻\";\n".as_bytes(),
            "const S: &str = \"cafe\u{301} 日本語 ☕\";\n".as_bytes(),
        ),
        (
            b"old\xff\x1b[2J\n".as_slice(),
            b"new\xfe\x1b]52;c;evil\x07\n".as_slice(),
        ),
        (b"".as_slice(), b"one new line\n".as_slice()),
        (long_old.as_slice(), long_new.as_slice()),
    ] {
        let (_dir, path) = generated_patch(old, new);
        let output = Command::new(viewer_binary())
            .arg("inspect")
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let model: Value = serde_json::from_slice(&output.stdout).unwrap();
        for (side, expected) in [("old", old), ("new", new)] {
            let mut actual = Vec::new();
            let mut numbers = Vec::new();
            for line in model["lines"].as_array().unwrap() {
                if let Some(n) = line[side].as_u64() {
                    numbers.push(n);
                    actual.extend(
                        line["bytes"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|b| b.as_u64().unwrap() as u8),
                    );
                }
            }
            assert_eq!(actual, expected, "{side} bytes changed");
            assert_eq!(numbers, (1..=numbers.len() as u64).collect::<Vec<_>>());
        }
        let reference = _dir.path().join("reference.json");
        let expected = Command::new("python3")
            .arg("tools/patch_reference.py")
            .arg(&path)
            .arg(&reference)
            .output()
            .unwrap();
        assert!(
            expected.status.success(),
            "{}",
            String::from_utf8_lossy(&expected.stderr)
        );
        let expected: Value = serde_json::from_slice(&expected.stdout).unwrap();
        let exported = Command::new(viewer_binary())
            .arg("inspect")
            .arg(&path)
            .arg("--canonical")
            .output()
            .unwrap();
        assert!(exported.status.success());
        let mut digest = Command::new("shasum")
            .args(["-a", "256"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        digest
            .stdin
            .take()
            .unwrap()
            .write_all(&exported.stdout)
            .unwrap();
        let digest = digest.wait_with_output().unwrap();
        assert!(digest.status.success());
        assert_eq!(
            String::from_utf8(digest.stdout)
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap(),
            expected["source_sha256"].as_str().unwrap()
        );
    }
}

#[test]
fn git_empty_file_metadata_survives_and_remains_visible() {
    let dir = tempfile::tempdir().unwrap();
    let names = ["plain", "with spaces", "quote\"name", "日本語"];
    let git = |args: &[&str]| {
        let result = Command::new("git")
            .args(args)
            .current_dir(dir.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        result.stdout
    };
    git(&["init", "--quiet"]);
    for name in names {
        fs::write(dir.path().join(name), []).unwrap();
    }
    git(&["add", "--", "."]);
    for (patch, status) in [
        (
            git(&["diff", "--cached", "--no-ext-diff", "--no-textconv"]),
            "Added",
        ),
        {
            for name in names {
                fs::remove_file(dir.path().join(name)).unwrap();
            }
            (git(&["diff", "--no-ext-diff", "--no-textconv"]), "Deleted")
        },
    ] {
        let path = dir.path().join("metadata.patch");
        fs::write(&path, patch).unwrap();
        let output = Command::new(viewer_binary())
            .arg("inspect")
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let doc: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(doc["lines"].as_array().unwrap().is_empty());
        let files = doc["files"].as_array().unwrap();
        assert_eq!(files.len(), names.len());
        for name in names {
            let file = files
                .iter()
                .find(|f| f["path"] == name)
                .expect("missing metadata path");
            assert_eq!(file["status"], status);
            assert_eq!(
                file["old_mode"],
                if status == "Added" { "0" } else { "100644" }
            );
            assert_eq!(
                file["new_mode"],
                if status == "Deleted" { "0" } else { "100644" }
            );
        }
        let mut terminal = Session::open(&path);
        terminal.wait(|s| {
            s.contents().contains("4 files") && s.contents().contains("no textual hunks")
        });
        let before = terminal.settled_grid();
        for key in [b"g", b"G"] {
            terminal.keys(key);
            assert_eq!(terminal.settled_grid(), before);
        }
        terminal.finish(b"q", 0);
    }
    let empty = dir.path().join("empty.patch");
    fs::write(&empty, []).unwrap();
    let mut terminal = Session::open(&empty);
    terminal.wait(|s| s.contents().contains("No changes"));
    let before = terminal.settled_grid();
    terminal.keys(b"GgnN");
    assert_eq!(terminal.settled_grid(), before);
    terminal.finish(b"q", 0);
}

#[test]
fn malformed_and_nonterminal_inputs_fail_without_escape_output() {
    let dir = tempfile::tempdir().unwrap();
    for input in [
        b"not a patch\n".as_slice(),
        b"diff --cc file.rs\n".as_slice(),
        b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1,5 +1,5 @@\n-x\n+y\n".as_slice(),
        b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -0,1 +1,1 @@\n-x\n+y\n".as_slice(),
        b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-x\n+y\n@@ -1 +1 @@\n-x\n+z\n"
            .as_slice(),
    ] {
        let path = dir.path().join("bad.patch");
        fs::write(&path, input).unwrap();
        let output = Command::new(viewer_binary())
            .arg("inspect")
            .arg(&path)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.contains(&0x1b));
    }
    let output = Command::new(viewer_binary())
        .args(["patch", "fixtures/review.patch"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.contains(&0x1b));
}

struct Session {
    master: Box<dyn portable_pty::MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    output: mpsc::Receiver<Vec<u8>>,
    parser: vt100::Parser,
    raw: Vec<u8>,
    pending: Vec<u8>,
    frames: Option<Vec<vt100::Screen>>,
    initial_modes: String,
}

impl Session {
    fn open(path: &std::path::Path) -> Self {
        Self::open_with(&viewer_binary(), path)
    }

    fn open_with(binary: &std::path::Path, path: &std::path::Path) -> Self {
        let mut cmd = CommandBuilder::new(binary);
        cmd.arg("--no-sidebar");
        cmd.arg("patch");
        cmd.arg(path);
        Self::spawn(cmd)
    }

    #[track_caller]
    fn settled_grid(&mut self) -> Vec<u8> {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            assert!(
                Instant::now() < deadline,
                "viewer did not settle:\n{}",
                self.parser.screen().contents()
            );
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "viewer exited before settling"
            );
            match self.output.recv_timeout(Duration::from_millis(100)) {
                Ok(bytes) => self.ingest(bytes),
                Err(mpsc::RecvTimeoutError::Disconnected) => panic!("viewer output closed"),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let text = self.parser.screen().contents();
                    if text.contains("q quit")
                        && !text.contains("Loading patch")
                        && !text.contains("Highlighting visible")
                        && !text.contains("coloring visible")
                        && !text.contains("Preparing source")
                    {
                        return self.parser.screen().contents_formatted();
                    }
                }
            }
        }
    }

    fn spawn(mut cmd: CommandBuilder) -> Self {
        let pair = NativePtySystem::default()
            .openpty(PtySize {
                rows: 50,
                cols: 160,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let initial_modes = format!("{:?}", pair.master.get_termios().unwrap());
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env_remove("NO_COLOR");
        let child = pair.slave.spawn_command(cmd).unwrap();
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();
        let writer = pair.master.take_writer().unwrap();
        let (send, output) = mpsc::channel();
        thread::spawn(move || {
            let mut buf = [0; 8192];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 || send.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        });
        Self {
            master: pair.master,
            child,
            writer,
            output,
            parser: vt100::Parser::new(50, 160, 0),
            raw: Vec::new(),
            pending: Vec::new(),
            frames: None,
            initial_modes,
        }
    }

    fn wait(&mut self, predicate: impl Fn(&vt100::Screen) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            if predicate(self.parser.screen()) {
                return;
            }
            if let Ok(bytes) = self.output.recv_timeout(Duration::from_millis(50)) {
                self.ingest(bytes);
            }
            if let Some(status) = self.child.try_wait().unwrap() {
                while let Ok(bytes) = self.output.try_recv() {
                    self.ingest(bytes);
                }
                panic!(
                    "Viewer exited before the expected screen: {status:?}\nLast output: {:?}",
                    String::from_utf8_lossy(&self.raw[self.raw.len().saturating_sub(2048)..])
                );
            }
        }
        panic!(
            "Terminal did not reach expected state:\n{}",
            self.parser.screen().contents()
        );
    }

    fn keys(&mut self, keys: &[u8]) {
        self.writer.write_all(keys).unwrap();
        self.writer.flush().unwrap();
    }

    fn ingest(&mut self, bytes: Vec<u8>) {
        self.raw.extend(&bytes);
        self.pending.extend(bytes);
        let end = b"\x1b[?2026l";
        while let Some(index) = self.pending.windows(end.len()).position(|s| s == end) {
            self.parser.process(&self.pending[..index + end.len()]);
            self.pending.drain(..index + end.len());
            if let Some(frames) = &mut self.frames {
                assert!(frames.len() < 512, "bounded frame capture overflowed");
                frames.push(self.parser.screen().clone());
            }
        }
    }

    fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.screen_mut().set_size(rows, cols);
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
    }

    fn capture(&mut self, name: &str) {
        let Ok(directory) = std::env::var("NATIVE_DIFF_CAPTURE_DIR") else {
            return;
        };
        while let Ok(bytes) = self.output.recv_timeout(Duration::from_millis(80)) {
            self.ingest(bytes);
        }
        let screen = self.parser.screen();
        let (rows, cols) = screen.size();
        let color = |c: vt100::Color| match c {
            vt100::Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
            _ => "#0a0a0a".into(),
        };
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"><title>PTY cell preview, not a Ghostty screenshot</title>",
            cols * 10,
            rows * 20
        );
        for y in 0..rows {
            for x in 0..cols {
                let cell = screen.cell(y, x).unwrap();
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"10\" height=\"20\" fill=\"{}\"/>",
                    x * 10,
                    y * 20,
                    color(cell.bgcolor())
                ));
            }
        }
        for y in 0..rows {
            for x in 0..cols {
                let cell = screen.cell(y, x).unwrap();
                if cell.contents().trim().is_empty() {
                    continue;
                }
                let escaped = cell
                    .contents()
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-family=\"Menlo,monospace\" font-size=\"15\" fill=\"{}\">{escaped}</text>",x*10,y*20+15,color(cell.fgcolor())));
            }
        }
        svg.push_str("</svg>");
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            std::path::Path::new(&directory).join(format!("{name}.svg")),
            svg,
        )
        .unwrap();
        fs::write(
            std::path::Path::new(&directory).join(format!("{name}.ansi")),
            screen.contents_formatted(),
        )
        .unwrap();
    }

    fn finish(&mut self, keys: &[u8], code: u32) {
        self.keys(keys);
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert_eq!(status.exit_code(), code);
                break;
            }
            assert!(Instant::now() < deadline, "viewer did not exit");
            thread::sleep(Duration::from_millis(10));
        }
        while let Ok(bytes) = self.output.recv_timeout(Duration::from_millis(100)) {
            self.ingest(bytes);
        }
        self.parser.process(&self.pending);
        self.pending.clear();
        assert!(
            !self.parser.screen().alternate_screen(),
            "alternate screen left active"
        );
        assert!(!self.parser.screen().hide_cursor(), "cursor left hidden");
        assert!(
            self.raw.windows(8).any(|w| w == b"\x1b[?1000l"),
            "mouse mode not restored"
        );
        assert_eq!(
            format!("{:?}", self.master.get_termios().unwrap()),
            self.initial_modes,
            "terminal modes changed after exit"
        );
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn has_active_source(screen: &vt100::Screen, gutter: &str, text: &str) -> bool {
    let (rows, columns) = screen.size();
    let expected = format!("▎{gutter}{text}");
    (1..rows - 1).any(|y| {
        (0..columns)
            .map(|x| screen.cell(y, x).unwrap().contents())
            .collect::<String>()
            .contains(&expected)
    })
}

#[test]
fn shared_position_fixture_is_reproducible_and_preserves_both_sources() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/position");
    let patch = Command::new("git")
        .args([
            "-c",
            "core.quotePath=true",
            "diff",
            "--no-index",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--unified=1000",
            "--",
            "old",
            "new",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    assert_eq!(patch.status.code(), Some(1));
    assert_eq!(patch.stdout, fs::read(root.join("review.patch")).unwrap());
    let output = Command::new(viewer_binary())
        .arg("inspect")
        .arg(root.join("review.patch"))
        .output()
        .unwrap();
    assert!(output.status.success());
    let doc: Value = serde_json::from_slice(&output.stdout).unwrap();
    for side in ["old", "new"] {
        let mut bytes = Vec::new();
        for line in doc["lines"].as_array().unwrap() {
            if line[side].is_number() {
                bytes.extend(
                    line["bytes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|b| b.as_u64().unwrap() as u8),
                );
            }
        }
        assert_eq!(bytes, fs::read(root.join(side).join("review.ts")).unwrap());
    }
}

fn reference_wrapped_segments(line: &str, width: usize) -> Vec<(usize, String)> {
    use unicode_segmentation::UnicodeSegmentation;
    use unicode_width::UnicodeWidthStr;
    let mut segments = vec![(0, String::new())];
    let mut column = 0;
    let mut used = 0;
    for (byte, text) in line.grapheme_indices(true) {
        let size = if text == "\t" {
            4 - column % 4
        } else {
            text.width().max(1)
        };
        if used != 0 && used + size > width {
            segments.push((byte, String::new()));
            used = 0;
        }
        let row = &mut segments.last_mut().unwrap().1;
        if text == "\t" {
            row.push_str(&" ".repeat(size));
        } else {
            row.push_str(text);
        }
        used += size;
        column += size;
    }
    segments
}

fn active_pane(screen: &vt100::Screen, split: bool, old: bool, numbers: bool) -> (u16, String) {
    let (height, width) = screen.size();
    let start = if split && !old { width.div_ceil(2) } else { 0 };
    let end = if split && old { (width - 1) / 2 } else { width };
    let y = (1..height - 1)
        .find(|y| screen.cell(*y, start).unwrap().contents() == "▎")
        .unwrap_or_else(|| panic!("active source side lost:\n{}", screen.contents()));
    let gutter = if numbers {
        if split { 6 } else { 10 }
    } else {
        3
    };
    let body = (start + gutter..end)
        .map(|x| screen.cell(y, x).unwrap().contents())
        .collect();
    (y, body)
}

#[test]
fn text_anchor_survives_wrap_layout_and_width_changes_on_both_sides() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/position");
    for (old, marker) in [
        (true, "OLD_ANCHOR"),
        (false, "NEW_ANCHOR"),
        (true, "SHARED_ANCHOR"),
        (false, "SHARED_ANCHOR"),
    ] {
        let source =
            fs::read_to_string(root.join(if old { "old" } else { "new" }).join("review.ts"))
                .unwrap();
        let line = source.lines().find(|line| line.contains(marker)).unwrap();
        let anchor = line.find(marker).unwrap();
        let mut terminal = Session::open(&root.join("review.patch"));
        terminal.settled_grid();
        terminal.keys(b"w");
        terminal.wait(|s| s.contents().contains('↪'));
        let (start, end) = if old { (0, 79) } else { (80, 160) };
        let (y, x) = (1..49)
            .flat_map(|y| (start..end - marker.len() as u16).map(move |x| (y, x)))
            .find(|(y, x)| {
                (0..marker.len() as u16)
                    .map(|n| {
                        terminal
                            .parser
                            .screen()
                            .cell(*y, *x + n)
                            .unwrap()
                            .contents()
                    })
                    .collect::<String>()
                    == marker
            })
            .expect("fixture anchor is visible before selection");
        terminal.keys(format!("\x1b[<0;{};{}M", x + 1, y + 1).as_bytes());
        terminal.wait(|s| s.cell(y, start).unwrap().contents() == "▎");
        for (width, height, key, split, wrap, numbers) in [
            (119, 30, "", false, true, true),
            (80, 24, "", false, true, true),
            (160, 50, "", true, true, true),
            (160, 50, "u", false, true, true),
            (160, 50, "u", true, true, true),
            (160, 50, "w", true, false, true),
            (100, 30, "", true, false, true),
            (100, 30, "w", true, true, true),
            (100, 30, "l", true, true, false),
            (160, 50, "l", true, true, true),
        ] {
            terminal.resize(height, width);
            terminal.keys(format!("{key}?").as_bytes());
            terminal.wait(|s| s.contents().contains("Reading controls"));
            terminal.keys(b"\x1b");
            terminal.wait(|s| !s.contents().contains("Reading controls"));
            let (marker_y, body) = active_pane(terminal.parser.screen(), split, old, numbers);
            assert!(
                marker_y <= y.min(height - 2) + 1,
                "anchor fell below its screen position"
            );
            let pane_width = if split {
                if old { (width - 1) / 2 } else { width / 2 }
            } else {
                width
            };
            let gutter = if numbers {
                if split { 6 } else { 10 }
            } else {
                3
            };
            if wrap {
                let segments = reference_wrapped_segments(line, (pane_width - gutter) as usize);
                let expected = &segments
                    .iter()
                    .rev()
                    .find(|(byte, _)| *byte <= anchor)
                    .unwrap()
                    .1;
                assert_eq!(
                    body.trim_end(),
                    expected.trim_end(),
                    "text position changed: {marker}, {width}x{height}, key {key}"
                );
            } else {
                assert!(body.contains(marker), "unwrapping lost {marker}: {body}");
            }
            if !split && width == 160 {
                let x = (10..width - marker.len() as u16)
                    .find(|x| {
                        (0..marker.len() as u16)
                            .map(|n| {
                                terminal
                                    .parser
                                    .screen()
                                    .cell(marker_y, *x + n)
                                    .unwrap()
                                    .contents()
                            })
                            .collect::<String>()
                            == marker
                    })
                    .expect("anchor text is visible for unified selection");
                terminal.keys(format!("\x1b[<0;{};{}M?", x + 1, marker_y + 1).as_bytes());
                terminal.wait(|s| s.contents().contains("Reading controls"));
                terminal.keys(b"\x1b");
                terminal.wait(|s| !s.contents().contains("Reading controls"));
            }
        }
        terminal.resize(8, 30);
        terminal.wait(|s| s.contents().contains("Enlarge terminal"));
        terminal.resize(50, 160);
        terminal.keys(b"?");
        terminal.wait(|s| s.contents().contains("Reading controls"));
        terminal.keys(b"\x1b");
        terminal.wait(|s| !s.contents().contains("Reading controls"));
        let (restored_y, body) = active_pane(terminal.parser.screen(), true, old, true);
        let segments = reference_wrapped_segments(line, if old { 73 } else { 74 });
        let selected = segments.partition_point(|(byte, _)| *byte <= anchor) - 1;
        assert_eq!(body.trim_end(), segments[selected].1.trim_end());
        assert_eq!(
            restored_y, y,
            "wide/narrow round trip moved the source vertically"
        );
        terminal.capture(&format!(
            "position-{marker}-{}",
            if old { "old" } else { "new" }
        ));
        terminal.keys(b"j?");
        terminal.wait(|s| s.contents().contains("Reading controls"));
        terminal.keys(b"\x1b");
        terminal.wait(|s| !s.contents().contains("Reading controls"));
        assert_eq!(
            active_pane(terminal.parser.screen(), true, old, true)
                .1
                .trim_end(),
            segments[selected + 1].1.trim_end()
        );
        terminal.resize(30, 119);
        terminal.keys(b"?");
        terminal.wait(|s| s.contents().contains("Reading controls"));
        terminal.keys(b"\x1b");
        terminal.wait(|s| !s.contents().contains("Reading controls"));
        let moved = reference_wrapped_segments(line, 53);
        let expected = &moved
            .iter()
            .rev()
            .find(|(byte, _)| *byte <= segments[selected + 1].0)
            .unwrap()
            .1;
        assert_eq!(
            active_pane(terminal.parser.screen(), true, old, true)
                .1
                .trim_end(),
            expected.trim_end(),
            "resize restored an obsolete anchor after keyboard movement"
        );
        terminal.finish(b"q", 0);
    }
}

#[test]
fn rapid_resizes_keep_the_text_anchor_while_highlighting_is_pending() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("fixtures/position/new/review.ts")).unwrap();
    let line = source
        .lines()
        .find(|line| line.contains("NEW_ANCHOR"))
        .unwrap();
    let mut large = String::new();
    for n in 0..20_000 {
        use std::fmt::Write;
        writeln!(
            large,
            "const row_{n} = {{ text: `review ${{items[{n}]?.name ?? '日本語'}}`, ready: true }};"
        )
        .unwrap();
    }
    large.push_str(line);
    large.push('\n');
    let (_dir, path) = generated_patch_in_language(b"", large.as_bytes(), "ts");
    if let Ok(directory) = std::env::var("NATIVE_DIFF_CAPTURE_DIR") {
        fs::create_dir_all(&directory).unwrap();
        fs::copy(
            &path,
            std::path::Path::new(&directory).join("highlight-resize.patch"),
        )
        .unwrap();
    }
    let mut terminal = Session::open(&path);
    terminal.wait(|s| s.contents().contains("row_0") && !s.contents().contains("Loading patch"));
    terminal.keys(b"Gwjjt");
    terminal.wait(|s| {
        s.contents().contains("NEW_ANCHOR") && s.contents().contains("Highlighting visible")
    });
    // Five-digit gutters leave 72 source cells in the right pane.
    let anchor = reference_wrapped_segments(line, 72)[2].0;
    for width in [119, 80, 160, 100, 120, 81, 159, 80, 160] {
        while let Ok(bytes) = terminal.output.try_recv() {
            terminal.ingest(bytes);
        }
        terminal.resize(30, width);
        terminal.keys(b"t");
        thread::sleep(Duration::from_millis(10));
    }
    terminal.resize(50, 160);
    terminal.keys(b"?");
    terminal.wait(|s| s.contents().contains("Reading controls"));
    terminal.keys(b"\x1b");
    terminal.wait(|s| !s.contents().contains("Reading controls") && s.contents().contains("SPLIT"));
    let screen = terminal.parser.screen();
    let y = (1..49)
        .find(|y| screen.cell(*y, 80).unwrap().contents() == "▎")
        .expect("right-side anchor remains visible");
    let body: String = (88..160)
        .map(|x| screen.cell(y, x).unwrap().contents())
        .collect();
    let segments = reference_wrapped_segments(line, 72);
    assert_eq!(
        body.trim_end(),
        segments
            .iter()
            .rev()
            .find(|(byte, _)| *byte <= anchor)
            .unwrap()
            .1
            .trim_end()
    );
    terminal.keys(b"t");
    terminal.wait(|s| s.contents().contains("Highlighting visible"));
    terminal.finish(b"q", 0);
}

#[test]
fn start_and_end_select_source_lines_and_skip_metadata_and_newline_markers() {
    let source = |prefix: &str| {
        (1..=100)
            .map(|n| {
                format!(
                    "{prefix}_{n:03}{}",
                    "x".repeat(usize::from(n == 1 || n == 100) * 200)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let old = source("OLD");
    let new = source("NEW");
    for (old, new, first, last, first_old, last_old) in [
        (
            old.as_bytes(),
            new.as_bytes(),
            "OLD_001",
            "NEW_100",
            true,
            false,
        ),
        (
            old.as_bytes(),
            b"".as_slice(),
            "OLD_001",
            "OLD_100",
            true,
            true,
        ),
        (
            b"".as_slice(),
            new.as_bytes(),
            "NEW_001",
            "NEW_100",
            false,
            false,
        ),
    ] {
        let (_dir, path) = generated_patch(old, new);
        let patch = fs::read(&path).unwrap();
        let metadata = |name: &str| {
            format!("diff --git a/{name} b/{name}\nold mode 100644\nnew mode 100755\n")
        };
        fs::write(
            &path,
            [
                metadata("before.txt").as_bytes(),
                &patch,
                metadata("after.txt").as_bytes(),
            ]
            .concat(),
        )
        .unwrap();
        for split in [true, false] {
            let mut cmd = CommandBuilder::new(viewer_binary());
            cmd.arg("--no-sidebar");
            cmd.args(["--layout", if split { "split" } else { "unified" }, "patch"]);
            cmd.arg(&path);
            let mut terminal = Session::spawn(cmd);
            terminal.wait(|s| {
                s.contents().contains("3 files") && !s.contents().contains("Loading patch")
            });
            let gutter = |number: u32, old: bool| {
                let sign = if old { '−' } else { '+' };
                if split {
                    format!("{number:>3}{sign} ")
                } else if old {
                    format!("{number:>3}    {sign} ")
                } else {
                    format!("    {number:>3}{sign} ")
                }
            };
            for (keys, number) in [
                (b"g".as_slice(), 1),
                (b"\x1b[B", 2),
                (b"j", 3),
                (b"k", 2),
                (b"\x1b[A", 1),
                (b"\x1b[6~", 47),
                (b"\x1b[5~", 1),
            ] {
                let prefix = if first_old { "OLD" } else { "NEW" };
                let text = format!("{prefix}_{number:03}");
                terminal.keys(keys);
                terminal.wait(|s| has_active_source(s, &gutter(number, first_old), &text));
            }
            for key in [b"G", b"g", b"G"] {
                let (number, text, old) = if key == b"g" {
                    (1, first, first_old)
                } else {
                    (100, last, last_old)
                };
                terminal.keys(key);
                terminal.wait(|s| has_active_source(s, &gutter(number, old), text));
                let screen = terminal.parser.screen();
                let marker_column = if split && !old { 80 } else { 0 };
                assert!((1..49).any(|y| screen.cell(y, marker_column).unwrap().contents() == "▎"));
            }
            terminal.keys(b"j");
            terminal.wait(|s| {
                s.contents().contains("No newline at end of file")
                    && !has_active_source(s, &gutter(100, last_old), last)
            });
            terminal.keys(b"G");
            terminal.wait(|s| has_active_source(s, &gutter(100, last_old), last));
            terminal.keys(b"w");
            terminal.wait(|s| s.contents().contains('↪'));
            terminal.keys(b"j");
            terminal.wait(|s| {
                s.contents()
                    .lines()
                    .any(|line| line.contains('▎') && line.contains('↪'))
            });
            terminal.keys(b"g");
            terminal.wait(|s| has_active_source(s, &gutter(1, first_old), first));
            terminal.keys(b"G");
            terminal.wait(|s| has_active_source(s, &gutter(100, last_old), last));
            terminal.finish(b"q", 0);
        }
    }
}

#[test]
fn pager_file_keys_visit_adjacent_files_without_wrapping() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.patch");
    let mut patch = fs::read(root.join("fixtures/review.patch")).unwrap();
    patch.extend_from_slice(
        b"diff --git a/empty.txt b/empty.txt\nold mode 100644\nnew mode 100755\n",
    );
    fs::write(&path, patch).unwrap();
    let files = [
        "src/retry.rs",
        "src/cache.c",
        "web/Review.tsx",
        "tools/review.py",
        "settings.json",
        "empty.txt",
    ];
    let at_file = |screen: &vt100::Screen, name: &str| {
        screen
            .contents()
            .lines()
            .nth(1)
            .is_some_and(|line| line.trim_start().starts_with(name))
    };
    for layout in ["split", "unified"] {
        let mut cmd = CommandBuilder::new(viewer_binary());
        cmd.arg("--no-sidebar");
        cmd.args(["--layout", layout, "patch"]);
        cmd.arg(&path);
        let mut terminal = Session::spawn(cmd);
        terminal
            .wait(|s| s.contents().contains("6 files") && !s.contents().contains("Loading patch"));
        for name in &files[1..] {
            terminal.keys(b"n");
            terminal.wait(|s| at_file(s, name));
        }
        assert!(
            terminal
                .parser
                .screen()
                .contents()
                .contains("no textual hunks")
        );
        for (boundary, key) in [("empty.txt", b'n'), ("src/retry.rs", b'N')] {
            terminal.settled_grid();
            let before = terminal.parser.screen().contents();
            terminal.keys(&[key, b'?']);
            terminal.wait(|s| s.contents().contains("Reading controls"));
            let help = terminal.parser.screen().contents();
            assert!(help.contains("n/N            Next / previous file"));
            assert!(help.contains("Wheel          Scroll the pane under the pointer"));
            assert!(!help.contains("{ / }"));
            terminal.keys(b"\x1b");
            terminal.wait(|s| !s.contents().contains("Reading controls") && at_file(s, boundary));
            terminal.settled_grid();
            assert_eq!(
                before,
                terminal.parser.screen().contents(),
                "file navigation wrapped at {boundary}"
            );
            if key == b'n' {
                for name in files[..files.len() - 1].iter().rev() {
                    terminal.keys(b"jjN");
                    terminal.wait(|s| at_file(s, name));
                }
            }
        }
        terminal.keys(b"jjN?");
        terminal.wait(|s| s.contents().contains("Reading controls"));
        terminal.keys(b"?");
        terminal.wait(|s| !s.contents().contains("Reading controls"));
        assert!(has_active_source(
            terminal.parser.screen(),
            if layout == "split" {
                "  1  "
            } else {
                "  1   1  "
            },
            "use std::time::Duration;"
        ));
        terminal.finish(b"q", 0);
    }
}

#[test]
fn real_terminal_switches_layout_themes_wraps_and_recovers_from_resize() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/review.patch");
    let mut terminal = Session::open(&path);
    terminal.wait(|s| {
        s.contents().contains("max_attempts") && !s.contents().contains("Highlighting visible")
    });
    let contents = terminal.parser.screen().contents();
    assert!(contents.contains("attempts: usize"));
    assert!(contents.contains("SPLIT"));
    assert!(contents.contains("│"));
    let screen = terminal.parser.screen();
    let (word_y, word_x) = (0..50)
        .flat_map(|y| (0..148).map(move |x| (y, x)))
        .find(|(y, x)| {
            (0..12)
                .map(|n| screen.cell(*y, *x + n).unwrap().contents())
                .collect::<String>()
                == "max_attempts"
        })
        .expect("new identifier is visible");
    assert_ne!(
        screen.cell(word_y, word_x).unwrap().bgcolor(),
        screen.cell(word_y, word_x - 1).unwrap().bgcolor(),
        "changed identifier has no inline emphasis"
    );
    terminal.capture("split-dark");
    let before_scroll = terminal.parser.screen().contents();
    terminal.keys(b"\x1b[<65;100;15M");
    terminal.wait(|s| s.contents() != before_scroll && !s.contents().contains("@@ -1,12"));
    terminal.keys(b"g");
    terminal.wait(|s| has_active_source(s, "  1  ", "use std::time::Duration;"));
    let mut changed_backgrounds = std::collections::HashSet::new();
    for y in 0..50 {
        for x in 0..160 {
            changed_backgrounds.insert(format!(
                "{:?}",
                terminal.parser.screen().cell(y, x).unwrap().bgcolor()
            ));
        }
    }
    assert!(
        changed_backgrounds.len() >= 6,
        "missing layered diff backgrounds: {changed_backgrounds:?}"
    );
    terminal.keys(b"u");
    terminal.wait(|s| {
        s.contents().contains("UNIFIED")
            && s.contents().contains("max_attempts")
            && !s.contents().contains("│")
    });
    assert!(!terminal.parser.screen().contents().contains("│"));
    terminal.capture("unified-dark");
    terminal.keys(b"t");
    terminal.wait(|s| {
        s.contents().contains("light")
            && s.cell(49, 0).unwrap().bgcolor() != vt100::Color::Rgb(23, 23, 23)
    });
    terminal.capture("unified-light");
    terminal.keys(b"G");
    terminal.wait(|s| s.contents().contains("readOnly"));
    terminal.resize(24, 80);
    terminal.keys(b"w");
    terminal.wait(|s| s.contents().contains("readOnly") && s.contents().contains("q quit"));
    terminal.resize(8, 30);
    terminal.wait(|s| s.contents().contains("Enlarge terminal"));
    terminal.resize(30, 100);
    terminal.wait(|s| s.contents().contains("readOnly"));
    terminal.keys(b"g");
    terminal.wait(|s| s.contents().contains("RetryPolicy"));
    terminal.finish(b"q", 0);
}

#[test]
fn real_terminal_sanitizes_source_control_sequences_and_cancels() {
    let (_dir, path) = generated_patch(b"old\n", b"safe\x1b]52;c;EVIL\x07\nlast\r");
    let mut terminal = Session::open(&path);
    terminal.wait(|s| s.contents().contains("EVIL"));
    assert!(terminal.parser.screen().contents().contains("\\u{1b}"));
    assert!(!terminal.raw.windows(5).any(|w| w == b"\x1b]52;"));
    assert!(terminal.parser.screen().contents().contains("last\\u{d}"));
    terminal.keys(b"Gu");
    terminal.wait(|s| s.contents().contains("UNIFIED"));
    assert!(
        terminal
            .parser
            .screen()
            .contents()
            .lines()
            .any(|line| line.contains('▎') && line.contains("last\\u{d}")),
        "layout switch lost the final source anchor beside the no-newline marker"
    );
    terminal.finish(b"\x03", 130);
}

#[test]
fn stdin_and_signals_use_the_controlling_terminal_and_restore_it() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/review.patch");
    let mut command = CommandBuilder::new("/bin/sh");
    command.args([
        "-c",
        "cat \"$1\" | \"$2\" --no-sidebar patch -",
        "native-diff-test",
    ]);
    command.arg(&path);
    command.arg(viewer_binary());
    let mut terminal = Session::spawn(command);
    terminal.wait(|s| s.contents().contains("RetryPolicy"));
    terminal.keys(b"u");
    terminal.wait(|s| s.contents().contains("UNIFIED"));
    terminal.finish(b"q", 0);

    for (signal, code) in [("-TERM", 143), ("-HUP", 129)] {
        let mut terminal = Session::open(&path);
        terminal.wait(|s| s.contents().contains("RetryPolicy"));
        assert!(
            Command::new("/bin/kill")
                .arg(signal)
                .arg(terminal.child.process_id().unwrap().to_string())
                .status()
                .unwrap()
                .success()
        );
        terminal.finish(b"", code);
    }
}

#[test]
fn long_lines_do_not_poison_later_syntax_or_leave_false_wrapped_gutters() {
    let new = format!("{}\nfn tail() {{}}\n", "x".repeat(10_001));
    let (_dir, path) = generated_patch(b"old\n", new.as_bytes());
    let mut terminal = Session::open(&path);
    terminal.wait(|s| {
        s.contents().contains("fn tail()") && s.contents().contains("Plain styling fallback")
    });
    let screen = terminal.parser.screen();
    let y = (0..50)
        .find(|y| screen.cell(*y, 86).unwrap().contents() == "f")
        .expect("tail function is visible in the new pane");
    assert_eq!(
        screen.cell(y, 86).unwrap().fgcolor(),
        screen.cell(y, 89).unwrap().fgcolor(),
        "syntax state stayed valid after a skipped source line"
    );
    terminal.keys(b"w");
    terminal.wait(|s| s.contents().contains('↪'));
    for y in 4..49 {
        for x in 0..79 {
            assert!(
                terminal
                    .parser
                    .screen()
                    .cell(y, x)
                    .unwrap()
                    .contents()
                    .trim()
                    .is_empty(),
                "empty old-side continuation contains source or gutter text"
            );
        }
    }
    terminal.keys(b"G");
    terminal.wait(|s| s.contents().contains("fn tail()"));
    terminal.finish(b"q", 0);
}

#[test]
fn loading_from_a_blocked_producer_is_cancellable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blocked.patch");
    assert!(
        Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    let mut terminal = Session::open(&path);
    terminal.wait(|s| s.contents().contains("Loading patch"));
    terminal.finish(b"q", 0);
}

#[test]
fn canonical_stream_publishes_complete_files_and_rejects_a_later_truncation() {
    let first = b"diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n-old\n+first\n";
    let next_header = b"diff --git a/b.rs b/b.rs\n";
    let remainder = b"--- a/b.rs\n+++ b/b.rs\n@@ -1 +1 @@\n-old\n+second\n";
    for malformed in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let fifo = dir.path().join("input.patch");
        assert!(
            Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        let first_path = dir.path().join("first.patch");
        fs::write(&first_path, first).unwrap();
        let expected = Command::new(viewer_binary())
            .arg("inspect")
            .arg(&first_path)
            .arg("--canonical")
            .output()
            .unwrap();
        assert!(expected.status.success());
        let mut child = Command::new(viewer_binary())
            .arg("inspect")
            .arg(&fifo)
            .arg("--canonical")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let (continue_send, continue_receive) = mpsc::channel();
        let producer = thread::spawn(move || {
            let mut input = fs::OpenOptions::new().write(true).open(fifo).unwrap();
            input.write_all(first).unwrap();
            input.write_all(next_header).unwrap();
            if continue_receive
                .recv_timeout(Duration::from_secs(5))
                .is_ok()
            {
                let bytes = if malformed {
                    &remainder[..remainder.len() - 8]
                } else {
                    remainder.as_slice()
                };
                input.write_all(bytes).unwrap();
            }
        });
        let mut output = child.stdout.take().unwrap();
        let (send, receive) = mpsc::channel();
        let expected_length = expected.stdout.len();
        let consumer = thread::spawn(move || {
            let mut prefix = vec![0; expected_length];
            let result = output.read_exact(&mut prefix);
            send.send((result, prefix)).unwrap();
            let mut tail = Vec::new();
            output.read_to_end(&mut tail).unwrap();
            tail
        });
        let prefix = receive.recv_timeout(Duration::from_secs(3));
        if prefix.is_err() {
            let _ = child.kill();
        }
        let (result, prefix) = prefix.expect("complete file was withheld until producer EOF");
        result.unwrap();
        assert_eq!(prefix, expected.stdout);
        assert!(
            child.try_wait().unwrap().is_none(),
            "producer is still blocked"
        );
        continue_send.send(()).unwrap();
        producer.join().unwrap();
        let output = child.wait_with_output().unwrap();
        let tail = consumer.join().unwrap();
        assert_eq!(output.status.code(), Some(if malformed { 1 } else { 0 }));
        if malformed {
            assert!(tail.is_empty(), "incomplete file leaked source records");
        } else {
            assert!(tail.windows(6).any(|bytes| bytes == b"second"));
        }
    }
}

#[test]
fn large_patch_loads_and_navigates_to_exact_end() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.patch");
    let lines = 40_000_usize;
    let padding = "x".repeat(180);
    {
        let mut output = std::io::BufWriter::new(fs::File::create(&path).unwrap());
        writeln!(output,"diff --git a/large.rs b/large.rs\n--- a/large.rs\n+++ b/large.rs\n@@ -1,{lines} +1,{lines} @@").unwrap();
        for sign in ['-', '+'] {
            for line in 1..=lines {
                writeln!(
                    output,
                    "{sign}const LINE_{line:05}: &str = \"{sign}{padding}\";"
                )
                .unwrap();
            }
        }
    }
    let output = Command::new(viewer_binary())
        .arg("inspect")
        .arg(&path)
        .arg("--summary")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["source_lines"], lines * 2);
    assert_eq!(summary["additions"], lines);
    assert_eq!(summary["deletions"], lines);
    assert_eq!(
        summary["storage"]["source_index_bytes"],
        (lines * 2).div_ceil(8)
    );
    assert_eq!(
        summary["storage"]["source_index_stored_bytes"],
        lines * 2 * 8
    );
    assert_eq!(
        summary["storage"]["spool_bytes"],
        fs::metadata(&path).unwrap().len()
    );
    let mut terminal = Session::open(&path);
    terminal.wait(|s| s.contents().contains("LINE_00001"));
    terminal.keys(b"G");
    terminal.wait(|s| s.contents().contains("LINE_40000"));
    terminal.keys(b"u");
    terminal.wait(|s| {
        s.contents().contains("UNIFIED")
            && s.contents().contains("LINE_40000")
            && !s.contents().contains("│")
    });
    terminal.keys(b"g");
    terminal.wait(|s| s.contents().contains("LINE_00001"));
    terminal.finish(b"q", 0);
}

#[test]
fn progressive_terminal_keeps_complete_files_readable_and_rejects_partial_files() {
    let mut first =
        b"diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -1,80 +1,80 @@\n".to_vec();
    for sign in ['-', '+'] {
        for n in 1..=80 {
            writeln!(first, "{sign}FIRST_{n:04}").unwrap();
        }
    }
    let second = b"--- a/b.txt\n+++ b/b.txt\n@@ -1 +1 @@\n-old\n+SECOND_COMPLETE\n";
    for outcome in ["complete", "malformed", "cancel"] {
        let dir = tempfile::tempdir().unwrap();
        let fifo = dir.path().join("input.patch");
        assert!(
            Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        let (resume, blocked) = mpsc::channel();
        let prefix = first.clone();
        let writer_path = fifo.clone();
        let producer = thread::spawn(move || {
            let mut writer = fs::OpenOptions::new()
                .write(true)
                .open(writer_path)
                .unwrap();
            writer.write_all(&prefix).unwrap();
            writer.write_all(b"diff --git a/b.txt b/b.txt\n").unwrap();
            if blocked.recv_timeout(Duration::from_secs(20)).is_ok() && outcome != "cancel" {
                let bytes = if outcome == "malformed" {
                    &second[..second.len() - 5]
                } else {
                    second.as_slice()
                };
                writer.write_all(bytes).unwrap();
            }
        });
        let mut terminal = Session::open(&fifo);
        terminal.wait(|s| {
            s.contents().contains("FIRST_0001") && !s.contents().contains("Highlighting visible")
        });
        assert!(
            terminal
                .parser
                .screen()
                .contents()
                .contains("Loading patch")
        );
        assert!(!terminal.parser.screen().contents().contains("b.txt"));
        terminal.keys(b"G");
        terminal.wait(|s| {
            has_active_source(s, " 80+ ", "FIRST_0080")
                && !s.contents().contains("Highlighting visible")
        });
        terminal.keys(b"n?");
        terminal.wait(|s| s.contents().contains("Reading controls"));
        terminal.keys(b"\x1b");
        terminal.wait(|s| !s.contents().contains("Reading controls"));
        assert!(has_active_source(
            terminal.parser.screen(),
            " 80+ ",
            "FIRST_0080"
        ));
        let before: Vec<_> = terminal
            .parser
            .screen()
            .contents()
            .lines()
            .skip(1)
            .take(48)
            .map(str::to_owned)
            .collect();
        if outcome == "cancel" {
            terminal.finish(b"q", 0);
            resume.send(()).unwrap();
        } else {
            resume.send(()).unwrap();
            if outcome == "malformed" {
                terminal.wait(|s| s.contents().contains("INCOMPLETE"));
                assert!(!terminal.parser.screen().contents().contains("SECOND"));
                assert!(terminal.parser.screen().contents().contains("FIRST_0080"));
                terminal.keys(b"g");
                terminal.wait(|s| has_active_source(s, "  1− ", "FIRST_0001"));
                terminal.keys(b"uG");
                terminal.wait(|s| {
                    s.contents().contains("UNIFIED") && s.contents().contains("FIRST_0080")
                });
                assert!(terminal.parser.screen().contents().contains("INCOMPLETE"));
                terminal.finish(b"q", 1);
            } else {
                terminal.wait(|s| {
                    s.contents().contains("2 files") && !s.contents().contains("Loading patch")
                });
                let after: Vec<_> = terminal
                    .parser
                    .screen()
                    .contents()
                    .lines()
                    .skip(1)
                    .take(48)
                    .map(str::to_owned)
                    .collect();
                assert_eq!(before, after, "appending a file moved the reading position");
                terminal.keys(b"n");
                terminal.wait(|s| {
                    s.contents()
                        .lines()
                        .nth(1)
                        .is_some_and(|line| line.trim_start().starts_with("b.txt"))
                });
                terminal.keys(b"jjN");
                terminal.wait(|s| {
                    s.contents()
                        .lines()
                        .nth(1)
                        .is_some_and(|line| line.trim_start().starts_with("a.txt"))
                });
                terminal.keys(b"G");
                terminal.wait(|s| {
                    has_active_source(s, "  1+ ", "SECOND_COMPLETE")
                        && !s.contents().contains("Highlighting visible")
                });
                terminal.keys(b"u");
                terminal.wait(|s| {
                    s.contents().contains("UNIFIED") && s.contents().contains("SECOND_COMPLETE")
                });
                terminal.finish(b"q", 0);
            }
        }
        producer.join().unwrap();
        assert_eq!(
            fs::read_dir(dir.path()).unwrap().count(),
            1,
            "spool leaked into the input directory"
        );
    }
}

#[test]
fn source_cache_revisits_keep_exact_lines_after_reading_beyond_capacity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache-revisit.patch");
    let context_lines = 4608;
    let last_number = context_lines + 2;
    let mut patch = std::io::BufWriter::new(fs::File::create(&path).unwrap());
    writeln!(
        patch,
        "diff --git a/read.txt b/read.txt\n--- a/read.txt\n+++ b/read.txt"
    )
    .unwrap();
    writeln!(
        patch,
        "@@ -1,{last_number} +1,{last_number} @@\n-OLD_FIRST\n+NEW_FIRST"
    )
    .unwrap();
    let suffix = "abcdefgh".repeat(1024);
    for n in 0..context_lines {
        writeln!(patch, " CONTEXT_{n:04}_{suffix}").unwrap();
    }
    writeln!(patch, "-OLD_LAST\n+NEW_LAST").unwrap();
    patch.flush().unwrap();
    drop(patch);
    assert!(fs::metadata(&path).unwrap().len() > 32 * 1024 * 1024);

    let mut terminal = Session::open(&path);
    terminal.settled_grid();
    assert!(terminal.parser.screen().contents().contains("NEW_FIRST"));
    terminal.keys(b"G");
    terminal.wait(|s| has_active_source(s, &format!("{last_number}+ "), "NEW_LAST"));
    terminal.settled_grid();
    fs::write(
        &path,
        b"The loaded source must remain in its private spool.\n",
    )
    .unwrap();
    for (key, expected) in [
        (b"g".as_slice(), "NEW_FIRST"),
        (b"t", "NEW_FIRST"),
        (b"u", "NEW_FIRST"),
        (b"G", "NEW_LAST"),
        (b"u", "NEW_LAST"),
        (b"g", "NEW_FIRST"),
    ] {
        terminal.keys(key);
        if key == b"t" {
            terminal.wait(|s| s.contents().contains("light"));
        }
        terminal.settled_grid();
        assert!(terminal.parser.screen().contents().contains(expected));
    }
    terminal.finish(b"q", 0);
}

#[test]
fn completed_patch_uses_its_private_spool_after_the_input_is_replaced() {
    let old = (1..=500)
        .map(|n| format!("old_{n:04}\n"))
        .collect::<String>();
    let new = (1..=500)
        .map(|n| format!("new_{n:04}\n"))
        .collect::<String>();
    let (_dir, path) = generated_patch(old.as_bytes(), new.as_bytes());
    let mut terminal = Session::open(&path);
    terminal.settled_grid();
    fs::write(&path, b"This is no longer the loaded input.\n").unwrap();
    terminal.keys(b"G");
    terminal.wait(|s| s.contents().contains("new_0500"));
    terminal.keys(b"u");
    terminal.wait(|s| s.contents().contains("UNIFIED") && s.contents().contains("new_0500"));
    terminal.keys(b"g");
    terminal.wait(|s| s.contents().contains("old_0001"));
    terminal.finish(b"q", 0);
}

#[test]
#[ignore = "Requires the preserved pre-storage executable in evidence/large-diff-baseline"]
fn equivalent_reading_actions_match_the_preserved_pre_storage_viewer() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let reference = root.join("evidence/large-diff-baseline/native-diff-pre-storage");
    let hash = Command::new("shasum")
        .args(["-a", "256"])
        .arg(&reference)
        .output()
        .unwrap();
    assert!(hash.status.success());
    assert!(
        String::from_utf8(hash.stdout)
            .unwrap()
            .starts_with("b9a03928761c52abe63792d65053340c8061784d3cc7e7a0cdb55f76405987cb ")
    );
    let path = root.join("fixtures/review.patch");
    let trace = |binary: &std::path::Path, next: &[u8], previous: &[u8]| {
        let mut terminal = Session::open_with(binary, &path);
        let mut grids = vec![terminal.settled_grid()];
        for keys in [
            b"\x1b[<65;100;15M".as_slice(),
            b"\x1b[<64;100;15M",
            b"u",
            b"t",
            next,
            b"u",
        ] {
            terminal.keys(keys);
            grids.push(terminal.settled_grid());
        }
        terminal.resize(24, 80);
        grids.push(terminal.settled_grid());
        for keys in [b"w".as_slice(), previous, next, b"l", b"t"] {
            terminal.keys(keys);
            grids.push(terminal.settled_grid());
        }
        terminal.resize(50, 160);
        grids.push(terminal.settled_grid());
        terminal.finish(b"q", 0);
        grids
    };
    let expected = trace(&reference, b"}", b"{");
    let actual = trace(&viewer_binary(), b"n", b"N");
    for (step, (expected, actual)) in expected.iter().zip(&actual).enumerate() {
        assert_eq!(
            expected, actual,
            "complete cell/style grid changed at trace step {step}"
        );
    }
}

#[test]
#[ignore = "Requires the five pinned DiffsHub bodies under evidence/diffshub-corpus"]
fn diffshub_corpus_opens_and_keeps_exact_end_anchors_in_both_layouts() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let corpus: Value =
        serde_json::from_slice(&fs::read(root.join("fixtures/diffshub/corpus.json")).unwrap())
            .unwrap();
    let reference: Value =
        serde_json::from_slice(&fs::read(root.join("fixtures/diffshub/reference.json")).unwrap())
            .unwrap();
    let decode = |value: &Value| {
        let hex = value.as_str().unwrap();
        String::from_utf8(
            (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect(),
        )
        .unwrap()
    };
    for fixture in corpus["fixtures"].as_array().unwrap() {
        let expected = reference["fixtures"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == fixture["id"])
            .unwrap();
        let path = root.join(fixture["local_path"].as_str().unwrap());
        let hash = Command::new("shasum")
            .args(["-a", "256"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(hash.status.success());
        assert_eq!(
            String::from_utf8(hash.stdout)
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap(),
            fixture["sha256"].as_str().unwrap()
        );
        let last = decode(&expected["last"]["prefix_hex"]);
        let last = last.trim();
        assert!(!last.is_empty());
        let number = expected["last"]["new"]
            .as_u64()
            .or(expected["last"]["old"].as_u64())
            .unwrap()
            .to_string();
        let header = decode(
            &expected["quantile_headers"]
                .as_array()
                .unwrap()
                .last()
                .unwrap()["header_hex"],
        );
        let file = header.rsplit_once(" b/").unwrap().1.trim();
        let at_end = |s: &vt100::Screen| {
            let text = s.contents();
            text.contains(file)
                && text
                    .lines()
                    .any(|line| line.contains('▎') && line.contains(last) && line.contains(&number))
        };
        let first = decode(&expected["first"]["prefix_hex"]);
        let first = first.trim_end();
        let first_number = expected["first"]["new"]
            .as_u64()
            .or(expected["first"]["old"].as_u64())
            .unwrap()
            .to_string();
        let mut terminal = Session::open(&path);
        terminal.wait(|s| {
            s.contents().contains(&format!(
                "{} files",
                expected["file_count"].as_u64().unwrap()
            )) && !s.contents().contains("Loading patch")
        });
        terminal.keys(b"G");
        terminal.wait(at_end);
        terminal.keys(b"u");
        terminal.wait(|s| s.contents().contains("UNIFIED") && at_end(s));
        terminal.resize(24, 80);
        terminal.wait(at_end);
        terminal.resize(50, 160);
        terminal.keys(b"ug");
        terminal.wait(|s| {
            s.contents().contains("SPLIT")
                && s.contents().lines().any(|line| {
                    line.split_once('▎').is_some_and(|(_, active)| {
                        active.trim_start().starts_with(&first_number) && active.contains(first)
                    })
                })
                && !s.contents().contains(file)
        });
        terminal.finish(b"q", 0);
        eprintln!(
            "{}: complete input, exact end anchors, layout, resize, return, exit passed",
            fixture["id"]
        );
    }
}
