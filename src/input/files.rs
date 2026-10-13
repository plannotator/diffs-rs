use super::Cancellation;
use crate::{
    source::{Snapshot, SourceStore},
    storage::ByteStore,
};
use anyhow::{Context, Result, ensure};
use std::{
    fs::OpenOptions,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    sync::Arc,
};

pub(super) fn capture(
    path: &Path,
    cache: &ByteStore,
    cancel: &Cancellation,
) -> Result<(Arc<Snapshot>, tempfile::TempPath)> {
    let mut input = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .with_context(|| format!("Cannot open {}", path.display()))?;
    let before = input.metadata()?;
    ensure!(before.is_file(), "Two-file inputs must be regular files");
    let (mut storage, private_path) = SourceStore::named_sharing(cache)?;
    let snapshot = storage.capture(&mut input, &|| cancel.check())?;
    let after = input.metadata()?;
    ensure!(
        before.dev() == after.dev()
            && before.ino() == after.ino()
            && before.len() == after.len()
            && before.mtime() == after.mtime()
            && before.mtime_nsec() == after.mtime_nsec()
            && before.ctime() == after.ctime()
            && before.ctime_nsec() == after.ctime_nsec()
            && snapshot.len() == after.len(),
        "Input changed during capture. Open stable files."
    );
    Ok((snapshot, private_path))
}
