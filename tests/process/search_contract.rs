use super::*;

fn row(screen: &vt100::Screen, y: u16) -> String {
    screen.rows(0, screen.size().1).nth(y as usize).unwrap()
}

pub(super) fn cells_for(screen: &vt100::Screen, text: &str, old: bool) -> Vec<(u16, u16)> {
    let width = screen.size().1;
    let left = (width - 1) / 2;
    let columns = if old { 0..left } else { left + 1..width };
    for y in 1..screen.size().0 - 1 {
        for x in columns.clone() {
            let end = x + text.len() as u16;
            if end > columns.end {
                continue;
            }
            let found: String = (x..end)
                .map(|x| screen.cell(y, x).unwrap().contents())
                .collect();
            if found == text {
                return (x..end).map(|x| (y, x)).collect();
            }
        }
    }
    panic!(
        "missing {text} on {} side:\n{}",
        if old { "old" } else { "new" },
        screen.contents()
    );
}

fn expect_match(terminal: &mut Session, side: &str, line: u32) {
    let message = format!("Match · {side} line {line}");
    terminal.wait(|screen| screen.contents().contains(&message));
}

#[test]
fn literal_search_routes_prompt_keys_and_preserves_diff_colors_and_file_navigation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("review.patch");
    fs::write(&path, b"diff --git a/first.rs b/first.rs\n--- a/first.rs\n+++ b/first.rs\n@@ -1 +1 @@\n-// old q needle\n+// new q needle\ndiff --git a/second.rs b/second.rs\n--- a/second.rs\n+++ b/second.rs\n@@ -1 +1 @@\n-// before second\n+// after second\n").unwrap();
    let mut terminal = Session::open(&path);
    terminal.settled_grid();
    let before = terminal.parser.screen().clone();
    let old = cells_for(&before, "q needle", true);
    let new = cells_for(&before, "q needle", false);
    terminal.keys(b"/q needle\r");
    expect_match(&mut terminal, "old", 1);
    for &(y, x) in &old {
        let actual = terminal.parser.screen().cell(y, x).unwrap();
        let expected = before.cell(y, x).unwrap();
        assert!(actual.underline() && actual.bold());
        assert_eq!(actual.fgcolor(), expected.fgcolor());
        assert_eq!(actual.bgcolor(), expected.bgcolor());
    }
    assert!(
        new.iter()
            .all(|&(y, x)| !terminal.parser.screen().cell(y, x).unwrap().underline())
    );
    terminal.keys(b"\x1bOR");
    expect_match(&mut terminal, "new", 1);
    assert!(
        old.iter()
            .all(|&(y, x)| !terminal.parser.screen().cell(y, x).unwrap().underline())
    );
    assert!(
        new.iter()
            .all(|&(y, x)| terminal.parser.screen().cell(y, x).unwrap().underline())
    );
    terminal.keys(b"\x1b[13;2~");
    expect_match(&mut terminal, "old", 1);
    terminal.keys(b"n");
    terminal.wait(|screen| row(screen, 1).contains("second.rs"));
    terminal.keys(b"N");
    terminal.wait(|screen| row(screen, 1).contains("first.rs"));
    terminal.keys(b"/\r\x1bOR");
    expect_match(&mut terminal, "new", 1);
    terminal.keys(b"/discard q needle");
    terminal.wait(|screen| row(screen, 49).contains("/diff: discard q needle"));
    terminal.keys(b"\x1b");
    terminal.wait(|screen| !row(screen, 49).contains("/diff:"));
    terminal.keys(b"\x1bOR");
    terminal.wait(|screen| screen.contents().contains("Match · old line 1 · wrapped"));
    terminal.keys(b"/absent\r");
    terminal.wait(|screen| screen.contents().contains("No match in diff text"));
    terminal.keys(b"?");
    terminal.wait(|screen| {
        screen.contents().contains("F3 / Shift-F3")
            && screen.contents().contains("Next / previous file")
    });
    terminal.finish(b"q", 0);
    assert!(terminal.raw.windows(8).any(|part| part == b"\x1b[?2004h"));
    assert!(terminal.raw.windows(8).any(|part| part == b"\x1b[?2004l"));
}

#[test]
fn search_paste_is_atomic_bounded_and_unicode_queries_survive_prompt_resize() {
    let (_dir, path) = generated_patch(
        b"// \xff\x1b e\xcc\x81\t\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e old\n",
        b"// replacement\n",
    );
    let mut terminal = Session::open(&path);
    terminal.settled_grid();
    terminal.keys(b"/\x1b[200~q\nu\r\n\x1b[201~");
    terminal.wait(|screen| screen.contents().contains("paste was not inserted"));
    assert!(row(terminal.parser.screen(), 49).contains("/diff:"));
    let oversized = format!("\x1b[200~{}\x1b[201~", "日".repeat(1366));
    terminal.keys(oversized.as_bytes());
    terminal.wait(|screen| screen.contents().contains("4096 UTF-8 bytes"));
    terminal.keys("\x1b[200~日本語x\x1b[201~".as_bytes());
    terminal.wait(|screen| row(screen, 49).contains("日本語x"));
    terminal.resize(18, 80);
    terminal.wait(|screen| row(screen, 17).contains("日本語x"));
    terminal.keys(b"\x1b[D\x1b[3~\r");
    expect_match(&mut terminal, "old", 1);
    let text: String = (1..17)
        .flat_map(|y| (0..80).map(move |x| (y, x)))
        .filter_map(|(y, x)| {
            let cell = terminal.parser.screen().cell(y, x).unwrap();
            cell.underline().then(|| cell.contents().to_owned())
        })
        .collect();
    assert_eq!(text, "日本語");
    terminal
        .keys("/\x1b[200~👩💻日本語\x1b[201~\x1b[H\x1b[C\x1b[200~\u{200d}\x1b[201~\x7f".as_bytes());
    terminal.wait(|screen| row(screen, 17).contains("/diff: 日本語"));
    terminal.keys(b"\r");
    expect_match(&mut terminal, "old", 1);
    terminal.keys(b"w");
    terminal.resize(10, 40);
    terminal.wait(|screen| screen.size() == (10, 40) && screen.contents().contains("/ search"));
    terminal.keys(b"/q");
    terminal.wait(|screen| row(screen, 9).contains("/diff: q"));
    terminal.finish(b"\x03", 130);
}

#[test]
fn distant_unicode_match_keeps_exact_emphasis_through_wrapping_layout_theme_and_resize() {
    let query = "日本語-e\u{301}-needle";
    let old = format!("//\t{} {query} old\n", "padding ".repeat(180));
    let (_dir, path) = generated_patch(old.as_bytes(), b"// replacement\n");
    let mut terminal = Session::open(&path);
    terminal.settled_grid();
    terminal.keys(format!("/{query}\r").as_bytes());
    expect_match(&mut terminal, "old", 1);
    for (keys, cols) in [
        ("", 160),
        ("w", 70),
        ("u", 120),
        ("t", 70),
        ("u", 160),
        ("w", 120),
    ] {
        terminal.keys(keys.as_bytes());
        terminal.resize(18, cols);
        terminal.settled_grid();
        let screen = terminal.parser.screen();
        let marked: String = (1..17)
            .flat_map(|y| (0..cols).map(move |x| (y, x)))
            .filter_map(|(y, x)| {
                let cell = screen.cell(y, x).unwrap();
                cell.underline().then(|| cell.contents().to_owned())
            })
            .collect();
        assert_eq!(
            marked,
            query,
            "keys {keys:?}, cols {cols}\n{}",
            screen.contents()
        );
    }
    terminal.finish(b"q", 0);
}

#[test]
fn expanded_context_invalidates_search_and_f3_restarts_with_the_new_scope() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.rs");
    let new = dir.path().join("new.rs");
    let mut lines: Vec<_> = (1..=100).map(|n| format!("// source {n}\n")).collect();
    lines[43] = "// hiddenneedle\n".into();
    fs::write(&old, lines.concat()).unwrap();
    lines[49] = "// changed source 50\n".into();
    fs::write(&new, lines.concat()).unwrap();
    let scratch = dir.path().join("private");
    fs::create_dir(&scratch).unwrap();
    let mut command = CommandBuilder::new(viewer_binary());
    command.args(["--no-sidebar", "files"]);
    command.arg(&old);
    command.arg(&new);
    command.env("TMPDIR", &scratch);
    let mut terminal = Session::spawn(command);
    terminal.settled_grid();
    terminal.keys(b"/hiddenneedle\r");
    terminal.wait(|screen| screen.contents().contains("No match in diff text"));
    let gap = (1..49)
        .find(|&y| row(terminal.parser.screen(), y).contains("unchanged lines"))
        .unwrap();
    terminal.keys(format!("\x1b[<0;100;{}M\x1b[<0;100;{}m", gap + 1, gap + 1).as_bytes());
    terminal.wait(|screen| screen.contents().contains("Search scope changed"));
    terminal.keys(b"\x1bOR");
    expect_match(&mut terminal, "new", 44);
    terminal.finish(b"q", 0);
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
}

#[test]
fn incomplete_review_rejects_search_without_losing_the_prompt_or_quit_cleanup() {
    use nix::{sys::stat::Mode, unistd::mkfifo};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("waiting.patch");
    mkfifo(&path, Mode::S_IRUSR | Mode::S_IWUSR).unwrap();
    let mut terminal = Session::open(&path);
    terminal.wait(|screen| screen.contents().contains("Loading patch"));
    terminal.keys(b"/query\r");
    terminal.wait(|screen| {
        screen
            .contents()
            .contains("Wait for the review to finish loading")
    });
    assert!(
        row(terminal.parser.screen(), 49).contains("/diff: query"),
        "loading prompt: {:?}\n{}",
        row(terminal.parser.screen(), 49),
        terminal.parser.screen().contents()
    );
    terminal.finish(b"\x03", 130);
    assert!(terminal.raw.windows(8).any(|part| part == b"\x1b[?2004l"));
}
