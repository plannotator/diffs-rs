//! Rename and copy names that libgit2 cannot read verbatim.
//!
//! Git writes an unquoted `rename from`, `rename to`, `copy from`, or `copy to` name literally
//! through the line ending, so it can contain leading, interior, and trailing spaces. libgit2
//! stops an unquoted name at the first whitespace byte. It decodes Git's C-quoted form exactly,
//! so the parser receives those names rewritten into quoted form. Hunk lines are not changed,
//! and [`QuotedNames::original_offset`] maps parser offsets back to the supplied bytes.

/// Header line prefixes whose remainder is one unprefixed file name.
const NAME_LINES: [&[u8]; 6] = [
    b"rename from ",
    b"rename to ",
    b"rename old ",
    b"rename new ",
    b"copy from ",
    b"copy to ",
];

/// Lines that end a Git extended header. A name line after one of these is not a header.
const HEADER_ENDS: [&[u8]; 5] = [
    b"--- ",
    b"+++ ",
    b"@@ ",
    b"Binary files ",
    b"GIT binary patch",
];

/// A patch with rewritten rename and copy names.
#[derive(Debug)]
pub(super) struct QuotedNames {
    pub(super) bytes: Vec<u8>,
    /// One entry per rewritten line, in order: the end of that line in `bytes` and the total
    /// number of bytes added up to it.
    shifts: Vec<(usize, usize)>,
}

impl QuotedNames {
    /// Maps an offset in [`Self::bytes`] that lies outside every rewritten line to the same
    /// byte in the supplied patch.
    pub(super) fn original_offset(&self, offset: usize) -> Option<usize> {
        let before = self.shifts.partition_point(|&(end, _)| end <= offset);
        let added = before
            .checked_sub(1)
            .map_or(0, |index| self.shifts[index].1);
        offset.checked_sub(added)
    }
}

/// Returns `None` when libgit2 already reads every rename and copy name as written.
pub(super) fn quote_rename_names(input: &[u8]) -> Option<QuotedNames> {
    // A name line always follows a `diff --git` line, so it never starts the input.
    if memchr::memmem::find(input, b"\nrename ").is_none()
        && memchr::memmem::find(input, b"\ncopy ").is_none()
    {
        return None;
    }
    let mut result: Option<QuotedNames> = None;
    let mut copied = 0;
    let mut start = 0;
    let mut header = false;
    for line in input.split_inclusive(|byte| *byte == b'\n') {
        let end = start + line.len();
        if line.starts_with(b"diff --git ") {
            header = true;
        } else if HEADER_ENDS.iter().any(|prefix| line.starts_with(prefix)) {
            header = false;
        } else if header && let Some((prefix, name, ending)) = unreadable_name(line) {
            let quoted = result.get_or_insert_with(|| QuotedNames {
                bytes: Vec::with_capacity(input.len() + 64),
                shifts: Vec::new(),
            });
            quoted.bytes.extend_from_slice(&input[copied..start]);
            quoted.bytes.extend_from_slice(prefix);
            quote(name, &mut quoted.bytes);
            quoted.bytes.extend_from_slice(ending);
            quoted
                .shifts
                .push((quoted.bytes.len(), quoted.bytes.len() - end));
            copied = end;
        }
        start = end;
    }
    let mut quoted = result?;
    quoted.bytes.extend_from_slice(&input[copied..]);
    Some(quoted)
}

/// Splits a complete name line that libgit2 would misread into its prefix, name, and ending.
fn unreadable_name(line: &[u8]) -> Option<(&[u8], &[u8], &[u8])> {
    let prefix = NAME_LINES.iter().find(|prefix| line.starts_with(prefix))?;
    // libgit2 stops reading the header at a line without a newline.
    let body = line[prefix.len()..].strip_suffix(b"\n")?;
    let (name, ending): (_, &[u8]) = match body.strip_suffix(b"\r") {
        Some(name) => (name, b"\r\n"),
        None => (body, b"\n"),
    };
    // Quoted names already use libgit2's decoder, and empty names remain errors.
    if name.first() == Some(&b'"') || !name.iter().any(|byte| is_libgit2_space(*byte)) {
        return None;
    }
    Some((prefix, name, ending))
}

/// The bytes that end an unquoted name in libgit2's `git__isspace`.
fn is_libgit2_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Writes `name` in Git's C-quoted form, which libgit2's `git_str_unquote` reverses exactly.
fn quote(name: &[u8], output: &mut Vec<u8>) {
    output.push(b'"');
    for &byte in name {
        let escape = match byte {
            b'"' => b'"',
            b'\\' => b'\\',
            b'\x07' => b'a',
            b'\x08' => b'b',
            b'\t' => b't',
            b'\n' => b'n',
            b'\x0b' => b'v',
            b'\x0c' => b'f',
            b'\r' => b'r',
            0..=0x1f | 0x7f => {
                output.extend_from_slice(&[
                    b'\\',
                    b'0' + (byte >> 6),
                    b'0' + ((byte >> 3) & 7),
                    b'0' + (byte & 7),
                ]);
                continue;
            }
            _ => {
                output.push(byte);
                continue;
            }
        };
        output.extend_from_slice(&[b'\\', escape]);
    }
    output.push(b'"');
}

#[cfg(test)]
mod tests {
    use super::super::{Document, FileDiff};
    use super::*;

    fn rewritten(input: &[u8]) -> Vec<u8> {
        quote_rename_names(input).expect("rewritten").bytes
    }

    fn rename(old: &str, new: &str, ending: &str) -> String {
        [
            format!("diff --git a/{old} b/{new}"),
            "similarity index 80%".into(),
            format!("rename from {old}"),
            format!("rename to {new}"),
            "index 1111111..2222222 100644".into(),
            format!("--- a/{old}"),
            format!("+++ b/{new}"),
            "@@ -1,3 +1,3 @@".into(),
            " keep".into(),
            "-before".into(),
            "+after".into(),
            " rename from tail".into(),
            String::new(),
        ]
        .join(ending)
    }

    fn pure_copy(old: &str, new: &str) -> String {
        format!(
            "diff --git a/{old} b/{new}\nsimilarity index 100%\ncopy from {old}\ncopy to {new}\n"
        )
    }

    /// Every parsed line must address its exact bytes in the supplied patch.
    fn check_offsets(input: &[u8], doc: &Document) {
        assert!(!doc.lines.is_empty());
        for line in &doc.lines {
            let end = line.offset + line.bytes.len();
            assert_eq!(&input[line.offset..end], line.bytes.as_slice());
        }
    }

    fn only_file(doc: &Document) -> &FileDiff {
        assert_eq!(doc.files.len(), 1);
        &doc.files[0]
    }

    #[test]
    fn names_libgit2_reads_as_written_are_left_alone() {
        for input in [
            rename("old.txt", "new.txt", "\n"),
            rename("old.txt", "new.txt", "\r\n"),
            rename("old-日本語.txt", "new-日本語.txt", "\n"),
            pure_copy("dir/old.rs", "dir/new.rs"),
            "diff --git \"a/old name\" \"b/new name\"\nsimilarity index 100%\n\
             rename from \"old name\"\nrename to \"new\\tname\"\n"
                .into(),
            "diff --git a/old b/new\nsimilarity index 100%\nrename from \nrename to new\n".into(),
            "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n-x\n+y\n".into(),
        ] {
            assert!(quote_rename_names(input.as_bytes()).is_none(), "{input:?}");
        }
    }

    #[test]
    fn whitespace_names_are_quoted_with_the_line_ending_kept() {
        let cases: [(&str, &str); 7] = [
            ("old name.txt", "\"old name.txt\""),
            ("old.txt  ", "\"old.txt  \""),
            ("  old.txt", "\"  old.txt\""),
            ("old\tname", "\"old\\tname\""),
            ("日本 語.txt", "\"日本 語.txt\""),
            ("a \\b\"c\".txt", "\"a \\\\b\\\"c\\\".txt\""),
            (
                "a\rb c\x01\x7f\x0b\x0c\x07\x08",
                "\"a\\rb c\\001\\177\\v\\f\\a\\b\"",
            ),
        ];
        for (name, expected) in cases {
            for ending in ["\n", "\r\n"] {
                for prefix in NAME_LINES {
                    let mut input = b"diff --git a/x b/y\n".to_vec();
                    input.extend_from_slice(prefix);
                    input.extend_from_slice(name.as_bytes());
                    input.extend_from_slice(ending.as_bytes());
                    let mut expected_bytes = b"diff --git a/x b/y\n".to_vec();
                    expected_bytes.extend_from_slice(prefix);
                    expected_bytes.extend_from_slice(expected.as_bytes());
                    expected_bytes.extend_from_slice(ending.as_bytes());
                    assert_eq!(rewritten(&input), expected_bytes, "{name:?} {ending:?}");
                }
            }
        }
    }

    #[test]
    fn only_extended_header_lines_are_rewritten() {
        let input = b"commit message\n\
            rename from not a header\n\
            diff --git a/a b b/c d\n\
            similarity index 50%\n\
            rename from a b\n\
            rename to c d\n\
            --- a/a b\n\
            +++ b/c d\n\
            rename from not a header\n\
            @@ -1 +1 @@\n\
            -x\n\
            +y\n\
            rename to not a header\n\
            rename to unterminated name";
        let expected = String::from_utf8(input.to_vec())
            .unwrap()
            .replacen("rename from a b\n", "rename from \"a b\"\n", 1)
            .replacen("rename to c d\n", "rename to \"c d\"\n", 1);
        assert_eq!(rewritten(input), expected.as_bytes());
    }

    #[test]
    fn offsets_map_back_across_several_rewritten_lines() {
        let input = b"diff --git a/a b b/c d\nsimilarity index 100%\n\
            rename from a b\nrename to c d\n\
            diff --git a/e b/f g\nsimilarity index 100%\ncopy from e\ncopy to f g\n";
        let quoted = quote_rename_names(input).unwrap();
        // Each rewritten line gains two quotes. `copy from e` is readable and stays as written.
        let expected = [
            (0, 0),
            (23, 23),
            (45, 45),
            (61, 63),
            (75, 79),
            (118, 122),
            (130, 134),
        ];
        for (original, rewritten) in expected {
            assert_eq!(quoted.bytes[rewritten], input[original]);
            assert_eq!(quoted.original_offset(rewritten), Some(original));
        }
        assert_eq!(quoted.bytes.len(), input.len() + 6);
        assert_eq!(
            quoted.original_offset(quoted.bytes.len()),
            Some(input.len())
        );
    }

    #[test]
    fn rewritten_names_match_the_git_header_and_quoted_form() {
        let names = [
            ("old name.txt", "new name.txt"),
            ("old.txt ", "new.txt  "),
            (" old.txt", " new.txt"),
            ("old\tname", "new\tname"),
            ("old 日本語.txt", "new 日本語.txt"),
            ("dir/a \\b\".txt", "dir/c \\d\".txt"),
        ];
        for (old, new) in names {
            for ending in ["\n", "\r\n"] {
                let input = rename(old, new, ending);
                let doc = Document::parse_file(input.as_bytes()).unwrap();
                let file = only_file(&doc);
                assert_eq!(file.status, "Renamed");
                assert_eq!(file.identity.old_path, old.as_bytes(), "{old:?} {ending:?}");
                assert_eq!(file.identity.new_path, new.as_bytes(), "{new:?} {ending:?}");
                assert_eq!(file.raw_path, new.as_bytes());
                // The rename lines agree with the untouched `diff --git` header.
                let header = input.lines().next().unwrap().trim_end_matches('\r');
                let old_path = String::from_utf8(file.identity.old_path.clone()).unwrap();
                let new_path = String::from_utf8(file.identity.new_path.clone()).unwrap();
                assert_eq!(header, format!("diff --git a/{old_path} b/{new_path}"));
                assert_eq!(doc.lines.len(), 4);
                assert_eq!(doc.lines[1].bytes, format!("before{ending}").as_bytes());
                assert_eq!(
                    doc.lines.iter().map(|l| (l.old, l.new)).collect::<Vec<_>>(),
                    [
                        (Some(1), Some(1)),
                        (Some(2), None),
                        (None, Some(2)),
                        (Some(3), Some(3))
                    ]
                );
                check_offsets(input.as_bytes(), &doc);

                // Git's own quoted spelling of the same names parses identically.
                let mut quoted_old = Vec::new();
                quote(old.as_bytes(), &mut quoted_old);
                let mut quoted_new = Vec::new();
                quote(new.as_bytes(), &mut quoted_new);
                let quoted_input = input
                    .replacen(
                        &format!("rename from {old}{ending}"),
                        &format!(
                            "rename from {}{ending}",
                            String::from_utf8(quoted_old).unwrap()
                        ),
                        1,
                    )
                    .replacen(
                        &format!("rename to {new}{ending}"),
                        &format!(
                            "rename to {}{ending}",
                            String::from_utf8(quoted_new).unwrap()
                        ),
                        1,
                    );
                assert_ne!(quoted_input, input);
                let quoted_doc = Document::parse_file(quoted_input.as_bytes()).unwrap();
                let quoted_file = only_file(&quoted_doc);
                assert_eq!(quoted_file.identity.old_path, file.identity.old_path);
                assert_eq!(quoted_file.identity.new_path, file.identity.new_path);
                check_offsets(quoted_input.as_bytes(), &quoted_doc);
                let shift = quoted_input.len() - input.len();
                for (line, quoted_line) in doc.lines.iter().zip(&quoted_doc.lines) {
                    assert_eq!(line.bytes, quoted_line.bytes);
                    assert_eq!(line.offset + shift, quoted_line.offset);
                }
            }
        }
    }

    #[test]
    fn copies_and_multiple_files_keep_exact_offsets() {
        let mut input = rename("a b.txt", "c d.txt", "\n");
        input.push_str(&pure_copy("e f.txt ", "g h.txt "));
        input.push_str(&rename("i j.txt", "k l.txt", "\n").replace("rename ", "copy "));
        let doc = Document::parse_file(input.as_bytes()).unwrap();
        let paths: Vec<_> = doc
            .files
            .iter()
            .map(|file| {
                (
                    file.status.as_str(),
                    file.identity.old_path.as_slice(),
                    file.identity.new_path.as_slice(),
                )
            })
            .collect();
        assert_eq!(
            paths,
            [
                ("Renamed", &b"a b.txt"[..], &b"c d.txt"[..]),
                ("Copied", b"e f.txt ", b"g h.txt "),
                ("Copied", b"i j.txt", b"k l.txt"),
            ]
        );
        assert_eq!(doc.lines.len(), 8);
        check_offsets(input.as_bytes(), &doc);
        // The last file's lines sit after three rewritten lines.
        let last = &doc.lines[4];
        assert_eq!(
            last.offset,
            input.rfind("\n keep\n").unwrap() + 2,
            "{:?}",
            last.bytes
        );
    }

    #[test]
    fn libgit2_is_built_from_the_vendored_source() {
        assert!(git2::Version::get().vendored());
    }

    #[test]
    fn malformed_names_remain_errors() {
        for name in ["\"unterminated", "\"a\" b", "\"bad\\q\"", "\"\""] {
            let input = format!(
                "diff --git a/old b/new\nsimilarity index 100%\nrename from {name}\nrename to new\n"
            );
            assert!(Document::parse_file(input.as_bytes()).is_err(), "{name:?}");
        }
    }
}
