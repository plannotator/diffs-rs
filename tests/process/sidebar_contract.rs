use super::*;
use std::path::{Path, PathBuf};

fn open(path: &Path, args: &[&str]) -> Session {
    let binary = viewer_binary();
    let mut command = CommandBuilder::new(binary);
    command.args(args);
    command.arg("patch");
    command.arg(Path::new(env!("CARGO_MANIFEST_DIR")).join(path));
    Session::spawn(command)
}

fn cells(screen: &vt100::Screen, y: u16, x: u16, width: u16) -> String {
    (x..x + width)
        .map(|x| screen.cell(y, x).unwrap().contents())
        .collect()
}

fn tree_text(screen: &vt100::Screen) -> String {
    (1..screen.size().0 - 1)
        .map(|y| cells(screen, y, 0, 32))
        .collect::<Vec<_>>()
        .join("\n")
}

fn input(terminal: &mut Session, keys: &[u8]) {
    terminal.keys(keys);
    terminal.settled_grid();
}

fn click_tree(terminal: &mut Session, name: &str) {
    let y = (2..terminal.parser.screen().size().0 - 1)
        .find(|y| cells(terminal.parser.screen(), *y, 0, 32).contains(name))
        .unwrap();
    input(terminal, format!("\x1b[<0;8;{}M", y + 1).as_bytes());
}

fn git(root: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_COUNT", "0")
        .args(["-c", "core.autocrlf=false", "-c", "core.filemode=true"])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn repository_patch() -> (tempfile::TempDir, PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "--quiet"]);
    for name in ["src/nested", "assets", "old", "dest"] {
        fs::create_dir_all(root.join(name)).unwrap();
    }
    for (path, body) in [
        ("src/nested/value.rs", "fn value() -> u8 { 1 }\n"),
        ("old/original.rs", "fn renamed() {}\n"),
        ("removed.rs", "fn removed() {}\n"),
        ("script.sh", "#!/bin/sh\necho unchanged\n"),
        ("assets/icon.bin", "old\0binary"),
    ] {
        fs::write(root.join(path), body).unwrap();
    }
    git(root, &["add", "--all"]);
    let tree = String::from_utf8(git(root, &["write-tree"])).unwrap();
    fs::rename(root.join("old/original.rs"), root.join("dest/renamed.rs")).unwrap();
    fs::remove_file(root.join("removed.rs")).unwrap();
    fs::write(
        root.join("src/nested/value.rs"),
        "fn value() -> u8 {\n    2\n}\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "# Added documentation\n").unwrap();
    fs::write(root.join("assets/icon.bin"), b"new\0binary").unwrap();
    fs::set_permissions(root.join("script.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    git(root, &["add", "--all"]);
    let bytes = git(
        root,
        &[
            "diff",
            "--cached",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--binary",
            "--find-renames",
            tree.trim(),
        ],
    );
    let patch = root.join("input.patch");
    fs::write(&patch, bytes).unwrap();
    (dir, patch)
}

#[test]
fn sidebar_real_git_hierarchy_counts_status_activation_and_focus() {
    let (_dir, path) = repository_patch();
    let mut terminal = open(&path, &[]);
    terminal.settled_grid();
    let tree = tree_text(terminal.parser.screen());
    for expected in [
        "Files · 6",
        "assets/",
        "icon.bin",
        "B binary",
        "dest/",
        "renamed.rs",
        "R +0 −0",
        "src/",
        "nested/",
        "value.rs",
        "M +3 −1",
        "README.md",
        "A +1 −0",
        "removed.rs",
        "D +0 −1",
        "script.sh",
        "M +0 −0",
    ] {
        assert!(tree.contains(expected), "missing {expected}:\n{tree}");
    }
    assert!(tree.find("assets/").unwrap() < tree.find("dest/").unwrap());
    assert!(tree.find("src/").unwrap() < tree.find("README.md").unwrap());
    let before = cells(terminal.parser.screen(), 1, 33, 127);
    input(&mut terminal, b"f\x1b[H");
    assert_eq!(cells(terminal.parser.screen(), 1, 33, 127), before);
    input(&mut terminal, b"\x1b[D");
    assert!(!tree_text(terminal.parser.screen()).contains("icon.bin"));
    input(&mut terminal, b"\x1b[C");
    assert!(tree_text(terminal.parser.screen()).contains("icon.bin"));
    click_tree(&mut terminal, "renamed.rs");
    assert!(
        cells(terminal.parser.screen(), 1, 33, 127).contains("old/original.rs → dest/renamed.rs")
    );
    click_tree(&mut terminal, "script.sh");
    assert!(cells(terminal.parser.screen(), 1, 33, 127).contains("script.sh"));
    assert!(
        terminal
            .parser
            .screen()
            .contents()
            .contains("mode 100644 → 100755")
    );
    click_tree(&mut terminal, "value.rs");
    assert!(cells(terminal.parser.screen(), 1, 33, 127).contains("src/nested/value.rs"));
    terminal.capture("sidebar-dark");
    input(&mut terminal, b"t");
    terminal.capture("sidebar-light");
    click_tree(&mut terminal, "src/");
    input(&mut terminal, b"\tt");
    assert!(!tree_text(terminal.parser.screen()).contains("nested/"));
    input(&mut terminal, b"f?");
    assert!(
        terminal
            .parser
            .screen()
            .contents()
            .contains("Current diff file")
    );
    input(&mut terminal, b"\x1b");
    assert!(tree_text(terminal.parser.screen()).contains("[focus]"));
    input(&mut terminal, b"\t");
    assert!(!tree_text(terminal.parser.screen()).contains("[focus]"));
    input(&mut terminal, b"ff");
    assert!(!cells(terminal.parser.screen(), 1, 0, 32).contains("Files"));
    assert!(cells(terminal.parser.screen(), 1, 0, 160).contains("src/nested/value.rs"));
    terminal.finish(b"q", 0);
}

#[test]
fn sidebar_mouse_translation_wrap_and_resize_preserve_exact_text_anchor() {
    let line = format!(
        "{}日e\u{301}\tZ{}日本\tEND{}",
        "a".repeat(255),
        "b".repeat(252),
        "c".repeat(80)
    );
    let (_dir, path) = generated_patch_in_language(
        format!("{line}\n").as_bytes(),
        format!("{line}\nfinished\n").as_bytes(),
        "txt",
    );
    let mut terminal = open(&path, &["--layout", "split", "--sidebar"]);
    terminal.settled_grid();
    input(&mut terminal, &b"\x1b[C".repeat(130));
    // The right pane starts at 97; its code starts at 103. END is at source column 520.
    input(&mut terminal, b"\x1b[<0;104;4M");
    assert_eq!(
        terminal.parser.screen().cell(3, 97).unwrap().contents(),
        "▎"
    );
    input(&mut terminal, b"w");
    let anchor = line.find("END").unwrap();
    for (width, shown, keys, split) in [
        (120, true, "", true),
        (96, false, "", true),
        (160, true, "", true),
        (160, false, "ff", true),
        (160, true, "f\t", true),
        (120, true, "u", false),
        (98, true, "", false),
    ] {
        terminal.resize(30, width);
        input(&mut terminal, keys.as_bytes());
        let inset = if shown { 33 } else { 0 };
        let diff_width = width - inset;
        let start = if split {
            inset + (diff_width - 1) / 2 + 1
        } else {
            inset
        };
        let gutter = if split { 6 } else { 10 };
        let available = width - start - gutter;
        let screen = terminal.parser.screen();
        let y = (2..29)
            .find(|y| screen.cell(*y, start).unwrap().contents() == "▎")
            .unwrap();
        let body = cells(screen, y, start + gutter, available);
        let parts = reference_wrapped_segments(&line, available as usize);
        let expected = parts
            .iter()
            .rev()
            .find(|(byte, _)| *byte <= anchor)
            .unwrap();
        assert_eq!(
            body.trim_end(),
            expected.1.trim_end(),
            "width {width}, sidebar {shown}, split {split}"
        );
    }
    terminal.resize(8, 30);
    terminal.wait(|s| s.contents().contains("Enlarge terminal"));
    terminal.keys(b"\x1b[<0;10;4M");
    terminal.resize(30, 160);
    terminal.settled_grid();
    assert!(terminal.parser.screen().contents().contains("END"));
    for width in [97, 140, 80, 180, 96, 120, 100, 160] {
        terminal.resize(30, width);
    }
    terminal.settled_grid();
    let screen = terminal.parser.screen();
    let y = (2..29)
        .find(|y| screen.cell(*y, 33).unwrap().contents() == "▎")
        .unwrap();
    let parts = reference_wrapped_segments(&line, 117);
    let expected = parts
        .iter()
        .rev()
        .find(|(byte, _)| *byte <= anchor)
        .unwrap();
    assert_eq!(cells(screen, y, 43, 117).trim_end(), expected.1.trim_end());
    terminal.finish(b"q", 0);
}

fn entry(path: &str, content: &str) -> String {
    format!(
        "diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n@@ -1 +1 @@\n-old\n+{content}\n"
    )
}

#[test]
fn sidebar_many_files_scroll_independently_and_activate_patch_identity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("many.patch");
    let patch = (0..300)
        .rev()
        .map(|n| entry(&format!("src/file_{n:04}.txt"), &format!("SOURCE_{n:04}")))
        .collect::<String>();
    fs::write(&path, patch).unwrap();
    let mut terminal = open(&path, &[]);
    terminal.settled_grid();
    let before = cells(terminal.parser.screen(), 1, 33, 127);
    input(&mut terminal, b"f\x1b[H\x1b[<65;8;8M");
    assert_eq!(cells(terminal.parser.screen(), 1, 33, 127), before);
    assert!(!tree_text(terminal.parser.screen()).contains("file_0000.txt"));
    input(&mut terminal, b"\x1b[F\r");
    assert!(cells(terminal.parser.screen(), 1, 33, 127).contains("file_0299.txt"));
    input(&mut terminal, b"n");
    assert!(cells(terminal.parser.screen(), 1, 33, 127).contains("file_0298.txt"));
    input(&mut terminal, b"f\x1b[H\x1b[B\r");
    assert!(cells(terminal.parser.screen(), 1, 33, 127).contains("file_0000.txt"));
    input(&mut terminal, b"N");
    assert!(cells(terminal.parser.screen(), 1, 33, 127).contains("file_0001.txt"));
    terminal.finish(b"q", 0);
}

#[test]
fn sidebar_streaming_insertions_preserve_selection_and_partial_failures() {
    let dir = tempfile::tempdir().unwrap();
    let fifo = dir.path().join("stream.patch");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let writer_path = fifo.clone();
    let (release, paused) = mpsc::channel::<()>();
    let producer = thread::spawn(move || {
        let mut writer = fs::OpenOptions::new()
            .write(true)
            .open(writer_path)
            .unwrap();
        writer
            .write_all(entry("z.rs", "FIRST_SOURCE").as_bytes())
            .unwrap();
        writer.write_all(b"diff --git a/a.rs b/a.rs\n").unwrap();
        paused.recv().unwrap();
        writer.write_all(b"--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n-old\n+SECOND_SOURCE\ndiff --git a/bad.rs b/bad.rs\n").unwrap();
        paused.recv().unwrap();
        writer
            .write_all(b"--- a/bad.rs\n+++ b/bad.rs\n@@ -1 +1 @@\n+incomplete\n")
            .unwrap();
    });
    let mut terminal = open(&fifo, &["--sidebar"]);
    terminal.wait(|s| s.contents().contains("FIRST_SOURCE") && tree_text(s).contains("1 partial"));
    terminal.keys(b"f");
    terminal.wait(|s| tree_text(s).contains("[focus]"));
    release.send(()).unwrap();
    terminal.wait(|s| tree_text(s).contains("2 partial"));
    assert!(
        tree_text(terminal.parser.screen())
            .lines()
            .any(|l| l.starts_with('>') && l.contains("z.rs"))
    );
    terminal.keys(b"\r");
    terminal.wait(|s| !tree_text(s).contains("[focus]"));
    assert!(cells(terminal.parser.screen(), 1, 33, 127).contains("z.rs"));
    release.send(()).unwrap();
    producer.join().unwrap();
    terminal.wait(|s| s.contents().contains("INCOMPLETE"));
    assert!(!tree_text(terminal.parser.screen()).contains("bad.rs"));
    click_tree(&mut terminal, "a.rs");
    assert!(cells(terminal.parser.screen(), 1, 33, 127).contains("a.rs"));
    assert!(terminal.parser.screen().contents().contains("INCOMPLETE"));
    terminal.finish(b"q", 1);
}

#[test]
fn sidebar_flags_empty_input_and_narrow_window_recover() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.patch");
    fs::write(&path, "").unwrap();
    let mut terminal = open(&path, &["--sidebar"]);
    terminal.settled_grid();
    assert!(
        terminal
            .parser
            .screen()
            .contents()
            .contains("No completed files")
    );
    terminal.resize(24, 80);
    terminal.wait(|s| s.contents().contains("UNIFIED"));
    input(&mut terminal, b"f");
    assert!(
        terminal
            .parser
            .screen()
            .contents()
            .contains("Files need at least 97 columns")
    );
    terminal.resize(24, 97);
    terminal.settled_grid();
    assert!(tree_text(terminal.parser.screen()).contains("Files · 0"));
    terminal.resize(10, 97);
    terminal.wait(|s| s.size() == (10, 97) && s.contents().contains("q quit"));
    terminal.keys(b"f?");
    terminal.wait(|s| s.contents().contains("Collapse / expand folders"));
    assert!(
        terminal
            .parser
            .screen()
            .contents()
            .contains("Collapse / expand folders")
    );
    assert!(
        terminal
            .parser
            .screen()
            .contents()
            .contains("Quit / cancel")
    );
    terminal.finish(b"q", 0);
    let output = Command::new(viewer_binary())
        .args(["--sidebar", "--no-sidebar", "patch"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn sidebar_duplicate_names_keep_visible_occurrences_and_distinct_destinations() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("duplicates.patch");
    fs::write(
        &path,
        [
            entry("a_very_long_duplicate_filename.rs", "FIRST_COPY"),
            entry("a_very_long_duplicate_filename.rs", "SECOND_COPY"),
        ]
        .concat(),
    )
    .unwrap();
    let mut terminal = open(&path, &[]);
    terminal.settled_grid();
    let tree = tree_text(terminal.parser.screen());
    assert!(tree.contains("… [1]"), "{tree}");
    assert!(tree.contains("… [2]"), "{tree}");
    input(&mut terminal, b"f\x1b[F\r");
    assert!(cells(terminal.parser.screen(), 3, 33, 127).contains("SECOND_COPY"));
    input(&mut terminal, b"f\x1b[H\r");
    assert!(cells(terminal.parser.screen(), 3, 33, 127).contains("FIRST_COPY"));
    terminal.finish(b"q", 0);
}

#[test]
fn sidebar_default_multifile_views_preserve_source_and_auto_layout_threshold() {
    let mut terminal = open(Path::new("fixtures/review.patch"), &[]);
    terminal.settled_grid();
    assert!(tree_text(terminal.parser.screen()).contains("Files · 5"));
    assert!(terminal.parser.screen().contents().contains("max_attempts"));
    terminal.capture("sidebar-review-dark");
    input(&mut terminal, b"t");
    terminal.capture("sidebar-review-light");
    terminal.resize(30, 152);
    terminal.wait(|s| cells(s, 0, 0, 152).contains("UNIFIED"));
    terminal.resize(30, 153);
    terminal.wait(|s| cells(s, 0, 0, 153).contains("SPLIT"));
    terminal.finish(b"q", 0);
    let mut single = open(Path::new("fixtures/position/review.patch"), &[]);
    single.settled_grid();
    assert!(!cells(single.parser.screen(), 1, 0, 32).contains("Files"));
    single.finish(b"q", 0);
}

#[test]
fn sidebar_resize_roundtrip_repaints_when_resize_signals_coalesce() {
    let mut terminal = open(Path::new("fixtures/review.patch"), &[]);
    terminal.settled_grid();
    let expected = terminal.parser.screen().clone();
    let pid = terminal.child.process_id().unwrap().to_string();
    assert!(
        Command::new("/bin/kill")
            .args(["-STOP", &pid])
            .status()
            .unwrap()
            .success()
    );
    let stopped_by = Instant::now() + Duration::from_secs(2);
    loop {
        let state = Command::new("ps")
            .args(["-o", "stat=", "-p", &pid])
            .output()
            .unwrap();
        assert!(state.status.success());
        if String::from_utf8_lossy(&state.stdout)
            .trim()
            .starts_with('T')
        {
            break;
        }
        assert!(
            Instant::now() < stopped_by,
            "viewer did not stop before resize"
        );
        thread::sleep(Duration::from_millis(10));
    }
    terminal.resize(50, 80);
    terminal.resize(50, 160);
    assert!(
        Command::new("/bin/kill")
            .args(["-CONT", &pid])
            .status()
            .unwrap()
            .success()
    );
    terminal.wait(|s| (0..50).all(|y| (0..160).all(|x| s.cell(y, x) == expected.cell(y, x))));
    terminal.finish(b"q", 0);
}

#[test]
#[ignore = "requires NATIVE_DIFF_BASELINE_BINARY pointing to the preserved pre-sidebar executable"]
fn sidebar_hidden_body_and_canonical_source_match_preserved_executable() {
    let baseline = PathBuf::from(
        std::env::var_os("NATIVE_DIFF_BASELINE_BINARY").expect("explicit baseline binary"),
    );
    let current = viewer_binary();
    let (_dir, generated) = repository_patch();
    for path in [
        PathBuf::from("fixtures/review.patch"),
        PathBuf::from("fixtures/position/review.patch"),
        generated,
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
        let mut before = CommandBuilder::new(&baseline);
        before.arg("patch");
        before.arg(&path);
        let mut before = Session::spawn(before);
        let mut after = open(&path, &["--no-sidebar"]);
        for keys in ["", "n", "G", "w", "u", "t", "g"] {
            input(&mut before, keys.as_bytes());
            input(&mut after, keys.as_bytes());
            for y in 1..49 {
                for x in 0..160 {
                    assert_eq!(
                        before.parser.screen().cell(y, x),
                        after.parser.screen().cell(y, x),
                        "{} after {keys:?}, cell {x},{y}",
                        path.display()
                    );
                }
            }
        }
        before.finish(b"q", 0);
        after.finish(b"q", 0);
        let canonical = |binary: &Path| {
            let output = Command::new(binary)
                .args(["inspect", "--canonical"])
                .arg(&path)
                .output()
                .unwrap();
            assert!(output.status.success());
            output.stdout
        };
        assert_eq!(canonical(&baseline), canonical(&current));
    }
}
