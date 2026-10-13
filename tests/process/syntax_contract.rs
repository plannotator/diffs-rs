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

fn keyword(light: bool) -> vt100::Color {
    if light {
        vt100::Color::Rgb(211, 42, 97)
    } else {
        vt100::Color::Rgb(255, 103, 141)
    }
}

fn literal(language: &str, light: bool) -> vt100::Color {
    if language != "py" {
        vt100::Color::Rgb(115, 115, 115)
    } else {
        string_color(light)
    }
}

fn string_color(light: bool) -> vt100::Color {
    if light {
        vt100::Color::Rgb(25, 159, 67)
    } else {
        vt100::Color::Rgb(94, 204, 113)
    }
}

fn deep_cells(screen: &vt100::Screen, language: &str, old_literal: bool, light: bool, end: bool) {
    let (first, last) = if end { (468, 512) } else { (422, 468) };
    for (row, n) in (first..=last).enumerate() {
        let y = 2 + row as u16;
        let text = if language == "py" {
            format!("return 1 # row_{n:04}")
        } else {
            format!("return 1; // row_{n:04}")
        };
        for (old, x, width) in [(true, 0, 79), (false, 80, 80)] {
            let in_literal = old == old_literal;
            let number = n + usize::from(in_literal);
            let expected = format!(" {number:3}  {text}");
            assert_eq!(
                cells(screen, y, x, width).replace('▎', " "),
                format!("{expected:<width$}", width = width as usize)
            );
            for column in 6..12 {
                assert_eq!(
                    screen.cell(y, x + column).unwrap().fgcolor(),
                    if in_literal {
                        literal(language, light)
                    } else {
                        keyword(light)
                    },
                    "{language}, old={old}, light={light}, source={number}"
                );
            }
            let number_color = if light {
                vt100::Color::Rgb(28, 161, 199)
            } else {
                vt100::Color::Rgb(104, 205, 242)
            };
            assert_eq!(
                screen.cell(y, x + 13).unwrap().fgcolor(),
                if in_literal {
                    literal(language, light)
                } else {
                    number_color
                }
            );
            let comment_start = if language == "py" { 9 } else { 10 };
            for column in comment_start..text.len() as u16 {
                assert_eq!(
                    screen.cell(y, x + 6 + column).unwrap().fgcolor(),
                    if in_literal {
                        literal(language, light)
                    } else {
                        vt100::Color::Rgb(115, 115, 115)
                    }
                );
            }
        }
    }
    if end {
        let ending = if language == "py" {
            "return 2"
        } else {
            "return 2;"
        };
        for x in [0, 80] {
            assert_eq!(cells(screen, 48, x + 6, ending.len() as u16), ending);
            for column in 6..12 {
                assert_eq!(
                    screen.cell(48, x + column).unwrap().fgcolor(),
                    keyword(light)
                );
            }
        }
    }
}

#[test]
fn deep_replay_and_forward_resume_keep_each_sides_syntax() {
    for language in ["c", "rs", "ts", "py"] {
        for old_literal in [true, false] {
            let body: String = (1..=512)
                .map(|n| {
                    if language == "py" {
                        format!("return 1 # row_{n:04}\n")
                    } else {
                        format!("return 1; // row_{n:04}\n")
                    }
                })
                .collect();
            let (prefix, suffix, ending) = if language == "py" {
                ("value = \"\"\"\n", "\"\"\"\n", "return 2\n")
            } else {
                ("/*\n", "*/\n", "return 2;\n")
            };
            let in_literal = format!("{prefix}{body}{suffix}{ending}");
            let code = format!("{body}{ending}");
            let (old, new) = if old_literal {
                (&in_literal, &code)
            } else {
                (&code, &in_literal)
            };
            let (_dir, path) =
                generated_patch_in_language(old.as_bytes(), new.as_bytes(), language);
            let mut terminal = open(&path);
            terminal.settled_grid();
            for (keys, light, end) in [
                (b"G".as_slice(), false, true),
                (b"\x1b[5~\x1b[5~", false, false),
                (b"t", true, false),
                (b"\x1b[6~\x1b[6~", true, true),
                (b"tgGt", true, true),
            ] {
                terminal.keys(keys);
                terminal.settled_grid();
                deep_cells(terminal.parser.screen(), language, old_literal, light, end);
            }
            terminal.keys(b"u");
            terminal.settled_grid();
            let s = terminal.parser.screen();
            let mut checked = 0;
            for y in 2..49 {
                if cells(s, y, 10, 30).starts_with("return 1") {
                    for x in 10..16 {
                        assert_eq!(
                            s.cell(y, x).unwrap().fgcolor(),
                            if old_literal {
                                keyword(true)
                            } else {
                                literal(language, true)
                            }
                        );
                    }
                    checked += 1;
                }
            }
            assert!(checked >= 40);
            terminal.capture(&format!("syntax-deep-{language}-{old_literal}"));
            terminal.finish(b"q", 0);
        }
    }
}

#[test]
fn pending_resize_finishes_with_exact_side_colors() {
    let body = (1..=12_000)
        .map(|n| format!("return 1; // row_{n:05}\n"))
        .collect::<String>();
    let old = format!("/*\n{body}");
    let (_dir, path) = generated_patch_in_language(old.as_bytes(), body.as_bytes(), "ts");
    let mut terminal = open(&path);
    terminal.settled_grid();
    terminal.keys(b"G");
    terminal.wait(|s| {
        s.contents().contains("row_12000") && s.contents().contains("Highlighting visible")
    });
    for width in [119, 80, 160, 100, 120] {
        terminal.resize(30, width);
        terminal.keys(b"t");
        thread::sleep(Duration::from_millis(10));
    }
    terminal.resize(50, 160);
    terminal.settled_grid();
    terminal.keys(b"g");
    terminal.wait(|s| s.contents().contains("row_00001"));
    terminal.keys(b"G");
    terminal.settled_grid();
    let screen = terminal.parser.screen();
    assert!(screen.contents().contains("SPLIT"));
    assert!(screen.contents().contains("light"));
    for (row, n) in (11_954..=12_000).enumerate() {
        let y = 2 + row as u16;
        let text = format!("return 1; // row_{n:05}");
        for (old, x, width) in [(true, 0, 79), (false, 80, 80)] {
            let number = n + usize::from(old);
            let expected = format!(" {number:5}  {text}");
            assert_eq!(
                cells(screen, y, x, width).replace('▎', " "),
                format!("{expected:<width$}", width = width as usize)
            );
            for offset in 0..text.len() as u16 {
                let expected_color = if old || offset >= 10 {
                    vt100::Color::Rgb(115, 115, 115)
                } else if offset < 6 {
                    keyword(true)
                } else if offset == 7 {
                    vt100::Color::Rgb(28, 161, 199)
                } else {
                    continue;
                };
                assert_eq!(
                    screen.cell(y, x + 8 + offset).unwrap().fgcolor(),
                    expected_color
                );
            }
        }
    }
    terminal.finish(b"q", 0);
}

#[test]
fn heredoc_end_delimiters_stay_private_to_each_side() {
    let body = (0..320)
        .map(|n| format!("return 1 # shared_{n:04}\n"))
        .collect::<String>();
    let ending = "OLD_STOP\nreturn 2 # independent\nNEW_STOP\nreturn 3 # done\n";
    let tail = (0..100)
        .map(|n| format!("return 4 # tail_{n:04}\n"))
        .collect::<String>();
    let old = format!("value = <<OLD_STOP\n{body}{ending}{tail}");
    let new = format!("value = <<NEW_STOP\n{body}{ending}{tail}");
    let (_dir, path) = generated_patch_in_language(old.as_bytes(), new.as_bytes(), "rb");
    let mut terminal = open(&path);
    terminal.settled_grid();
    for (keys, light) in [(b"G".as_slice(), false), (b"tgG", true), (b"tgG", false)] {
        terminal.keys(keys);
        terminal.settled_grid();
        for _ in 0..3 {
            terminal.keys(b"\x1b[5~");
            terminal.settled_grid();
        }
        let screen = terminal.parser.screen();
        for (text, old_string, new_string) in [
            ("return 1 # shared_0319", true, true),
            ("return 2 # independent", false, true),
            ("return 3 # done", false, false),
        ] {
            for (x, string) in [(0, old_string), (80, new_string)] {
                let y = (2..49)
                    .find(|y| cells(screen, *y, x + 6, text.len() as u16) == text)
                    .expect("expected source on its original side");
                for column in 6..12 {
                    assert_eq!(
                        screen.cell(y, x + column).unwrap().fgcolor(),
                        if string {
                            string_color(light)
                        } else {
                            keyword(light)
                        },
                        "{text}, x={x}, light={light}"
                    );
                }
            }
        }
    }
    terminal.finish(b"q", 0);
}

#[test]
fn continuation_preserves_fallback_and_resets_at_a_new_hunk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fallback.patch");
    let mut patch = format!(
        "diff --git a/fallback.ts b/fallback.ts\n--- a/fallback.ts\n+++ b/fallback.ts\n@@ -1,351 +1,350 @@\n-{}\n",
        "x".repeat(10_001)
    );
    for n in 1..=350 {
        patch.push_str(&format!(" return 1; // row_{n:04}\n"));
    }
    patch.push_str("@@ -1000,2 +999,2 @@\n return 2; // fresh\n-old();\n+new();\n");
    fs::write(&path, patch).unwrap();
    let mut terminal = open(&path);
    terminal.settled_grid();
    for keys in [
        b"g".as_slice(),
        b"\x1b[6~",
        b"\x1b[6~",
        b"\x1b[6~",
        b"G",
        b"\x1b[5~\x1b[5~",
    ] {
        terminal.keys(keys);
        terminal.settled_grid();
        let s = terminal.parser.screen();
        assert!(s.contents().contains("Plain styling fallback"));
        let mut checked = 0;
        for y in 2..49 {
            if cells(s, y, 7, 30).starts_with("return 1;") {
                for x in 7..13 {
                    assert_eq!(
                        s.cell(y, x).unwrap().fgcolor(),
                        vt100::Color::Rgb(250, 250, 250)
                    );
                    assert_eq!(s.cell(y, x + 80).unwrap().fgcolor(), keyword(false));
                }
                checked += 1;
            }
        }
        assert!(checked >= 40);
    }
    terminal.keys(b"]");
    terminal.wait(|s| s.contents().contains("return 2; // fresh"));
    terminal.settled_grid();
    let s = terminal.parser.screen();
    assert!(!s.contents().contains("Plain styling fallback"));
    let y = (2..49)
        .find(|y| cells(s, *y, 7, 25).starts_with("return 2;"))
        .unwrap();
    for x in 7..13 {
        assert_eq!(s.cell(y, x).unwrap().fgcolor(), keyword(false));
        assert_eq!(s.cell(y, x + 80).unwrap().fgcolor(), keyword(false));
    }
    terminal.finish(b"q", 0);
}
