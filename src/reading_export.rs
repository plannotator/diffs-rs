use std::{
    fs,
    io::{BufWriter, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::Arc,
};

use anyhow::{Context, Result, ensure};
use diffs_rs::{engine::ReviewId, reading::SourceRead};
use tempfile::{TempDir, TempPath};

pub struct Prepared {
    source: Arc<SourceRead>,
    destination: PathBuf,
    path: TempPath,
    directory: TempDir,
    bytes: u64,
}

pub struct Published {
    pub destination: PathBuf,
    pub bytes: u64,
    pub cleanup: Result<()>,
}

#[derive(Debug)]
pub struct CleanupError(anyhow::Error);

impl std::fmt::Display for CleanupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Save staging cleanup failed: {:#}", self.0)
    }
}

impl std::error::Error for CleanupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

fn cleanup(directory: TempDir) -> Result<()> {
    let path = directory.path().to_owned();
    directory
        .close()
        .with_context(|| format!("Cannot remove private save directory {}", path.display()))
        .map_err(|error| CleanupError(error).into())
}

fn failed(error: anyhow::Error, cleaned: Result<()>) -> anyhow::Error {
    match cleaned {
        Ok(()) => error,
        Err(cleanup) => cleanup.context(format!("Save also failed: {error:#}")),
    }
}

impl Prepared {
    pub fn prepare(
        source: Arc<SourceRead>,
        destination: PathBuf,
        cancel: &impl Fn() -> Result<()>,
    ) -> Result<Self> {
        cancel()?;
        source.check()?;
        let destination = std::path::absolute(destination).context("Cannot resolve save path")?;
        ensure!(
            destination.file_name().is_some(),
            "Choose a destination file"
        );
        match fs::symlink_metadata(&destination) {
            Ok(_) => anyhow::bail!("Destination already exists; choose a new file"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("Cannot inspect save destination"),
        }
        let parent = destination
            .parent()
            .context("Save destination has no parent")?;
        let directory = tempfile::Builder::new()
            .prefix(".diffs-save-")
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir_in(parent)
            .context("Cannot create private staging in the destination directory")?;
        let prepared = (|| -> Result<_> {
            let mut file = tempfile::NamedTempFile::new_in(directory.path())
                .context("Cannot create private save file")?;
            let bytes = {
                let mut output = BufWriter::with_capacity(64 * 1024, &mut file);
                let bytes = source.write_to(&mut output, cancel)?;
                output.flush().context("Cannot flush prepared source")?;
                bytes
            };
            file.as_file()
                .sync_all()
                .context("Cannot sync prepared source")?;
            cancel()?;
            source.check()?;
            Ok((file.into_temp_path(), bytes))
        })();
        match prepared {
            Ok((path, bytes)) => Ok(Self {
                source,
                destination,
                path,
                directory,
                bytes,
            }),
            Err(error) => Err(failed(error, cleanup(directory))),
        }
    }

    pub fn discard(self) -> Result<()> {
        let Self {
            path, directory, ..
        } = self;
        drop(path);
        cleanup(directory)
    }

    pub fn publish(self, review: ReviewId) -> Result<Published> {
        let checked = (|| {
            ensure!(
                self.source.range().review == review,
                "Save belongs to another review"
            );
            self.source.check()
        })();
        if let Err(error) = checked {
            return Err(failed(error, self.discard()));
        }
        let Self {
            destination,
            path,
            directory,
            bytes,
            ..
        } = self;
        match path.persist_noclobber(&destination) {
            Ok(()) => Ok(Published {
                destination,
                bytes,
                cleanup: cleanup(directory),
            }),
            Err(error) => {
                let cause = error.error;
                drop(error.path);
                Err(failed(
                    anyhow::Error::new(cause)
                        .context("Cannot save without overwriting; choose a new destination"),
                    cleanup(directory),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests;
