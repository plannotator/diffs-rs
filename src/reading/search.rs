use std::ops::Range;

use anyhow::{Context, Result, ensure};

use super::{
    domain::{Domain, SourceFile},
    literal::{Direction, Literal},
    scan,
};
use crate::{engine::ReviewId, storage::PatchScan, view::LineId};

/// A boundary in frozen unified order. Byte offsets refer to original source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Position {
    pub(crate) file: usize,
    pub(crate) row: usize,
    pub(crate) byte: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Hit {
    pub(crate) review: ReviewId,
    pub(crate) position: Position,
    pub(crate) source: LineId,
    pub(crate) bytes: Range<u64>,
    pub(crate) wrapped: bool,
}

impl Domain {
    pub(crate) fn find(
        &self,
        query: &str,
        origin: Position,
        direction: Direction,
        cancel: &impl Fn() -> Result<()>,
    ) -> Result<Option<Hit>> {
        let check = || {
            self.check()?;
            cancel()
        };
        check()?;
        let mut literal = Literal::new(query)?;
        let end = Position {
            file: self.files.len(),
            ..Position::default()
        };
        ensure!(origin <= end, "Search starts outside the review");
        if let Some(file) = self.files.get(origin.file) {
            ensure!(
                origin.row <= file.order.len(),
                "Search starts outside the file"
            );
            ensure!(
                origin.byte == 0 || file.order.source(origin.row).is_some(),
                "Search byte boundary has no source line"
            );
            if origin.byte != 0 {
                match file.order.source(origin.row) {
                    Some(LineId::Patch(id)) => {
                        file.order.file.uncached_lines().line(id, origin.byte)?;
                    }
                    Some(LineId::Context { new, .. }) => {
                        file.context
                            .as_ref()
                            .context("Expanded search context has no source")?
                            .uncached_lines()
                            .line(new.get(), origin.byte)?;
                    }
                    None => unreachable!("A byte boundary requires a source line"),
                }
                check()?;
            }
        }
        let segments = match direction {
            Direction::Forward => [(origin..end, false), (Position::default()..origin, true)],
            Direction::Backward => [(Position::default()..origin, false), (origin..end, true)],
        };
        let mut patch = PatchScan::default();
        for (range, wrapped) in segments {
            if let Some(mut hit) = self.scan(&mut literal, &mut patch, range, direction, &check)? {
                check()?;
                hit.wrapped = wrapped;
                return Ok(Some(hit));
            }
        }
        check()?;
        Ok(None)
    }

    fn scan(
        &self,
        literal: &mut Literal<'_>,
        patch: &mut PatchScan,
        range: Range<Position>,
        direction: Direction,
        check: &impl Fn() -> Result<()>,
    ) -> Result<Option<Hit>> {
        if range.is_empty() || self.files.is_empty() {
            return Ok(None);
        }
        let files = range.start.file..=range.end.file.min(self.files.len() - 1);
        let mut visit = |file| {
            check()?;
            self.scan_file(literal, patch, file, &range, direction, check)
        };
        match direction {
            Direction::Forward => {
                for file in files {
                    if let Some(hit) = visit(file)? {
                        return Ok(Some(hit));
                    }
                }
            }
            Direction::Backward => {
                for file in files.rev() {
                    if let Some(hit) = visit(file)? {
                        return Ok(Some(hit));
                    }
                }
            }
        }
        Ok(None)
    }

    /// The first (forward) or last (backward) hit in `range` within one file.
    fn scan_file(
        &self,
        literal: &mut Literal<'_>,
        patch: &mut PatchScan,
        file_id: usize,
        range: &Range<Position>,
        direction: Direction,
        check: &impl Fn() -> Result<()>,
    ) -> Result<Option<Hit>> {
        let file = &self.files[file_id];
        let first = if file_id == range.start.file {
            range.start.row
        } else {
            0
        };
        let last = if file_id == range.end.file {
            range
                .end
                .row
                .saturating_add(usize::from(range.end.byte != 0))
                .min(file.order.len())
        } else {
            file.order.len()
        };
        if first >= last {
            return Ok(None);
        }
        if !file.order.is_projection() {
            // Revealed context comes from another store: read records one by one.
            return self.scan_rows(literal, file_id, first..last, range, direction, check);
        }
        let index = &file.order.file;
        if index.source_end() == index.source_base {
            return Ok(None);
        }
        // Row bounds become spool bounds only when rows follow record order. Check that
        // once per bounded file; other files check it only when they contain a hit.
        let bounded = file_id == range.start.file || file_id == range.end.file;
        let mut ordered = None;
        if bounded && !*ordered.insert(index.unified_follows_source()) {
            return self.scan_rows(literal, file_id, first..last, range, direction, check);
        }
        let (line, begin) = if file_id == range.start.file {
            match self.spool_bound(patch, file, range.start.row, range.start.byte)? {
                Some(bound) => bound,
                None => return Ok(None),
            }
        } else {
            let id = index.source_base;
            (id, patch.spans.get(index, id)?.0)
        };
        let limit = match (file_id == range.end.file)
            .then(|| self.spool_bound(patch, file, range.end.row, range.end.byte))
            .transpose()?
            .flatten()
        {
            Some((_, limit)) => limit,
            None => scan::file_end(&mut patch.spans, index)?,
        };
        if begin >= limit {
            return Ok(None);
        }
        let found = match direction {
            Direction::Forward => scan::first(patch, literal, index, line, begin, limit, check)?,
            Direction::Backward => scan::last(patch, literal, index, begin, limit, check)?,
        };
        let Some((id, byte)) = found else {
            return Ok(None);
        };
        if !ordered.unwrap_or_else(|| index.unified_follows_source()) {
            // Interleaved changes: the first record in the spool may not be the first row.
            return self.scan_rows(literal, file_id, first..last, range, direction, check);
        }
        let source = LineId::Patch(id);
        let position = Position {
            file: file_id,
            row: file
                .order
                .row_for_source(source)
                .context("Search hit is outside the frozen source order")?,
            byte,
        };
        ensure!(
            range.contains(&position),
            "Search hit lies outside its scan range"
        );
        Ok(Some(Hit {
            review: self.review,
            position,
            source,
            bytes: byte..byte + literal.len() as u64,
            wrapped: false,
        }))
    }

    /// The first patch record at or after `row`, and the spool offset of `byte` in `row`.
    /// Only meaningful for rows that follow record order.
    fn spool_bound(
        &self,
        patch: &mut PatchScan,
        file: &SourceFile,
        row: usize,
        byte: u64,
    ) -> Result<Option<(usize, u64)>> {
        for candidate in row..file.order.len() {
            if let Some(LineId::Patch(id)) = file.order.source(candidate) {
                let offset = patch.spans.get(&file.order.file, id)?.0;
                let byte = if candidate == row { byte } else { 0 };
                return Ok(Some((id, offset + byte)));
            }
        }
        Ok(None)
    }

    /// Reads each source record of `rows` in order.
    fn scan_rows(
        &self,
        literal: &mut Literal<'_>,
        file_id: usize,
        rows: Range<usize>,
        range: &Range<Position>,
        direction: Direction,
        check: &impl Fn() -> Result<()>,
    ) -> Result<Option<Hit>> {
        let file = &self.files[file_id];
        let mut found = None;
        let mut patch = file.order.file.uncached_lines();
        let mut context = file.context.as_ref().map(|source| source.uncached_lines());
        for row in rows {
            check()?;
            let Some(source) = file.order.source(row) else {
                continue;
            };
            let start = if file_id == range.start.file && row == range.start.row {
                range.start.byte
            } else {
                0
            };
            let end = if file_id == range.end.file && row == range.end.row {
                range.end.byte
            } else {
                u64::MAX
            };
            let input = match source {
                LineId::Patch(id) => patch.line(id, start)?,
                LineId::Context { new, .. } => context
                    .as_mut()
                    .context("Expanded search context has no source")?
                    .line(new.get(), start)?,
            };
            if let Some(bytes) = literal.find(input, start..end, direction, check)? {
                found = Some(Hit {
                    review: self.review,
                    position: Position {
                        file: file_id,
                        row,
                        byte: bytes.start,
                    },
                    source,
                    bytes,
                    wrapped: false,
                });
                if direction == Direction::Forward {
                    return Ok(found);
                }
            }
        }
        Ok(found)
    }
}
