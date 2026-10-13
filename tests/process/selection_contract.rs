use super::{search_contract::cells_for, *};

fn footer(screen: &vt100::Screen) -> String {
    let (rows, cols) = screen.size();
    screen.rows(0, cols).nth(usize::from(rows - 1)).unwrap()
}

fn expect_range(terminal: &mut Session, range: &str) {
    terminal.wait(|screen| footer(screen).contains(range));
}

fn click(terminal: &mut Session, y: u16, x: u16, shift: bool) {
    let button = if shift { 4 } else { 0 };
    terminal.keys(
        format!(
            "\x1b[<{button};{};{}M\x1b[<{button};{};{}m",
            x + 1,
            y + 1,
            x + 1,
            y + 1
        )
        .as_bytes(),
    );
}

fn resize_and_wait(terminal: &mut Session, keys: &[u8], width: u16, header: &str, range: &str) {
    if !keys.is_empty() {
        let screen = terminal.parser.screen();
        let (marker, header_only) = match keys {
            b"w" => ("↪", false),
            b"u" => ("SPLIT", true),
            b"t" => ("dark", true),
            _ => panic!("setting requires an explicit frame witness"),
        };
        let had_marker = if header_only {
            screen
                .rows(0, screen.size().1)
                .next()
                .unwrap()
                .contains(marker)
        } else {
            screen.contents().contains(marker)
        };
        terminal.keys(keys);
        terminal.wait(|screen| {
            let has_marker = if header_only {
                screen
                    .rows(0, screen.size().1)
                    .next()
                    .unwrap()
                    .contains(marker)
            } else {
                screen.contents().contains(marker)
            };
            has_marker != had_marker
        });
    }
    let resized = terminal.parser.screen().size() != (24, width);
    terminal.frames = Some(Vec::new());
    if resized {
        terminal.resize(24, width);
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let screen = terminal.parser.screen();
        let start = width - header.chars().count() as u16 - 1;
        let right: String = (start..width - 1)
            .map(|x| screen.cell(0, x).unwrap().contents())
            .collect();
        if (!resized || !terminal.frames.as_ref().unwrap().is_empty())
            && right == header
            && footer(screen).contains(range)
            && !screen.contents().contains("Highlighting visible")
            && !screen.contents().contains("coloring visible")
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "resize did not reach {header} / {range}:\n{}",
            screen.contents()
        );
        if let Ok(bytes) = terminal.output.recv_timeout(Duration::from_millis(50)) {
            terminal.ingest(bytes);
        }
        assert!(terminal.child.try_wait().unwrap().is_none());
    }
    terminal.frames = None;
}

#[test]
fn line_selection_preserves_colors_and_survives_focus_without_hiding_search_outcomes() {
    let dir = tempfile::tempdir().unwrap();
    let patch = dir.path().join("review.patch");
    fs::write(&patch, b"diff --git a/first.rs b/first.rs\n--- a/first.rs\n+++ b/first.rs\n@@ -1,3 +1,3 @@\n-// old needle\n-// old middle\n-// old last\n+// new needle\n+// new middle\n+// new last\ndiff --git a/second.rs b/second.rs\n--- a/second.rs\n+++ b/second.rs\n@@ -1 +1 @@\n-before\n+after\n").unwrap();
    let mut terminal = Session::open(&patch);
    terminal.settled_grid();
    let before = terminal.parser.screen().clone();
    let old = cells_for(&before, "// old middle", true);
    let new = cells_for(&before, "// new middle", false);
    terminal.keys(b"gVj");
    expect_range(&mut terminal, "Selecting old 1–2");
    for &(y, x) in &old {
        let actual = terminal.parser.screen().cell(y, x).unwrap();
        let expected = before.cell(y, x).unwrap();
        assert!(actual.underline());
        assert_eq!(actual.fgcolor(), expected.fgcolor());
        assert_eq!(actual.bgcolor(), expected.bgcolor());
    }
    assert!(
        new.iter()
            .all(|&(y, x)| !terminal.parser.screen().cell(y, x).unwrap().underline())
    );
    terminal.keys(b"Vj");
    expect_range(&mut terminal, "Selected old 1–2");
    terminal.keys(b"?");
    terminal.wait(|screen| screen.contents().contains("Shift-click"));
    terminal.keys(b"\x1b");
    terminal.wait(|screen| {
        !screen.contents().contains("Shift-click") && footer(screen).contains("Selected old 1–2")
    });
    terminal.keys(b"f");
    terminal.wait(|screen| screen.contents().contains("Enter open"));
    terminal.keys(b"\x1b");
    terminal.wait(|screen| !screen.contents().contains("Enter open"));
    expect_range(&mut terminal, "Selected old 1–2");
    terminal.keys(b"\x1b");
    terminal.wait(|screen| footer(screen).contains("Selection cleared"));
    terminal.keys(b"ffg/needle\r");
    terminal.wait(|screen| footer(screen).contains("Match · old line 1"));
    terminal.keys(b"V\x1bOR");
    terminal.wait(|screen| footer(screen).contains("Match · new line 1 · selection cleared"));
    terminal.keys(b"V\x1bOR");
    terminal
        .wait(|screen| footer(screen).contains("Match · old line 1 · wrapped · selection cleared"));
    terminal.keys(b"V");
    expect_range(&mut terminal, "Selecting old 1–1");
    terminal.keys(b"n");
    terminal.wait(|screen| {
        screen.rows(0, 160).nth(1).unwrap().contains("second.rs")
            && footer(screen).contains("Selection cleared")
    });
    terminal.finish(b"q", 0);
}

#[test]
fn shift_click_and_reflow_keep_original_range_and_narrow_footer_visible() {
    let old = format!(
        "// old first\n//\t日本語 e\u{301} {}tail\n// old last\n",
        "segment ".repeat(18)
    );
    let (_dir, patch) = generated_patch(old.as_bytes(), b"// new short\n");
    let mut terminal = Session::open(&patch);
    terminal.settled_grid();
    terminal.keys(b"g");
    terminal.wait(|screen| has_active_source(screen, "  1− ", "// old first"));
    let (y, x) = cells_for(terminal.parser.screen(), "// old last", true)[0];
    click(&mut terminal, y, x, true);
    expect_range(&mut terminal, "Selected old 1–3");
    terminal.keys(b"k");
    expect_range(&mut terminal, "Selected old 1–3");
    for (keys, width, header) in [
        ("w", 70, "1 files  ·  UNIFIED  ·  dark"),
        ("u", 120, "1 files  ·  SPLIT  ·  dark"),
        ("t", 40, "1 files  ·  SPLIT  ·  light"),
        ("u", 160, "1 files  ·  UNIFIED  ·  light"),
    ] {
        resize_and_wait(
            &mut terminal,
            keys.as_bytes(),
            width,
            header,
            "Selected old 1–3",
        );
        let marked: String = (1..23)
            .flat_map(|y| (0..width).map(move |x| (y, x)))
            .filter_map(|(y, x)| {
                let cell = terminal.parser.screen().cell(y, x).unwrap();
                cell.underline().then(|| cell.contents().to_owned())
            })
            .collect();
        assert!(marked.contains("日本語 e\u{301}"), "{marked:?}");
        assert!(
            marked.contains("tail") && marked.contains("// old last"),
            "keys {keys:?}, width {width}, marked {marked:?}\n{}",
            terminal.parser.screen().contents()
        );
        assert_eq!(marked.matches("segment").count(), 18, "{marked:?}");
        assert!(!marked.contains("new short"));
    }
    resize_and_wait(
        &mut terminal,
        b"u",
        160,
        "1 files  ·  SPLIT  ·  light",
        "Selected old 1–3",
    );
    let (y, x) = cells_for(terminal.parser.screen(), "// new short", false)[0];
    click(&mut terminal, y, x, false);
    terminal.wait(|screen| footer(screen).contains("Selection cleared"));
    terminal.keys(b"V");
    expect_range(&mut terminal, "Selecting new 1–1");
    let (y, _) = cells_for(terminal.parser.screen(), "// old last", true)[0];
    click(&mut terminal, y, 100, false);
    terminal.keys(b"\x1b");
    terminal.wait(|screen| footer(screen).contains("Selection cleared"));
    terminal.keys(b"V");
    terminal.wait(|screen| footer(screen).contains("not a header, gap or padding"));
    terminal.finish(b"q", 0);
}

#[test]
fn expanding_a_context_gap_keeps_the_range_until_a_real_source_endpoint_is_chosen() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.rs");
    let new = dir.path().join("new.rs");
    let mut lines: Vec<_> = (1..=100).map(|n| format!("// source {n}\n")).collect();
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
    terminal.keys(b"gV");
    expect_range(&mut terminal, "Selecting new 47–47");
    let gap = terminal
        .parser
        .screen()
        .rows(0, 160)
        .position(|row| row.contains("unchanged lines"))
        .unwrap() as u16;
    click(&mut terminal, gap, 100, false);
    terminal.wait(|screen| {
        screen.contents().contains("// source 46") && footer(screen).contains("Selecting new 47–47")
    });
    let (y, x) = cells_for(terminal.parser.screen(), "// source 46", false)[0];
    click(&mut terminal, y, x, true);
    expect_range(&mut terminal, "Selected new 46–47");
    resize_and_wait(
        &mut terminal,
        b"",
        40,
        "1 files  ·  UNIFIED  ·  dark",
        "Selected new 46–47",
    );
    terminal.keys(b"\t");
    terminal.wait(|screen| footer(screen).contains("Selection cleared"));
    terminal.finish(b"q", 0);
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
}
