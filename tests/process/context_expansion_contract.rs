use super::*;

#[path = "../support/readiness.rs"]
mod readiness;

#[test]
fn one_end_command_keeps_the_last_source_visible_after_lazy_readiness() {
    let dir = readiness::repository(false);
    let scratch = dir.path().join("private");
    fs::create_dir(&scratch).unwrap();
    let binary = viewer_binary();
    for layout in ["split", "unified"] {
        let mut command = CommandBuilder::new(&binary);
        command.args(["--no-sidebar", "--layout", layout, "git", "--repo"]);
        command.arg(dir.path());
        command.args(["--base", "HEAD^", "--head", "HEAD"]);
        command.env("TMPDIR", &scratch);
        let mut terminal = Session::spawn(command);
        terminal.settled_grid();
        terminal.keys(b"G");
        terminal.wait(|screen| {
            screen.contents().contains("file-11.ts")
                && screen.contents().contains("37 unchanged lines")
        });
        let deadline = Instant::now() + Duration::from_millis(300);
        while Instant::now() < deadline {
            if let Ok(bytes) = terminal.output.recv_timeout(Duration::from_millis(10)) {
                terminal.ingest(bytes);
            }
        }
        let gutter = if layout == "split" {
            "763  "
        } else {
            "763 763  "
        };
        assert!(
            has_active_source(
                terminal.parser.screen(),
                gutter,
                "return 763; // source 763"
            ),
            "Selected endpoint disappeared after source readiness:\n{}",
            terminal.parser.screen().contents()
        );
        terminal.settled_grid();
        assert!(has_active_source(
            terminal.parser.screen(),
            gutter,
            "return 763; // source 763"
        ));
        assert!(
            terminal
                .parser
                .screen()
                .contents()
                .contains("complete-context coloring")
        );
        terminal.capture(&format!("source-ready-end-{layout}"));
        terminal.finish(b"q", 0);
        assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
    }
}

fn row_text(screen: &vt100::Screen, row: u16) -> String {
    (0..screen.size().1)
        .map(|x| screen.cell(row, x).unwrap().contents())
        .collect()
}

fn click_gap(terminal: &mut Session, text: &str) {
    terminal.settled_grid();
    let y = (1..terminal.parser.screen().size().0 - 1)
        .find(|&y| row_text(terminal.parser.screen(), y).contains(text))
        .unwrap_or_else(|| {
            panic!(
                "missing gap {text}:\n{}",
                terminal.parser.screen().contents()
            )
        });
    terminal.keys(format!("\x1b[<0;100;{}M\x1b[<0;100;{}m", y + 1, y + 1).as_bytes());
    terminal.settled_grid();
}

fn assert_active(terminal: &Session, old: u32, new: u32, split: bool) {
    let screen = terminal.parser.screen();
    let gutter = if split {
        format!("{new:3}  ")
    } else {
        format!("{old:3} {new:3}  ")
    };
    assert!(
        has_active_source(screen, &gutter, &format!("return {old};")),
        "{}",
        screen.contents()
    );
}

#[test]
fn expand_real_files_by_click_and_enter_through_layout_resize_and_eof() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("old.ts");
    let b = dir.path().join("new.ts");
    let old: Vec<_> = (1..=163)
        .map(|n| format!("return {n}; // row {n}"))
        .collect();
    let mut new = old.clone();
    new.splice(
        43..44,
        [
            "return 430;".into(),
            "return 431;".into(),
            "return 432;".into(),
        ],
    );
    new[106] = "return 1060;".into();
    fs::write(&a, old.join("\n")).unwrap();
    fs::write(&b, new.join("\n")).unwrap();
    let scratch = dir.path().join("private");
    fs::create_dir(&scratch).unwrap();
    let mut command = CommandBuilder::new(viewer_binary());
    command.args(["--no-sidebar", "--layout", "split", "files"]);
    command.arg(&a);
    command.arg(&b);
    command.env("TMPDIR", &scratch);
    #[cfg(feature = "delivery-trace")]
    command.env(
        "NATIVE_DIFF_DELIVERY_TRACE",
        dir.path().join("delivery.json"),
    );
    let mut terminal = Session::spawn(command);
    terminal.settled_grid();
    assert!(
        terminal
            .parser
            .screen()
            .contents()
            .contains("40 unchanged lines · Show 20")
    );
    fs::write(&a, "changed after capture").unwrap();
    fs::write(&b, "changed after capture").unwrap();
    click_gap(&mut terminal, "40 unchanged");
    terminal.keys(b"g");
    terminal.settled_grid();
    assert_active(&terminal, 21, 21, true);
    terminal.keys(b"k\r");
    terminal.settled_grid();
    assert_active(&terminal, 1, 1, true);
    terminal.keys(b"]");
    terminal.settled_grid();
    click_gap(&mut terminal, "54 unchanged");
    // This is the forward gap: the newly exposed edge is old line 67, new line 69.
    assert_active(&terminal, 67, 69, true);
    for (width, keys, split) in [
        (100, b"tw".as_slice(), true),
        (121, b"u", false),
        (160, b"t", false),
        (160, b"uw", true),
    ] {
        terminal.resize(30, width);
        terminal.settled_grid();
        terminal.keys(keys);
        terminal.settled_grid();
        assert_active(&terminal, 67, 69, split);
    }
    for (old, new) in [(128, 130), (148, 150), (163, 165)] {
        terminal.keys(b"Gj\r");
        terminal.settled_grid();
        assert_active(&terminal, old, new, true);
        terminal.keys(b"G");
        terminal.settled_grid();
        assert_active(&terminal, old, new, true);
    }
    assert!(
        terminal
            .parser
            .screen()
            .contents()
            .contains("No newline at end of file")
    );
    terminal.capture("context-expanded-eof");
    terminal.keys(b"g");
    terminal.settled_grid();
    assert_active(&terminal, 1, 1, true);
    terminal.finish(b"q", 0);
    assert_eq!(fs::read_dir(scratch).unwrap().count(), 0);
    #[cfg(feature = "delivery-trace")]
    {
        let path = dir.path().join("delivery.json");
        let trace: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let mut observed = 0;
        for event in trace["events"].as_array().unwrap() {
            if event["kind"] == "drain" && event["b"].is_object() {
                assert_eq!(event["b"]["file"], 0);
                assert!(
                    event["b"]["old"]
                        .as_u64()
                        .is_some_and(|n| (1..=163).contains(&n))
                );
                assert!(
                    event["b"]["new"]
                        .as_u64()
                        .is_some_and(|n| (1..=165).contains(&n))
                );
                observed += 1;
            }
        }
        assert!(observed >= 20);
        if let Some(directory) = std::env::var_os("NATIVE_DIFF_CAPTURE_DIR") {
            fs::copy(
                path,
                std::path::Path::new(&directory).join("context-expanded-delivery.json"),
            )
            .unwrap();
        }
    }
}

#[test]
fn expanded_unicode_text_keeps_exact_anchor_through_sidebar_wrap_and_width_changes() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("old.ts");
    let b = dir.path().join("new.ts");
    let line = format!(
        "// {}日e\u{301}\tZ{}日本\tANCHOR{}",
        "a".repeat(255),
        "b".repeat(252),
        "c".repeat(80)
    );
    let mut old: Vec<_> = (1..=75).map(|n| format!("return {n};")).collect();
    old[27] = line.clone();
    let mut new = old.clone();
    new[43] = "return 440;".into();
    fs::write(&a, old.join("\n") + "\n").unwrap();
    fs::write(&b, new.join("\n") + "\n").unwrap();
    let mut command = CommandBuilder::new(viewer_binary());
    command.args(["--no-sidebar", "--layout", "split", "files"]);
    command.arg(&a);
    command.arg(&b);
    let mut terminal = Session::spawn(command);
    terminal.settled_grid();
    click_gap(&mut terminal, "40 unchanged");
    terminal.keys(b"gjjjjjjj");
    terminal.keys(&b"\x1b[C".repeat(131));
    terminal.settled_grid();
    let y = (1..49)
        .find(|&y| terminal.parser.screen().cell(y, 80).unwrap().contents() == "▎")
        .unwrap();
    // Click the A at display column 524 of the expanded source, then preserve that byte.
    terminal.keys(format!("\x1b[<0;87;{}M", y + 1).as_bytes());
    terminal.keys(b"w");
    terminal.settled_grid();
    let anchor = line.find("ANCHOR").unwrap();
    for (width, keys, split, tree) in [
        (82, b"".as_slice(), true, false),
        (160, b"f\x1b", true, true),
        (121, b"u", false, true),
        (80, b"", false, false),
        (160, b"ff", false, false),
    ] {
        terminal.resize(30, width);
        terminal.settled_grid();
        terminal.keys(keys);
        terminal.settled_grid();
        let available_width = width - if tree { 33 } else { 0 };
        let available = if split {
            available_width / 2 - 6
        } else {
            available_width - 10
        };
        let screen = terminal.parser.screen();
        let start = if tree { 33 } else { 0 };
        let active_x = start
            + if split {
                available_width.div_ceil(2)
            } else {
                0
            };
        let y = (1..screen.size().0 - 1)
            .find(|&y| screen.cell(y, active_x).unwrap().contents() == "▎")
            .unwrap_or_else(|| {
                panic!(
                    "lost anchor at width={width} tree={tree} x={active_x}:\n{}",
                    screen.contents()
                )
            });
        let body: String = (active_x + if split { 6 } else { 10 }..width)
            .map(|x| screen.cell(y, x).unwrap().contents())
            .collect();
        let parts = reference_wrapped_segments(&line, available as usize);
        let expected = parts
            .iter()
            .rev()
            .find(|(byte, _)| *byte <= anchor)
            .unwrap();
        assert_eq!(
            body.trim_end(),
            expected.1.trim_end(),
            "width {width}, split={split}, tree={tree}\n{}",
            screen.contents()
        );
    }
    terminal.capture("context-expanded-unicode-anchor");
    terminal.finish(b"\x03", 130);
}

#[test]
fn git_expansion_keeps_file_order_tree_targets_and_pinned_sources() {
    use super::local_source_contract::git;
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    git(repo, &["init", "-q"]);
    for name in ["a", "b", "c"] {
        fs::write(
            repo.join(format!("{name}.ts")),
            (1..=80)
                .map(|n| format!("return {n}; // {name} source {n}\n"))
                .collect::<String>(),
        )
        .unwrap();
    }
    git(repo, &["add", "."]);
    git(repo, &["commit", "-qm", "old"]);
    for name in ["a", "b", "c"] {
        let path = repo.join(format!("{name}.ts"));
        fs::write(
            &path,
            fs::read_to_string(&path)
                .unwrap()
                .replace("return 44;", "return 440;"),
        )
        .unwrap();
    }
    git(repo, &["commit", "-qam", "new"]);
    let mut command = CommandBuilder::new(viewer_binary());
    command.args(["--sidebar", "--layout", "split", "git", "--repo"]);
    command.arg(repo);
    command.args(["--base", "HEAD^", "--head", "HEAD"]);
    let mut terminal = Session::spawn(command);
    terminal.settled_grid();
    for name in ["a", "b", "c"] {
        fs::write(repo.join(format!("{name}.ts")), "unrelated later content\n").unwrap();
    }
    git(
        repo,
        &["commit", "-qam", "move branch after review started"],
    );
    for (index, name) in ["a", "b", "c"].into_iter().enumerate() {
        if index != 0 {
            terminal.keys(b"n");
            terminal.settled_grid();
        }
        click_gap(&mut terminal, "40 unchanged");
        let text = terminal.parser.screen().contents();
        assert!(text.contains(&format!("{name} source 21")), "{text}");
        assert!(!text.contains("unrelated later content"));
    }
    terminal.keys(b"N");
    terminal.settled_grid();
    assert!(terminal.parser.screen().contents().contains("b source 21"));
    terminal.keys(b"f\x1b[H\x1b[B\r");
    terminal.settled_grid();
    assert!(terminal.parser.screen().contents().contains("b source 21"));
    // Neither layout changes nor source replies discard any file's revealed extent.
    terminal.keys(b"uNg");
    terminal.settled_grid();
    assert!(terminal.parser.screen().contents().contains("a source 21"));
    terminal.keys(b"G");
    terminal.settled_grid();
    assert!(terminal.parser.screen().contents().contains("c source 47"));
    terminal.capture("context-expanded-git-tree");
    terminal.finish(b"q", 0);
}
