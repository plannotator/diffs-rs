//! Source-faithful reads, independent of display rows and terminal ownership.

use std::{
    collections::HashMap,
    io::{BufReader, Write},
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::{Result, ensure};

use crate::{
    engine::{ReviewId, Side},
    source::{CheckedPair, Snapshot},
    storage::{FileIndex, copy_checked},
};

pub(crate) mod domain;
pub(crate) mod literal;
pub(crate) mod search;
mod search_read;
pub use literal::Direction as SearchDirection;
pub(crate) use search_read::SearchStart;
pub use search_read::{SearchMatch, SearchRead};

/// An inclusive range of original lines on one side of one open review.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRange {
    pub review: ReviewId,
    pub file: usize,
    pub side: Side,
    pub first: u32,
    pub last: u32,
}

pub(crate) struct LineSelection {
    pub range: SourceRange,
    pub anchor: u32,
    pub extending: bool,
    pub checked: bool,
}

enum Source {
    Patch(Arc<FileIndex>),
    Full {
        snapshot: Arc<Snapshot>,
        valid: Arc<AtomicBool>,
    },
}

#[derive(Default)]
pub(crate) struct SourceLeases {
    files: HashMap<usize, Weak<AtomicBool>>,
}

impl SourceLeases {
    pub(crate) fn lease(&mut self, file: usize) -> Arc<AtomicBool> {
        self.files.retain(|_, lease| lease.strong_count() != 0);
        if let Some(lease) = self.files.get(&file).and_then(Weak::upgrade) {
            return lease;
        }
        let lease = Arc::new(AtomicBool::new(true));
        self.files.insert(file, Arc::downgrade(&lease));
        lease
    }

    pub(crate) fn invalidate(&mut self, file: usize) {
        if let Some(lease) = self.files.remove(&file).and_then(|lease| lease.upgrade()) {
            lease.store(false, Ordering::Release);
        }
    }
}

/// A source job. Drop it after use to release its source ownership.
/// Closing the engine revokes reads; it does not drop a caller-owned job.
pub struct SourceRead {
    range: SourceRange,
    source: Source,
    active: Arc<AtomicBool>,
}

impl SourceRead {
    pub(crate) fn new(
        range: SourceRange,
        file: Arc<FileIndex>,
        pair: Option<(&Arc<CheckedPair>, Arc<AtomicBool>)>,
        active: Arc<AtomicBool>,
    ) -> Result<Self> {
        ensure!(
            range.first != 0 && range.first <= range.last,
            "Invalid source line range"
        );
        ensure!(
            file.id == range.file,
            "Source range belongs to another file"
        );
        let source = if let Some((pair, valid)) = pair {
            ensure!(
                Arc::ptr_eq(pair.file(), &file),
                "Full source belongs to another file occurrence"
            );
            let snapshot = match range.side {
                Side::Old => pair.old(),
                Side::New => pair.new_source(),
            };
            let snapshot =
                snapshot.ok_or_else(|| anyhow::anyhow!("The requested side has no source"))?;
            ensure!(
                range.last <= snapshot.lines(),
                "Source range is outside the full file"
            );
            Source::Full {
                snapshot: snapshot.clone(),
                valid,
            }
        } else {
            Source::Patch(file)
        };
        Ok(Self {
            range,
            source,
            active,
        })
    }

    pub fn range(&self) -> SourceRange {
        self.range
    }

    /// Checks source identity without reading bytes. Use before publishing prepared output.
    pub fn check(&self) -> Result<()> {
        ensure!(
            self.active.load(Ordering::Acquire),
            "Source read belongs to a closed review"
        );
        if let Source::Full { valid, .. } = &self.source {
            ensure!(
                valid.load(Ordering::Acquire),
                "Checked source is no longer available"
            );
        }
        Ok(())
    }

    /// Streams exact source bytes without flushing the caller's output.
    /// Run off the input thread. Cancellation is checked between bounded copies.
    /// The caller must unblock a sink if its `Write` implementation can wait indefinitely.
    /// A failed sink can contain partial output; stage before visible publication.
    pub fn write_to(
        &self,
        output: &mut impl Write,
        cancel: &impl Fn() -> Result<()>,
    ) -> Result<u64> {
        let check = || {
            self.check()?;
            cancel()
        };
        check()?;
        let written = match &self.source {
            Source::Patch(file) => file.write_source_lines(
                self.range.side == Side::Old,
                self.range.first..=self.range.last,
                output,
                &check,
            )?,
            Source::Full { snapshot, .. } => {
                let reader = snapshot.uncached_range(self.range.first - 1..self.range.last)?;
                let len = reader.remaining();
                copy_checked(
                    &mut BufReader::with_capacity(64 * 1024, reader),
                    output,
                    len,
                    &check,
                )?;
                len
            }
        };
        check()?;
        Ok(written)
    }
}
