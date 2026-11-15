use std::{
    io::{self, Read},
    ops::Range,
};

use anyhow::{Context, Result, ensure};
use memchr::memmem::{Finder, FinderRev};

pub(crate) const QUERY_BYTES: usize = 4096;
const CHUNK: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

pub(crate) struct Literal<'a> {
    pub(super) forward: Finder<'a>,
    pub(super) backward: FinderRev<'a>,
    /// Read scratch, zeroed once: per-record searches must not clear it again.
    buffer: Box<[u8]>,
}

pub(crate) fn validate(query: &str) -> Result<()> {
    ensure!(!query.is_empty(), "Search text is empty");
    ensure!(
        query.len() <= QUERY_BYTES,
        "Search text exceeds {QUERY_BYTES} UTF-8 bytes"
    );
    ensure!(
        !query.contains(['\n', '\r']),
        "Search text must be one line"
    );
    Ok(())
}

impl<'a> Literal<'a> {
    pub(crate) fn new(query: &'a str) -> Result<Self> {
        validate(query)?;
        Ok(Self {
            forward: Finder::new(query.as_bytes()),
            backward: FinderRev::new(query.as_bytes()),
            buffer: vec![0; CHUNK + query.len() - 1].into_boxed_slice(),
        })
    }

    pub(super) fn len(&self) -> usize {
        self.forward.needle().len()
    }

    /// The reader begins at `starts.start` within one original source record.
    /// Bounds restrict match starts, not ends; overlapping matches remain eligible.
    /// Backward search scans forward and retains only the latest eligible match.
    pub(crate) fn find(
        &mut self,
        input: &mut impl Read,
        starts: Range<u64>,
        direction: Direction,
        check: &impl Fn() -> Result<()>,
    ) -> Result<Option<Range<u64>>> {
        ensure!(starts.start <= starts.end, "Invalid search byte range");
        check()?;
        if starts.is_empty() {
            return Ok(None);
        }
        let needle = self.forward.needle().len();
        let mut base = starts.start;
        let mut retained = 0;
        let mut found = None;
        loop {
            check()?;
            let count = loop {
                match input.read(&mut self.buffer[retained..retained + CHUNK]) {
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => check()?,
                    result => break result?,
                }
            };
            let filled = retained + count;
            check()?;
            if count == 0 {
                return Ok(found);
            }
            if filled >= needle {
                let complete = base
                    .checked_add((filled - needle + 1) as u64)
                    .context("Search byte offset overflow")?;
                let eligible = complete.min(starts.end);
                if eligible > base {
                    let end = usize::try_from(eligible - base)? + needle - 1;
                    let haystack = &self.buffer[..end];
                    let local = match direction {
                        Direction::Forward => self.forward.find(haystack),
                        Direction::Backward => self.backward.rfind(haystack),
                    };
                    if let Some(local) = local {
                        let first = base
                            .checked_add(local as u64)
                            .context("Search byte offset overflow")?;
                        let last = first
                            .checked_add(needle as u64)
                            .context("Search byte offset overflow")?;
                        found = Some(first..last);
                    }
                }
                if (found.is_some() && direction == Direction::Forward) || complete >= starts.end {
                    check()?;
                    return Ok(found);
                }
            }
            let keep = filled.min(needle - 1);
            let consumed = filled - keep;
            self.buffer.copy_within(consumed..filled, 0);
            retained = keep;
            base = base
                .checked_add(consumed as u64)
                .context("Search byte offset overflow")?;
        }
    }
}

#[cfg(test)]
mod tests;
