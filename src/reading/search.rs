use std::ops::Range;

use anyhow::{Context, Result, ensure};

use super::{
    domain::Domain,
    literal::{Direction, Literal},
};
use crate::{engine::ReviewId, view::LineId};

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
        for (range, wrapped) in segments {
            if let Some(mut hit) = self.scan(&mut literal, range, direction, &check)? {
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
        range: Range<Position>,
        direction: Direction,
        check: &impl Fn() -> Result<()>,
    ) -> Result<Option<Hit>> {
        if range.is_empty() {
            return Ok(None);
        }
        let mut found = None;
        for (file_id, file) in self.files.iter().enumerate().skip(range.start.file) {
            if file_id > range.end.file {
                break;
            }
            check()?;
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
            let mut patch = file.order.file.uncached_lines();
            let mut context = file.context.as_ref().map(|source| source.uncached_lines());
            for row in first..last {
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
        }
        Ok(found)
    }
}
