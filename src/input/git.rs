use std::{
    env,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

use anyhow::{Context, Result, ensure};

use super::Cancellation;

pub(super) struct Git {
    executable: PathBuf,
    directory: PathBuf,
}

struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Git {
    pub fn new(directory: &Path, cancel: &Cancellation) -> Result<Self> {
        let executable = env::split_paths(&env::var_os("PATH").context("Git is not on PATH")?)
            .map(|dir| dir.join("git"))
            .find(|path| {
                path.metadata()
                    .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            })
            .context("Git is required for local source inputs")?
            .canonicalize()?;
        let git = Self {
            executable,
            directory: directory.canonicalize()?,
        };
        let version = git.small(&["--version"], cancel)?;
        let version = String::from_utf8(version)?;
        let number = version
            .split_whitespace()
            .nth(2)
            .context("Cannot identify the Git version")?;
        let mut parts = number.split('.').map(str::parse::<u32>);
        let major = parts.next().context("Invalid Git version")??;
        let minor = parts.next().context("Invalid Git version")??;
        ensure!(
            (major, minor) >= (2, 50),
            "Local source inputs require Git 2.50 or newer"
        );
        Ok(git)
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new(&self.executable);
        cmd.current_dir(&self.directory);
        for (name, _) in env::vars_os() {
            if name.as_encoded_bytes().starts_with(b"GIT_") {
                cmd.env_remove(name);
            }
        }
        cmd.env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("LC_ALL", "C");
        cmd.args([
            "--no-pager",
            "--no-replace-objects",
            "--no-optional-locks",
            "--no-lazy-fetch",
        ]);
        for setting in [
            "core.fsmonitor=false",
            "core.hooksPath=/dev/null",
            "core.attributesFile=/dev/null",
            "protocol.allow=never",
            "diff.submodule=short",
            "diff.suppressBlankEmpty=false",
            "diff.noprefix=false",
            "diff.mnemonicPrefix=false",
            "core.quotePath=true",
        ] {
            cmd.args(["-c", setting]);
        }
        cmd
    }

    /// Child stdout goes to a private file. No pipe can block cancellation or require a full in-memory response.
    pub fn run(
        &self,
        args: &[&std::ffi::OsStr],
        input: Option<File>,
        differences: bool,
        cancel: &Cancellation,
    ) -> Result<File> {
        cancel.check()?;
        let mut output = tempfile::tempfile()?;
        let mut cmd = self.command();
        cmd.args(args)
            .stdin(input.map(Stdio::from).unwrap_or_else(Stdio::null))
            .stdout(Stdio::from(output.try_clone()?))
            .stderr(Stdio::piped());
        let mut child = Running(cmd.spawn().context("Cannot start Git")?);
        let stderr = child.0.stderr.take().context("Git stderr is unavailable")?;
        let (finished, completion) = mpsc::sync_channel(1);
        let drain = thread::spawn(move || {
            let result = read_stderr(stderr);
            let _ = finished.send(());
            result
        });
        // Stream closure is a wake-up hint, not proof that the child exited.
        let mut notified = false;
        let result = loop {
            if let Err(error) = cancel.check() {
                break Err(error);
            }
            match child.0.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) => {
                    if notified {
                        thread::sleep(Duration::from_millis(10));
                    } else {
                        match completion.recv_timeout(Duration::from_millis(10)) {
                            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => notified = true,
                            Err(mpsc::RecvTimeoutError::Timeout) => {}
                        }
                    }
                }
                Err(error) => break Err(error.into()),
            }
        };
        if result.is_err() {
            let _ = child.0.kill();
            let _ = child.0.wait();
        }
        let diagnostic = drain
            .join()
            .map_err(|_| anyhow::anyhow!("Git diagnostic reader stopped"))??;
        let status = result?;
        ensure!(
            status.success() || (differences && status.code() == Some(1)),
            "Git failed: {}",
            crate::text::safe_text(&diagnostic)
        );
        output.seek(SeekFrom::Start(0))?;
        Ok(output)
    }

    pub fn small(&self, args: &[&str], cancel: &Cancellation) -> Result<Vec<u8>> {
        let output = self.run(
            &args.iter().map(std::ffi::OsStr::new).collect::<Vec<_>>(),
            None,
            false,
            cancel,
        )?;
        ensure!(
            output.metadata()?.len() <= 64 * 1024,
            "Git metadata response is too large"
        );
        let mut bytes = Vec::new();
        output.take(64 * 1024).read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    pub fn pin(&self, revision: &str, cancel: &Cancellation) -> Result<git2::Oid> {
        let bytes = self.small(
            &[
                "rev-parse",
                "--verify",
                "--end-of-options",
                &format!("{revision}^{{commit}}"),
            ],
            cancel,
        )?;
        oid(bytes
            .strip_suffix(b"\n")
            .context("Invalid revision response")?)
    }

    pub fn objects(&self, ids: &[git2::Oid], cancel: &Cancellation) -> Result<File> {
        let mut requests = tempfile::tempfile()?;
        for id in ids {
            writeln!(requests, "{id}")?;
        }
        requests.seek(SeekFrom::Start(0))?;
        self.run(
            &["cat-file".as_ref(), "--batch".as_ref()],
            Some(requests),
            false,
            cancel,
        )
    }
}

fn read_stderr(mut input: impl Read) -> std::io::Result<Vec<u8>> {
    let mut tail = Vec::new();
    let mut buf = [0; 4096];
    loop {
        let n = input.read(&mut buf)?;
        if n == 0 {
            return Ok(tail);
        }
        tail.extend_from_slice(&buf[..n]);
        if tail.len() > 8192 {
            tail.drain(..tail.len() - 8192);
        }
    }
}

#[cfg(test)]
mod tests;

pub(super) fn oid(bytes: &[u8]) -> Result<git2::Oid> {
    ensure!(
        bytes.len() == 40 && bytes.iter().all(u8::is_ascii_hexdigit),
        "Expected a complete SHA-1 object ID"
    );
    git2::Oid::from_str(std::str::from_utf8(bytes)?).context("Invalid Git object ID")
}

pub(super) const DIFF: &[&str] = &[
    "diff",
    "--patch",
    "--full-index",
    "--no-ext-diff",
    "--no-textconv",
    "--no-color",
    "--unified=3",
    "--inter-hunk-context=0",
    "--diff-algorithm=myers",
    "--indent-heuristic",
    "--no-relative",
    "--src-prefix=a/",
    "--dst-prefix=b/",
    "--line-prefix=",
    "--output-indicator-new=+",
    "--output-indicator-old=-",
    "--output-indicator-context= ",
    "--ignore-submodules=none",
    "-O/dev/null",
];
