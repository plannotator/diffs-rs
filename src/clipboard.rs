use std::{
    fs::File,
    io::{self, BufWriter, Read, Seek, Write},
    os::{fd::OwnedFd, unix::net::UnixStream},
    process::{Child, Command, Stdio},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use diffs_rs::{engine::ReviewId, reading::SourceRead};

const MAX_BYTES: u64 = 16 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(5);
const DIAGNOSTIC_BYTES: usize = 4096;
const TOO_LARGE: &str = "Copy exceeds 16 MiB; use Ctrl-S to save the original bytes";
const NOT_UTF8: &str = "Copy requires UTF-8 text; use Ctrl-S to save the original bytes";

pub struct Prepared {
    source: Arc<SourceRead>,
    input: File,
    diagnostics: Diagnostics,
    stderr: UnixStream,
    bytes: u64,
}

pub struct Delivery {
    source: Arc<SourceRead>,
    child: Child,
    diagnostics: Diagnostics,
    bytes: u64,
    started: Instant,
    reaped: bool,
}

pub struct Receipt {
    source: Arc<SourceRead>,
    pub bytes: u64,
}

struct Diagnostics {
    input: UnixStream,
    tail: Vec<u8>,
}

impl Diagnostics {
    fn drain(&mut self) -> io::Result<bool> {
        let mut buffer = [0; DIAGNOSTIC_BYTES];
        for _ in 0..16 {
            match self.input.read(&mut buffer) {
                Ok(0) => return Ok(false),
                Ok(count) => {
                    let excess = (self.tail.len() + count).saturating_sub(DIAGNOSTIC_BYTES);
                    self.tail.drain(..excess);
                    self.tail.extend_from_slice(&buffer[..count]);
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(false),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(true)
    }
}

#[derive(Debug)]
pub struct CleanupError(anyhow::Error);

impl std::fmt::Display for CleanupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Clipboard helper cleanup failed: {:#}", self.0)
    }
}

impl std::error::Error for CleanupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

pub fn available() -> Result<()> {
    ensure!(
        cfg!(all(target_os = "macos", feature = "clipboard")),
        "Clipboard is unavailable in this build; use Ctrl-S to save"
    );
    Ok(())
}

pub fn command(bytes: u64) -> Result<Command> {
    available()?;
    let mut command =
        Command::new(std::env::current_exe().context("Cannot locate clipboard helper")?);
    command
        .arg("clipboard-helper")
        .arg("--bytes")
        .arg(bytes.to_string());
    Ok(command)
}

struct Limited<W> {
    output: W,
    bytes: u64,
}

impl<W: Write> Write for Limited<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() as u64 > MAX_BYTES - self.bytes {
            return Err(io::Error::other(TOO_LARGE));
        }
        let written = self.output.write(bytes)?;
        self.bytes += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

fn validate(input: &mut File, cancel: &impl Fn() -> Result<()>) -> Result<()> {
    input.rewind()?;
    let mut buffer = [0; 64 * 1024 + 3];
    let mut carried = 0;
    loop {
        cancel()?;
        let count = input.read(&mut buffer[carried..])?;
        if count == 0 {
            ensure!(carried == 0, NOT_UTF8);
            input.rewind()?;
            return Ok(());
        }
        let length = carried + count;
        carried = match std::str::from_utf8(&buffer[..length]) {
            Ok(_) => 0,
            Err(error) if error.error_len().is_none() => {
                let valid = error.valid_up_to();
                buffer.copy_within(valid..length, 0);
                length - valid
            }
            Err(_) => anyhow::bail!(NOT_UTF8),
        };
    }
}

impl Prepared {
    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    pub fn prepare(source: Arc<SourceRead>, cancel: &impl Fn() -> Result<()>) -> Result<Self> {
        cancel()?;
        source.check()?;
        // Linux `O_TMPFILE` files take the umask-derived mode, so restrict it explicitly.
        let mut input = tempfile::tempfile().context("Cannot create private clipboard input")?;
        input
            .set_permissions(std::os::unix::fs::PermissionsExt::from_mode(0o600))
            .context("Cannot restrict private clipboard input")?;
        let bytes = {
            let mut output = Limited {
                output: BufWriter::with_capacity(64 * 1024, &mut input),
                bytes: 0,
            };
            let bytes = source.write_to(&mut output, cancel)?;
            output.flush().context("Cannot flush clipboard input")?;
            bytes
        };
        validate(&mut input, cancel)?;
        cancel()?;
        source.check()?;
        let (diagnostic_input, stderr) =
            UnixStream::pair().context("Cannot open clipboard diagnostics")?;
        diagnostic_input.set_nonblocking(true)?;
        let diagnostics = Diagnostics {
            input: diagnostic_input,
            tail: Vec::with_capacity(DIAGNOSTIC_BYTES),
        };
        Ok(Self {
            source,
            input,
            diagnostics,
            stderr,
            bytes,
        })
    }

    pub fn start(self, review: ReviewId, mut command: Command) -> Result<Delivery> {
        ensure!(
            self.source.range().review == review,
            "Copy belongs to another review"
        );
        self.source.check()?;
        let child = command
            .stdin(Stdio::from(self.input))
            .stdout(Stdio::null())
            .stderr(Stdio::from(OwnedFd::from(self.stderr)))
            .spawn()
            .context("Cannot start clipboard helper; clipboard was not changed")?;
        Ok(Delivery {
            source: self.source,
            child,
            diagnostics: self.diagnostics,
            bytes: self.bytes,
            started: Instant::now(),
            reaped: false,
        })
    }
}

impl Delivery {
    pub fn finish(self, cancel: &impl Fn() -> Result<()>) -> Result<Receipt> {
        self.wait(cancel, TIMEOUT)
    }

    fn wait(mut self, cancel: &impl Fn() -> Result<()>, timeout: Duration) -> Result<Receipt> {
        let result = (|| {
            loop {
                cancel()?;
                self.source.check()?;
                ensure!(
                    self.started.elapsed() < timeout,
                    "Clipboard helper timed out"
                );
                let more_diagnostics = self
                    .diagnostics
                    .drain()
                    .context("Cannot read clipboard diagnostics")?;
                if let Some(status) = self
                    .child
                    .try_wait()
                    .context("Cannot observe clipboard helper")?
                {
                    self.reaped = true;
                    let final_diagnostics = self
                        .diagnostics
                        .drain()
                        .context("Cannot read final clipboard diagnostics")?;
                    if more_diagnostics || final_diagnostics {
                        continue;
                    }
                    if !status.success() {
                        anyhow::bail!(
                            "Clipboard helper exited {status}: {}",
                            diffs_rs::text::safe_label(&self.diagnostics.tail)
                        );
                    }
                    cancel()?;
                    self.source.check()?;
                    return Ok(Receipt {
                        source: self.source.clone(),
                        bytes: self.bytes,
                    });
                }
                thread::sleep(Duration::from_millis(10));
            }
        })();
        if let Err(error) = self.reap() {
            let cleanup = anyhow::Error::new(CleanupError(error));
            return Err(match result {
                Ok(_) => cleanup,
                Err(error) => cleanup.context(format!("Copy also failed: {error:#}")),
            });
        }
        result.context("Copy delivery did not finish; clipboard contents may have changed")
    }

    fn reap(&mut self) -> Result<()> {
        if !self.reaped {
            let killed = self.child.kill();
            match self.child.wait() {
                Ok(_) => self.reaped = true,
                Err(error) => {
                    return Err(anyhow::anyhow!(
                        "Cannot reap helper {}: {error}; kill result: {killed:?}",
                        self.child.id()
                    ));
                }
            }
        }
        Ok(())
    }
}

impl Drop for Delivery {
    fn drop(&mut self) {
        let _ = self.reap();
    }
}

impl Receipt {
    pub fn check(&self, review: ReviewId) -> Result<()> {
        ensure!(
            self.source.range().review == review,
            "Copy belongs to another review"
        );
        self.source.check()
    }
}

fn text(input: impl Read, expected: u64) -> Result<String> {
    ensure!(expected <= MAX_BYTES, TOO_LARGE);
    let mut bytes = Vec::new();
    input.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= MAX_BYTES, TOO_LARGE);
    ensure!(
        bytes.len() as u64 == expected,
        "Clipboard input length changed"
    );
    String::from_utf8(bytes).map_err(|_| anyhow::anyhow!(NOT_UTF8))
}

pub fn helper(bytes: u64) -> Result<()> {
    available()?;
    let text = text(io::stdin().lock(), bytes)?;
    #[cfg(all(target_os = "macos", feature = "clipboard"))]
    {
        arboard::Clipboard::new()
            .context("Cannot open the macOS clipboard")?
            .set_text(text)
            .context("Cannot publish plain text to the macOS clipboard")?;
    }
    #[cfg(not(all(target_os = "macos", feature = "clipboard")))]
    drop(text);
    Ok(())
}

#[cfg(test)]
mod tests;
