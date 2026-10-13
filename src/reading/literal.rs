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
    forward: Finder<'a>,
    backward: FinderRev<'a>,
    buffer: Vec<u8>,
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
            buffer: Vec::with_capacity(CHUNK + query.len() - 1),
        })
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
        self.buffer.clear();
        if starts.is_empty() {
            return Ok(None);
        }
        let needle = self.forward.needle().len();
        let mut base = starts.start;
        let mut found = None;
        loop {
            check()?;
            let retained = self.buffer.len();
            self.buffer.resize(retained + CHUNK, 0);
            let count = loop {
                match input.read(&mut self.buffer[retained..]) {
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => check()?,
                    result => break result?,
                }
            };
            self.buffer.truncate(retained + count);
            check()?;
            if count == 0 {
                return Ok(found);
            }
            if self.buffer.len() >= needle {
                let complete = base
                    .checked_add((self.buffer.len() - needle + 1) as u64)
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
            let keep = self.buffer.len().min(needle - 1);
            let consumed = self.buffer.len() - keep;
            self.buffer.copy_within(consumed.., 0);
            self.buffer.truncate(keep);
            base = base
                .checked_add(consumed as u64)
                .context("Search byte offset overflow")?;
        }
    }
}

#[cfg(test)]
mod tests;
