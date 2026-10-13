use super::*;
use std::path::{Path, PathBuf};

#[test]
#[ignore = "requires NATIVE_DIFF_BASELINE_BINARY pointing to a compatible preserved executable"]
fn storage_refactor_preserves_full_terminal_grids_and_canonical_sources() {
    let baseline = PathBuf::from(
        std::env::var_os("NATIVE_DIFF_BASELINE_BINARY").expect("explicit baseline binary"),
    );
    let current = viewer_binary();
    let trace = |binary: &Path, path: &Path, sidebar: bool| {
        let mut command = CommandBuilder::new(binary);
        command.arg(if sidebar { "--sidebar" } else { "--no-sidebar" });
        command.arg("patch");
        command.arg(path);
        let mut terminal = Session::spawn(command);
        let mut grids = vec![terminal.settled_grid()];
        for keys in [
            b"n".as_slice(),
            b"N",
            b"G",
            b"g",
            b"w",
            b"u",
            b"t",
            b"\x1b[<65;100;15M",
            b"\x1b[<64;100;15M",
            b"\x1b[<0;100;14M",
            b"\x1b[<0;100;14m",
        ] {
            terminal.keys(keys);
            grids.push(terminal.settled_grid());
        }
        for (rows, columns) in [(24, 80), (30, 121), (10, 40), (50, 160)] {
            terminal.resize(rows, columns);
            grids.push(terminal.settled_grid());
        }
        for keys in [b"w".as_slice(), b"\x1b[C\x1b[C\x1b[D", b"u", b"g", b"t"] {
            terminal.keys(keys);
            grids.push(terminal.settled_grid());
        }
        if sidebar {
            for keys in [b"f".as_slice(), b"\x1b[B", b"\r", b"\x1b", b"f", b"f"] {
                terminal.keys(keys);
                grids.push(terminal.settled_grid());
            }
        }
        terminal.finish(b"q", 0);
        grids
    };
    let mut views = 0;
    for fixture in ["fixtures/review.patch", "fixtures/position/review.patch"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(fixture);
        for sidebar in [false, true] {
            let expected = trace(&baseline, &path, sidebar);
            let actual = trace(&current, &path, sidebar);
            assert_eq!(actual.len(), expected.len());
            for (step, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
                assert_eq!(
                    actual, expected,
                    "{fixture}, sidebar={sidebar}, step={step}"
                );
            }
            views += actual.len();
        }
        let canonical = |binary: &Path| {
            let output = Command::new(binary)
                .args(["inspect", "--canonical"])
                .arg(&path)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            output.stdout
        };
        assert_eq!(canonical(&current), canonical(&baseline), "{fixture}");
    }
    eprintln!("Matched {views} complete terminal grids and two canonical source streams");
}
