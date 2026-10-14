use super::*;

fn command() -> CommandBuilder {
    let binary = std::env::var_os("NATIVE_DIFF_HOST_BINARY")
        .expect("Build embedded_viewer and set NATIVE_DIFF_HOST_BINARY to that exact executable");
    assert!(std::path::Path::new(&binary).is_file());
    CommandBuilder::new(binary)
}

fn ready(session: &mut Session, expected: &str) {
    session.wait(|screen| {
        let text = screen.contents();
        text.contains("State: ready") && text.contains(expected)
    });
}

fn click(session: &mut Session, text: &str, column: u16) {
    session.wait(|screen| screen.contents().contains(text));
    let screen = session.parser.screen();
    let row = (0..screen.size().0)
        .find(|&row| {
            (28..screen.size().1)
                .map(|x| screen.cell(row, x).unwrap().contents())
                .collect::<String>()
                .contains(text)
        })
        .unwrap();
    session.keys(format!("\x1b[<0;{column};{}M\x1b[<0;{column};{}m", row + 1, row + 1).as_bytes());
}

#[test]
#[ignore = "requires NATIVE_DIFF_HOST_BINARY pointing to a freshly built embedded_viewer example"]
fn embedded_host_repeats_files_and_pinned_git_with_expansion_and_reflow() {
    use super::local_source_contract::git;
    let (directory, patch) = generated_patch(b"old patch\n", b"replacement patch\n");
    let old = directory.path().join("before.ts");
    let new = directory.path().join("after.ts");
    let original: String = (1..=80)
        .map(|n| format!("let file_{n:03} = {n};\n"))
        .collect();
    fs::write(&old, &original).unwrap();
    fs::write(&new, original.replace("file_044 = 44", "file_044 = 444")).unwrap();
    let repo = directory.path().join("repository");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    for name in ["a", "b", "c"] {
        fs::write(
            repo.join(format!("{name}.ts")),
            original.replace("file_", &format!("git_{name}_")),
        )
        .unwrap();
    }
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "before"]);
    let base = String::from_utf8(git(&repo, &["rev-parse", "HEAD"])).unwrap();
    for name in ["a", "b", "c"] {
        fs::write(
            repo.join(format!("{name}.ts")),
            original
                .replace("file_", &format!("git_{name}_"))
                .replace("_044 = 44", "_044 = 444"),
        )
        .unwrap();
    }
    git(&repo, &["commit", "-qam", "after"]);
    let head = String::from_utf8(git(&repo, &["rev-parse", "HEAD"])).unwrap();
    let scratch = directory.path().join("private");
    fs::create_dir(&scratch).unwrap();
    let mut launch = command();
    launch.arg(patch);
    launch.arg("--files");
    launch.arg(&old);
    launch.arg(&new);
    launch.arg("--git");
    launch.arg(&repo);
    launch.args([base.trim(), head.trim()]);
    launch.arg("--files");
    launch.arg(directory.path().join("absent.ts"));
    launch.arg(&new);
    launch.env("TMPDIR", &scratch);
    let mut session = Session::spawn(launch);
    ready(&mut session, "replacement patch");
    session.keys(b"uwt");
    session.wait(|s| s.contents().contains("UNIFIED") && s.contents().contains("light"));
    for cycle in 0..2 {
        // The host handles one event per loop and draws in between, and the input thread
        // queues one event at a time. Tab must precede `r`, so the new review's readiness
        // also proves the focus change was handled. With `r` first, a fast load can draw a
        // ready frame before Tab arrives.
        session.keys(b"\tr");
        let value = if cycle == 0 { 444 } else { 999 };
        ready(&mut session, &format!("file_044 = {value}"));
        let text = session.parser.screen().contents();
        assert!(text.contains("Review: 2") && text.contains("Focus: host"));
        assert!(text.contains("UNIFIED") && text.contains("light"));
        assert!(text.contains("40 unchanged lines"));
        assert!(!text.contains("replacement patch"));
        fs::write(&new, original.replace("file_044 = 44", "file_044 = 999")).unwrap();
        click(&mut session, "40 unchanged lines", 130);
        ready(&mut session, "file_025 = 25");
        click(&mut session, "file_025 = 25", 130);
        session.wait(|s| {
            s.contents().contains("Old: Some(25)") && s.contents().contains("New: Some(25)")
        });
        for (rows, cols) in [(20, 80), (34, 116), (50, 160)] {
            session.resize(rows, cols);
            session.settled_grid();
            let text = session.parser.screen().contents();
            assert!(text.contains("Old: Some(25)") && text.contains("New: Some(25)"));
        }
        session.keys(b"G");
        session.wait(|s| s.contents().contains("New: Some(47)"));
        ready(&mut session, &format!("file_044 = {value}"));
        session.capture(&format!("embedded-files-{cycle}"));

        session.keys(b"r");
        ready(&mut session, "git_a_044 = 444");
        assert!(session.parser.screen().contents().contains("Review: 3"));
        session.keys(b"G");
        session
            .wait(|s| s.contents().contains("File: 2") && s.contents().contains("New: Some(47)"));
        ready(&mut session, "git_c_044 = 444");
        assert!(session.parser.screen().contents().contains("New: Some(47)"));
        if cycle == 0 {
            fs::write(repo.join("c.ts"), "unrelated later branch\n").unwrap();
            git(&repo, &["commit", "-qam", "move branch"]);
        }
        session.keys(b"c");
        session.wait(|s| s.contents().contains("State: closed"));
        assert!(session.child.try_wait().unwrap().is_none());
        assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
        session.keys(b"o");
        ready(&mut session, "git_a_044 = 444");
        session.keys(b"G");
        session
            .wait(|s| s.contents().contains("File: 2") && s.contents().contains("New: Some(47)"));
        ready(&mut session, "git_c_044 = 444");
        assert!(
            !session
                .parser
                .screen()
                .contents()
                .contains("unrelated later branch")
        );
        session.capture(&format!("embedded-pinned-git-{cycle}"));
        session.keys(b"r");
        session.wait(|s| s.contents().contains("State: input error"));
        assert!(!session.parser.screen().contents().contains("git_c_"));
        session.keys(b"r");
        ready(&mut session, "replacement patch");
        assert!(session.parser.screen().contents().contains("Review: 1"));
    }
    session.finish(b"q", 0);
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
}

#[test]
#[ignore = "requires NATIVE_DIFF_HOST_BINARY pointing to a freshly built embedded_viewer example"]
fn embedded_host_replaces_pending_full_source_coloring_without_stale_styles() {
    let (directory, patch) = generated_patch(b"return 3;\n", b"return 4;\n");
    let old = directory.path().join("before.ts");
    let new = directory.path().join("after.ts");
    let prefix = format!("/*\n{}", "hidden comment\n".repeat(120_000));
    fs::write(&old, format!("{prefix}return 1;\n")).unwrap();
    fs::write(&new, format!("{prefix}return 2;\n")).unwrap();
    let scratch = directory.path().join("private");
    fs::create_dir(&scratch).unwrap();
    let mut launch = command();
    launch.arg("--files");
    launch.arg(&old);
    launch.arg(&new);
    launch.arg(patch);
    launch.env("TMPDIR", &scratch);
    let mut session = Session::spawn(launch);
    for _ in 0..2 {
        session.wait(|s| s.contents().contains("coloring visible"));
        session.keys(b"\tr");
        ready(&mut session, "return 4;");
        assert!(session.parser.screen().contents().contains("Focus: host"));
        assert!(session.parser.screen().contents().contains("Review: 2"));
        assert!(
            !session
                .parser
                .screen()
                .contents()
                .contains("hidden comment")
        );
        let screen = session.parser.screen();
        let mut words = 0;
        for y in 0..screen.size().0 {
            for x in 28..screen.size().1 - 6 {
                let word: String = (x..x + 6)
                    .map(|x| screen.cell(y, x).unwrap().contents())
                    .collect();
                if word == "return" {
                    assert_eq!(
                        screen.cell(y, x).unwrap().fgcolor(),
                        vt100::Color::Rgb(255, 103, 141)
                    );
                    words += 1;
                }
            }
        }
        assert_eq!(words, 2);
        session.keys(b"\tr");
    }
    session.wait(|s| s.contents().contains("coloring visible"));
    session.finish(b"q", 0);
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
}

#[test]
#[ignore = "requires NATIVE_DIFF_HOST_BINARY pointing to a freshly built embedded_viewer example"]
fn embedded_host_reports_missing_lazy_sources_and_recovers_with_a_new_review() {
    use super::local_source_contract::git;
    let (directory, patch) = generated_patch(b"before\n", b"recovered patch\n");
    let repo = directory.path().join("repository");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    for n in 0..30 {
        fs::write(
            repo.join(format!("file-{n:02}.rs")),
            format!("const OLD_{n}: u32 = {n};\n"),
        )
        .unwrap();
    }
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "before"]);
    for n in 0..30 {
        fs::write(
            repo.join(format!("file-{n:02}.rs")),
            format!("const NEW_{n}: u32 = {};\n", n + 1),
        )
        .unwrap();
    }
    git(&repo, &["commit", "-qam", "after"]);
    let blob = String::from_utf8(git(&repo, &["rev-parse", "HEAD:file-29.rs"])).unwrap();
    let blob = blob.trim();
    let scratch = directory.path().join("private");
    fs::create_dir(&scratch).unwrap();
    let mut launch = command();
    launch.arg("--git");
    launch.arg(&repo);
    launch.args(["HEAD^", "HEAD"]);
    launch.arg(patch);
    launch.env("TMPDIR", &scratch);
    let mut session = Session::spawn(launch);
    ready(&mut session, "NEW_0");
    fs::remove_file(repo.join(".git/objects").join(&blob[..2]).join(&blob[2..])).unwrap();
    session.keys(b"G");
    session.wait(|s| s.contents().contains("State: source unavailable"));
    assert!(session.parser.screen().contents().contains("NEW_29"));
    session.capture("embedded-source-unavailable");
    session.keys(b"r");
    ready(&mut session, "recovered patch");
    assert!(
        !session
            .parser
            .screen()
            .contents()
            .contains("Source unavailable")
    );
    session.keys(b"c");
    session.wait(|s| s.contents().contains("State: closed"));
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
    session.finish(b"q", 0);
}

#[test]
#[ignore = "requires NATIVE_DIFF_HOST_BINARY pointing to a freshly built embedded_viewer example"]
fn embedded_host_routes_focus_replaces_failed_reviews_and_restores_terminal() {
    let old: String = (1..=80)
        .map(|n| format!("fn source_{n:03}() {{ old(); }}\n"))
        .collect();
    let new = old.replace("old();", "new();");
    let (directory, path) = generated_patch(old.as_bytes(), new.as_bytes());
    let (_second, replacement) = generated_patch(b"fn previous() {}\n", b"fn replacement() {}\n");
    let scratch = directory.path().join("private");
    fs::create_dir(&scratch).unwrap();
    let mut launch = command();
    launch.arg(&path);
    launch.arg(&replacement);
    launch.arg(directory.path().join("absent.patch"));
    launch.env("TMPDIR", &scratch);
    let mut session = Session::spawn(launch);
    ready(&mut session, "fn source_001() { new(); }");
    session.keys(b"g");
    session.wait(|s| s.contents().contains("Old: Some(1)"));
    session.keys(b"\tG");
    session.wait(|s| s.contents().contains("Focus: host"));
    session.settled_grid();
    assert!(session.parser.screen().contents().contains("Old: Some(1)"));
    session.keys(b"\tG");
    session.wait(|s| s.contents().contains("New: Some(80)"));
    ready(&mut session, "fn source_080() { new(); }");
    session.capture("embedded-bottom");

    session.keys(b"uw");
    session.resize(34, 116);
    session.wait(|s| s.contents().contains("UNIFIED"));
    ready(&mut session, "fn source_080() { new(); }");
    assert!(session.parser.screen().contents().contains("New: Some(80)"));
    session.keys(b"r");
    ready(&mut session, "fn replacement() {}");
    assert!(!session.parser.screen().contents().contains("source_080"));
    session.keys(b"r");
    session.wait(|s| s.contents().contains("State: input error"));
    session.keys(b"r");
    ready(&mut session, "fn source_001() { old(); }");
    for _ in 0..3 {
        session.keys(b"c");
        session.wait(|s| {
            s.contents().contains("State: closed") && s.contents().contains("Review closed")
        });
        assert!(session.child.try_wait().unwrap().is_none());
        assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
        session.keys(b"o");
        ready(&mut session, "fn source_001() { old(); }");
    }
    session.capture("embedded-reopened");
    session.finish(b"q", 0);
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
}

#[test]
#[ignore = "requires NATIVE_DIFF_HOST_BINARY pointing to a freshly built embedded_viewer example"]
fn embedded_host_cancels_unconnected_and_partial_fifo_without_exiting() {
    use std::os::unix::fs::OpenOptionsExt;
    let (directory, path) = generated_patch(b"old\n", b"complete replacement\n");
    let fifo = directory.path().join("stream.patch");
    nix::unistd::mkfifo(
        &fifo,
        nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
    )
    .unwrap();
    let mut launch = command();
    launch.arg(&fifo);
    launch.arg(path);
    let mut session = Session::spawn(launch);
    session.wait(|s| s.contents().contains("Loading patch"));
    session.keys(b"c");
    session.wait(|s| s.contents().contains("State: closed"));
    session.keys(b"r");
    ready(&mut session, "complete replacement");
    session.keys(b"r");
    session.wait(|s| s.contents().contains("Loading patch"));
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut producer = loop {
        match fs::OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&fifo)
        {
            Ok(file) => break file,
            Err(error) => {
                assert_eq!(error.raw_os_error(), Some(libc::ENXIO));
                assert!(Instant::now() < deadline, "FIFO reader did not connect");
                thread::sleep(Duration::from_millis(5));
            }
        }
    };
    producer.write_all(b"diff --git a/").unwrap();
    session.keys(b"r");
    ready(&mut session, "complete replacement");
    assert_eq!(
        producer.write(b"remaining").unwrap_err().raw_os_error(),
        Some(libc::EPIPE)
    );
    drop(producer);
    session.finish(b"q", 0);
}

#[test]
#[ignore = "requires NATIVE_DIFF_HOST_BINARY pointing to a freshly built embedded_viewer example"]
fn embedded_host_replaces_loading_files_and_reaps_the_git_child() {
    use super::local_source_contract::stop_git_child;
    let (directory, patch) = generated_patch(b"old\n", b"usable replacement\n");
    let old = directory.path().join("before.txt");
    let new = directory.path().join("after.txt");
    let mut old_file = std::io::BufWriter::new(fs::File::create(&old).unwrap());
    let mut new_file = std::io::BufWriter::new(fs::File::create(&new).unwrap());
    for n in 0..120_000 {
        writeln!(old_file, "old {n} {}", "x".repeat(120)).unwrap();
        writeln!(new_file, "new {n} {}", "y".repeat(120)).unwrap();
    }
    old_file.flush().unwrap();
    new_file.flush().unwrap();
    let scratch = directory.path().join("private");
    fs::create_dir(&scratch).unwrap();
    let mut launch = command();
    launch.arg("--files");
    launch.arg(old);
    launch.arg(new);
    launch.arg(patch);
    launch.env("TMPDIR", &scratch);
    let mut session = Session::spawn(launch);
    for cycle in 0..2 {
        session.wait(|s| s.contents().contains("State: loading"));
        let stopped = stop_git_child(&session);
        session.keys(b"r");
        ready(&mut session, "usable replacement");
        assert!(session.child.try_wait().unwrap().is_none());
        stopped.assert_reaped();
        drop(stopped);
        assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
        session.keys(b"c");
        session.wait(|s| s.contents().contains("State: closed"));
        if cycle == 0 {
            session.keys(b"r");
        }
    }
    session.finish(b"q", 0);
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
}
