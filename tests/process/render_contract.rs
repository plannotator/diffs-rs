use super::*;
use std::path::Path;

#[derive(Clone, Copy)]
struct Line<'a> {
    old: Option<u32>,
    new: Option<u32>,
    sign: char,
    text: &'a str,
    changed: &'a [(usize, usize)],
}

fn context(n: u32, m: u32, text: &str) -> Line<'_> {
    Line {
        old: Some(n),
        new: Some(m),
        sign: ' ',
        text,
        changed: &[],
    }
}

fn removed<'a>(n: u32, text: &'a str, changed: &'a [(usize, usize)]) -> Line<'a> {
    Line {
        old: Some(n),
        new: None,
        sign: '−',
        text,
        changed,
    }
}

fn added<'a>(n: u32, text: &'a str, changed: &'a [(usize, usize)]) -> Line<'a> {
    Line {
        old: None,
        new: Some(n),
        sign: '+',
        text,
        changed,
    }
}

struct Example<'a> {
    name: &'a str,
    old: &'a str,
    new: &'a str,
    lines: Vec<Line<'a>>,
    split: Vec<(Option<usize>, Option<usize>)>,
}

fn examples() -> Vec<Example<'static>> {
    vec![
        Example {
            name: "insert-before-replacements",
            old: "start\nlet retries = 3;\nlet delay = 10;\nend\n",
            new: "start\n// inserted\nlet retries = 5;\nlet delay = 20;\nend\n",
            lines: vec![
                context(1, 1, "start"),
                removed(2, "let retries = 3;", &[(14, 15)]),
                removed(3, "let delay = 10;", &[(12, 14)]),
                added(2, "// inserted", &[]),
                added(3, "let retries = 5;", &[(14, 15)]),
                added(4, "let delay = 20;", &[(12, 14)]),
                context(4, 5, "end"),
            ],
            split: vec![
                (Some(0), Some(0)),
                (None, Some(3)),
                (Some(1), Some(4)),
                (Some(2), Some(5)),
                (Some(6), Some(6)),
            ],
        },
        Example {
            name: "remove-before-replacement",
            old: "start\nobsolete_one();\nobsolete_two();\nlet retries = 3;\nend\n",
            new: "start\nlet retries = 5;\nend\n",
            lines: vec![
                context(1, 1, "start"),
                removed(2, "obsolete_one();", &[]),
                removed(3, "obsolete_two();", &[]),
                removed(4, "let retries = 3;", &[(14, 15)]),
                added(2, "let retries = 5;", &[(14, 15)]),
                context(5, 3, "end"),
            ],
            split: vec![
                (Some(0), Some(0)),
                (Some(1), None),
                (Some(2), None),
                (Some(3), Some(4)),
                (Some(5), Some(5)),
            ],
        },
        Example {
            name: "repeat-and-blank",
            old: "start\nsame();\n\nsame();\nend\n",
            new: "start\nsame();\n\nsame();\nsame();\nend\n",
            lines: vec![
                context(1, 1, "start"),
                context(2, 2, "same();"),
                context(3, 3, ""),
                context(4, 4, "same();"),
                added(5, "same();", &[]),
                context(5, 6, "end"),
            ],
            split: vec![
                (Some(0), Some(0)),
                (Some(1), Some(1)),
                (Some(2), Some(2)),
                (Some(3), Some(3)),
                (None, Some(4)),
                (Some(5), Some(5)),
            ],
        },
        Example {
            name: "new-file",
            old: "",
            new: "one\n\nthree\n",
            lines: vec![
                added(1, "one", &[]),
                added(2, "", &[]),
                added(3, "three", &[]),
            ],
            split: vec![(None, Some(0)), (None, Some(1)), (None, Some(2))],
        },
        Example {
            name: "deleted-file",
            old: "one\n\nthree\n",
            new: "",
            lines: vec![
                removed(1, "one", &[]),
                removed(2, "", &[]),
                removed(3, "three", &[]),
            ],
            split: vec![(Some(0), None), (Some(1), None), (Some(2), None)],
        },
    ]
}

fn patch(old: &[u8], new: &[u8], language: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let old_path = format!("old.{language}");
    let new_path = format!("new.{language}");
    fs::write(dir.path().join(&old_path), old).unwrap();
    fs::write(dir.path().join(&new_path), new).unwrap();
    let output = Command::new("git")
        .args([
            "diff",
            "--no-index",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--diff-algorithm=myers",
            "--no-indent-heuristic",
            "--unified=100000",
            &old_path,
            &new_path,
        ])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_COUNT", "0")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let path = dir.path().join("input.patch");
    fs::write(&path, output.stdout).unwrap();
    (dir, path)
}

fn open(path: &Path, layout: &str) -> Session {
    let binary = viewer_binary();
    let mut cmd = CommandBuilder::new(binary);
    cmd.arg("--no-sidebar");
    cmd.args(["--layout", layout, "patch"]);
    cmd.arg(path);
    Session::spawn(cmd)
}

fn cells(s: &vt100::Screen, y: u16, x: u16, width: u16) -> String {
    (x..x + width)
        .map(|x| s.cell(y, x).unwrap().contents())
        .collect()
}

fn assert_line(
    s: &vt100::Screen,
    y: u16,
    x: u16,
    width: u16,
    line: Option<Line<'_>>,
    split: bool,
    old: bool,
) {
    let actual = cells(s, y, x, width);
    let Some(line) = line else {
        assert!(
            actual.trim().is_empty(),
            "fabricated opposite-side content at {y}:{x}: {actual:?}"
        );
        let gap = gap_color(s);
        for col in x..x + width {
            assert_eq!(s.cell(y, col).unwrap().bgcolor(), gap);
        }
        return;
    };
    let n = |n: Option<u32>| n.map_or_else(|| "   ".into(), |n| format!("{n:>3}"));
    let expected = if split {
        format!(
            "{}{} {}",
            n(if old { line.old } else { line.new }),
            line.sign,
            line.text
        )
    } else {
        format!("{} {}{} {}", n(line.old), n(line.new), line.sign, line.text)
    };
    assert!(matches!(s.cell(y, x).unwrap().contents(), " " | "▎"));
    assert_eq!(
        actual.chars().skip(1).collect::<String>(),
        format!("{expected:<remaining$}", remaining = width as usize - 1),
        "wrong source row at {y}:{x}"
    );
    let gutter = if split { 6 } else { 10 };
    let base = s.cell(y, x + width - 1).unwrap().bgcolor();
    if let vt100::Color::Rgb(r, g, b) = base {
        match line.sign {
            '+' => assert!(g > r && g > b, "addition has no green background"),
            '−' => assert!(r > g && r > b, "deletion has no red background"),
            _ => assert!(r == g && g == b, "context has a change background"),
        }
    } else {
        panic!("truecolor contract has no RGB source background");
    }
    for (i, _) in line.text.char_indices() {
        let emphasized = s.cell(y, x + gutter + i as u16).unwrap().bgcolor() != base;
        assert_eq!(
            emphasized,
            line.changed
                .iter()
                .any(|&(start, end)| (start..end).contains(&i)),
            "wrong change emphasis for {:?}, column {i}, row {y}",
            line.text
        );
    }
}

fn gap_color(s: &vt100::Screen) -> vt100::Color {
    let (_, width) = s.size();
    if cells(s, 0, 0, width).contains("light") {
        vt100::Color::Rgb(227, 227, 227)
    } else {
        vt100::Color::Rgb(48, 48, 48)
    }
}

#[test]
fn alignment_gaps_remain_distinct_from_blank_source_through_reflow() {
    let old = format!("before\n\nsame: {} END\nafter\n", "a".repeat(40));
    let new = format!("before\n\nsame: {} END\nafter\n", "a".repeat(75));
    let (_dir, path) = patch(old.as_bytes(), new.as_bytes(), "txt");
    let mut terminal = open(&path, "split");
    terminal.settled_grid();
    terminal.keys(b"w");
    for light in [false, true] {
        if light {
            terminal.keys(b"t");
        }
        for (width, gap_row) in [(80, 7), (160, 6), (81, 7)] {
            terminal.resize(24, width);
            terminal.settled_grid();
            let s = terminal.parser.screen();
            let pane = (width - 1) / 2;
            let gap = gap_color(s);
            let context = if light {
                vt100::Color::Rgb(251, 251, 251)
            } else {
                vt100::Color::Rgb(27, 27, 27)
            };
            assert_eq!(cells(s, 4, 1, 5), "  2  ");
            assert_eq!(cells(s, 4, pane + 2, 5), "  2  ");
            for col in 0..width {
                if col != pane {
                    assert_eq!(s.cell(4, col).unwrap().bgcolor(), context);
                }
            }
            assert!(cells(s, gap_row, 0, pane).trim().is_empty());
            for col in 0..pane {
                assert_eq!(s.cell(gap_row, col).unwrap().bgcolor(), gap);
            }
            assert_eq!(cells(s, gap_row, pane + 1, 6), "    ↪ ");
            assert_eq!(cells(s, gap_row + 1, 1, 10), "  4  after");
            assert_eq!(cells(s, gap_row + 1, pane + 2, 10), "  4  after");
        }
        terminal.keys(b"uw");
        terminal.settled_grid();
        let s = terminal.parser.screen();
        let (height, width) = s.size();
        for row in 0..height {
            for col in 0..width {
                assert_ne!(s.cell(row, col).unwrap().bgcolor(), gap_color(s));
            }
        }
        terminal.keys(b"uw");
    }
    terminal.finish(b"q", 0);
}

fn assert_example(s: &vt100::Screen, example: &Example<'_>, split: bool) {
    let (height, width) = s.size();
    assert!(cells(s, 1, 0, width).contains("old.rs → new.rs"));
    assert!(
        cells(s, 2, 0, width).trim_start().starts_with("@@ "),
        "missing hunk header:\n{}",
        s.contents()
    );
    let count = if split {
        let left = (width - 1) / 2;
        for (i, &(a, b)) in example.split.iter().enumerate() {
            let y = i as u16 + 3;
            assert_line(s, y, 0, left, a.map(|n| example.lines[n]), true, true);
            assert_eq!(s.cell(y, left).unwrap().contents(), "│");
            assert_line(
                s,
                y,
                left + 1,
                width - left - 1,
                b.map(|n| example.lines[n]),
                true,
                false,
            );
        }
        example.split.len()
    } else {
        for (i, &line) in example.lines.iter().enumerate() {
            assert_line(s, i as u16 + 3, 0, width, Some(line), false, false);
        }
        example.lines.len()
    };
    for y in count as u16 + 3..height - 1 {
        assert!(
            cells(s, y, 0, width).trim().is_empty(),
            "unexpected extra content at row {y}"
        );
    }
}

#[test]
fn all_small_source_rows_and_exact_inline_spans_match_explicit_answers() {
    for example in examples() {
        eprintln!("contract: {}", example.name);
        for (old, source) in [(true, example.old), (false, example.new)] {
            let lines: Vec<_> = example
                .lines
                .iter()
                .filter(|l| {
                    if old {
                        l.old.is_some()
                    } else {
                        l.new.is_some()
                    }
                })
                .collect();
            assert_eq!(
                lines
                    .iter()
                    .map(|l| format!("{}\n", l.text))
                    .collect::<String>(),
                source,
                "incorrect independent example"
            );
            assert_eq!(
                lines
                    .iter()
                    .map(|l| if old { l.old.unwrap() } else { l.new.unwrap() })
                    .collect::<Vec<_>>(),
                (1..=lines.len() as u32).collect::<Vec<_>>()
            );
        }
        let (_dir, path) = patch(example.old.as_bytes(), example.new.as_bytes(), "rs");
        let mut terminal = open(&path, "split");
        for (width, key, split) in [
            (160, b"".as_slice(), true),
            (80, b"", true),
            (80, b"u", false),
            (119, b"tw", false),
            (120, b"u", true),
            (160, b"tw", true),
        ] {
            terminal.resize(50, width);
            terminal.keys(key);
            terminal.settled_grid();
            assert_example(terminal.parser.screen(), &example, split);
        }
        terminal.capture(&format!("contract-{}", example.name));
        terminal.finish(b"q", 0);
    }
}

#[test]
fn wide_and_combining_characters_keep_exact_cells_at_wrap_and_clip_boundaries() {
    const PREFIX: &str = "012345678901234567890123456789012";
    let extent = "0123456789".repeat(8);
    let old = format!("before\nafter\n{extent}\n");
    let new = format!("before\n{PREFIX}日本e\u{301}\tEND\nafter\n{extent}\n");
    let (_dir, path) = patch(old.as_bytes(), new.as_bytes(), "txt");
    let mut terminal = open(&path, "split");
    terminal.resize(24, 80);
    terminal.settled_grid();
    assert_eq!(
        cells(terminal.parser.screen(), 4, 46, 34),
        format!("{PREFIX} ")
    );
    terminal.keys(b"w");
    terminal.settled_grid();
    let s = terminal.parser.screen();
    assert_eq!(cells(s, 4, 40, 6), "   2+ ");
    assert_eq!(cells(s, 5, 40, 6), "    ↪ ");
    assert_eq!(s.cell(5, 46).unwrap().contents(), "日");
    assert!(s.cell(5, 47).unwrap().is_wide_continuation());
    assert_eq!(s.cell(5, 48).unwrap().contents(), "本");
    assert!(s.cell(5, 49).unwrap().is_wide_continuation());
    assert_eq!(s.cell(5, 50).unwrap().contents(), "e\u{301}");
    assert_eq!(cells(s, 5, 51, 5), "  END");
    assert_eq!(cells(s, 6, 40, 11), "   3  after");
    for y in [4, 5] {
        assert!(
            cells(s, y, 0, 39).trim().is_empty(),
            "opposite pane has a false continuation"
        );
    }
    terminal.resize(24, 82);
    terminal.settled_grid();
    let s = terminal.parser.screen();
    assert_eq!(s.cell(4, 80).unwrap().contents(), "日");
    assert_eq!(s.cell(5, 47).unwrap().contents(), "本");
    assert_eq!(s.cell(5, 49).unwrap().contents(), "e\u{301}");
    assert_eq!(cells(s, 5, 50, 5), "  END");
    terminal.resize(24, 80);
    terminal.keys(b"w\x1b[C");
    terminal.settled_grid();
    assert_eq!(
        cells(terminal.parser.screen(), 4, 46, 34),
        format!("{}日本e\u{301}", &PREFIX[4..])
    );
    terminal.keys(&b"\x1b[C".repeat(8));
    terminal.settled_grid();
    assert_eq!(
        cells(terminal.parser.screen(), 4, 46, 8).trim_end(),
        " e\u{301}  END"
    );
    terminal.keys(&b"\x1b[D".repeat(9));
    terminal.keys(b"u");
    terminal.settled_grid();
    assert_eq!(cells(terminal.parser.screen(), 4, 0, 10), "       2+ ");
    assert!(
        cells(terminal.parser.screen(), 4, 10, 70)
            .starts_with(&format!("{PREFIX}日本e\u{301}  END"))
    );
    terminal.capture("contract-unicode");
    terminal.finish(b"q", 0);
}

#[test]
fn unsafe_text_and_no_newline_markers_are_literal_and_side_specific() {
    let old = b"first\r\nlast\x1b[2J\xff";
    let new = b"first\r\nlast\x1b]52;c;payload\x07\xfe";
    let (_dir, path) = patch(old, new, "txt");
    let mut terminal = open(&path, "split");
    terminal.settled_grid();
    let s = terminal.parser.screen();
    assert_eq!(cells(s, 3, 6, 20).trim_end(), "first");
    assert_eq!(cells(s, 4, 6, 50).trim_end(), "last\\u{1b}[2J\\xff");
    assert_eq!(
        cells(s, 4, 86, 60).trim_end(),
        "last\\u{1b}]52;c;payload\\u{7}\\xfe"
    );
    assert_eq!(cells(s, 5, 0, 79).trim(), "No newline at end of file");
    assert_eq!(cells(s, 5, 80, 80).trim(), "No newline at end of file");
    assert!(
        !terminal.raw.windows(5).any(|w| w == b"\x1b]52;"),
        "source invoked clipboard escape"
    );
    terminal.keys(b"u");
    terminal.settled_grid();
    let s = terminal.parser.screen();
    assert_eq!(cells(s, 4, 0, 10), "   2    − ");
    assert!(cells(s, 5, 0, 160).contains("No newline at end of file"));
    assert_eq!(cells(s, 6, 0, 10), "       2+ ");
    assert!(cells(s, 7, 0, 160).contains("No newline at end of file"));
    terminal.finish(b"q", 0);
}

#[test]
fn completed_updates_never_mix_source_rows_during_burst_navigation() {
    let old: String = (1..=240)
        .map(|n| format!("let old_{n:03} = {n};\n"))
        .collect();
    let new: String = (1..=240)
        .map(|n| format!("let new_{n:03} = {n};\n"))
        .collect();
    let (_dir, path) = patch(old.as_bytes(), new.as_bytes(), "rs");
    let mut terminal = open(&path, "split");
    terminal.settled_grid();
    terminal.frames = Some(Vec::new());
    terminal.keys(b"GGguu\x1b[6~\x1b[5~\x1b[<65;100;15M\x1b[<64;100;15MGgu");
    terminal.settled_grid();
    let frames = terminal.frames.take().unwrap();
    assert!(
        frames.len() >= 8,
        "burst did not exercise intermediate frames"
    );
    for (frame, s) in frames.iter().enumerate() {
        let split = cells(s, 0, 0, 160).contains("SPLIT");
        let mut previous = [None, None];
        let mut saw_addition = false;
        for y in 2..49 {
            for (side, x, width) in if split {
                vec![(0, 0, 79), (1, 80, 80)]
            } else {
                vec![(2, 0, 160)]
            } {
                let row = cells(s, y, x, width);
                if row.trim().is_empty() || row.trim_start().starts_with("@@") {
                    continue;
                }
                let (old, n) = if split {
                    (
                        side == 0,
                        cells(s, y, x + 1, 3)
                            .trim()
                            .parse::<u32>()
                            .unwrap_or_else(|_| panic!("frame {frame}, invalid gutter: {row:?}")),
                    )
                } else if cells(s, y, x + 1, 3).trim().is_empty() {
                    (false, cells(s, y, x + 5, 3).trim().parse::<u32>().unwrap())
                } else {
                    (true, cells(s, y, x + 1, 3).trim().parse::<u32>().unwrap())
                };
                let text = format!("let {}_{n:03} = {n};", if old { "old" } else { "new" });
                let index = usize::from(!old);
                if !split {
                    assert!(
                        !old || !saw_addition,
                        "frame {frame}: deletion after addition in one hunk"
                    );
                    saw_addition |= !old;
                }
                if let Some(p) = previous[index] {
                    assert_eq!(n, p + 1, "frame {frame}: reordered or missing source");
                }
                previous[index] = Some(n);
                let line = if old {
                    removed(n, &text, &[(4, 11)])
                } else {
                    added(n, &text, &[(4, 11)])
                };
                // Plain rows are valid until the worker publishes their styles.
                let base = s.cell(y, x + width - 1).unwrap().bgcolor();
                let word = s
                    .cell(y, x + if split { 10 } else { 14 })
                    .unwrap()
                    .bgcolor();
                assert_line(
                    s,
                    y,
                    x,
                    width,
                    Some(if word == base {
                        Line {
                            changed: &[],
                            ..line
                        }
                    } else {
                        line
                    }),
                    split,
                    old,
                );
            }
        }
    }
    assert!(has_active_source(
        terminal.parser.screen(),
        "  1    − ",
        "let old_001 = 1;"
    ));
    terminal.finish(b"q", 0);
}

#[test]
fn shared_text_uses_its_own_old_and_new_syntax_context() {
    for language in ["c", "rs", "ts", "py"] {
        let literal = if language == "py" {
            "value = \"\"\"\nreturn 1\n\"\"\"\n"
        } else {
            "/*\nreturn 1;\n*/\n"
        };
        let code = if language == "py" {
            "return 1\n"
        } else {
            "return 1;\n"
        };
        for old_literal in [true, false] {
            let (old, new) = if old_literal {
                (literal, code)
            } else {
                (code, literal)
            };
            let (_dir, path) = patch(old.as_bytes(), new.as_bytes(), language);
            let mut terminal = open(&path, "split");
            for (keys, light) in [(b"".as_slice(), false), (b"t", true), (b"tuutt", false)] {
                terminal.keys(keys);
                terminal.resize(24, 80);
                terminal.settled_grid();
                terminal.resize(50, 160);
                terminal.settled_grid();
                terminal.capture(&format!("contract-syntax-{language}-{old_literal}-{light}"));
                let s = terminal.parser.screen();
                let literal_color = if language != "py" {
                    vt100::Color::Rgb(115, 115, 115)
                } else if light {
                    vt100::Color::Rgb(25, 159, 67)
                } else {
                    vt100::Color::Rgb(94, 204, 113)
                };
                let keyword = if light {
                    vt100::Color::Rgb(211, 42, 97)
                } else {
                    vt100::Color::Rgb(255, 103, 141)
                };
                for (old, x) in [(true, 0), (false, 80)] {
                    let in_literal = old == old_literal;
                    assert_eq!(
                        cells(s, 4, x + 6, code.trim_end().len() as u16),
                        code.trim_end()
                    );
                    assert_eq!(
                        cells(s, 4, x + 1, 3),
                        if in_literal { "  2" } else { "  1" }
                    );
                    for col in 6..12 {
                        assert_eq!(
                            s.cell(4, x + col).unwrap().fgcolor(),
                            if in_literal { literal_color } else { keyword },
                            "{language}, old={old}, light={light}: wrong source-side syntax"
                        );
                    }
                }
            }
            terminal.keys(b"u");
            terminal.settled_grid();
            let expected = if old_literal {
                vt100::Color::Rgb(255, 103, 141)
            } else if language == "py" {
                vt100::Color::Rgb(94, 204, 113)
            } else {
                vt100::Color::Rgb(115, 115, 115)
            };
            assert_eq!(
                terminal.parser.screen().cell(4, 10).unwrap().fgcolor(),
                expected,
                "unified context must use new-side syntax"
            );
            terminal.finish(b"q", 0);
        }
    }
}

#[test]
fn inline_emphasis_follows_words_across_unequal_pane_wraps() {
    let prefix = "x".repeat(31);
    let old = format!("{prefix} OLD-14 END\n");
    let new = format!("{prefix} NEW-14 END\n");
    let (_dir, path) = patch(old.as_bytes(), new.as_bytes(), "txt");
    for light in [false, true] {
        let mut terminal = open(&path, "split");
        terminal.resize(24, 80);
        terminal.settled_grid();
        terminal.keys(if light { b"tw" } else { b"w" });
        terminal.settled_grid();
        let s = terminal.parser.screen();
        assert!(cells(s, 0, 0, 80).contains(if light { "light" } else { "dark" }));
        assert_eq!(cells(s, 3, 6, 33), format!("{prefix} O"));
        assert_eq!(cells(s, 4, 6, 9), "LD-14 END");
        assert_eq!(cells(s, 3, 46, 34), format!("{prefix} NE"));
        assert_eq!(cells(s, 4, 46, 8), "W-14 END");
        for (x, width, changed, end) in [(0, 39, 2, 9), (40, 40, 1, 8)] {
            let base = s.cell(4, x + width - 1).unwrap().bgcolor();
            for column in 0..end {
                assert_eq!(
                    s.cell(4, x + 6 + column).unwrap().bgcolor() != base,
                    column < changed
                );
            }
            assert_eq!(cells(s, 4, x, 6), "    ↪ ");
        }
        terminal.capture(if light {
            "inline-wrap-light"
        } else {
            "inline-wrap-dark"
        });
        terminal.resize(24, 81);
        terminal.settled_grid();
        let s = terminal.parser.screen();
        assert_eq!(cells(s, 3, 6, 34), format!("{prefix} OL"));
        assert_eq!(cells(s, 4, 6, 8), "D-14 END");
        assert_eq!(cells(s, 4, 47, 8), "W-14 END");
        for x in [0, 41] {
            let base = s.cell(4, x + 39).unwrap().bgcolor();
            for column in 0..8 {
                assert_eq!(
                    s.cell(4, x + 6 + column).unwrap().bgcolor() != base,
                    column == 0
                );
            }
        }
        terminal.resize(24, 80);
        terminal.keys(b"u");
        terminal.settled_grid();
        let s = terminal.parser.screen();
        assert_eq!(cells(s, 3, 10, 42), format!("{prefix} OLD-14 END"));
        assert_eq!(cells(s, 4, 10, 42), format!("{prefix} NEW-14 END"));
        for y in [3, 4] {
            let base = s.cell(y, 79).unwrap().bgcolor();
            for column in 10..52 {
                assert_eq!(
                    s.cell(y, column).unwrap().bgcolor() != base,
                    (42..45).contains(&column)
                );
            }
        }
        terminal.finish(b"q", 0);
    }
}

#[test]
fn replacement_sides_share_one_inline_alignment_through_reflow() {
    for (old, new, shared) in [
        (
            "import type { GitStatusEntry } from '../src/types';",
            "import { flushDom, installDom } from './helpers/dom';",
            "import    } from './/';",
        ),
        (
            "let value = left + right;",
            "let value = right + left;",
            "let value =  + ;",
        ),
    ] {
        let (_dir, path) = patch(
            format!("{old}\n").as_bytes(),
            format!("{new}\n").as_bytes(),
            "ts",
        );
        let mut terminal = open(&path, "split");
        for light in [false, true] {
            if light {
                terminal.keys(b"t");
            }
            for (width, keys, split, wrapped) in [
                (160, b"".as_slice(), true, false),
                (80, b"w", true, true),
                (81, b"", true, true),
                (80, b"u", false, true),
                (160, b"uw", true, false),
            ] {
                terminal.resize(24, width);
                terminal.keys(keys);
                terminal.settled_grid();
                let s = terminal.parser.screen();
                let gutter = if split { 6 } else { 10 };
                let left = (width - 1) / 2;
                for (is_new, text) in [(false, old), (true, new)] {
                    let (x, pane_width, row) = if split {
                        if is_new {
                            (left + 1, width - left - 1, 3)
                        } else {
                            (0, left, 3)
                        }
                    } else {
                        (0, width, if is_new { 4 } else { 3 })
                    };
                    let content_width = pane_width - gutter;
                    assert!(wrapped || text.len() <= content_width as usize);
                    let base = s.cell(row, x).unwrap().bgcolor();
                    let mut kept = String::new();
                    for (i, expected) in text.chars().enumerate() {
                        let y = row + i as u16 / content_width;
                        let col = x + gutter + i as u16 % content_width;
                        let cell = s.cell(y, col).unwrap();
                        assert_eq!(cell.contents(), expected.to_string(), "source changed");
                        if cell.bgcolor() == base {
                            kept.push(expected);
                        }
                    }
                    assert_eq!(
                        kept, shared,
                        "inconsistent common text: new={is_new}, split={split}, light={light}, width={width}"
                    );
                }
            }
        }
        terminal.finish(b"q", 0);
    }
}

#[test]
fn inline_spans_keep_unicode_and_tab_cell_offsets_in_both_layouts_and_themes() {
    let (_dir, path) = patch(
        "日本: cafe\u{301}-old\tEND\n".as_bytes(),
        "日本: cafe\u{301}-new\tEND\n".as_bytes(),
        "txt",
    );
    let mut terminal = open(&path, "split");
    terminal.resize(24, 80);
    for (keys, split) in [
        (b"".as_slice(), true),
        (b"t", true),
        (b"u", false),
        (b"t", false),
    ] {
        terminal.keys(keys);
        terminal.settled_grid();
        for (new, word) in [(false, "old"), (true, "new")] {
            let (y, start, end) = if split {
                (3, if new { 46 } else { 6 }, if new { 79 } else { 38 })
            } else {
                (if new { 4 } else { 3 }, 10, 79)
            };
            let s = terminal.parser.screen();
            assert_eq!(
                cells(s, y, start, 19),
                format!("日本: cafe\u{301}-{word}  END")
            );
            assert_eq!(s.cell(y, start + 9).unwrap().contents(), "e\u{301}");
            let base = s.cell(y, end).unwrap().bgcolor();
            for column in 0..19 {
                let cell = s.cell(y, start + column).unwrap();
                assert_eq!(cell.is_wide_continuation(), matches!(column, 1 | 3));
                if cell.is_wide_continuation() {
                    continue;
                }
                assert_eq!(
                    cell.bgcolor() != base,
                    (11..14).contains(&column),
                    "incorrect Unicode/tab emphasis at {column}, new={new}, split={split}"
                );
            }
        }
    }
    terminal.finish(b"q", 0);
}

#[test]
fn position_fixture_leaves_unchanged_numeric_suffixes_unemphasized() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/position/review.patch");
    let mut terminal = open(&path, "split");
    terminal.settled_grid();
    terminal.keys(b"w");
    for keys in [b"".as_slice(), b"t", b"u", b"t"] {
        terminal.keys(keys);
        terminal.settled_grid();
        let s = terminal.parser.screen();
        let (height, width) = s.size();
        for token in ["old-14", "new-14"] {
            let (row, col) = (0..height)
                .flat_map(|row| (0..width - 6).map(move |col| (row, col)))
                .find(|&(row, col)| cells(s, row, col, 6) == token)
                .unwrap_or_else(|| panic!("missing {token}:\n{}", s.contents()));
            let base = s.cell(row, col + 6).unwrap().bgcolor();
            for offset in 0..6 {
                assert_eq!(
                    s.cell(row, col + offset).unwrap().bgcolor() != base,
                    offset < 3,
                    "incorrect emphasis at {token}:{offset}"
                );
            }
        }
    }
    terminal.finish(b"q", 0);
}

#[test]
fn json_strings_and_booleans_retain_their_token_roles() {
    let (_dir, path) = patch(
        b"{\n  \"value\": \"old\",\n  \"enabled\": false\n}\n",
        b"{\n  \"value\": \"new\",\n  \"enabled\": true\n}\n",
        "json",
    );
    let mut terminal = open(&path, "split");
    for (key, string, boolean) in [
        (
            b"".as_slice(),
            vt100::Color::Rgb(94, 204, 113),
            vt100::Color::Rgb(104, 205, 242),
        ),
        (
            b"t",
            vt100::Color::Rgb(25, 159, 67),
            vt100::Color::Rgb(28, 161, 199),
        ),
    ] {
        terminal.keys(key);
        terminal.settled_grid();
        let s = terminal.parser.screen();
        for x in [0, 80] {
            for column in 18..21 {
                assert_eq!(s.cell(4, x + column).unwrap().fgcolor(), string);
            }
            for column in 19..23 {
                assert_eq!(s.cell(5, x + column).unwrap().fgcolor(), boolean);
            }
        }
    }
    terminal.finish(b"q", 0);
}

#[test]
fn hunk_gaps_and_metadata_do_not_fabricate_source_rows() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("metadata.patch");
    fs::write(&path, concat!(
        "diff --git a/text.txt b/text.txt\n--- a/text.txt\n+++ b/text.txt\n@@ -1,2 +1,2 @@\n-old\n+new\n same\n@@ -20 +20 @@\n-before\n+after\n",
        "diff --git a/old-name.txt b/new-name.txt\nsimilarity index 100%\nrename from old-name.txt\nrename to new-name.txt\n",
        "diff --git a/run.sh b/run.sh\nold mode 100644\nnew mode 100755\n",
        "diff --git a/image.bin b/image.bin\nindex 1111111..2222222 100644\nBinary files a/image.bin and b/image.bin differ\n",
    )).unwrap();
    let mut terminal = open(&path, "split");
    terminal.settled_grid();
    let s = terminal.parser.screen();
    assert_eq!(cells(s, 3, 0, 9), "   1− old");
    assert_eq!(cells(s, 3, 80, 9), "   1+ new");
    assert_eq!(cells(s, 4, 0, 10), "   2  same");
    assert!(cells(s, 5, 0, 160).contains("Context not included in patch"));
    assert!(cells(s, 6, 0, 160).contains("@@ -20 +20 @@"));
    assert_eq!(cells(s, 7, 0, 12), "  20− before");
    assert_eq!(cells(s, 7, 80, 11), "  20+ after");
    for (header, description) in [
        ("old-name.txt → new-name.txt", "Renamed"),
        ("run.sh", "mode 100644 → 100755"),
        ("image.bin", "Binary change"),
    ] {
        terminal.keys(b"n");
        terminal.settled_grid();
        let s = terminal.parser.screen();
        assert!(cells(s, 1, 0, 160).contains(header));
        assert!(
            cells(s, 2, 0, 160).contains(description),
            "expected metadata {description:?}:\n{}",
            s.contents()
        );
        assert!(cells(s, 2, 0, 160).contains("no textual hunks"));
    }
    terminal.keys(b"NNNu");
    terminal.settled_grid();
    let s = terminal.parser.screen();
    assert_eq!(
        cells(s, 3, 0, 13).chars().skip(1).collect::<String>(),
        "  1    − old"
    );
    assert_eq!(cells(s, 4, 0, 13), "       1+ new");
    assert_eq!(cells(s, 5, 0, 14), "   2   2  same");
    assert!(cells(s, 6, 0, 160).contains("Context not included in patch"));
    assert!(cells(s, 7, 0, 160).contains("@@ -20 +20 @@"));
    assert_eq!(cells(s, 8, 0, 16), "  20    − before");
    assert_eq!(cells(s, 9, 0, 15), "      20+ after");
    terminal.finish(b"q", 0);
}
