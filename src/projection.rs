use std::{num::NonZeroU32, ops::Range};

use crate::model::{
    Document, FileDiff, Kind, Row, RowKind, SourceLine, alignment_needed, alignment_offset,
};

mod storage;
use storage::{TableBuilder, Tables, compact_fits};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Header,
    Hunk(usize),
    Gap,
    Metadata,
    Context { source_start: usize },
    Replacement(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Block {
    start: [usize; 2],
    kind: BlockKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Replacement {
    old_start: usize,
    new_start: usize,
    end: usize,
    old_pad: usize,
    new_pad: usize,
}

impl Replacement {
    fn old_range(&self) -> Range<usize> {
        self.old_start..self.new_start
    }

    fn new_range(&self) -> Range<usize> {
        self.new_start..self.end
    }
}

const _: () = assert!(size_of::<Replacement>() == 5 * size_of::<usize>());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceRun {
    start: usize,
    block: usize,
    ordinal: usize,
    old: Option<NonZeroU32>,
    new: Option<NonZeroU32>,
    kind: Kind,
}

#[derive(Debug)]
struct Marker {
    logical: usize,
    display: usize,
    left: bool,
    right: bool,
}

#[derive(Debug, Default)]
pub struct Projection {
    tables: Tables,
    markers: [Box<[Marker]>; 2],
    hunks: Box<[[usize; 2]]>,
    len: [usize; 2],
    source_len: usize,
}

impl Projection {
    pub fn new(parsed: &Document, meta: &FileDiff) -> Self {
        let base = meta.hunks.first().map_or(0, |h| h.lines.start);
        let end = meta.hunks.last().map_or(base, |h| h.lines.end);
        Self::build(
            parsed,
            meta,
            base,
            end,
            compact_fits(end - base, meta.hunks.len()),
        )
    }

    fn build(parsed: &Document, meta: &FileDiff, base: usize, end: usize, compact: bool) -> Self {
        let mut builder = Builder {
            tables: TableBuilder::new(compact),
            len: [0; 2],
        };
        builder.structural(BlockKind::Header);
        if meta.hunks.is_empty() {
            builder.structural(BlockKind::Metadata);
        }
        builder.file(parsed, meta, base);
        let mut projection = Self {
            tables: builder.tables.finish(),
            len: builder.len,
            source_len: end - base,
            ..Self::default()
        };
        for (layout, split) in [false, true].into_iter().enumerate() {
            let mut markers: Vec<Marker> = parsed.lines[base..end]
                .iter()
                .enumerate()
                .filter(|(_, line)| line.no_newline)
                .map(|(id, line)| Marker {
                    logical: projection.logical_source_row(id, split),
                    display: 0,
                    left: line.old.is_some(),
                    right: line.new.is_some(),
                })
                .collect();
            markers.sort_unstable_by_key(|m| m.logical);
            markers.dedup_by(|next, previous| {
                if next.logical != previous.logical {
                    return false;
                }
                previous.left |= next.left;
                previous.right |= next.right;
                true
            });
            for (n, marker) in markers.iter_mut().enumerate() {
                marker.display = marker.logical + n + 1;
            }
            projection.markers[layout] = markers.into_boxed_slice();
        }
        projection.hunks = (0..projection.tables.block_count())
            .map(|n| projection.tables.block(n))
            .filter(|b| matches!(b.kind, BlockKind::Hunk(_)))
            .map(|b| {
                [
                    projection.display_row(b.start[0], false),
                    projection.display_row(b.start[1], true),
                ]
            })
            .collect();
        projection
    }

    fn run(&self, id: usize) -> SourceRun {
        self.tables.run_at(id)
    }

    fn run_len(&self, index: usize) -> usize {
        self.tables.run_end(index, self.source_len) - self.tables.run(index).start
    }

    fn side_len(&self, side: &Range<usize>) -> usize {
        if side.is_empty() {
            0
        } else {
            let r = self.tables.side_run(side.end - 1);
            self.tables.run(r).ordinal + self.run_len(r)
        }
    }

    fn side_source(&self, side: &Range<usize>, ordinal: usize) -> Option<usize> {
        let r = self.tables.side_run_at(side.clone(), ordinal)?;
        let run = self.tables.run(r);
        let delta = ordinal - run.ordinal;
        (delta < self.run_len(r)).then_some(run.start + delta)
    }

    pub fn info(&self, id: usize) -> (Kind, Option<u32>, Option<u32>) {
        let run = self.run(id);
        let delta = (id - run.start) as u32;
        (
            run.kind,
            run.old.map(|n| n.get() + delta),
            run.new.map(|n| n.get() + delta),
        )
    }

    pub(crate) fn source_intervals(
        &self,
        old: bool,
    ) -> impl Iterator<Item = (Range<usize>, Option<u32>)> + '_ {
        (0..self.tables.run_count()).map(move |index| {
            let run = self.tables.run(index);
            let line = if old { run.old } else { run.new };
            (
                run.start..run.start + self.run_len(index),
                line.map(NonZeroU32::get),
            )
        })
    }

    fn logical_source_row(&self, id: usize, split: bool) -> usize {
        let run = self.run(id);
        let block = self.tables.block(run.block);
        let ordinal = run.ordinal + id - run.start;
        block.start[usize::from(split)]
            + match &block.kind {
                BlockKind::Context { source_start } => id - source_start,
                BlockKind::Replacement(r) => {
                    let r = self.tables.replacement(*r);
                    match (split, run.kind) {
                        (true, Kind::Deletion) => r.old_pad + ordinal,
                        (true, _) => r.new_pad + ordinal,
                        (false, Kind::Deletion) => ordinal,
                        (false, _) => self.side_len(&r.old_range()) + ordinal,
                    }
                }
                _ => unreachable!("A source run must belong to a source block"),
            }
    }

    fn display_row(&self, logical: usize, split: bool) -> usize {
        logical + self.markers[usize::from(split)].partition_point(|m| m.logical < logical)
    }

    pub fn row_for_source(&self, id: usize, split: bool) -> usize {
        self.display_row(self.logical_source_row(id, split), split)
    }

    pub(crate) fn hunk_row(&self, index: usize, split: bool) -> usize {
        self.hunks[index][usize::from(split)]
    }

    pub fn rows(&self, split: bool, file: usize, source_base: usize) -> Rows<'_> {
        Rows {
            projection: self,
            split,
            file,
            source_base,
        }
    }

    pub fn source_run_bytes(&self) -> usize {
        self.tables.source_run_bytes()
    }
    pub fn block_bytes(&self) -> usize {
        self.tables.block_bytes()
    }
    pub fn side_index_bytes(&self) -> usize {
        self.tables.side_index_bytes()
    }
    pub fn marker_bytes(&self) -> usize {
        self.markers
            .iter()
            .map(|m| m.len() * size_of::<Marker>())
            .sum()
    }
    pub fn boundary_bytes(&self) -> usize {
        self.hunks.len() * size_of::<[usize; 2]>()
    }
    pub fn bytes(&self) -> usize {
        self.source_run_bytes()
            + self.block_bytes()
            + self.side_index_bytes()
            + self.marker_bytes()
            + self.boundary_bytes()
    }
}

pub struct Rows<'a> {
    projection: &'a Projection,
    split: bool,
    file: usize,
    source_base: usize,
}

impl Rows<'_> {
    pub fn len(&self) -> usize {
        let layout = usize::from(self.split);
        self.projection.len[layout] + self.projection.markers[layout].len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<Row> {
        if index >= self.len() {
            return None;
        }
        let p = self.projection;
        let layout = usize::from(self.split);
        let markers = &p.markers[layout];
        let before = markers.partition_point(|m| m.display <= index);
        if let Some(marker) = before.checked_sub(1).map(|i| &markers[i])
            && marker.display == index
        {
            return Some(Row {
                file: self.file,
                kind: RowKind::NoNewline {
                    left: marker.left,
                    right: marker.right,
                },
            });
        }
        let logical = index - before;
        let block = p.tables.block_at(layout, logical);
        let ordinal = logical - block.start[layout];
        let kind = match &block.kind {
            BlockKind::Header => RowKind::Header,
            BlockKind::Hunk(h) => RowKind::Hunk(*h),
            BlockKind::Gap => RowKind::Gap,
            BlockKind::Metadata => RowKind::Metadata,
            BlockKind::Context { source_start } => RowKind::Code {
                left: Some(self.source_base + source_start + ordinal),
                right: Some(self.source_base + source_start + ordinal),
            },
            BlockKind::Replacement(r) => {
                let r = p.tables.replacement(*r);
                let (left, right) = if self.split {
                    (
                        ordinal
                            .checked_sub(r.old_pad)
                            .and_then(|n| p.side_source(&r.old_range(), n)),
                        ordinal
                            .checked_sub(r.new_pad)
                            .and_then(|n| p.side_source(&r.new_range(), n)),
                    )
                } else {
                    let old_len = p.side_len(&r.old_range());
                    if ordinal < old_len {
                        (p.side_source(&r.old_range(), ordinal), None)
                    } else {
                        (None, p.side_source(&r.new_range(), ordinal - old_len))
                    }
                };
                RowKind::Code {
                    left: left.map(|n| n + self.source_base),
                    right: right.map(|n| n + self.source_base),
                }
            }
        };
        Some(Row {
            file: self.file,
            kind,
        })
    }

    pub fn boundary(&self, index: usize, forward: bool, file: bool) -> Option<usize> {
        if file {
            return (!forward && index > 0).then_some(0);
        }
        let h = &self.projection.hunks;
        let layout = usize::from(self.split);
        if forward {
            h.get(h.partition_point(|b| b[layout] <= index))
                .map(|b| b[layout])
        } else {
            h.partition_point(|b| b[layout] < index)
                .checked_sub(1)
                .map(|i| h[i][layout])
        }
    }

    pub fn first_boundary(&self, forward: bool, file: bool) -> Option<usize> {
        if file {
            return Some(0);
        }
        let h = &self.projection.hunks;
        (if forward { h.first() } else { h.last() }).map(|b| b[usize::from(self.split)])
    }
}

struct Builder {
    tables: TableBuilder,
    len: [usize; 2],
}

impl Builder {
    fn structural(&mut self, kind: BlockKind) {
        self.tables.push_block(Block {
            start: self.len,
            kind,
        });
        self.len[0] += 1;
        self.len[1] += 1;
    }

    fn source_runs(
        &mut self,
        lines: &[SourceLine],
        base: usize,
        range: Range<usize>,
    ) -> Range<usize> {
        let first = self.tables.run_count();
        let mut old_ordinal = 0;
        let mut new_ordinal = 0;
        for i in range {
            let line = &lines[i];
            let local = i - base;
            if self
                .tables
                .run_count()
                .checked_sub(1)
                .map(|n| self.tables.run(n))
                .is_none_or(|r| {
                    let delta = u32::try_from(local - r.start).ok();
                    r.block != self.tables.block_count()
                        || r.kind != line.kind
                        || r.old.and_then(|n| n.get().checked_add(delta?)) != line.old
                        || r.new.and_then(|n| n.get().checked_add(delta?)) != line.new
                })
            {
                self.tables.push_run(SourceRun {
                    start: local,
                    block: self.tables.block_count(),
                    ordinal: if line.kind == Kind::Deletion {
                        old_ordinal
                    } else {
                        new_ordinal
                    },
                    old: line
                        .old
                        .map(|n| NonZeroU32::new(n).expect("Validated source line is positive")),
                    new: line
                        .new
                        .map(|n| NonZeroU32::new(n).expect("Validated source line is positive")),
                    kind: line.kind,
                });
            }
            old_ordinal += usize::from(line.old.is_some());
            new_ordinal += usize::from(line.new.is_some());
        }
        first..self.tables.run_count()
    }

    fn file(&mut self, parsed: &Document, file: &FileDiff, base: usize) {
        let (mut old_end, mut new_end) = (1, 1);
        for (h, hunk) in file.hunks.iter().enumerate() {
            if hunk.old_start > old_end || hunk.new_start > new_end {
                self.structural(BlockKind::Gap);
            }
            self.structural(BlockKind::Hunk(h));
            let mut i = hunk.lines.start;
            while i < hunk.lines.end {
                let start = i;
                let context = parsed.lines[i].kind == Kind::Context;
                while i < hunk.lines.end && (parsed.lines[i].kind == Kind::Context) == context {
                    i += 1;
                }
                let runs = self.source_runs(&parsed.lines, base, start..i);
                let (kind, split_len, unified_len) = if context {
                    (
                        BlockKind::Context {
                            source_start: start - base,
                        },
                        i - start,
                        i - start,
                    )
                } else {
                    let old_len = parsed.lines[start..i]
                        .iter()
                        .filter(|line| line.kind == Kind::Deletion)
                        .count();
                    let new_len = i - start - old_len;
                    let offset = if alignment_needed(old_len, new_len) {
                        let old: Vec<_> = (start..i)
                            .filter(|&id| parsed.lines[id].kind == Kind::Deletion)
                            .collect();
                        let new: Vec<_> = (start..i)
                            .filter(|&id| parsed.lines[id].kind == Kind::Addition)
                            .collect();
                        alignment_offset(&parsed.lines, &old, &new)
                    } else {
                        0
                    };
                    let old_pad = if new_len > old_len { offset } else { 0 };
                    let new_pad = if old_len > new_len { offset } else { 0 };
                    let side_start = self.tables.side_count();
                    for r in runs.clone() {
                        if self.tables.run(r).kind == Kind::Deletion {
                            self.tables.push_side(r);
                        }
                    }
                    let middle = self.tables.side_count();
                    for r in runs {
                        if self.tables.run(r).kind == Kind::Addition {
                            self.tables.push_side(r);
                        }
                    }
                    let replacement = self.tables.replacement_count();
                    self.tables.push_replacement(Replacement {
                        old_start: side_start,
                        new_start: middle,
                        end: self.tables.side_count(),
                        old_pad,
                        new_pad,
                    });
                    (
                        BlockKind::Replacement(replacement),
                        (old_len + old_pad).max(new_len + new_pad),
                        old_len + new_len,
                    )
                };
                self.tables.push_block(Block {
                    start: self.len,
                    kind,
                });
                self.len[0] += unified_len;
                self.len[1] += split_len;
            }
            for line in &parsed.lines[hunk.lines.clone()] {
                if let Some(n) = line.old {
                    old_end = n.saturating_add(1);
                }
                if let Some(n) = line.new {
                    new_end = n.saturating_add(1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Projection;
    use crate::model::{Document, RowKind};

    #[test]
    fn both_storage_forms_preserve_parsed_views_and_wide_document_bases() {
        let patch = concat!(
            "diff --git a/review b/review\n--- a/review\n+++ b/review\n",
            "@@ -2,4 +2,5 @@\n head\n-let retries = 3;\n+// inserted\n+let retries = 5;\n-let delay = 10;\n+let delay = 20;\n tail\n",
            "@@ -20,5 +21,3 @@\n start\n-obsolete_one();\n-obsolete_two();\n-let retries = 3;\n+let retries = 5;\n end\n",
            "@@ -60,0 +62,2 @@\n+first addition\n+second addition\n",
            "@@ -80,2 +84,0 @@\n-first deletion\n-second deletion\n",
            "@@ -100,2 +102,2 @@\n-old one\n-old two\n+new one\n+new two\n",
            "@@ -120,2 +121,3 @@\n shared\n-old end\n\\ No newline at end of file\n+inserted\n+new end\n\\ No newline at end of file\n",
        ).as_bytes();
        let parsed = Document::parse(patch).unwrap();
        let file = &parsed.files[0];
        let compact = Projection::new(&parsed, file);
        let wide = Projection::build(&parsed, file, 0, parsed.lines.len(), false);
        assert!(compact.bytes() < wide.bytes());
        assert_eq!(compact.marker_bytes(), wide.marker_bytes());
        assert_eq!(compact.boundary_bytes(), wide.boundary_bytes());
        let base = usize::try_from(u64::from(u32::MAX) + 17).unwrap_or(17);
        for projection in [&compact, &wide] {
            assert_eq!(projection.tables.replacement(0).old_pad, 1);
            assert_eq!(projection.tables.replacement(1).new_pad, 2);
            assert!(projection.tables.replacement(2).old_range().is_empty());
            assert!(projection.tables.replacement(3).new_range().is_empty());
            for (id, line) in parsed.lines.iter().enumerate() {
                assert_eq!(projection.info(id), (line.kind, line.old, line.new));
            }
            for split in [false, true] {
                let rows = projection.rows(split, 7, base);
                let (expected, inverse) = if split {
                    (&parsed.split, &parsed.split_locations)
                } else {
                    (&parsed.unified, &parsed.unified_locations)
                };
                assert_eq!(rows.len(), expected.len());
                assert!(rows.get(expected.len()).is_none());
                for (n, expected) in expected.iter().enumerate() {
                    let actual = rows.get(n).unwrap();
                    let kind = match expected.kind {
                        RowKind::Code { left, right } => RowKind::Code {
                            left: left.map(|id| base + id),
                            right: right.map(|id| base + id),
                        },
                        kind => kind,
                    };
                    assert_eq!(actual.file, 7);
                    assert_eq!(actual.kind, kind);
                }
                for (id, &row) in inverse.iter().enumerate() {
                    assert_eq!(projection.row_for_source(id, split), row);
                }
                for file_boundary in [false, true] {
                    let boundaries: Vec<_> = expected
                        .iter()
                        .enumerate()
                        .filter_map(|(n, row)| {
                            (if file_boundary {
                                matches!(row.kind, RowKind::Header)
                            } else {
                                matches!(row.kind, RowKind::Hunk(_))
                            })
                            .then_some(n)
                        })
                        .collect();
                    for forward in [false, true] {
                        assert_eq!(
                            rows.first_boundary(forward, file_boundary),
                            if forward {
                                boundaries.first().copied()
                            } else {
                                boundaries.last().copied()
                            }
                        );
                        for n in 0..rows.len() {
                            let expected = if forward {
                                boundaries.iter().copied().find(|&b| b > n)
                            } else {
                                boundaries.iter().rev().copied().find(|&b| b < n)
                            };
                            assert_eq!(rows.boundary(n, forward, file_boundary), expected);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn source_runs_preserve_line_number_endpoints_and_absent_sides() {
        let patch = b"diff --git a/numbers b/numbers\n--- a/numbers\n+++ b/numbers\n@@ -1,4 +1,4 @@\n a\n-b\n-c\n+d\n+e\n f\n";
        for first in [1, u32::MAX - 3] {
            let mut parsed = Document::parse_file(patch).unwrap();
            for line in &mut parsed.lines {
                line.old = line.old.map(|n| first + (n - 1));
                line.new = line.new.map(|n| first + (n - 1));
            }
            let file = &mut parsed.files[0];
            file.hunks[0].old_start = first;
            file.hunks[0].new_start = first;
            let projection = Projection::new(&parsed, &parsed.files[0]);
            for (id, line) in parsed.lines.iter().enumerate() {
                assert_eq!(projection.info(id), (line.kind, line.old, line.new));
            }
            for (split, expected) in [
                (
                    true,
                    &[
                        (Some(0), Some(0)),
                        (Some(1), Some(3)),
                        (Some(2), Some(4)),
                        (Some(5), Some(5)),
                    ][..],
                ),
                (
                    false,
                    &[
                        (Some(0), Some(0)),
                        (Some(1), None),
                        (Some(2), None),
                        (None, Some(3)),
                        (None, Some(4)),
                        (Some(5), Some(5)),
                    ][..],
                ),
            ] {
                let rows = projection.rows(split, 0, 0);
                let start = 2 + usize::from(first > 1);
                assert_eq!(rows.len(), start + expected.len());
                for (offset, &(left, right)) in expected.iter().enumerate() {
                    let row = start + offset;
                    assert_eq!(rows.get(row).unwrap().kind, RowKind::Code { left, right });
                    for id in left.into_iter().chain(right) {
                        assert_eq!(projection.row_for_source(id, split), row);
                    }
                }
            }
        }
    }
}
