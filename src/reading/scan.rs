//! Raw patch-byte scanning for literal search.
//!
//! A file's patch source records lie in the spool in record order, separated by signs,
//! hunk headers, and markers. A raw occurrence counts only when it lies entirely inside one
//! record's source bytes, so headers, paths, signs, and markers never match.

use anyhow::Result;

use super::literal::Literal;
use crate::storage::{FileIndex, PatchScan, SpanWindow};

/// Cancellation granularity for raw scans.
const SLICE: usize = 16 * 1024;

#[derive(Clone, Copy)]
struct Line {
    id: usize,
    start: u64,
    end: u64,
}

impl Line {
    fn read(spans: &mut SpanWindow, file: &FileIndex, id: usize) -> Result<Self> {
        let (start, len) = spans.get(file, id)?;
        Ok(Self {
            id,
            start,
            end: start + len as u64,
        })
    }

    fn contains(&self, hit: u64, needle: usize) -> bool {
        self.start <= hit && hit + needle as u64 <= self.end
    }
}

/// The spool offset just past a file's last patch source record.
pub(super) fn file_end(spans: &mut SpanWindow, file: &FileIndex) -> Result<u64> {
    Ok(Line::read(spans, file, file.source_end() - 1)?.end)
}

/// The first record from `line` onward whose end lies after `hit`, by galloping search.
fn seek_forward(
    spans: &mut SpanWindow,
    file: &FileIndex,
    line: Line,
    hit: u64,
) -> Result<Option<Line>> {
    if line.end > hit {
        return Ok(Some(line));
    }
    let last = file.source_end() - 1;
    // Invariant: `below` ends at or before `hit`; `above` ends after it.
    let mut below = line.id;
    let mut step = 1;
    let mut above = loop {
        if below == last {
            return Ok(None);
        }
        let probe = below.saturating_add(step).min(last);
        if Line::read(spans, file, probe)?.end > hit {
            break probe;
        }
        below = probe;
        step *= 2;
    };
    while above - below > 1 {
        let middle = below + (above - below) / 2;
        if Line::read(spans, file, middle)?.end > hit {
            above = middle;
        } else {
            below = middle;
        }
    }
    Line::read(spans, file, above).map(Some)
}

/// The last record from `line` backward whose start is at or before `hit`.
fn seek_backward(
    spans: &mut SpanWindow,
    file: &FileIndex,
    line: Line,
    hit: u64,
) -> Result<Option<Line>> {
    if line.start <= hit {
        return Ok(Some(line));
    }
    let first = file.source_base;
    // Invariant: `below` starts at or before `hit`; `above` starts after it.
    let mut above = line.id;
    let mut step = 1;
    let mut below = loop {
        if above == first {
            return Ok(None);
        }
        let probe = above.saturating_sub(step).max(first);
        if Line::read(spans, file, probe)?.start <= hit {
            break probe;
        }
        above = probe;
        step *= 2;
    };
    while above - below > 1 {
        let middle = below + (above - below) / 2;
        if Line::read(spans, file, middle)?.start <= hit {
            below = middle;
        } else {
            above = middle;
        }
    }
    Line::read(spans, file, below).map(Some)
}

/// The first occurrence inside one source record that starts in `begin..limit`.
/// `line` is the record that holds `begin`, or the first record after it.
/// Returns the record and the occurrence's byte offset within it.
pub(super) fn first(
    scan: &mut PatchScan,
    literal: &Literal<'_>,
    file: &FileIndex,
    line: usize,
    begin: u64,
    limit: u64,
    check: &impl Fn() -> Result<()>,
) -> Result<Option<(usize, u64)>> {
    let PatchScan { bytes, spans } = scan;
    let needle = literal.len();
    let end = file_end(spans, file)?.min(limit.saturating_add(needle as u64 - 1));
    let mut line = Line::read(spans, file, line)?;
    let mut position = begin;
    while position + needle as u64 <= end {
        check()?;
        let window = bytes.after(&file.spool, position, end, needle)?;
        let window = &window[..window.len().min(SLICE + needle - 1)];
        let mut from = 0;
        while let Some(index) = literal.forward.find(&window[from..]) {
            let hit = position + (from + index) as u64;
            let Some(next) = seek_forward(spans, file, line, hit)? else {
                return Ok(None);
            };
            line = next;
            if line.contains(hit, needle) {
                return Ok(Some((line.id, hit - line.start)));
            }
            from += index + 1;
        }
        position += (window.len() - needle + 1) as u64;
    }
    Ok(None)
}

/// The last occurrence inside one source record that starts in `floor..limit`.
pub(super) fn last(
    scan: &mut PatchScan,
    literal: &Literal<'_>,
    file: &FileIndex,
    floor: u64,
    limit: u64,
    check: &impl Fn() -> Result<()>,
) -> Result<Option<(usize, u64)>> {
    let PatchScan { bytes, spans } = scan;
    let needle = literal.len();
    let mut line = Line::read(spans, file, file.source_end() - 1)?;
    let mut end = line.end.min(limit.saturating_add(needle as u64 - 1));
    let mut hits = Vec::new();
    while end >= floor + needle as u64 {
        check()?;
        let (start, window) = bytes.before(&file.spool, floor, end, needle)?;
        let skip = window.len().saturating_sub(SLICE + needle - 1);
        let (start, window) = (start + skip as u64, &window[skip..]);
        // memchr's reverse searcher has no vector prefilter, so collect this slice's
        // occurrences forward and validate them from the end.
        hits.clear();
        let mut from = 0;
        while let Some(index) = literal.forward.find(&window[from..]) {
            hits.push(start + (from + index) as u64);
            from += index + 1;
        }
        for &hit in hits.iter().rev() {
            let Some(previous) = seek_backward(spans, file, line, hit)? else {
                return Ok(None);
            };
            line = previous;
            if line.contains(hit, needle) {
                return Ok(Some((line.id, hit - line.start)));
            }
        }
        if start == floor {
            break;
        }
        end = start + needle as u64 - 1;
    }
    Ok(None)
}
