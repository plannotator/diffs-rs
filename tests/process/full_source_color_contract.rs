use super::*;

fn assert_roles(screen: &vt100::Screen, light: bool, split: bool) {
    let (rows, cols) = screen.size();
    let mut observed = 0;
    for y in 1..rows - 1 {
        let start: String = (0..4)
            .map(|x| screen.cell(y, x).unwrap().contents())
            .collect();
        if start.trim_start().starts_with("@@") {
            continue;
        }
        for x in 0..cols.saturating_sub(6) {
            let word: String = (x..x + 6)
                .map(|x| screen.cell(y, x).unwrap().contents())
                .collect();
            if word != "return" {
                continue;
            }
            let gutter: String = (0..x)
                .map(|x| screen.cell(y, x).unwrap().contents())
                .collect();
            let old_side = if split {
                x < cols / 2
            } else {
                gutter.contains('−')
            };
            let expected = if old_side {
                if light {
                    vt100::Color::Rgb(211, 42, 97)
                } else {
                    vt100::Color::Rgb(255, 103, 141)
                }
            } else {
                vt100::Color::Rgb(115, 115, 115)
            };
            for x in x..x + 6 {
                assert_eq!(
                    screen.cell(y, x).unwrap().fgcolor(),
                    expected,
                    "cell {x},{y}:\n{}",
                    screen.contents()
                );
            }
            observed += 1;
        }
    }
    assert!(
        observed >= if split { 4 } else { 2 },
        "missing expected code:\n{}",
        screen.contents()
    );
}

#[test]
fn full_source_colors_survive_real_navigation_layout_theme_wrap_and_resize() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("old.ts");
    let b = dir.path().join("new.ts");
    let mut old = vec!["return 1; // unchanged".to_owned(); 900];
    old[..300].fill("early comment".into());
    old[0] = "/*".into();
    old[300] = "*/".into();
    let mut new = old.clone();
    new[300] = "not closed".into();
    new[895] = format!(
        "return 2; // anchor\t日本語 café {}",
        "long text ".repeat(30)
    );
    fs::write(&a, old.join("\n") + "\n").unwrap();
    fs::write(&b, new.join("\n") + "\n").unwrap();
    let mut command = CommandBuilder::new(viewer_binary());
    command.args(["--no-sidebar", "--layout", "split", "files"]);
    command.arg(&a);
    command.arg(&b);
    let mut terminal = Session::spawn(command);
    terminal.wait(|s| s.contents().contains("complete-context coloring"));
    terminal.keys(b"G");
    terminal.settled_grid();
    assert_roles(terminal.parser.screen(), false, true);
    terminal.keys(b"w");
    terminal.settled_grid();
    assert_roles(terminal.parser.screen(), false, true);
    terminal.keys(b"t");
    terminal.settled_grid();
    assert_roles(terminal.parser.screen(), true, true);
    for (rows, cols) in [(35, 100), (50, 160), (20, 80), (50, 160)] {
        terminal.resize(rows, cols);
        terminal.settled_grid();
        assert_roles(terminal.parser.screen(), true, true);
    }
    terminal.keys(b"u");
    terminal.settled_grid();
    assert_roles(terminal.parser.screen(), true, false);
    terminal.keys(b"gutG");
    terminal.settled_grid();
    assert_roles(terminal.parser.screen(), false, true);
    terminal.finish(b"q", 0);
}

#[test]
fn git_source_coloring_remains_responsive_during_hidden_prefix_replay_and_cleans_up() {
    use super::local_source_contract::git;
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    let scratch = repo.join("private");
    fs::create_dir(&scratch).unwrap();
    git(repo, &["init", "-q"]);
    let prefix = format!("/*\n{}", "hidden comment\n".repeat(120_000));
    fs::write(repo.join("code.ts"), format!("{prefix}return 1;\n")).unwrap();
    git(repo, &["add", "code.ts"]);
    git(repo, &["commit", "-qm", "old"]);
    fs::write(repo.join("code.ts"), format!("{prefix}return 2;\n")).unwrap();
    git(repo, &["commit", "-qam", "new"]);
    for cancel in [false, true] {
        let mut command = CommandBuilder::new(viewer_binary());
        command.args(["--no-sidebar", "git", "--repo"]);
        command.arg(repo);
        command.args(["--base", "HEAD^", "--head", "HEAD"]);
        command.env("TMPDIR", &scratch);
        let mut terminal = Session::spawn(command);
        terminal.wait(|s| s.contents().contains("coloring visible"));
        if cancel {
            terminal.finish(b"\x03", 130);
        } else {
            terminal.keys(b"tuw");
            for (rows, cols) in [(25, 80), (12, 40), (50, 160)] {
                terminal.resize(rows, cols);
            }
            terminal.settled_grid();
            assert!(
                terminal
                    .parser
                    .screen()
                    .contents()
                    .contains("complete-context coloring")
            );
            assert!(terminal.parser.screen().contents().contains("UNIFIED"));
            let screen = terminal.parser.screen();
            let mut count = 0;
            for y in 1..49 {
                for x in 0..154 {
                    let word: String = (x..x + 6)
                        .map(|x| screen.cell(y, x).unwrap().contents())
                        .collect();
                    if word == "return" {
                        assert_eq!(
                            screen.cell(y, x).unwrap().fgcolor(),
                            vt100::Color::Rgb(115, 115, 115)
                        );
                        count += 1;
                    }
                }
            }
            assert_eq!(count, 2);
            terminal.finish(b"q", 0);
        }
        assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
    }
}
