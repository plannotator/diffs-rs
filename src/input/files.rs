use super::Cancellation;
use crate::{
    source::{Snapshot, SourceStore},
    storage::ByteStore,
};
use anyhow::{Context, Result, ensure};
use std::{
    fs::{Metadata, OpenOptions},
    path::Path,
    sync::Arc,
};

pub(super) fn capture(
    path: &Path,
    cache: &ByteStore,
    cancel: &Cancellation,
) -> Result<(Arc<Snapshot>, tempfile::TempPath)> {
    let mut options = OpenOptions::new();
    options.read(true);
    // A FIFO given as a file must not block the open; it is refused below as not regular.
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::custom_flags(&mut options, libc::O_NONBLOCK);
    let mut input = options
        .open(path)
        .with_context(|| format!("Cannot open {}", path.display()))?;
    let before = input.metadata()?;
    ensure!(before.is_file(), "Two-file inputs must be regular files");
    let (mut storage, private_path) = SourceStore::named_sharing(cache)?;
    let snapshot = storage.capture(&mut input, &|| cancel.check())?;
    let after = input.metadata()?;
    ensure!(
        unchanged(&before, &after)? && snapshot.len() == after.len(),
        "Input changed during capture. Open stable files."
    );
    Ok((snapshot, private_path))
}

/// Whether a file is the same one, unchanged, before and after it was read.
#[cfg(unix)]
fn unchanged(before: &Metadata, after: &Metadata) -> Result<bool> {
    use std::os::unix::fs::MetadataExt;
    Ok(before.dev() == after.dev()
        && before.ino() == after.ino()
        && before.len() == after.len()
        && before.mtime() == after.mtime()
        && before.mtime_nsec() == after.mtime_nsec()
        && before.ctime() == after.ctime()
        && before.ctime_nsec() == after.ctime_nsec())
}

/// Whether a file is unchanged before and after it was read: its length and its last write
/// time (Windows' stable file identity and change time are not in std).
#[cfg(not(unix))]
fn unchanged(before: &Metadata, after: &Metadata) -> Result<bool> {
    Ok(before.len() == after.len() && before.modified()? == after.modified()?)
}
