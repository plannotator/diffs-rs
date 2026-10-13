use super::*;

fn styled_screen(terminal: &mut Session, marker: &str, light: bool) -> vt100::Screen {
    terminal.wait(|s| {
        let text = s.contents();
        text.contains(marker)
            && text.contains(if light { "light" } else { "dark" })
            && !text.contains("Highlighting visible")
    });
    let screen = terminal.parser.screen();
    assert!(!screen.contents().contains("Plain styling fallback"));
    let (rows, columns) = screen.size();
    let mut keywords = 0;
    for y in 2..rows - 1 {
        for x in 0..columns - 6 {
            let word: String = (x..x + 6)
                .map(|column| screen.cell(y, column).unwrap().contents())
                .collect();
            if word == "return" {
                for column in x..x + 6 {
                    assert_eq!(
                        screen.cell(y, column).unwrap().fgcolor(),
                        if light {
                            vt100::Color::Rgb(211, 42, 97)
                        } else {
                            vt100::Color::Rgb(255, 103, 141)
                        }
                    );
                }
                keywords += 1;
            }
        }
    }
    assert!(keywords >= 20, "too few visible source lines were checked");
    screen.clone()
}

fn final_view(terminal: &mut Session) -> vt100::Screen {
    terminal.keys(b"G");
    terminal.wait(|s| {
        s.contents()
            .lines()
            .any(|line| line.contains('▎') && line.contains("row_0256"))
    });
    terminal.keys(b"t");
    terminal.resize(30, 100);
    terminal.wait(|s| s.contents().contains("UNIFIED"));
    styled_screen(terminal, "row_0256", true)
}

#[test]
fn streamed_source_is_fully_styled_before_eof_and_matches_ordinary_input() {
    let source = |value| {
        (1..=256)
            .map(|n| format!("return {value}; // row_{n:04}\n"))
            .collect::<String>()
    };
    let (dir, patch) =
        generated_patch_in_language(source(1).as_bytes(), source(2).as_bytes(), "ts");
    let mut ordinary = Session::open(&patch);
    let expected = [
        styled_screen(&mut ordinary, "row_0001", false),
        final_view(&mut ordinary),
    ];
    ordinary.finish(b"q", 0);

    let fifo = dir.path().join("stream.patch");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let prefix = fs::read(&patch).unwrap();
    let writer_path = fifo.clone();
    let (release, paused) = mpsc::channel::<()>();
    let producer = thread::spawn(move || {
        let mut writer = fs::OpenOptions::new()
            .write(true)
            .open(writer_path)
            .unwrap();
        writer.write_all(&prefix).unwrap();
        writer
            .write_all(b"diff --git a/later.ts b/later.ts\n")
            .unwrap();
        let _ = paused.recv();
    });
    let mut streamed = Session::open(&fifo);
    for (step, expected) in expected.iter().enumerate() {
        let actual = if step == 0 {
            styled_screen(&mut streamed, "row_0001", false)
        } else {
            final_view(&mut streamed)
        };
        assert!(
            actual.contents().contains("Loading patch"),
            "styling must finish while the source producer is still paused"
        );
        assert!(!actual.contents().contains("later.ts"));
        assert_eq!(actual.size(), expected.size());
        let (rows, columns) = actual.size();
        for y in 1..rows - 1 {
            for x in 0..columns {
                assert_eq!(
                    actual.cell(y, x),
                    expected.cell(y, x),
                    "streamed source or styling differs at step {step}, cell {x},{y}"
                );
            }
        }
    }
    streamed.finish(b"q", 0);
    release.send(()).unwrap();
    producer.join().unwrap();
}

fn open_split(path: &std::path::Path) -> Session {
    let binary = viewer_binary();
    let mut cmd = CommandBuilder::new(binary);
    cmd.arg("--no-sidebar");
    cmd.args(["--layout", "split", "patch"]);
    cmd.arg(path);
    Session::spawn(cmd)
}

fn cells(screen: &vt100::Screen, y: u16, x: u16, width: u16) -> String {
    (x..x + width)
        .map(|x| screen.cell(y, x).unwrap().contents())
        .collect()
}

fn pending_multifile_patch() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pending.patch");
    let mut patch = String::from(
        "diff --git a/guard.rb b/guard.rb\n--- a/guard.rb\n+++ b/guard.rb\n@@ -1,65 +1,64 @@\n-value = <<STOP\n",
    );
    for n in 1..=64 {
        patch.push_str(&format!(" return 2 # guard_{n:04} 日本語 café\n"));
    }
    patch.push_str(
        "diff --git a/deep.ts b/deep.ts\n--- a/deep.ts\n+++ b/deep.ts\n@@ -1,12001 +1,12000 @@\n-/*\n",
    );
    for n in 1..=12_000 {
        patch.push_str(&format!(" return 1; // row_{n:05}\n"));
    }
    fs::write(&path, patch).unwrap();
    (dir, path)
}

fn pending_end(terminal: &mut Session) {
    terminal.keys(b"G");
    terminal.wait(|s| {
        let text = s.contents();
        text.contains("row_12000") && text.contains("Highlighting visible")
    });
}

fn checked_syntax_view(
    terminal: &mut Session,
    guard: bool,
    split: bool,
    light: bool,
) -> vt100::Screen {
    terminal.settled_grid();
    let screen = terminal.parser.screen();
    let text = screen.contents();
    assert!(text.contains(if guard { "guard.rb" } else { "deep.ts" }));
    assert!(text.contains(if split { "SPLIT" } else { "UNIFIED" }));
    assert!(text.contains(if light { "light" } else { "dark" }));
    assert!(!text.contains("Plain styling fallback"));
    let (rows, columns) = screen.size();
    let panes = if split {
        vec![(0, (columns - 1) / 2, true), (columns / 2, columns, false)]
    } else {
        vec![(0, columns, false)]
    };
    for (start, end, old) in panes {
        let mut checked = 0;
        for y in 2..rows - 1 {
            let row = cells(screen, y, start, end - start);
            if !row.contains(if guard { "guard_" } else { "row_" }) {
                continue;
            }
            let x = (start..end - 6)
                .find(|x| cells(screen, y, *x, 6) == "return")
                .expect("expected source keyword");
            for column in x..x + 6 {
                assert_eq!(
                    screen.cell(y, column).unwrap().fgcolor(),
                    match (old, guard, light) {
                        (true, true, true) => vt100::Color::Rgb(25, 159, 67),
                        (true, true, false) => vt100::Color::Rgb(94, 204, 113),
                        (true, false, _) => vt100::Color::Rgb(115, 115, 115),
                        (false, _, true) => vt100::Color::Rgb(211, 42, 97),
                        (false, _, false) => vt100::Color::Rgb(255, 103, 141),
                    },
                    "wrong syntax at {column},{y}, old={old}, guard={guard}"
                );
            }
            checked += 1;
        }
        assert!(checked >= 20, "too few independently checked source lines");
    }
    screen.clone()
}

#[test]
fn pending_file_navigation_and_theme_changes_match_fresh_views() {
    let (_dir, path) = pending_multifile_patch();
    let mut terminal = open_split(&path);
    terminal.settled_grid();
    pending_end(&mut terminal);
    terminal.keys(b"g");
    terminal.wait(|s| s.contents().contains("guard_0001"));
    let first = checked_syntax_view(&mut terminal, true, true, false);

    terminal.keys(b"t");
    pending_end(&mut terminal);
    terminal.keys(b"g");
    terminal.wait(|s| s.contents().contains("guard_0001"));
    terminal.resize(30, 100);
    terminal.keys(b"uw");
    terminal.wait(|s| s.contents().contains("guard_0001") && s.contents().contains("UNIFIED"));
    let second = checked_syntax_view(&mut terminal, true, false, true);

    terminal.resize(50, 160);
    terminal.keys(b"uwG");
    terminal.wait(|s| s.contents().contains("row_12000") && s.contents().contains("SPLIT"));
    let third = checked_syntax_view(&mut terminal, false, true, true);
    terminal.finish(b"q", 0);

    let mut fresh = open_split(&path);
    fresh.settled_grid();
    fresh.keys(b"n");
    fresh.wait(|s| s.contents().contains("deep.ts"));
    fresh.settled_grid();
    fresh.keys(b"g");
    fresh.wait(|s| s.contents().contains("guard_0001"));
    for (step, expected) in [first, second, third].iter().enumerate() {
        if step == 1 {
            fresh.keys(b"tG");
            fresh.wait(|s| s.contents().contains("row_12000"));
            fresh.settled_grid();
            fresh.keys(b"g");
            fresh.wait(|s| s.contents().contains("guard_0001"));
            fresh.resize(30, 100);
            fresh.keys(b"uw");
            fresh.wait(|s| s.contents().contains("UNIFIED"));
        } else if step == 2 {
            fresh.resize(50, 160);
            fresh.keys(b"uwG");
            fresh.wait(|s| s.contents().contains("row_12000") && s.contents().contains("SPLIT"));
        }
        let actual = checked_syntax_view(&mut fresh, step != 2, step != 1, step != 0);
        assert_eq!(actual.size(), expected.size());
        let (rows, columns) = actual.size();
        for y in 0..rows {
            for x in 0..columns {
                assert_eq!(
                    actual.cell(y, x),
                    expected.cell(y, x),
                    "interrupted and fresh views differ at step {step}, cell {x},{y}"
                );
            }
        }
    }
    fresh.finish(b"q", 0);
}

#[test]
fn quitting_with_pending_coloring_restores_the_terminal() {
    let (_dir, path) = pending_multifile_patch();
    for (keys, code) in [(b"q".as_slice(), 0), (b"\x03", 130)] {
        let mut terminal = open_split(&path);
        terminal.settled_grid();
        pending_end(&mut terminal);
        terminal.finish(keys, code);
    }
}
