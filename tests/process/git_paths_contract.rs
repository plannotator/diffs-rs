use super::*;
use std::path::{Path, PathBuf};

struct Case {
    _directory: tempfile::TempDir,
    patch: PathBuf,
    old: Vec<u8>,
    new: Vec<u8>,
}

fn git(directory: &Path, quoted: bool, args: &[&str]) -> Vec<u8> {
    let mut command = Command::new("git");
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    let output = command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("LC_ALL", "C")
        .args(["-c", "core.autocrlf=false", "-c", "diff.algorithm=myers"])
        .args([
            "-c",
            if quoted {
                "core.quotePath=true"
            } else {
                "core.quotePath=false"
            },
        ])
        .args(args)
        .current_dir(directory)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn generate(old_name: &str, new_name: &str, copy: bool, edited: bool, quoted: bool) -> Case {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    git(root, quoted, &["init", "--quiet"]);
    let mut old = (0..20)
        .map(|n| format!("const keep_{n} = {n};\n"))
        .collect::<String>();
    old.push_str("old_tail");
    let new = if edited {
        old.replace("keep_10 = 10", "keep_10 = 11")
            .replace("old_tail", "new_tail")
    } else {
        old.clone()
    };
    fs::write(root.join(old_name), &old).unwrap();
    git(root, quoted, &["add", "--", "."]);
    let tree = String::from_utf8(git(root, quoted, &["write-tree"])).unwrap();
    if copy {
        fs::copy(root.join(old_name), root.join(new_name)).unwrap();
    } else {
        fs::rename(root.join(old_name), root.join(new_name)).unwrap();
    }
    fs::write(root.join(new_name), &new).unwrap();
    git(root, quoted, &["add", "--all"]);
    let bytes = git(
        root,
        quoted,
        &[
            "diff",
            "--cached",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--full-index",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            "--unified=1000",
            "--find-renames=50%",
            "--find-copies=50%",
            "--find-copies-harder",
            tree.trim(),
            "--",
        ],
    );
    let expected_header = if copy { "copy from " } else { "rename from " };
    assert!(
        String::from_utf8_lossy(&bytes)
            .lines()
            .any(|line| line.starts_with(expected_header)),
        "Git did not produce the required change: {}",
        String::from_utf8_lossy(&bytes)
    );
    let patch = root.join("input.patch");
    fs::write(&patch, bytes).unwrap();
    Case {
        _directory: directory,
        patch,
        old: old.into_bytes(),
        new: new.into_bytes(),
    }
}

fn check_model(
    model: &Value,
    old_name: &str,
    new_name: &str,
    copy: bool,
    edited: bool,
    case: &Case,
) {
    let files = model["files"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    let file = &files[0];
    assert_eq!(file["old_path"], old_name.replace('\t', "\\t"));
    assert_eq!(file["path"], new_name.replace('\t', "\\t"));
    assert_eq!(file["status"], if copy { "Copied" } else { "Renamed" });
    assert_eq!(file["old_mode"], if edited { "100644" } else { "0" });
    assert_eq!(file["new_mode"], if edited { "100644" } else { "0" });
    let lines = model["lines"].as_array().unwrap();
    if !edited {
        assert!(lines.is_empty());
        assert_eq!(file["additions"], 0);
        assert_eq!(file["deletions"], 0);
        assert!(file["hunks"].as_array().unwrap().is_empty());
        return;
    }
    assert_eq!(file["additions"], 2);
    assert_eq!(file["deletions"], 2);
    assert_eq!(file["hunks"].as_array().unwrap().len(), 1);
    for (side, expected) in [("old", &case.old), ("new", &case.new)] {
        let source: Vec<_> = lines.iter().filter(|line| !line[side].is_null()).collect();
        let bytes: Vec<u8> = source
            .iter()
            .flat_map(|line| line["bytes"].as_array().unwrap())
            .map(|value| u8::try_from(value.as_u64().unwrap()).unwrap())
            .collect();
        assert_eq!(&bytes, expected, "{side} bytes changed");
        for (index, line) in source.iter().enumerate() {
            assert_eq!(line[side], index + 1);
            assert_eq!(line["no_newline"], index + 1 == source.len());
            assert!(
                matches!(line["kind"].as_str(), Some("Context"))
                    || line["kind"]
                        == if side == "old" {
                            "Deletion"
                        } else {
                            "Addition"
                        }
            );
        }
    }
}

#[test]
fn git_rename_and_copy_names_and_source_bytes_survive() {
    let capture = std::env::var_os("NATIVE_DIFF_GIT_PATH_CASE_DIR").map(PathBuf::from);
    if let Some(root) = &capture {
        fs::create_dir(root).unwrap();
    }
    let names = [
        ("ordinary", "old.txt", "new.txt"),
        ("interior", "old name.txt", "new name.txt"),
        ("leading", " old.txt", " new.txt"),
        ("trailing", "old.txt ", "new.txt "),
        ("quote", "old\"name.txt", "new\"name.txt"),
        ("tab", "old\tname.txt", "new\tname.txt"),
        ("unicode", "old-日本語.txt", "new-日本語.txt"),
    ];
    let mut rejected = Vec::new();
    for (label, old_name, new_name) in names {
        for copy in [false, true] {
            for edited in [false, true] {
                for quoted in [false, true] {
                    let id = format!("{label}-copy-{copy}-edited-{edited}-quoted-{quoted}");
                    let case = generate(old_name, new_name, copy, edited, quoted);
                    if let Some(root) = &capture {
                        let folder = root.join(&id);
                        fs::create_dir(&folder).unwrap();
                        fs::copy(&case.patch, folder.join("input.patch")).unwrap();
                    }
                    let output = Command::new(viewer_binary())
                        .arg("inspect")
                        .arg(&case.patch)
                        .output()
                        .unwrap();
                    if !output.status.success() {
                        rejected.push(format!("{id}: {}", String::from_utf8_lossy(&output.stderr)));
                        continue;
                    }
                    let model: Value = serde_json::from_slice(&output.stdout).unwrap();
                    check_model(&model, old_name, new_name, copy, edited, &case);
                    check_stored_source(&case, &model);
                    let summary = Command::new(viewer_binary())
                        .args(["inspect", "--summary"])
                        .arg(&case.patch)
                        .output()
                        .unwrap();
                    assert!(
                        summary.status.success(),
                        "{id}: {}",
                        String::from_utf8_lossy(&summary.stderr)
                    );
                    let summary: Value = serde_json::from_slice(&summary.stdout).unwrap();
                    assert_eq!(summary["files"], 1);
                    assert_eq!(
                        summary["source_lines"],
                        model["lines"].as_array().unwrap().len()
                    );
                }
            }
        }
    }
    assert!(rejected.is_empty(), "{}", rejected.join("\n"));
}

#[test]
fn malformed_rename_and_copy_headers_remain_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid.patch");
    for operation in ["rename", "copy"] {
        for tail in [
            "",
            "\"unterminated",
            "\"old.txt\" trailing",
            "\"old.txt\" \"extra\"",
            "\"\"",
            "\"bad\\q\"",
        ] {
            let patch = format!(
                "diff --git a/old.txt b/new.txt\nsimilarity index 100%\n{operation} from {tail}\n{operation} to new.txt\n"
            );
            fs::write(&path, patch).unwrap();
            let output = Command::new(viewer_binary())
                .arg("inspect")
                .arg(&path)
                .output()
                .unwrap();
            assert!(
                !output.status.success(),
                "accepted malformed {operation} path {tail:?}"
            );
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("Cannot parse this patch"));
        }
    }
}

#[test]
fn renamed_and_copied_paths_remain_visible_in_both_layouts() {
    let first = generate("old name.txt", "new name.txt", false, true, true);
    let second = generate(" source.txt ", " target.txt ", true, false, true);
    let mut patch = fs::read(&first.patch).unwrap();
    patch.extend(fs::read(&second.patch).unwrap());
    fs::write(&first.patch, patch).unwrap();
    let mut terminal = Session::open(&first.patch);
    terminal.wait(|screen| {
        screen.contents().contains("new name.txt") && screen.contents().contains("new_tail")
    });
    for layout in ["SPLIT", "UNIFIED"] {
        terminal.wait(|screen| screen.contents().contains(layout));
        terminal.settled_grid();
        let screen = terminal.parser.screen().contents();
        assert!(screen.contains("old name.txt → new name.txt"), "{screen}");
        assert!(screen.contains("Renamed   +2 −2"), "{screen}");
        assert!(
            screen.contains("old_tail") && screen.contains("new_tail"),
            "{screen}"
        );
        terminal.keys(b"n");
        terminal.wait(|screen| header(screen).starts_with("  source.txt  →  target.txt "));
        terminal.settled_grid();
        assert!(terminal.parser.screen().contents().contains("Copied"));
        terminal.keys(b"N");
        terminal.wait(|screen| header(screen).starts_with(" old name.txt → new name.txt"));
        terminal.keys(b"u");
    }
    terminal.finish(b"q", 0);
}

fn header(screen: &vt100::Screen) -> String {
    (0..screen.size().1)
        .map(|x| screen.cell(1, x).unwrap().contents())
        .collect()
}

fn check_stored_source(case: &Case, checked_model: &Value) {
    let mut expected = b"F".to_vec();
    expected.extend(0_u64.to_le_bytes());
    for hunk in checked_model["files"][0]["hunks"].as_array().unwrap() {
        expected.push(b'H');
        expected.extend(hunk["old_start"].as_u64().unwrap().to_le_bytes());
        expected.extend(hunk["new_start"].as_u64().unwrap().to_le_bytes());
        let start = hunk["lines"]["start"].as_u64().unwrap() as usize;
        let end = hunk["lines"]["end"].as_u64().unwrap() as usize;
        for line in &checked_model["lines"].as_array().unwrap()[start..end] {
            let sign = match line["kind"].as_str().unwrap() {
                "Context" => b' ',
                "Deletion" => b'-',
                "Addition" => b'+',
                other => panic!("Unexpected line kind: {other}"),
            };
            expected.extend([b'L', sign]);
            for side in ["old", "new"] {
                expected.extend(line[side].as_u64().unwrap_or(0).to_le_bytes());
            }
            expected.push(u8::from(line["no_newline"].as_bool().unwrap()));
            let bytes = line["bytes"].as_array().unwrap();
            expected.extend((bytes.len() as u64).to_le_bytes());
            expected.extend(
                bytes
                    .iter()
                    .map(|v| u8::try_from(v.as_u64().unwrap()).unwrap()),
            );
        }
    }
    let output = Command::new(viewer_binary())
        .args(["inspect", "--canonical"])
        .arg(&case.patch)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout, expected,
        "Retained source export differs from the checked source"
    );
}
