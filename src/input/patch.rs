#[cfg(unix)]
use std::os::fd::{AsFd, AsRawFd};
use std::{
    fs::{File, Metadata, OpenOptions},
    io::{self, Read},
    sync::Arc,
};

use anyhow::{Context, Result, ensure};
#[cfg(unix)]
use nix::{
    errno::Errno,
    fcntl::{FcntlArg, OFlag, fcntl},
    sys::{
        select::{FD_SETSIZE, FdSet, select},
        time::{TimeVal, TimeValLike},
    },
};

use super::{Cancellation, PatchInput};
use crate::storage::{self, FileIndex};

#[derive(Debug)]
pub(super) struct RestoreFailure(anyhow::Error);

impl std::fmt::Display for RestoreFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Cannot restore patch input flags: {:#}", self.0)
    }
}

impl std::error::Error for RestoreFailure {}

pub(super) fn read(
    input: PatchInput,
    cancel: &Cancellation,
    mut publish: impl FnMut(Arc<FileIndex>) -> Result<()>,
) -> Result<usize> {
    cancel.check()?;
    let file = match input {
        PatchInput::Path(path) => {
            let mut options = OpenOptions::new();
            options.read(true);
            // A FIFO's open must not wait for its writer: the reader waits, and can be cancelled.
            #[cfg(unix)]
            std::os::unix::fs::OpenOptionsExt::custom_flags(&mut options, libc::O_NONBLOCK);
            options
                .open(&path)
                .with_context(|| format!("Cannot open {}", path.display()))?
        }
        PatchInput::Stream(file) => file,
    };
    let writer = storage::StoreWriter::new()?;
    let mut reader = Reader::new(file, cancel.clone())?;
    let mut files = 0;
    let result = storage::read_with(&mut reader, writer, &|| cancel.check(), |file, _| {
        cancel.check()?;
        publish(file)?;
        files += 1;
        Ok(())
    })
    .and_then(|_| reader.check_stable())
    .map(|()| files);
    if let Err(restore) = reader.restore() {
        return Err(RestoreFailure(match result {
            Ok(_) => restore,
            Err(error) => restore.context(format!("Reading also failed: {error:#}")),
        })
        .into());
    }
    cancel.check()?;
    result
}

/// Reads a patch input with cancellation. On Unix it waits for bytes with `select`, so a pipe
/// whose writer is idle can be cancelled. Windows has no readiness wait for files and pipes:
/// there a read blocks until bytes come or the input ends, and cancellation is checked
/// between reads.
struct Reader {
    file: File,
    before: Metadata,
    cancel: Cancellation,
    #[cfg(unix)]
    restore_blocking: bool,
    eof: bool,
}

impl Reader {
    #[cfg(not(unix))]
    fn new(file: File, cancel: Cancellation) -> Result<Self> {
        let before = file.metadata()?;
        Ok(Self {
            file,
            before,
            cancel,
            eof: false,
        })
    }

    #[cfg(unix)]
    fn new(file: File, cancel: Cancellation) -> Result<Self> {
        ensure!(
            (file.as_raw_fd() as usize) < FD_SETSIZE,
            "Patch input descriptor exceeds the readiness monitor limit ({FD_SETSIZE})"
        );
        let before = file.metadata()?;
        let flags = OFlag::from_bits_retain(fcntl(file.as_raw_fd(), FcntlArg::F_GETFL)?);
        let restore_blocking = !flags.contains(OFlag::O_NONBLOCK);
        if restore_blocking {
            fcntl(
                file.as_raw_fd(),
                FcntlArg::F_SETFL(flags | OFlag::O_NONBLOCK),
            )?;
        }
        Ok(Self {
            file,
            before,
            cancel,
            restore_blocking,
            eof: false,
        })
    }

    fn check_stable(&self) -> Result<()> {
        if self.before.is_file() {
            let after = self.file.metadata()?;
            ensure!(
                self.before.len() == after.len() && self.before.modified()? == after.modified()?,
                "Input changed during reading. Open a stable patch."
            );
        }
        Ok(())
    }

    #[cfg(not(unix))]
    fn restore(&mut self) -> Result<()> {
        Ok(())
    }

    #[cfg(unix)]
    fn restore(&mut self) -> Result<()> {
        if self.restore_blocking {
            let flags = OFlag::from_bits_retain(fcntl(self.file.as_raw_fd(), FcntlArg::F_GETFL)?);
            fcntl(
                self.file.as_raw_fd(),
                FcntlArg::F_SETFL(flags & !OFlag::O_NONBLOCK),
            )
            .context("Cannot restore patch input flags")?;
            self.restore_blocking = false;
        }
        Ok(())
    }
}

#[cfg(not(unix))]
impl Read for Reader {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if bytes.is_empty() || self.eof {
            return Ok(0);
        }
        loop {
            self.cancel.check().map_err(io::Error::other)?;
            match self.file.read(bytes) {
                // std reads a pipe whose writer closed as 0 bytes.
                Ok(0) => {}
                Ok(count) => return Ok(count),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
            self.eof = true;
            return Ok(0);
        }
    }
}

#[cfg(unix)]
impl Read for Reader {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if bytes.is_empty() || self.eof {
            return Ok(0);
        }
        loop {
            self.cancel.check().map_err(io::Error::other)?;
            let mut ready = FdSet::new();
            ready.insert(self.file.as_fd());
            let mut timeout = TimeVal::milliseconds(25);
            match select(None, Some(&mut ready), None, None, Some(&mut timeout)) {
                Ok(0) | Err(Errno::EINTR) => continue,
                Err(error) => return Err(error.into()),
                Ok(_) => {}
            }
            self.cancel.check().map_err(io::Error::other)?;
            // On Darwin, an eager FIFO read can consume the empty-producer EOF indication.
            match self.file.read(bytes) {
                Ok(0) => {
                    self.eof = true;
                    return Ok(0);
                }
                Ok(count) => return Ok(count),
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(error) => return Err(error),
            }
        }
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests;
