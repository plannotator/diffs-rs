//! Real `git show`, `git log -p`, `git format-patch`, and `--stat -p` output.
//!
//! `fixtures/git-output/generate.sh` writes each command's output beside the file patches of the
//! same commits, as produced by `git diff-tree` or `git diff`. The viewer must read the command
//! output exactly as it reads those file patches alone, with source bytes taken from the
//! original stream at their exact offsets.

use std::{fs, path::PathBuf};

use native_diff::{model::Document, storage};

const FIXTURES: &[&str] = &[
    "show",
    "show-stat",
    "log",
    "log-3",
    "log-stat-binary",
    "format-patch",
    "diff-stat",
];

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/git-output")
            .join(name),
    )
    .unwrap()
}

fn indexed(input: &[u8]) -> (storage::Document, Vec<u8>, u64) {
    let mut doc = storage::Document::default();
    let mut canonical = Vec::new();
    let spool = storage::read(input, |file| {
        file.write_canonical(&mut canonical)?;
        doc.append(file)
    })
    .unwrap();
    (doc, canonical, spool.len())
}

#[test]
fn git_command_output_reads_like_its_file_patches() {
    for name in FIXTURES {
        let output = fixture(&format!("{name}.patch"));
        let units = fixture(&format!("{name}.units.patch"));
        assert!(output.len() > units.len(), "{name} has no surrounding text");

        let parsed = Document::parse(&output).unwrap();
        let expected = Document::parse(&units).unwrap();
        assert!(!expected.files.is_empty(), "{name}");
        assert_eq!(
            serde_json::to_value(&parsed).unwrap(),
            serde_json::to_value(&expected).unwrap(),
            "{name}"
        );

        let (doc, canonical, spool_len) = indexed(&output);
        let (_, expected_canonical, _) = indexed(&units);
        assert_eq!(canonical, expected_canonical, "{name}");
        assert_eq!(spool_len, output.len() as u64, "{name}");
        assert_eq!(doc.files.len(), expected.files.len(), "{name}");
        assert_eq!(doc.source_len(), expected.lines.len(), "{name}");
        for (id, line) in expected.lines.iter().enumerate() {
            assert_eq!(doc.bytes(id).unwrap(), line.bytes, "{name} source {id}");
        }
    }
}

#[test]
fn git_log_keeps_header_only_changes_between_commits() {
    let doc = Document::parse(&fixture("log.patch")).unwrap();
    let files: Vec<_> = doc
        .files
        .iter()
        .map(|file| {
            (
                file.old_path.as_str(),
                file.path.as_str(),
                file.status.as_str(),
                file.new_mode.as_str(),
            )
        })
        .collect();
    assert_eq!(
        files,
        [
            ("Makefile", "Makefile", "Modified", "100644"),
            ("main.rs", "main.rs", "Modified", "100644"),
            ("blob.bin", "blob.bin", "Binary change", "100644"),
            ("side.txt", "side.txt", "Added", "100644"),
            ("script.sh", "script.sh", "Modified", "100755"),
            // A pure rename carries no mode line.
            ("notes.txt", "docs.txt", "Renamed", "0"),
            ("main.rs", "main.rs", "Modified", "100644"),
            ("Makefile", "Makefile", "Added", "100644"),
            ("empty.txt", "empty.txt", "Added", "100644"),
            ("main.rs", "main.rs", "Added", "100644"),
            ("notes.txt", "notes.txt", "Added", "100644"),
            ("script.sh", "script.sh", "Added", "100644"),
        ]
    );
    // The first commit's Makefile ends without a final newline before the next file patch.
    let makefile = doc.files[0].hunks[0].lines.clone();
    assert!(doc.lines[makefile].last().unwrap().no_newline);
}

#[test]
fn format_patch_reads_binary_patches_and_signatures() {
    let output = fixture("format-patch.patch");
    assert!(output.windows(17).any(|w| w == b"GIT binary patch\n"));
    assert!(output.windows(4).any(|w| w == b"\n-- "));
    let doc = Document::parse(&output).unwrap();
    assert_eq!(doc.files.len(), 12);
    assert!(doc.files.iter().any(|file| file.status == "Binary change"));
    assert!(doc.files.iter().any(|file| file.status == "Renamed"));
}
