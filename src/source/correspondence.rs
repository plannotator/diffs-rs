use super::*;
use crate::storage::SourceRef;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PAIR: AtomicU64 = AtomicU64::new(1);

/// Published only after both supplied records and every omitted range match.
#[derive(Debug)]
pub struct CheckedPair {
    pub(crate) identity: u64,
    pub(crate) file: Arc<FileIndex>,
    old: Option<Arc<Snapshot>>,
    new: Option<Arc<Snapshot>>,
    pub(crate) gaps: Box<[Gap]>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Gap {
    pub old: u32,
    pub new: u32,
    pub len: u32,
    pub before: Option<usize>,
    pub no_newline: bool,
}

impl CheckedPair {
    pub fn file(&self) -> &Arc<FileIndex> {
        &self.file
    }
    pub fn old(&self) -> Option<&Arc<Snapshot>> {
        self.old.as_ref()
    }
    pub fn new_source(&self) -> Option<&Arc<Snapshot>> {
        self.new.as_ref()
    }

    pub(crate) fn shared_gap_after(
        &self,
        old: usize,
        new: usize,
    ) -> Option<(Range<usize>, Range<usize>)> {
        let index = self.gaps.partition_point(|gap| {
            gap.old as usize + gap.len as usize <= old || gap.new as usize + gap.len as usize <= new
        });
        let gap = self.gaps.get(index)?;
        let offset = old
            .saturating_sub(gap.old as usize)
            .max(new.saturating_sub(gap.new as usize));
        Some((
            gap.old as usize + offset..gap.old as usize + gap.len as usize,
            gap.new as usize + offset..gap.new as usize + gap.len as usize,
        ))
    }

    pub(crate) fn context_info(&self, old: u32, new: u32) -> Result<crate::storage::LineInfo> {
        let index = self.gaps.partition_point(|gap| gap.old < old);
        let gap = index
            .checked_sub(1)
            .and_then(|index| self.gaps.get(index))
            .context("Context line is outside the checked gaps")?;
        let offset = old - gap.old - 1;
        ensure!(
            offset < gap.len && new.checked_sub(gap.new) == Some(offset + 1),
            "Context line does not match the checked source pair"
        );
        Ok(crate::storage::LineInfo {
            kind: crate::model::Kind::Context,
            old: Some(old),
            new: Some(new),
            no_newline: gap.no_newline && offset + 1 == gap.len,
        })
    }
}

pub fn check_pair(
    file: Arc<FileIndex>,
    old: Option<Arc<Snapshot>>,
    new: Option<Arc<Snapshot>>,
    check: &impl Fn() -> Result<()>,
) -> Result<CheckedPair> {
    ensure!(
        file.status.as_ref() != "Binary change",
        "Binary entries do not supply textual context"
    );
    let mut old_line = 0;
    let mut new_line = 0;
    let mut comparison = ByteComparison::new(check);
    let mut gaps = Vec::new();
    for (index, hunk) in file.hunks.iter().enumerate() {
        check()?;
        let has_old = hunk.lines.clone().any(|id| file.info(id).old.is_some());
        let has_new = hunk.lines.clone().any(|id| file.info(id).new.is_some());
        let old_start = hunk
            .old_start
            .checked_sub(u32::from(has_old))
            .context("Invalid old source range")?;
        let new_start = hunk
            .new_start
            .checked_sub(u32::from(has_new))
            .context("Invalid new source range")?;
        gap(
            old.as_deref(),
            new.as_deref(),
            old_line..old_start,
            new_line..new_start,
            &mut comparison,
        )?;
        if old_start > old_line {
            gaps.push(Gap {
                old: old_line,
                new: new_line,
                len: old_start - old_line,
                before: Some(index),
                no_newline: false,
            });
        }
        old_line = old_start;
        new_line = new_start;
        let mut old_cursor = old
            .as_ref()
            .filter(|_| has_old)
            .map(|s| s.line_cursor(old_start))
            .transpose()?;
        let mut new_cursor = new
            .as_ref()
            .filter(|_| has_new)
            .map(|s| s.line_cursor(new_start))
            .transpose()?;
        for (id, reference) in hunk
            .lines
            .clone()
            .zip(file.source_refs(hunk.lines.clone())?)
        {
            check()?;
            let info = file.info(id);
            let reference = reference?;
            if let Some(number) = info.old {
                line(
                    &file,
                    reference,
                    info.no_newline,
                    old_cursor.as_mut(),
                    number,
                    &mut old_line,
                    &mut comparison,
                )?;
            }
            if let Some(number) = info.new {
                line(
                    &file,
                    reference,
                    info.no_newline,
                    new_cursor.as_mut(),
                    number,
                    &mut new_line,
                    &mut comparison,
                )?;
            }
        }
    }
    gap(
        old.as_deref(),
        new.as_deref(),
        old_line..old.as_ref().map_or(0, |s| s.lines()),
        new_line..new.as_ref().map_or(0, |s| s.lines()),
        &mut comparison,
    )?;
    let end = old.as_ref().map_or(0, |s| s.lines());
    if !file.hunks.is_empty() && end > old_line {
        gaps.push(Gap {
            old: old_line,
            new: new_line,
            len: end - old_line,
            before: None,
            no_newline: old.as_ref().is_some_and(|s| !s.final_newline()),
        });
    }
    let identity = NEXT_PAIR
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .map_err(|_| anyhow::anyhow!("Checked source identity exhausted"))?;
    Ok(CheckedPair {
        identity,
        file,
        old,
        new,
        gaps: gaps.into_boxed_slice(),
    })
}

fn line(
    file: &FileIndex,
    reference: SourceRef,
    no_newline: bool,
    cursor: Option<&mut LineCursor<'_>>,
    number: u32,
    next: &mut u32,
    comparison: &mut ByteComparison<'_>,
) -> Result<()> {
    let cursor =
        cursor.context("Source does not match this diff: absent side has a source line")?;
    let source = cursor.source;
    let end = next.checked_add(1).context("Source line overflow")?;
    ensure!(
        number == end,
        "Source does not match this diff: incorrect line number"
    );
    ensure!(
        no_newline == (number == source.lines && !source.final_newline),
        "Source does not match this diff: final newline differs"
    );
    comparison.equal(cursor.next()?, file.spool.source_reader(reference)?)?;
    *next = end;
    Ok(())
}

fn gap(
    old: Option<&Snapshot>,
    new: Option<&Snapshot>,
    a: Range<u32>,
    b: Range<u32>,
    comparison: &mut ByteComparison<'_>,
) -> Result<()> {
    ensure!(
        a.start <= a.end && b.start <= b.end && a.end - a.start == b.end - b.start,
        "Source does not match this diff: omitted line ranges differ"
    );
    if a.is_empty() {
        return Ok(());
    }
    let old = old.context("Source does not match this diff: absent old context")?;
    let new = new.context("Source does not match this diff: absent new context")?;
    comparison.equal(old.uncached_range(a)?, new.uncached_range(b)?)
}
