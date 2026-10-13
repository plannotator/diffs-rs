use std::{fs, process::Command};

pub use native_diff::{ingest, text};
use native_diff::{model, storage};

#[path = "../examples/projection_reference/model.rs"]
#[allow(dead_code)]
mod reference;

struct Case {
    name: &'static str,
    left: Vec<Vec<u8>>,
    right: Vec<Vec<u8>>,
    offset: usize,
}

fn case(name: &'static str, left: &[&[u8]], right: &[&[u8]], offset: usize) -> Case {
    Case {
        name,
        left: left.iter().map(|s| s.to_vec()).collect(),
        right: right.iter().map(|s| s.to_vec()).collect(),
        offset,
    }
}

#[test]
fn alignment_keeps_exact_offsets_source_bytes_and_reference_rows() {
    let mut cases = vec![
        case("threshold-equality", &[b"abcd"], &[b"xxxx", b"abzz"], 0),
        case("above-threshold", &[b"abcd"], &[b"xxxx", b"abcz"], 1),
        case(
            "earliest-winning-tie",
            &[b"abcd"],
            &[b"xxxx", b"abcz", b"abcz"],
            1,
        ),
        case("whitespace-only", &[b" \t\r\x0c"], &[b"", b" \t"], 0),
        case(
            "ascii-whitespace",
            &[b"a \tb\rc\x0cd"],
            &[b"xxxx", b"abcd"],
            1,
        ),
        case(
            "vertical-tab-is-not-ascii-whitespace",
            &[b"a\x0b\x0bb"],
            &[b"xxx", b"ab"],
            0,
        ),
        case("non-ascii-space", &[b"a\xc2\xa0b"], &[b"xxxx", b"ab"], 0),
        case("invalid-utf8", &[b"a\xffbc"], &[b"xxxx", b"a\xffbz"], 1),
        case("prefix-suffix-overlap", &[b"aa"], &[b"xx", b"aaa"], 1),
        case("suffix-after-prefix", &[b"abcd"], &[b"xxxx", b"axcd"], 1),
        case(
            "multiple-pairs-changing-length",
            &[b"abcdefgh", b"a"],
            &[b"xxxxxxxxxxxx", b"abcdefgh", b"a"],
            1,
        ),
        case("empty-side", &[], &[b"added"], 0),
        case("equal-sides", &[b"abcd"], &[b"abcz"], 0),
    ];
    for length in [4095, 4096] {
        cases.push(Case {
            name: if length == 4095 {
                "raw-4096"
            } else {
                "raw-4097"
            },
            left: vec![vec![b'a'; length]],
            right: vec![vec![b'x'; length], vec![b'a'; length]],
            offset: usize::from(length == 4095),
        });
    }
    for count in [4096, 4097] {
        let mut right = vec![b"x".to_vec(); count];
        *right.last_mut().unwrap() = b"a".to_vec();
        cases.push(Case {
            name: if count == 4096 {
                "work-4096"
            } else {
                "work-4097"
            },
            left: vec![b"a".to_vec()],
            right,
            offset: if count == 4096 { count - 1 } else { 0 },
        });
    }
    let directory = tempfile::tempdir().unwrap();
    let binary = std::env::var_os("NATIVE_DIFF_CONTRACT_BINARY")
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_native-diff").into());
    for case in cases {
        for reversed in [false, true] {
            let (left, right) = if reversed {
                (&case.right, &case.left)
            } else {
                (&case.left, &case.right)
            };
            let label = format!("{}-reversed-{reversed}", case.name);
            let mut patch = format!(
                "diff --git a/review b/review\n--- a/review\n+++ b/review\n@@ -{},{} +{},{} @@\n",
                usize::from(!left.is_empty()),
                left.len(),
                usize::from(!right.is_empty()),
                right.len()
            )
            .into_bytes();
            for (sign, lines) in [(b'-', left), (b'+', right)] {
                for line in lines {
                    patch.push(sign);
                    patch.extend_from_slice(line);
                    patch.push(b'\n');
                }
            }
            let path = directory.path().join("review.patch");
            fs::write(&path, &patch).unwrap();
            let expected = reference::Document::parse(&patch).unwrap();
            let parsed = model::Document::parse(&patch).unwrap();
            let output = Command::new(&binary)
                .arg("inspect")
                .arg(&path)
                .output()
                .unwrap();
            assert!(output.status.success(), "{label}: {:?}", output.stderr);
            let actual: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(
                actual,
                serde_json::to_value(&expected).unwrap(),
                "{label}: process source"
            );
            assert_eq!(
                serde_json::to_value(&parsed).unwrap(),
                actual,
                "{label}: parsed source"
            );
            let mut indexed = storage::Document::default();
            storage::read(patch.as_slice(), |file| indexed.append(file)).unwrap();
            for split in [false, true] {
                let wanted = if split {
                    &expected.split
                } else {
                    &expected.unified
                };
                assert_eq!(indexed.row_len(split), wanted.len(), "{label}: row count");
                for (n, row) in wanted.iter().enumerate() {
                    let actual = indexed.row(n, split).unwrap();
                    let kind = match actual.kind {
                        model::RowKind::Header => reference::RowKind::Header,
                        model::RowKind::Hunk(h) => reference::RowKind::Hunk(h),
                        model::RowKind::Gap => reference::RowKind::Gap,
                        model::RowKind::Metadata => reference::RowKind::Metadata,
                        model::RowKind::Code { left, right } => {
                            reference::RowKind::Code { left, right }
                        }
                        model::RowKind::NoNewline { left, right } => {
                            reference::RowKind::NoNewline { left, right }
                        }
                    };
                    assert_eq!(actual.file, row.file, "{label}: row {n} file");
                    assert_eq!(kind, row.kind, "{label}: row {n}");
                }
            }
            let file = &indexed.files[0];
            for n in 0..left.len() {
                let paired = if left.len() <= right.len() {
                    n.checked_add(case.offset).filter(|r| *r < right.len())
                } else {
                    n.checked_sub(case.offset).filter(|r| *r < right.len())
                };
                assert_eq!(
                    file.partner(n),
                    paired.map(|r| left.len() + r),
                    "{label}: old line {n}"
                );
            }
            for n in 0..right.len() {
                let paired = if right.len() <= left.len() {
                    n.checked_add(case.offset).filter(|l| *l < left.len())
                } else {
                    n.checked_sub(case.offset).filter(|l| *l < left.len())
                };
                assert_eq!(
                    file.partner(left.len() + n),
                    paired,
                    "{label}: new line {n}"
                );
            }
        }
    }
}
