use std::{
    io::{BufRead, BufReader, Read},
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use crate::{
    source::{CheckedPair, Snapshot, SourceStore, check_pair},
    storage::{self, FileIndex, StoreWriter},
};

mod files;
mod git;
mod metadata;
mod patch;
#[cfg(test)]
mod tests;
mod worker;
pub use worker::{Message, Worker};

pub enum PatchInput {
    Path(PathBuf),
    /// Reads from the current offset. Requires exclusive control of reading, the shared
    /// cursor, and status flags, including any retained duplicates, until worker close.
    Stream(std::fs::File),
}

#[derive(Clone, Debug)]
pub enum LocalInput {
    Files {
        old: PathBuf,
        new: PathBuf,
    },
    Git {
        repo: PathBuf,
        base: String,
        head: String,
    },
}

impl LocalInput {
    pub fn label(&self) -> String {
        let label = match self {
            Self::Files { old, new } => format!("{} → {}", old.display(), new.display()),
            Self::Git { repo, base, head } => format!("{} · {base} → {head}", repo.display()),
        };
        crate::text::safe_label(label.as_bytes())
    }
}

#[derive(Debug)]
pub struct Cancelled;
impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Source preparation canceled")
    }
}
impl std::error::Error for Cancelled {}

#[derive(Clone, Default)]
pub struct Cancellation {
    stopped: Arc<AtomicBool>,
    destination: Option<(Arc<AtomicUsize>, usize)>,
}

impl Cancellation {
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
    }
    pub fn check(&self) -> Result<()> {
        if self.stopped.load(Ordering::Acquire)
            || self
                .destination
                .as_ref()
                .is_some_and(|(current, expected)| current.load(Ordering::Acquire) != *expected)
        {
            return Err(Cancelled.into());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Origin {
    Files {
        old: PathBuf,
        new: PathBuf,
    },
    Git {
        repository: PathBuf,
        base: String,
        head: String,
    },
}

/// Unique within this process, independent of display labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ReviewId(u64);

#[derive(Clone, Debug)]
pub enum SourceState {
    Pending,
    Checked(Arc<CheckedPair>),
    Metadata,
    Unavailable(Arc<str>),
}

impl SourceState {
    pub fn status(&self) -> &'static str {
        match self {
            Self::Pending => "Preparing source",
            Self::Checked(_) => "Source checked",
            Self::Metadata => "Metadata only",
            Self::Unavailable(_) => "Source unavailable",
        }
    }
}

enum Provider {
    Files {
        old: Arc<Snapshot>,
        new: Arc<Snapshot>,
    },
    Git {
        git: git::Git,
        entries: Vec<metadata::Entry>,
    },
}

/// One complete local comparison. Source readiness is separate and belongs to each file occurrence.
pub struct Review {
    id: ReviewId,
    doc: storage::Document,
    origin: Origin,
    sources: Vec<SourceState>,
    provider: Provider,
    storage: SourceStore,
}

impl Review {
    pub fn id(&self) -> ReviewId {
        self.id
    }
    pub fn doc(&self) -> &storage::Document {
        &self.doc
    }
    pub fn origin(&self) -> &Origin {
        &self.origin
    }
    pub fn sources(&self) -> &[SourceState] {
        &self.sources
    }
    pub fn open(
        input: LocalInput,
        cancel: &Cancellation,
        mut publish: impl FnMut(Arc<FileIndex>) -> Result<()>,
    ) -> Result<Self> {
        let patch_writer = StoreWriter::new()?;
        let cache = patch_writer.store();
        let mut doc = storage::Document::default();
        let (origin, provider) = match input {
            LocalInput::Files { old, new } => {
                let (old_source, old_private) = files::capture(&old, &cache, cancel)?;
                let (new_source, new_private) = files::capture(&new, &cache, cancel)?;
                let git = git::Git::new(
                    old_private
                        .parent()
                        .context("Private capture has no directory")?,
                    cancel,
                )?;
                let mut args: Vec<&std::ffi::OsStr> =
                    git::DIFF.iter().map(std::ffi::OsStr::new).collect();
                args.extend([
                    "--no-index".as_ref(),
                    "--no-renames".as_ref(),
                    "--".as_ref(),
                    old_private.as_os_str(),
                    new_private.as_os_str(),
                ]);
                let output = git.run(&args, None, true, cancel)?;
                storage::read_with(
                    Cancellable {
                        input: output,
                        cancel,
                    },
                    patch_writer,
                    &|| cancel.check(),
                    |mut file, _| {
                        cancel.check()?;
                        ensure!(
                            file.id == 0,
                            "Two-file comparison returned multiple file entries"
                        );
                        let index = Arc::get_mut(&mut file)
                            .context("File entry was shared before binding")?;
                        index.meta.old_path =
                            crate::text::safe_label(old.as_os_str().as_bytes()).into_boxed_str();
                        index.meta.path =
                            crate::text::safe_label(new.as_os_str().as_bytes()).into_boxed_str();
                        index.meta.raw_path = new.as_os_str().as_bytes().into();
                        doc.append(file.clone())?;
                        publish(file)
                    },
                )?;
                (
                    Origin::Files { old, new },
                    Provider::Files {
                        old: old_source,
                        new: new_source,
                    },
                )
            }
            LocalInput::Git { repo, base, head } => {
                let git = git::Git::new(&repo, cancel)?;
                ensure!(
                    git.small(&["rev-parse", "--show-object-format"], cancel)? == b"sha1\n",
                    "Only SHA-1 Git repositories are supported"
                );
                let directory = git.small(&["rev-parse", "--absolute-git-dir"], cancel)?;
                let repository = PathBuf::from(std::ffi::OsString::from_vec(
                    directory
                        .strip_suffix(b"\n")
                        .context("Invalid repository identity")?
                        .to_vec(),
                ));
                let base = git.pin(&base, cancel)?.to_string();
                let head = git.pin(&head, cancel)?.to_string();
                let mut args: Vec<&std::ffi::OsStr> =
                    git::DIFF.iter().map(std::ffi::OsStr::new).collect();
                args.extend(
                    [
                        "--raw",
                        "-z",
                        "--abbrev=40",
                        "--no-renames",
                        "--find-renames=50%",
                        "-l1000",
                        &base,
                        &head,
                        "--",
                    ]
                    .map(std::ffi::OsStr::new),
                );
                let output = git.run(&args, None, false, cancel)?;
                let mut output = BufReader::new(output);
                let entries = metadata::envelope(&mut output, cancel)?;
                storage::read_with(
                    Cancellable {
                        input: output,
                        cancel,
                    },
                    patch_writer,
                    &|| cancel.check(),
                    |mut file, identity| {
                        cancel.check()?;
                        entries
                            .get(file.id)
                            .context("Patch has more entries than its Git inventory")?
                            .matches(&file, &identity)?;
                        let entry = &entries[file.id];
                        let index = Arc::get_mut(&mut file)
                            .context("File entry was shared before binding")?;
                        index.meta.old_mode = format!("{:o}", entry.old.mode).into_boxed_str();
                        index.meta.new_mode = format!("{:o}", entry.new.mode).into_boxed_str();
                        doc.append(file.clone())?;
                        publish(file)
                    },
                )?;
                ensure!(
                    doc.files.len() == entries.len(),
                    "Git inventory has entries missing from its patch"
                );
                (
                    Origin::Git {
                        repository,
                        base,
                        head,
                    },
                    Provider::Git { git, entries },
                )
            }
        };
        let sources = doc
            .files
            .iter()
            .map(|file| {
                if file.status.as_ref() == "Binary change"
                    || file.old_mode.as_ref() == "160000"
                    || file.new_mode.as_ref() == "160000"
                {
                    SourceState::Metadata
                } else {
                    SourceState::Pending
                }
            })
            .collect();
        Ok(Self {
            id: ReviewId(cache.identity()),
            doc,
            origin,
            sources,
            provider,
            storage: SourceStore::sharing(&cache)?,
        })
    }

    pub fn prepare(&mut self, id: usize, cancel: &Cancellation) -> Result<SourceState> {
        cancel.check()?;
        let state = self
            .sources
            .get(id)
            .context("Source request is outside this review")?;
        if !matches!(state, SourceState::Pending) {
            return Ok(state.clone());
        }
        let checkpoint = self.storage.checkpoint();
        let result = self.prepare_pair(id, cancel);
        let state = match result {
            Ok(pair) => SourceState::Checked(Arc::new(pair)),
            Err(error) => {
                self.storage.rollback(checkpoint)?;
                if error.is::<Cancelled>() || error.is::<crate::source_spans::ReadFailure>() {
                    return Err(error);
                }
                SourceState::Unavailable(
                    crate::text::safe_text(format!("{error:#}").as_bytes()).into(),
                )
            }
        };
        self.sources[id] = state.clone();
        Ok(state)
    }

    fn prepare_pair(&mut self, id: usize, cancel: &Cancellation) -> Result<CheckedPair> {
        cancel.check()?;
        let (old, new) = match &self.provider {
            Provider::Files { old, new } => (Some(old.clone()), Some(new.clone())),
            Provider::Git { git, entries } => {
                let entry = &entries[id];
                let ids: Vec<_> = [&entry.old, &entry.new]
                    .iter()
                    .filter(|side| side.mode != 0)
                    .map(|side| side.oid)
                    .collect();
                let mut output = BufReader::new(git.objects(&ids, cancel)?);
                let mut capture = |side: &metadata::Endpoint| -> Result<Option<Arc<Snapshot>>> {
                    if side.mode == 0 {
                        return Ok(None);
                    }
                    let header = metadata::field(&mut output, b'\n')?;
                    let mut fields = header.split(|byte| *byte == b' ');
                    ensure!(
                        git::oid(fields.next().context("Missing object ID")?)? == side.oid,
                        "Git delivered the wrong source object"
                    );
                    ensure!(
                        fields.next() == Some(b"blob"),
                        "Git source object is not a blob"
                    );
                    let length: u64 =
                        std::str::from_utf8(fields.next().context("Missing object length")?)?
                            .parse()?;
                    ensure!(fields.next().is_none(), "Invalid object header");
                    let source = self
                        .storage
                        .capture(output.by_ref().take(length), &|| cancel.check())?;
                    ensure!(source.len() == length, "Truncated source object");
                    let mut newline = [0];
                    output.read_exact(&mut newline)?;
                    ensure!(newline == [b'\n'], "Invalid source object terminator");
                    Ok(Some(source))
                };
                let old = capture(&entry.old)?;
                let new = capture(&entry.new)?;
                ensure!(
                    output.fill_buf()?.is_empty(),
                    "Unexpected data after source objects"
                );
                (old, new)
            }
        };
        check_pair(self.doc.files[id].clone(), old, new, &|| cancel.check())
    }

    pub fn report(&mut self, cancel: &Cancellation) -> Result<serde_json::Value> {
        let mut files = Vec::new();
        for id in 0..self.doc.files.len() {
            let state = self.prepare(id, cancel)?;
            let sides = if let SourceState::Checked(pair) = &state {
                let describe = |side: Option<&Arc<Snapshot>>| {
                    side.map(|s| serde_json::json!({ "bytes": s.len(), "lines": s.lines(), "final_newline": s.final_newline() }))
                };
                serde_json::json!({ "old": describe(pair.old()), "new": describe(pair.new_source()) })
            } else {
                serde_json::Value::Null
            };
            let provenance = match &self.provider {
                Provider::Git { entries, .. } => {
                    let endpoint = |side: &metadata::Endpoint| serde_json::json!({"path_bytes": side.path, "mode": format!("{:o}", side.mode), "object": (!side.oid.is_zero()).then(|| side.oid.to_string())});
                    serde_json::json!({"old": endpoint(&entries[id].old), "new": endpoint(&entries[id].new)})
                }
                Provider::Files { .. } => serde_json::Value::Null,
            };
            files.push(serde_json::json!({ "file": id, "path": self.doc.files[id].path, "source": state.status(), "error": if let SourceState::Unavailable(reason) = &state { Some(reason.as_ref()) } else { None }, "sides": sides, "provenance": provenance }));
        }
        let source_storage_bytes = match &self.provider {
            Provider::Files { old, new } => [old, new]
                .iter()
                .map(|s| s.len() + (u64::from(s.lines()) + 1) * 8)
                .sum(),
            Provider::Git { .. } => self.storage.bytes(),
        };
        Ok(
            serde_json::json!({"review_id": self.id, "comparison": self.origin, "completeness": "complete", "files": files, "source_storage_bytes": source_storage_bytes}),
        )
    }
}

struct Cancellable<'a, R> {
    input: R,
    cancel: &'a Cancellation,
}
impl<R: Read> Read for Cancellable<'_, R> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.cancel.check().map_err(std::io::Error::other)?;
        self.input.read(bytes)
    }
}
