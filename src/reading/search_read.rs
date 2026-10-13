use std::{ops::Range, sync::Arc};

use anyhow::{Context, Result, ensure};

use super::{
    domain::{Domain, Lease},
    literal::{self, Direction},
    search::{Hit, Position},
};
use crate::{
    engine::{ReviewId, Side, SourceLocation},
    model::Kind,
    text,
    view::{LineId, SourceOrder},
};

pub(crate) enum SearchStart {
    At(Position),
    AfterRow { display: SourceOrder, row: usize },
}

/// One frozen diff-text query. Run searches off the terminal input thread.
/// Drop the job after use to release its source owners.
pub struct SearchRead {
    domain: Domain,
    query: Arc<str>,
    start: SearchStart,
    preferred: Side,
}

/// An opaque match in one open review. It retains validity tokens, not source files.
#[derive(Clone, Debug)]
pub struct SearchMatch {
    pub(crate) hit: Hit,
    pub(crate) display: Range<usize>,
    location: SourceLocation,
    lease: Lease,
    query: Arc<str>,
}

impl SearchMatch {
    pub fn location(&self) -> SourceLocation {
        self.location
    }
    pub fn source_bytes(&self) -> Range<u64> {
        self.hit.bytes.clone()
    }
    pub fn wrapped(&self) -> bool {
        self.hit.wrapped
    }

    pub(crate) fn check(&self, review: ReviewId) -> Result<()> {
        ensure!(
            self.hit.review == review,
            "Search match belongs to another review"
        );
        self.lease.check()
    }
}

impl SearchRead {
    pub(crate) fn new(
        domain: Domain,
        query: String,
        start: SearchStart,
        preferred: Side,
    ) -> Result<Self> {
        literal::validate(&query)?;
        domain.check()?;
        Ok(Self {
            domain,
            query: query.into(),
            start,
            preferred,
        })
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    /// Checks whether this captured scope still belongs to the open review.
    pub fn check(&self) -> Result<()> {
        self.domain.check()
    }

    fn origin(&self, check: &impl Fn() -> Result<()>) -> Result<Position> {
        match &self.start {
            SearchStart::At(position) => Ok(*position),
            SearchStart::AfterRow { display, row } => {
                let file = display.file.id;
                for row in row.saturating_add(1)..display.len() {
                    check()?;
                    if let Some(source) = display.source(row) {
                        let row = self.domain.files[file]
                            .order
                            .row_for_source(source)
                            .context("Search origin is outside the frozen source order")?;
                        return Ok(Position { file, row, byte: 0 });
                    }
                }
                Ok(Position {
                    file: file + 1,
                    ..Position::default()
                })
            }
        }
    }

    /// Finds one occurrence, wrapping at most once. Previous must come from this query.
    /// No match and cancellation leave the caller's selected match unchanged.
    pub fn find(
        &self,
        previous: Option<&SearchMatch>,
        direction: Direction,
        cancel: &impl Fn() -> Result<()>,
    ) -> Result<Option<SearchMatch>> {
        let check = || {
            self.domain.check()?;
            cancel()
        };
        check()?;
        let origin = if let Some(previous) = previous {
            previous.check(self.domain.review)?;
            ensure!(
                Arc::ptr_eq(&self.query, &previous.query),
                "Match belongs to another search"
            );
            Position {
                byte: previous
                    .hit
                    .bytes
                    .start
                    .checked_add(u64::from(direction == Direction::Forward))
                    .context("Search byte offset overflow")?,
                ..previous.hit.position
            }
        } else {
            self.origin(&check)?
        };
        let Some(hit) = self.domain.find(&self.query, origin, direction, cancel)? else {
            return Ok(None);
        };
        let file = &self.domain.files[hit.position.file];
        let (display, old, new, kind) = match hit.source {
            LineId::Patch(id) => {
                let info = file.order.file.info(id);
                let mut reader = file.order.file.uncached_lines();
                (
                    text::source_offsets(reader.line(id, 0)?, hit.bytes.clone(), &check)?,
                    info.old,
                    info.new,
                    info.kind,
                )
            }
            LineId::Context { old, new, .. } => {
                let mut reader = file
                    .context
                    .as_ref()
                    .context("Search context is no longer available")?
                    .uncached_lines();
                (
                    text::source_offsets(reader.line(new.get(), 0)?, hit.bytes.clone(), &check)?,
                    Some(old.get()),
                    Some(new.get()),
                    Kind::Context,
                )
            }
        };
        check()?;
        let side = match kind {
            Kind::Deletion => Side::Old,
            Kind::Addition => Side::New,
            Kind::Context => self.preferred,
        };
        Ok(Some(SearchMatch {
            location: SourceLocation {
                review: self.domain.review,
                file: hit.position.file,
                side,
                old_line: old,
                new_line: new,
            },
            hit,
            display,
            lease: self.domain.lease.clone(),
            query: self.query.clone(),
        }))
    }
}
