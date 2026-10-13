use super::visit_files;

/// Returns each unit with its offset, after checking that the offset addresses the unit's
/// exact bytes in the original input and that the reported total covers the whole input.
fn units(input: &[u8]) -> anyhow::Result<Vec<(u64, String)>> {
    let mut units = Vec::new();
    let total = visit_files(input, |start, unit| {
        let start_index = usize::try_from(start)?;
        assert_eq!(&input[start_index..start_index + unit.len()], unit);
        units.push((start, String::from_utf8(unit.to_vec())?));
        Ok(())
    })?;
    assert_eq!(total, input.len() as u64);
    Ok(units)
}

fn error(input: &[u8]) -> String {
    format!("{:#}", units(input).unwrap_err())
}

const TEXT: &str = "diff --git a/a.txt b/a.txt\nindex 1111111..2222222 100644\n--- a/a.txt\n+++ b/a.txt\n@@ -1,2 +1,2 @@\n a\n-b\n+c\n";
const RENAME: &str = "diff --git a/old.txt b/new.txt\nsimilarity index 100%\nrename from old.txt\nrename to new.txt\n";
const MODE: &str = "diff --git a/run.sh b/run.sh\nold mode 100644\nnew mode 100755\n";
const BINARY: &str = "diff --git a/b.bin b/b.bin\nnew file mode 100644\nindex 0000000..0f49c4a\nBinary files /dev/null and b/b.bin differ\n";
const GIT_BINARY: &str = "diff --git a/b.bin b/b.bin\nnew file mode 100644\nindex 0000000000000000000000000000000000000000..0f49c4a5a0ee2b0cbbb46a5b5ed0d2d2ab7fc0fb\nGIT binary patch\nliteral 9\nQcmZQzWJ=1+ODw7c00^)Gi2wiq\n\nliteral 0\nHcmV?d00001\n\n";
const EMPTY_FILE: &str =
    "diff --git a/empty.txt b/empty.txt\nnew file mode 100644\nindex 0000000..e69de29\n";

fn commit(id: char, subject: &str) -> String {
    format!(
        "commit {}\nAuthor: Fixture <fixture@example.com>\nDate:   Thu Jan 1 00:00:00 2026 +0000\n\n    {subject}\n\n",
        id.to_string().repeat(40)
    )
}

#[test]
fn plain_git_diff_units_are_unchanged() {
    let input = format!("{TEXT}{RENAME}{MODE}{TEXT}");
    let found = units(input.as_bytes()).unwrap();
    let expected = [TEXT, RENAME, MODE, TEXT];
    assert_eq!(found.len(), expected.len());
    let mut offset = 0;
    for ((start, unit), expected) in found.iter().zip(expected) {
        assert_eq!((*start, unit.as_str()), (offset, expected));
        offset += expected.len() as u64;
    }
}

#[test]
fn empty_and_whitespace_input_have_no_units() {
    assert!(units(b"").unwrap().is_empty());
    assert!(units(b"\n \n\t\n").unwrap().is_empty());
}

#[test]
fn git_show_header_and_message_are_skipped() {
    let input = format!("{}{TEXT}", commit('a', "change a"));
    let found = units(input.as_bytes()).unwrap();
    assert_eq!(found, [(commit('a', "change a").len() as u64, TEXT.into())]);
}

#[test]
fn git_log_units_end_before_each_commit_header() {
    // `git log -p` separates each commit's last file patch from the next commit with a blank line.
    let input = [
        commit('a', "text"),
        format!("{TEXT}\n"),
        commit('b', "rename only"),
        format!("{RENAME}\n"),
        commit('c', "merge without a diff"),
        commit('d', "mode only"),
        format!("{MODE}\n"),
        commit('e', "binary"),
        format!("{BINARY}\n"),
        commit('f', "empty file"),
        format!("{EMPTY_FILE}\n"),
        commit('g', "binary patch"),
        format!("{GIT_BINARY}\n"),
        commit('h', "last"),
        TEXT.into(),
    ]
    .concat();
    let found: Vec<_> = units(input.as_bytes())
        .unwrap()
        .into_iter()
        .map(|(_, unit)| unit)
        .collect();
    assert_eq!(
        found,
        [TEXT, RENAME, MODE, BINARY, EMPTY_FILE, GIT_BINARY, TEXT]
    );
}

#[test]
fn header_only_unit_followed_directly_by_commit_header() {
    for header_only in [RENAME, MODE, BINARY, EMPTY_FILE, GIT_BINARY] {
        let input = format!("{header_only}{}{TEXT}", commit('b', "next"));
        let found: Vec<_> = units(input.as_bytes())
            .unwrap()
            .into_iter()
            .map(|(_, unit)| unit)
            .collect();
        assert_eq!(found, [header_only, TEXT]);
    }
}

#[test]
fn forward_only_binary_patch_ends_before_next_text() {
    let forward =
        "diff --git a/b.bin b/b.bin\nGIT binary patch\nliteral 9\nQcmZQzWJ=1+ODw7c00^)Gi2wiq\n\n";
    let input = format!("{forward}{}{TEXT}", commit('b', "next"));
    let found: Vec<_> = units(input.as_bytes())
        .unwrap()
        .into_iter()
        .map(|(_, unit)| unit)
        .collect();
    assert_eq!(found, [forward, TEXT]);
}

#[test]
fn format_patch_mail_headers_and_signatures_are_skipped() {
    let mail = |n: u8, subject: &str| {
        format!(
            "From {} Mon Sep 17 00:00:00 2001\nFrom: Fixture <fixture@example.com>\nDate: Thu, 1 Jan 2026 00:00:00 +0000\nSubject: [PATCH {n}/3] {subject}\n\nUnindented body text.\n---\n a.txt | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)\n\n",
            "0".repeat(40)
        )
    };
    let signature = "-- \n2.50.1\n\n";
    let no_newline =
        "diff --git a/z b/z\n--- a/z\n+++ b/z\n@@ -1 +1 @@\n-x\n+y\n\\ No newline at end of file\n";
    let input = [
        mail(1, "text"),
        TEXT.into(),
        signature.into(),
        mail(2, "rename"),
        RENAME.into(),
        signature.into(),
        mail(3, "no newline"),
        no_newline.into(),
        "-- \r\n2.50.1\r\n".into(),
    ]
    .concat();
    let found: Vec<_> = units(input.as_bytes())
        .unwrap()
        .into_iter()
        .map(|(_, unit)| unit)
        .collect();
    assert_eq!(found, [TEXT, RENAME, no_newline]);
}

#[test]
fn diffstat_before_patch_is_skipped() {
    let input = format!(
        " a.txt   | 2 +-\n new.txt | 0\n 2 files changed, 1 insertion(+), 1 deletion(-)\n\n{TEXT}{RENAME}"
    );
    let found: Vec<_> = units(input.as_bytes())
        .unwrap()
        .into_iter()
        .map(|(_, unit)| unit)
        .collect();
    assert_eq!(found, [TEXT, RENAME]);
}

#[test]
fn text_without_file_headers_is_an_error() {
    assert!(error(b"just text\nno headers\n").contains("no `diff --git` file header"));
    // A merge commit shown without a diff has no file patch to review.
    assert!(error(commit('a', "merge").as_bytes()).contains("no `diff --git` file header"));
}

#[test]
fn patches_without_git_headers_are_an_error() {
    let traditional = "--- a.txt\t2026-01-01\n+++ b.txt\t2026-01-01\n@@ -1 +1 @@\n-a\n+b\n";
    assert!(error(traditional.as_bytes()).contains("without a `diff --git` line"));
    let after_git = format!("{TEXT}\n{traditional}");
    assert!(error(after_git.as_bytes()).contains("without a `diff --git` line"));
}

#[test]
fn hunk_header_outside_a_file_patch_is_an_error() {
    // An unrecognized extended header ends the unit, as in `git apply`; its hunk must not vanish.
    let input = "diff --git a/a b/a\nunknown header\n@@ -1 +1 @@\n-a\n+b\n";
    assert!(error(input.as_bytes()).contains("hunk header outside a file patch"));
    // Text that merely starts like a hunk header is ordinary outside text.
    let input = format!("@@ not a hunk\n{TEXT}");
    assert_eq!(units(input.as_bytes()).unwrap().len(), 1);
}

#[test]
fn incorrect_hunk_counts_remain_errors() {
    let extra = format!("{TEXT}+d\n");
    assert!(error(extra.as_bytes()).contains("incorrect hunk counts"));
    let extra_context = format!("{TEXT} d\n");
    assert!(error(extra_context.as_bytes()).contains("incorrect hunk counts"));
    let short = TEXT.strip_suffix("+c\n").unwrap();
    assert!(error(short.as_bytes()).contains("Hunk counts do not agree"));
    let interrupted = format!("{short}{TEXT}");
    assert!(error(interrupted.as_bytes()).contains("Hunk counts do not agree"));
    let interrupted = format!("{short}{}", commit('b', "next"));
    assert!(error(interrupted.as_bytes()).contains("incorrect hunk counts"));
}

#[test]
fn combined_diffs_remain_unsupported() {
    let input = format!("{}diff --cc a.txt\nindex 1,2..3\n", commit('a', "merge"));
    assert!(error(input.as_bytes()).contains("Combined merge diffs are not supported"));
}
