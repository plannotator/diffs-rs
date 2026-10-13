use super::*;

pub(super) fn git(repo: &std::path::Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .current_dir(repo)
        .args([
            "-c",
            "user.name=Source test",
            "-c",
            "user.email=source@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

pub(super) struct StoppedGit {
    pid: String,
    parent: String,
}

impl StoppedGit {
    pub(super) fn assert_reaped(&self) {
        let process = Command::new("ps")
            .args(["-p", &self.pid, "-o", "pid="])
            .output()
            .unwrap();
        assert!(
            process.stdout.is_empty(),
            "Git helper survived cancellation"
        );
    }
}

impl Drop for StoppedGit {
    fn drop(&mut self) {
        let parent = Command::new("ps")
            .args(["-p", &self.pid, "-o", "ppid="])
            .output();
        if parent.is_ok_and(|output| String::from_utf8_lossy(&output.stdout).trim() == self.parent)
        {
            let _ = Command::new("/bin/kill")
                .args(["-CONT", &self.pid])
                .stderr(std::process::Stdio::null())
                .status();
        }
    }
}

pub(super) fn stop_git_child(session: &Session) -> StoppedGit {
    let parent = session.child.process_id().unwrap().to_string();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        assert!(
            Instant::now() < deadline,
            "No Git helper appeared during the real comparison"
        );
        let children = Command::new("pgrep")
            .args(["-P", &parent])
            .output()
            .unwrap();
        for pid in std::str::from_utf8(&children.stdout).unwrap().lines() {
            let process = Command::new("ps")
                .args(["-p", pid, "-o", "comm="])
                .output()
                .unwrap();
            let name = String::from_utf8_lossy(&process.stdout);
            if std::path::Path::new(name.trim())
                .file_name()
                .is_some_and(|name| name == "git")
                && Command::new("/bin/kill")
                    .args(["-STOP", pid])
                    .stderr(std::process::Stdio::null())
                    .status()
                    .unwrap()
                    .success()
            {
                return StoppedGit {
                    pid: pid.into(),
                    parent,
                };
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn local_files_show_real_diff_keep_captures_and_restore_the_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("before.rs");
    let new = dir.path().join("after.rs");
    fs::copy("fixtures/position/old/review.ts", &old).unwrap();
    fs::copy("fixtures/position/new/review.ts", &new).unwrap();
    let output = Command::new(viewer_binary())
        .arg("files")
        .arg(&old)
        .arg(&new)
        .arg("--check")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["files"][0]["source"], "Source checked");
    let mut command = CommandBuilder::new(viewer_binary());
    command.arg("--no-sidebar");
    command.arg("files");
    command.arg(&old);
    command.arg(&new);
    let mut terminal = Session::spawn(command);
    terminal.wait(|screen| screen.contents().contains("complete-context coloring"));
    assert!(terminal.parser.screen().contents().contains("NEW_BEGIN"));
    assert!(
        terminal
            .parser
            .screen()
            .contents()
            .contains("complete-context coloring")
    );
    fs::write(&old, "WRONG NEW LIVE CONTENT\n").unwrap();
    fs::write(&new, "WRONG NEW LIVE CONTENT\n").unwrap();
    for keys in [b"w".as_slice(), b"u", b"G", b"g", b"t", b"f", b"f"] {
        terminal.keys(keys);
        terminal.settled_grid();
        assert!(
            !terminal
                .parser
                .screen()
                .contents()
                .contains("WRONG NEW LIVE CONTENT")
        );
    }
    for (rows, columns) in [(20, 80), (10, 40), (50, 160)] {
        terminal.resize(rows, columns);
        terminal.settled_grid();
        assert!(
            !terminal
                .parser
                .screen()
                .contents()
                .contains("WRONG NEW LIVE CONTENT")
        );
    }
    terminal.finish(b"q", 0);
}

#[test]
fn missing_lazy_git_source_keeps_the_patch_readable_and_exits_nonzero() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    git(repo, &["init", "-q"]);
    for i in 0..30 {
        fs::write(
            repo.join(format!("file-{i:02}.rs")),
            format!("const OLD_{i}: u32 = {i};\n"),
        )
        .unwrap();
    }
    git(repo, &["add", "."]);
    git(repo, &["commit", "-qm", "old"]);
    for i in 0..30 {
        fs::write(
            repo.join(format!("file-{i:02}.rs")),
            format!("const NEW_{i}: u32 = {};\n", i + 1),
        )
        .unwrap();
    }
    git(repo, &["commit", "-qam", "new"]);
    let object = String::from_utf8(git(repo, &["rev-parse", "HEAD:file-29.rs"])).unwrap();
    let object = object.trim();
    let mut command = CommandBuilder::new(viewer_binary());
    command.args(["--no-sidebar", "git", "--repo"]);
    command.arg(repo);
    command.args(["--base", "HEAD^", "--head", "HEAD"]);
    command.env("GIT_EXTERNAL_DIFF", "false");
    command.env("GIT_CONFIG_COUNT", "1");
    command.env("GIT_CONFIG_KEY_0", "diff.external");
    command.env("GIT_CONFIG_VALUE_0", "false");
    let mut terminal = Session::spawn(command);
    terminal.wait(|screen| screen.contents().contains("Source checked"));
    fs::remove_file(
        repo.join(".git/objects")
            .join(&object[..2])
            .join(&object[2..]),
    )
    .unwrap();
    terminal.keys(b"G");
    terminal.wait(|screen| screen.contents().contains("Source unavailable"));
    assert!(terminal.parser.screen().contents().contains("NEW_29"));
    terminal.keys(b"ug");
    terminal.settled_grid();
    assert!(terminal.parser.screen().contents().contains("OLD_0"));
    terminal.finish(b"q", 1);
}

#[test]
fn source_input_cancel_reaps_git_and_removes_private_files() {
    let dir = tempfile::tempdir().unwrap();
    let scratch = dir.path().join("private");
    fs::create_dir(&scratch).unwrap();
    let old = dir.path().join("old");
    let new = dir.path().join("new");
    let mut old_file = std::io::BufWriter::new(fs::File::create(&old).unwrap());
    let mut new_file = std::io::BufWriter::new(fs::File::create(&new).unwrap());
    for i in 0..120_000 {
        writeln!(old_file, "old {i} {}", "x".repeat(120)).unwrap();
        writeln!(new_file, "new {i} {}", "y".repeat(120)).unwrap();
    }
    old_file.flush().unwrap();
    new_file.flush().unwrap();
    let mut command = CommandBuilder::new(viewer_binary());
    command.arg("files");
    command.arg(&old);
    command.arg(&new);
    command.env("TMPDIR", &scratch);
    let mut terminal = Session::spawn(command);
    terminal.wait(|screen| screen.contents().contains("Loading patch"));
    let stopped = stop_git_child(&terminal);
    terminal.finish(b"\x03", 130);
    stopped.assert_reaped();
    drop(stopped);
    assert_eq!(
        fs::read_dir(scratch).unwrap().count(),
        0,
        "capture files survived cancellation"
    );
}
