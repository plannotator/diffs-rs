use diffs_rs::{
    input::{Cancellation, LocalInput, Review, SourceState},
    source::{SourceStore, check_pair},
    storage::{self, StoreWriter},
};
use std::{fs, io::Read, path::Path, process::Command};

fn git(repo: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .current_dir(repo)
        .args([
            "-c",
            "user.name=Source test",
            "-c",
            "user.email=source@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn read(source: &diffs_rs::source::Snapshot) -> Vec<u8> {
    let mut bytes = Vec::new();
    source.reader().unwrap().read_to_end(&mut bytes).unwrap();
    bytes
}

#[test]
fn captured_files_retain_exact_bytes_and_disk_line_boundaries() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.rs");
    let new = dir.path().join("new.rs");
    let cases = [
        (b"".as_slice(), b"new\n".as_slice()),
        (b"old\r\nlast".as_slice(), b"new\r\nlast!".as_slice()),
        (
            b"\xff\x1b[2J\nlast\r".as_slice(),
            b"\xfe\nlast\r".as_slice(),
        ),
        (b"a\n".as_slice(), b"a".as_slice()),
        (b"a".as_slice(), b"a\n".as_slice()),
        (b"old\n".as_slice(), b"".as_slice()),
    ];
    for (a, b) in cases {
        fs::write(&old, a).unwrap();
        fs::write(&new, b).unwrap();
        let mut review = Review::open(
            LocalInput::Files {
                old: old.clone(),
                new: new.clone(),
            },
            &Cancellation::default(),
            |_| Ok(()),
        )
        .unwrap();
        fs::write(&old, b"changed after capture").unwrap();
        fs::write(&new, b"changed after capture").unwrap();
        let SourceState::Checked(pair) = review.prepare(0, &Cancellation::default()).unwrap()
        else {
            panic!("{:?}", review.sources());
        };
        for (source, expected) in [(pair.old().unwrap(), a), (pair.new_source().unwrap(), b)] {
            assert_eq!(read(source), expected);
            let mut lines = Vec::new();
            for i in 0..source.lines() {
                source
                    .range(i..i + 1)
                    .unwrap()
                    .read_to_end(&mut lines)
                    .unwrap();
            }
            assert_eq!(lines, expected);
            assert_eq!(source.final_newline(), expected.last() == Some(&b'\n'));
        }
        assert_eq!(review.doc().files[0].path.as_ref(), new.to_str().unwrap());
    }
    fs::write(&old, b"same").unwrap();
    fs::write(&new, b"same").unwrap();
    assert!(
        Review::open(
            LocalInput::Files { old, new },
            &Cancellation::default(),
            |_| Ok(())
        )
        .unwrap()
        .doc()
        .files
        .is_empty()
    );
}

#[test]
fn git_inventory_binds_files_renames_binary_modes_and_symlinks() {
    use std::os::unix::{
        ffi::OsStringExt,
        fs::{PermissionsExt, symlink},
    };
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    git(repo, &["init", "-q"]);
    let odd = std::ffi::OsString::from_vec("odd\t\n日本語.rs".as_bytes().to_vec());
    fs::write(repo.join(&odd), b"old\r\nend").unwrap();
    fs::write(repo.join("rename.rs"), b"same\n").unwrap();
    fs::write(repo.join("mode.sh"), b"old\n").unwrap();
    fs::write(repo.join("binary"), b"\0old").unwrap();
    fs::write(repo.join("deleted"), b"deleted\n").unwrap();
    symlink("not-a-target", repo.join("link")).unwrap();
    git(repo, &["add", "."]);
    git(repo, &["commit", "-qm", "base"]);
    fs::rename(repo.join("rename.rs"), repo.join("renamed.ts")).unwrap();
    fs::write(repo.join(&odd), b"new\r\nend!").unwrap();
    fs::set_permissions(repo.join("mode.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(repo.join("binary"), b"\0new").unwrap();
    fs::remove_file(repo.join("deleted")).unwrap();
    fs::remove_file(repo.join("link")).unwrap();
    symlink("other-link-text", repo.join("link")).unwrap();
    fs::write(repo.join("added"), b"added\n").unwrap();
    fs::write(repo.join("empty"), b"").unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-qm", "head"]);
    let mut review = Review::open(
        LocalInput::Git {
            repo: repo.into(),
            base: "HEAD^".into(),
            head: "HEAD".into(),
        },
        &Cancellation::default(),
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(review.doc().files.len(), 8);
    let index_before = fs::read(repo.join(".git/index")).unwrap();
    git(repo, &["branch", "-f", "moving", "HEAD^"]);
    fs::write(repo.join(&odd), b"working tree is not the source").unwrap();
    for id in 0..review.doc().files.len() {
        let state = review.prepare(id, &Cancellation::default()).unwrap();
        assert!(
            matches!(state, SourceState::Checked(_) | SourceState::Metadata),
            "{:?}: {state:?}",
            review.doc().files[id].path
        );
        if let SourceState::Checked(pair) = &state {
            match review.doc().files[id].path.as_ref() {
                "link" => assert_eq!(read(pair.new_source().unwrap()), b"other-link-text"),
                "added" => assert!(pair.old().is_none()),
                "deleted" => assert!(pair.new_source().is_none()),
                "empty" => {
                    assert!(pair.old().is_none());
                    assert!(pair.new_source().unwrap().is_empty());
                }
                _ => {}
            }
        }
    }
    assert_eq!(index_before, fs::read(repo.join(".git/index")).unwrap());
}

#[test]
fn correspondence_rejects_hidden_changes_wrong_lines_and_newline_errors() {
    let dir = tempfile::tempdir().unwrap();
    let a = (0..90)
        .map(|i| format!("line {i:03}\n"))
        .collect::<String>();
    let b = a
        .replace("line 030", "changed thirty")
        .replace("line 060", "changed sixty");
    let old = dir.path().join("old");
    let new = dir.path().join("new");
    fs::write(&old, &a).unwrap();
    fs::write(&new, &b).unwrap();
    let patch = Command::new("git")
        .args([
            "diff",
            "--no-index",
            "--no-ext-diff",
            "--no-textconv",
            "--unified=3",
            "--",
        ])
        .arg(&old)
        .arg(&new)
        .output()
        .unwrap()
        .stdout;
    let mut parsed = Vec::new();
    let spool = storage::read(patch.as_slice(), |file| {
        parsed.push(file);
        Ok(())
    })
    .unwrap();
    let mut store = SourceStore::sharing(&spool).unwrap();
    let old_source = store.capture(a.as_bytes(), &|| Ok(())).unwrap();
    let new_source = store.capture(b.as_bytes(), &|| Ok(())).unwrap();
    check_pair(
        parsed[0].clone(),
        Some(old_source.clone()),
        Some(new_source),
        &|| Ok(()),
    )
    .unwrap();
    for bad in [
        b.replace("line 000", "evil 000"),
        b.replace("line 045", "evil 045"),
        b.replace("line 089", "evil 089"),
        b.replace("changed thirty", "changed wrong!"),
        b.trim_end_matches('\n').into(),
    ] {
        let source = store.capture(bad.as_bytes(), &|| Ok(())).unwrap();
        assert!(
            check_pair(
                parsed[0].clone(),
                Some(old_source.clone()),
                Some(source),
                &|| Ok(())
            )
            .is_err()
        );
    }
    let old_source = store.capture(b.as_bytes(), &|| Ok(())).unwrap();
    let new_source = store.capture(a.as_bytes(), &|| Ok(())).unwrap();
    assert!(
        check_pair(
            parsed[0].clone(),
            Some(old_source),
            Some(new_source),
            &|| Ok(())
        )
        .is_err()
    );
}

#[test]
fn large_lines_use_bounded_capture_and_canceled_tails_are_reclaimed() {
    let writer = StoreWriter::new().unwrap();
    let cache = writer.store();
    let mut store = SourceStore::sharing(&cache).unwrap();
    let source = store
        .capture(std::io::repeat(b'x').take(8 * 1024 * 1024), &|| Ok(()))
        .unwrap();
    assert_eq!(source.lines(), 1);
    assert_eq!(store.bytes(), 8 * 1024 * 1024 + 16);
    let before = store.bytes();
    let checks = std::cell::Cell::new(0);
    let failed = store.capture(std::io::repeat(b'\n').take(8 * 1024 * 1024), &|| {
        checks.set(checks.get() + 1);
        anyhow::ensure!(checks.get() < 4, "cancel capture");
        Ok(())
    });
    assert!(failed.is_err());
    assert_eq!(store.bytes(), before);
    assert_eq!(read(&source).len(), 8 * 1024 * 1024);
    assert!(cache.cached_bytes() <= 32 * 1024 * 1024);
}

#[test]
fn files_reject_nonregular_and_missing_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join("file");
    fs::write(&existing, b"ok").unwrap();
    for old in [dir.path().to_path_buf(), dir.path().join("missing")] {
        assert!(
            Review::open(
                LocalInput::Files {
                    old,
                    new: existing.clone()
                },
                &Cancellation::default(),
                |_| Ok(())
            )
            .is_err()
        );
    }
    let fifo = dir.path().join("pipe");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Review::open(
            LocalInput::Files {
                old: fifo,
                new: existing
            },
            &Cancellation::default(),
            |_| Ok(())
        )
        .is_err()
    );
}

#[test]
fn git_type_changes_and_gitlinks_keep_their_actual_patch_entries() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    git(repo, &["init", "-q"]);
    fs::write(repo.join("kind"), b"regular\n").unwrap();
    git(repo, &["add", "."]);
    git(repo, &["commit", "-qm", "old"]);
    let base = String::from_utf8(git(repo, &["rev-parse", "HEAD"])).unwrap();
    fs::remove_file(repo.join("kind")).unwrap();
    symlink("stored target", repo.join("kind")).unwrap();
    git(repo, &["add", "."]);
    git(
        repo,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{},module", base.trim()),
        ],
    );
    git(repo, &["commit", "-qm", "new"]);
    let mut review = Review::open(
        LocalInput::Git {
            repo: repo.into(),
            base: "HEAD^".into(),
            head: "HEAD".into(),
        },
        &Cancellation::default(),
        |_| Ok(()),
    )
    .unwrap();
    let report = review.report(&Cancellation::default()).unwrap();
    assert!(
        !review
            .sources()
            .iter()
            .any(|s| matches!(s, SourceState::Unavailable(_))),
        "{report}"
    );
    let module = review
        .doc()
        .files
        .iter()
        .position(|f| f.path.as_ref() == "module")
        .unwrap();
    assert!(matches!(review.sources()[module], SourceState::Metadata));
}

#[test]
fn direct_git_pins_revisions_ignores_helpers_and_keeps_missing_sources_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    git(repo, &["init", "-q"]);
    fs::write(repo.join("file.rs"), b"ancestor\n").unwrap();
    git(repo, &["add", "."]);
    git(repo, &["commit", "-qm", "ancestor"]);
    git(repo, &["branch", "base"]);
    git(repo, &["checkout", "-qb", "topic"]);
    fs::write(repo.join("file.rs"), b"head version\n").unwrap();
    git(repo, &["commit", "-qam", "head"]);
    git(repo, &["checkout", "-q", "base"]);
    fs::write(repo.join("file.rs"), b"base version\n").unwrap();
    git(repo, &["commit", "-qam", "base"]);
    git(repo, &["config", "diff.external", "false"]);
    git(repo, &["config", "diff.evil.textconv", "false"]);
    fs::write(repo.join(".gitattributes"), b"* diff=evil\n").unwrap();
    let input = || LocalInput::Git {
        repo: repo.into(),
        base: "base".into(),
        head: "topic".into(),
    };
    let mut review = Review::open(input(), &Cancellation::default(), |_| Ok(())).unwrap();
    git(repo, &["branch", "-f", "topic", "base"]);
    fs::write(repo.join("file.rs"), b"worktree modified").unwrap();
    let SourceState::Checked(pair) = review.prepare(0, &Cancellation::default()).unwrap() else {
        panic!("{:?}", review.sources());
    };
    assert_eq!(read(pair.old().unwrap()), b"base version\n");
    assert_eq!(read(pair.new_source().unwrap()), b"head version\n");
    assert!(
        Review::open(input(), &Cancellation::default(), |_| Ok(()))
            .unwrap()
            .doc()
            .files
            .is_empty()
    );
    let mut missing = Review::open(
        LocalInput::Git {
            repo: repo.into(),
            base: "base^".into(),
            head: "base".into(),
        },
        &Cancellation::default(),
        |_| Ok(()),
    )
    .unwrap();
    let object = String::from_utf8(git(repo, &["rev-parse", "base:file.rs"])).unwrap();
    let object = object.trim();
    fs::remove_file(
        repo.join(".git/objects")
            .join(&object[..2])
            .join(&object[2..]),
    )
    .unwrap();
    let state = missing.prepare(0, &Cancellation::default()).unwrap();
    assert!(matches!(state, SourceState::Unavailable(_)));
    assert_eq!(
        missing
            .doc()
            .bytes(missing.doc().files[0].source_base)
            .unwrap(),
        b"ancestor\n"
    );
}

#[test]
fn git_rejects_noncommit_revisions_and_unsupported_object_format() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    git(repo, &["init", "-q"]);
    fs::write(repo.join("file"), b"content").unwrap();
    git(repo, &["add", "."]);
    git(repo, &["commit", "-qm", "one"]);
    for revision in ["missing", "HEAD:file", "--help"] {
        assert!(
            Review::open(
                LocalInput::Git {
                    repo: repo.into(),
                    base: "HEAD".into(),
                    head: revision.into()
                },
                &Cancellation::default(),
                |_| Ok(())
            )
            .is_err()
        );
    }
    let other = tempfile::tempdir().unwrap();
    git(other.path(), &["init", "-q", "--object-format=sha256"]);
    let error = Review::open(
        LocalInput::Git {
            repo: other.path().into(),
            base: "HEAD".into(),
            head: "HEAD".into(),
        },
        &Cancellation::default(),
        |_| Ok(()),
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains("SHA-1"), "{error:#}");
}
