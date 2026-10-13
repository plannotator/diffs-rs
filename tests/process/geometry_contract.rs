use super::*;

fn open(path: &std::path::Path) -> Session {
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

#[test]
fn sparse_geometry_preserves_unicode_tabs_and_text_anchor() {
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
    let mut terminal = open(&path);
    terminal.settled_grid();
    for (steps, expected) in [
        (64, vec![" ", "e\u{301}", " ", " ", "Z", "b"]),
        (
            64,
            vec!["b", "日", "", "本", "", " ", " ", " ", "E", "N", "D"],
        ),
        (1, vec![" ", " ", " ", " ", "E", "N", "D"]),
    ] {
        terminal.keys(&b"\x1b[C".repeat(steps));
        terminal.settled_grid();
        let screen = terminal.parser.screen();
        for start in [6, 86] {
            for (offset, text) in expected.iter().enumerate() {
                let cell = screen.cell(3, start + offset as u16).unwrap();
                assert_eq!(cell.contents(), *text, "column {offset}, pane {start}");
                assert_eq!(cell.is_wide_continuation(), text.is_empty());
            }
        }
    }
    // E is at source column 520, beyond two sparse checkpoints and a clipped wide glyph.
    terminal.keys(b"\x1b[<0;91;4M");
    terminal.wait(|s| s.cell(3, 80).unwrap().contents() == "▎");
    terminal.keys(b"w");
    terminal.settled_grid();
    let anchor = line.find("END").unwrap();
    for (width, keys, split) in [
        (82, "", true),
        (160, "", true),
        (121, "", true),
        (121, "u", false),
    ] {
        terminal.resize(30, width);
        terminal.keys(keys.as_bytes());
        terminal.settled_grid();
        let (_, body) = active_pane(terminal.parser.screen(), split, false, true);
        let available = if split { width / 2 - 6 } else { width - 10 };
        let parts = reference_wrapped_segments(&line, available as usize);
        let expected = parts
            .iter()
            .rev()
            .find(|(byte, _)| *byte <= anchor)
            .unwrap();
        assert_eq!(
            body.trim_end(),
            expected.1.trim_end(),
            "width {width}, split {split}"
        );
    }
    terminal.capture("geometry-sparse-anchor");
    terminal.finish(b"q", 0);
}

fn assert_entry_rows(screen: &vt100::Screen, active: usize, initial: bool) {
    assert!(
        screen.contents().contains("entries.txt"),
        "{}",
        screen.contents()
    );
    assert!(has_active_source(
        screen,
        &format!("{active:4}+ "),
        &format!("ENTRY_{active:04}")
    ));
    let mut previous = None;
    let mut count = 0;
    for y in 2..49 {
        let row = cells(screen, y, 80, 80);
        if !row.contains("ENTRY_") {
            continue;
        }
        let number = cells(screen, y, 81, 4).trim().parse::<usize>().unwrap();
        assert_eq!(
            row.replace('▎', " "),
            format!(" {number:4}+ ENTRY_{number:04}") + &" ".repeat(63)
        );
        assert!(cells(screen, y, 0, 79).trim().is_empty());
        if let Some(previous) = previous {
            assert_eq!(number, previous + 1);
        } else {
            assert_eq!(number, active.saturating_sub(46).max(1));
        }
        previous = Some(number);
        count += 1;
    }
    assert_eq!(
        count,
        if initial { 46 } else { 47 },
        "missing source rows after navigation"
    );
}

fn assert_payload(screen: &vt100::Screen, number: usize) {
    for y in 1..49 {
        let file = number + (y as usize - 1) / 3;
        if file >= 40 {
            assert!(cells(screen, y, 0, 160).trim().is_empty());
        } else {
            match (y - 1) % 3 {
                0 => assert!(cells(screen, y, 0, 160).contains(&format!("payload-{file:02}.txt"))),
                1 => assert!(cells(screen, y, 0, 160).contains("@@ -0,0 +1 @@")),
                _ => {
                    assert_eq!(
                        cells(screen, y, 80, 80),
                        format!("   1+ BYTE_{file:02} {}", "x".repeat(66))
                    );
                    assert!(cells(screen, y, 0, 79).trim().is_empty());
                }
            }
        }
    }
    assert!(screen.contents().contains("Plain styling fallback"));
}

#[test]
fn geometry_eviction_preserves_cells_after_entry_and_byte_limits() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache-tour.patch");
    let mut patch = fs::File::create(&path).unwrap();
    writeln!(patch, "diff --git a/entries.txt b/entries.txt\nnew file mode 100644\n--- /dev/null\n+++ b/entries.txt\n@@ -0,0 +1,1152 @@").unwrap();
    for number in 1..=1152 {
        writeln!(patch, "+ENTRY_{number:04}").unwrap();
    }
    for number in 0..40 {
        writeln!(patch, "diff --git a/payload-{number:02}.txt b/payload-{number:02}.txt\nnew file mode 100644\n--- /dev/null\n+++ b/payload-{number:02}.txt\n@@ -0,0 +1 @@\n+BYTE_{number:02} {}", "x".repeat(512 * 1024)).unwrap();
    }
    drop(patch);
    let mut terminal = open(&path);
    terminal.settled_grid();
    terminal.keys(b"g");
    terminal.settled_grid();
    assert_entry_rows(terminal.parser.screen(), 1, true);
    for page in 1..=25 {
        terminal.keys(b"\x1b[6~");
        terminal.settled_grid();
        assert_entry_rows(terminal.parser.screen(), 1 + page * 46, false);
    }
    // The 1,151 visited source entries exceed the normal 1,024-entry limit.
    terminal.keys(b"g");
    terminal.settled_grid();
    assert_entry_rows(terminal.parser.screen(), 1, false);
    for number in 0..40 {
        terminal.keys(b"n");
        terminal.wait(|s| cells(s, 1, 0, 160).contains(&format!("payload-{number:02}.txt")));
        terminal.settled_grid();
        assert_payload(terminal.parser.screen(), number);
    }
    // More than 20 MiB of distinct display text exceeds the normal 16 MiB payload limit.
    terminal.keys(b"gn");
    terminal.wait(|s| cells(s, 1, 0, 160).contains("payload-00.txt"));
    terminal.settled_grid();
    assert_payload(terminal.parser.screen(), 0);
    terminal.capture("geometry-eviction-revisit");
    terminal.finish(b"q", 0);
}
