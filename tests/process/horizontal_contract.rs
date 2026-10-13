use super::*;

fn open(path: &std::path::Path, layout: &str) -> Session {
    let binary = viewer_binary();
    let mut command = CommandBuilder::new(binary);
    command.arg("--no-sidebar");
    command.args(["--layout", layout, "patch"]);
    command.arg(path);
    Session::spawn(command)
}

fn input(terminal: &mut Session, keys: &[u8]) {
    terminal.keys(keys);
    terminal.keys(b"?");
    terminal.wait(|s| s.contents().contains("Reading controls"));
    terminal.keys(b"\x1b");
    terminal.wait(|s| !s.contents().contains("Reading controls"));
    terminal.settled_grid();
}

fn cells(terminal: &Session, y: u16, x: u16, width: u16) -> String {
    (x..x + width)
        .map(|x| terminal.parser.screen().cell(y, x).unwrap().contents())
        .collect()
}

#[test]
fn horizontal_scroll_stops_at_source_cell_boundaries_without_hidden_offset() {
    let long = format!("{}\t日e\u{301}{}END\n", "a".repeat(78), "b".repeat(7));
    for (layout, width) in [("split", 160), ("unified", 80)] {
        for old_long in [true, false] {
            let (old, new) = if old_long {
                (long.as_bytes(), b"short\n".as_slice())
            } else {
                (b"short\n".as_slice(), long.as_bytes())
            };
            let (_dir, path) = generated_patch_in_language(old, new, "txt");
            let mut terminal = open(&path, layout);
            terminal.settled_grid();
            terminal.resize(20, width);
            input(&mut terminal, b"");
            let (y, x, available) = if layout == "split" {
                if old_long { (3, 6, 73) } else { (3, 86, 74) }
            } else {
                (if old_long { 3 } else { 4 }, 10, 70)
            };
            // 78 ASCII cells, a two-cell tab, CJK (2), accent (1), and ten ASCII cells.
            let limit = 93 - available as usize;
            input(&mut terminal, &b"\x1b[C".repeat(40));
            assert_eq!(
                cells(&terminal, y, x, available),
                format!("{}  日e\u{301}{}END", "a".repeat(78 - limit), "b".repeat(7)),
                "wrong right boundary: {layout}, old_long={old_long}"
            );
            let at_end = terminal.parser.screen().contents_formatted();
            input(&mut terminal, &b"\x1b[C".repeat(80));
            assert_eq!(terminal.parser.screen().contents_formatted(), at_end);
            input(&mut terminal, b"\x1b[D");
            assert_eq!(
                cells(&terminal, y, x, available),
                format!("{}  日e\u{301}{}", "a".repeat(82 - limit), "b".repeat(6)),
                "one Left must move immediately after excess Right input"
            );
            input(&mut terminal, &b"\x1b[D".repeat(40));
            assert_eq!(
                cells(&terminal, y, x, available),
                "a".repeat(available as usize)
            );
            let at_start = terminal.parser.screen().contents_formatted();
            input(&mut terminal, &b"\x1b[D".repeat(80));
            assert_eq!(terminal.parser.screen().contents_formatted(), at_start);
            terminal.finish(b"q", 0);
        }
    }
}

#[test]
fn horizontal_scroll_reclamps_after_resize_layout_and_file_changes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("multi-file.patch");
    let long = format!("BEGIN{}END", "x".repeat(172));
    let mut patch = String::from(
        "diff --git a/short.txt b/short.txt\nnew file mode 100644\n--- /dev/null\n+++ b/short.txt\n@@ -0,0 +1,30 @@\n",
    );
    for _ in 0..30 {
        patch.push_str("+short\n");
    }
    patch.push_str(&format!("diff --git a/long.txt b/long.txt\nnew file mode 100644\n--- /dev/null\n+++ b/long.txt\n@@ -0,0 +1 @@\n+{long}\n"));
    patch.push_str("diff --git a/last.txt b/last.txt\nnew file mode 100644\n--- /dev/null\n+++ b/last.txt\n@@ -0,0 +1 @@\n+last\n");
    patch.push_str("diff --git a/empty.txt b/empty.txt\nold mode 100644\nnew mode 100755\n");
    fs::write(&path, patch).unwrap();
    let mut terminal = open(&path, "split");
    terminal.settled_grid();
    terminal.resize(20, 80);
    input(&mut terminal, b"");
    let short = terminal.parser.screen().contents_formatted();
    input(&mut terminal, &b"\x1b[C".repeat(80));
    assert_eq!(
        terminal.parser.screen().contents_formatted(),
        short,
        "an off-screen long line must not allow blank scrolling"
    );
    input(&mut terminal, b"n");
    input(&mut terminal, &b"\x1b[C".repeat(80));
    assert_eq!(
        cells(&terminal, 3, 46, 34),
        format!("{}END", "x".repeat(31))
    );
    terminal.resize(20, 160);
    input(&mut terminal, b"");
    assert_eq!(
        cells(&terminal, 3, 86, 74),
        format!("{}END", "x".repeat(71))
    );
    input(&mut terminal, b"l");
    assert_eq!(
        cells(&terminal, 3, 83, 77),
        format!("{}END", "x".repeat(74))
    );
    input(&mut terminal, b"u");
    assert_eq!(
        cells(&terminal, 3, 3, 157),
        format!("{}END", "x".repeat(154))
    );
    terminal.resize(20, 400);
    input(&mut terminal, b"");
    assert_eq!(cells(&terminal, 3, 3, 180), long);
    terminal.resize(20, 80);
    input(&mut terminal, &b"\x1b[C".repeat(80));
    input(&mut terminal, b"n");
    assert_eq!(cells(&terminal, 3, 3, 4), "last");
    input(&mut terminal, b"N");
    assert_eq!(cells(&terminal, 3, 3, 5), "BEGIN");
    input(&mut terminal, &b"\x1b[C".repeat(80));
    input(&mut terminal, b"w");
    let wrapped = terminal.parser.screen().contents_formatted();
    input(&mut terminal, &b"\x1b[C".repeat(80));
    assert_eq!(terminal.parser.screen().contents_formatted(), wrapped);
    input(&mut terminal, b"wnn");
    let metadata = terminal.parser.screen().contents_formatted();
    assert!(
        terminal
            .parser
            .screen()
            .contents()
            .contains("no textual hunks")
    );
    input(&mut terminal, &b"\x1b[C".repeat(80));
    assert_eq!(terminal.parser.screen().contents_formatted(), metadata);
    terminal.finish(b"q", 0);
}
