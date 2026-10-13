use std::{fs, path::Path, process::Command};

fn git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(repo)
        .args([
            "-c",
            "user.name=Source test",
            "-c",
            "user.email=source@example.invalid",
            "-c",
            "commit.gpgSign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn repository(long_endpoint: bool) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    git(repo, &["init", "-q"]);
    let mut lines: Vec<_> = (1..=800)
        .map(|n| format!("return {n}; // source {n}"))
        .collect();
    if long_endpoint {
        lines[762].push_str(&format!(
            " 日本語\te\u{301}{} ANCHOR {}",
            "x".repeat(210),
            "z".repeat(100)
        ));
    }
    for changed in [false, true] {
        if changed {
            for n in [100, 400, 760] {
                lines[n - 1] = format!("return -{n}; // source {n}");
            }
        }
        let text = lines.join("\n") + "\n";
        for file in 0..12 {
            fs::write(repo.join(format!("file-{file:02}.ts")), &text).unwrap();
        }
        git(repo, &["add", "."]);
        git(
            repo,
            &["commit", "-qm", if changed { "new" } else { "old" }],
        );
    }
    dir
}
