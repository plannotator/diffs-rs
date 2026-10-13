use super::*;

fn footer(screen: &vt100::Screen) -> String {
    screen.rows(0, screen.size().1).last().unwrap()
}

#[test]
fn copy_rejects_non_source_and_invalid_text_then_save_preserves_the_original() {
    let original = b"// old\xff\r\n";
    let (dir, patch) = generated_patch(original, b"// new\n");
    let mut terminal = Session::open(&patch);
    terminal.settled_grid();
    terminal.keys(b"y");
    let enabled = cfg!(all(target_os = "macos", feature = "clipboard"));
    terminal.wait(|screen| {
        footer(screen).contains(if enabled {
            "not a header, gap or padding"
        } else {
            "Clipboard is unavailable"
        })
    });
    terminal.keys(b"gVy");
    terminal.wait(|screen| {
        footer(screen).contains("Selecting old 1–1")
            && footer(screen).contains(if enabled {
                "Copy requires UTF-8 text"
            } else {
                "Clipboard is unavailable"
            })
    });
    assert!(footer(terminal.parser.screen()).contains("Selecting old 1–1"));
    let destination = dir.path().join("original-bytes");
    terminal.keys(b"\x13");
    terminal.wait(|screen| footer(screen).contains("/save:"));
    terminal.keys(format!("\x1b[200~{}\x1b[201~\r", destination.display()).as_bytes());
    terminal.wait(|screen| footer(screen).contains("Saved 9 bytes"));
    assert_eq!(fs::read(destination).unwrap(), original);
    terminal.finish(b"q", 0);
}

#[test]
fn clipboard_helper_rejects_invalid_input_before_any_native_api() {
    for (bytes, expected, message) in [
        (b"\xff".as_slice(), 1, "UTF-8"),
        (b"short".as_slice(), 6, "length changed"),
        (b"".as_slice(), 16 * 1024 * 1024 + 1, "16 MiB"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("input");
        fs::write(&path, bytes).unwrap();
        let output = Command::new(viewer_binary())
            .args(["clipboard-helper", "--bytes", &expected.to_string()])
            .stdin(fs::File::open(path).unwrap())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains(if cfg!(all(target_os = "macos", feature = "clipboard")) {
                message
            } else {
                "Clipboard is unavailable"
            }),
            "{stderr}"
        );
        assert!(!stderr.contains('\x1b'));
    }
}
