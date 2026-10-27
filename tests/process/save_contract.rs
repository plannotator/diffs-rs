use super::{search_contract::cells_for, *};

fn footer(screen: &vt100::Screen) -> String {
    screen.rows(0, screen.size().1).last().unwrap()
}

fn submit_path(terminal: &mut Session, path: &std::path::Path) {
    terminal.keys(b"\x13");
    terminal.wait(|screen| footer(screen).contains("/save:"));
    terminal.keys(format!("\x15\x1b[200~{}\x1b[201~\r", path.display()).as_bytes());
}

fn no_staging(dir: &std::path::Path) {
    assert!(fs::read_dir(dir).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".diffs-save-")
    }));
}

#[test]
fn save_exports_selected_original_bytes_after_wrapping_and_layout_changes() {
    let mut old = format!("//\t{}日本語 e\u{301}\r\n", "old word ".repeat(20)).into_bytes();
    old.extend_from_slice(b"// old second\r\n// last\xff");
    let new = b"// new first\n// new second\n// new last\n";
    let (dir, patch) = generated_patch(&old, new);
    let mut terminal = Session::open(&patch);
    terminal.settled_grid();
    terminal.keys(b"gVjjV");
    terminal.wait(|screen| footer(screen).contains("Selected old 1–3"));
    terminal.keys(b"w");
    terminal.wait(|screen| screen.contents().contains('↪'));
    terminal.keys(b"u");
    terminal.wait(|screen| screen.rows(0, 160).next().unwrap().contains("UNIFIED"));
    let saved = dir.path().join("q export 日本語.txt");
    submit_path(&mut terminal, &saved);
    terminal
        .wait(|screen| footer(screen).contains("Saved ") && footer(screen).contains(" bytes to "));
    assert_eq!(fs::read(&saved).unwrap(), old);
    assert!(footer(terminal.parser.screen()).contains("Selected old 1–3"));
    no_staging(dir.path());
    terminal.keys(b"\x1b");
    terminal.wait(|screen| footer(screen).contains("Selection cleared"));
    terminal.keys(b"G");
    terminal.wait(|screen| has_active_source(screen, "      3+ ", "// new last"));
    let one = dir.path().join("current-line");
    submit_path(&mut terminal, &one);
    terminal.wait(|screen| footer(screen).contains("Saved 12 bytes"));
    assert_eq!(fs::read(&one).unwrap(), b"// new last\n");
    terminal.finish(b"q", 0);
    no_staging(dir.path());
}

#[test]
fn save_rejects_metadata_and_existing_paths_and_keeps_prompt_input_inert() {
    let (dir, patch) = generated_patch(b"old\n", b"new\n");
    let mut terminal = Session::open(&patch);
    terminal.settled_grid();
    terminal.keys(b"\x13");
    terminal.wait(|screen| footer(screen).contains("not a header, gap or padding"));
    terminal.keys(b"gV");
    terminal.wait(|screen| footer(screen).contains("Selecting old 1–1"));
    let existing = dir.path().join("existing");
    fs::write(&existing, b"keep me").unwrap();
    let missing = dir.path().join("absent");
    let link = dir.path().join("existing-link");
    std::os::unix::fs::symlink(&missing, &link).unwrap();
    for path in [&existing, &link] {
        submit_path(&mut terminal, path);
        terminal.wait(|screen| footer(screen).contains("Save failed: Destination already exists"));
        assert!(footer(terminal.parser.screen()).contains("Selecting old 1–1"));
        no_staging(dir.path());
    }
    assert_eq!(fs::read(&existing).unwrap(), b"keep me");
    assert_eq!(fs::read_link(&link).unwrap(), missing);
    assert!(!missing.exists());
    terminal.keys(b"\x13\x15\x1b[200~q\nu\x1b[201~");
    terminal.wait(|screen| screen.contents().contains("paste was not inserted"));
    assert!(terminal.parser.screen().contents().contains("SPLIT"));
    terminal.keys(b"\x1b[200~q\0tail\x1b[201~");
    terminal.wait(|screen| screen.contents().contains("cannot contain NUL"));
    let canceled = dir.path().join("q canceled");
    terminal.keys(format!("\x15\x1b[200~{}\x1b[201~", canceled.display()).as_bytes());
    terminal.wait(|screen| footer(screen).contains("q canceled"));
    terminal.resize(18, 80);
    terminal
        .wait(|screen| footer(screen).contains("/save:") && footer(screen).contains("q canceled"));
    terminal.keys(b"\x1b");
    terminal.wait(|screen| !footer(screen).contains("/save:"));
    assert!(!canceled.exists());
    let saved = dir.path().join("recovered");
    submit_path(&mut terminal, &saved);
    terminal.wait(|screen| footer(screen).contains("Saved 4 bytes"));
    assert_eq!(fs::read(&saved).unwrap(), b"old\n");
    terminal.finish(b"q", 0);
    no_staging(dir.path());
}

#[test]
fn save_refuses_missing_patch_context_and_recovers_without_publishing_a_partial_range() {
    let dir = tempfile::tempdir().unwrap();
    let patch = dir.path().join("review.patch");
    fs::write(&patch, b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n@@ -9 +9 @@\n-last\n+later\n").unwrap();
    let mut terminal = Session::open(&patch);
    terminal.settled_grid();
    terminal.keys(b"g");
    terminal.wait(|screen| has_active_source(screen, "  1− ", "old"));
    let (y, x) = cells_for(terminal.parser.screen(), "last", true)[0];
    terminal.keys(format!("\x1b[<4;{};{}M\x1b[<4;{};{}m", x + 1, y + 1, x + 1, y + 1).as_bytes());
    terminal.wait(|screen| footer(screen).contains("Selected old 1–9"));
    let saved = dir.path().join("no-gaps");
    submit_path(&mut terminal, &saved);
    terminal.wait(|screen| footer(screen).contains("Save failed:"));
    assert!(!saved.exists());
    no_staging(dir.path());
    assert!(footer(terminal.parser.screen()).contains("Selected old 1–9"));
    terminal.keys(b"\x1b");
    terminal.wait(|screen| footer(screen).contains("Selection cleared"));
    terminal.keys(b"g");
    terminal.wait(|screen| has_active_source(screen, "  1− ", "old"));
    submit_path(&mut terminal, &saved);
    terminal.wait(|screen| footer(screen).contains("Saved 4 bytes"));
    assert_eq!(fs::read(&saved).unwrap(), b"old\n");
    terminal.finish(b"q", 0);
    no_staging(dir.path());
}
