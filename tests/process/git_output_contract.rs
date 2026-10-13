use super::*;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/git-output")
        .join(name)
}

fn canonical(args: &[&std::ffi::OsStr], stdin: Option<&Path>) -> Vec<u8> {
    let mut command = Command::new(viewer_binary());
    command.arg("inspect").arg("--canonical").args(args);
    if let Some(stdin) = stdin {
        command.stdin(fs::File::open(stdin).unwrap());
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
fn git_command_output_matches_its_file_patches_through_the_executable() {
    for name in ["show", "log", "format-patch", "diff-stat"] {
        let output = fixture(&format!("{name}.patch"));
        let units = fixture(&format!("{name}.units.patch"));
        let expected = canonical(&[units.as_os_str()], None);
        assert!(!expected.is_empty(), "{name}");
        assert_eq!(canonical(&[output.as_os_str()], None), expected, "{name}");
        // A pipe from `git log -p` takes the same path through standard input.
        assert_eq!(
            canonical(&["-".as_ref()], Some(&output)),
            expected,
            "{name} on stdin"
        );
    }
}

#[test]
fn text_without_a_git_file_header_remains_an_error() {
    let dir = tempfile::tempdir().unwrap();
    for (input, message) in [
        (b"just text\n".as_slice(), "no `diff --git` file header"),
        (
            b"--- a.txt\n+++ b.txt\n@@ -1 +1 @@\n-a\n+b\n".as_slice(),
            "without a `diff --git` line",
        ),
    ] {
        let path = dir.path().join("input.txt");
        fs::write(&path, input).unwrap();
        let output = Command::new(viewer_binary())
            .arg("inspect")
            .arg(&path)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains(message));
    }
}

#[test]
fn git_log_output_opens_in_the_terminal_with_every_file() {
    let mut terminal = Session::open(&fixture("log.patch"));
    terminal.wait(|screen| screen.contents().contains("12 files"));
    terminal.settled_grid();
    let screen = terminal.parser.screen().contents();
    assert!(screen.contains("echo bye"), "{screen}");
    assert!(!screen.contains("Author:"), "{screen}");
    for _ in 0..5 {
        terminal.keys(b"n");
    }
    terminal.wait(|screen| screen.contents().contains("notes.txt → docs.txt"));
    terminal.finish(b"q", 0);
}
