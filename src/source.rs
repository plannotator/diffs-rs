use std::{
    io::{BufReader, Read, Write},
    ops::Range,
    sync::Arc,
};

use anyhow::{Context, Result, ensure};

use crate::storage::{ByteStore, FileIndex, ReadWindow, StoreReader, StoreWriter};

mod correspondence;
pub(crate) use correspondence::Gap;
pub use correspondence::{CheckedPair, check_pair};

#[cfg(test)]
mod tests;

const CHUNK: usize = 64 * 1024;

/// Immutable bytes and disk-backed line boundaries. Lines include their original terminators.
#[derive(Debug)]
pub struct Snapshot {
    body: Arc<ByteStore>,
    index: Arc<ByteStore>,
    body_start: u64,
    index_start: u64,
    len: u64,
    lines: u32,
    final_newline: bool,
}

impl Snapshot {
    pub(crate) fn uncached_lines(&self) -> SnapshotReader<'_> {
        SnapshotReader {
            source: self,
            offsets: ReadWindow::within(
                self.index.clone(),
                self.index_start..self.index_start + (u64::from(self.lines) + 1) * 8,
            ),
            body: ReadWindow::within(
                self.body.clone(),
                self.body_start..self.body_start + self.len,
            ),
        }
    }

    pub(crate) fn stored_line(
        &self,
        number: u32,
    ) -> Result<(&ByteStore, crate::storage::SourceRef)> {
        let line = number
            .checked_sub(1)
            .context("Source line numbers start at one")?;
        let start = self.offset(line)?;
        let end = self.offset(number)?;
        let len = usize::try_from(
            end.checked_sub(start)
                .context("Invalid source line index")?,
        )?;
        Ok((
            &self.body,
            self.body.source_ref(self.body_start + start, len)?,
        ))
    }
    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn lines(&self) -> u32 {
        self.lines
    }
    pub fn final_newline(&self) -> bool {
        self.final_newline
    }

    fn offset(&self, line: u32) -> Result<u64> {
        ensure!(line <= self.lines, "Source line is outside the snapshot");
        let bytes = self.index.read(self.index_start + u64::from(line) * 8, 8)?;
        Ok(u64::from_le_bytes(bytes.try_into().map_err(|_| {
            anyhow::anyhow!("Invalid source line index")
        })?))
    }

    pub fn reader(&self) -> Result<StoreReader> {
        self.body.reader(self.body_start, self.len)
    }

    /// Zero-based half-open line range, including the original line terminators.
    pub fn range(&self, lines: Range<u32>) -> Result<StoreReader> {
        ensure!(lines.start <= lines.end, "Invalid source line range");
        let start = self.offset(lines.start)?;
        let end = self.offset(lines.end)?;
        ensure!(start <= end && end <= self.len, "Invalid source line index");
        self.body.reader(self.body_start + start, end - start)
    }

    fn uncached_offset(&self, line: u32) -> Result<u64> {
        ensure!(line <= self.lines, "Source line is outside the snapshot");
        let mut bytes = [0; 8];
        self.index
            .reader(self.index_start + u64::from(line) * 8, 8)?
            .read_exact(&mut bytes)?;
        Ok(u64::from_le_bytes(bytes))
    }

    pub(crate) fn uncached_range(&self, lines: Range<u32>) -> Result<StoreReader> {
        ensure!(lines.start <= lines.end, "Invalid source line range");
        let start = self.uncached_offset(lines.start)?;
        let end = self.uncached_offset(lines.end)?;
        ensure!(start <= end && end <= self.len, "Invalid source line index");
        self.body.reader(self.body_start + start, end - start)
    }

    fn line_cursor(&self, line: u32) -> Result<LineCursor<'_>> {
        let offset = self.uncached_offset(line)?;
        let offsets = self.index.reader(
            self.index_start + (u64::from(line) + 1) * 8,
            u64::from(self.lines - line) * 8,
        )?;
        Ok(LineCursor {
            source: self,
            offset,
            offsets: BufReader::with_capacity(CHUNK, offsets),
            body: BufReader::with_capacity(
                CHUNK,
                self.body
                    .reader(self.body_start + offset, self.len - offset)?,
            ),
        })
    }
}

pub(crate) struct SnapshotReader<'a> {
    source: &'a Snapshot,
    offsets: ReadWindow,
    body: ReadWindow,
}

impl SnapshotReader<'_> {
    pub(crate) fn line(&mut self, number: u32, from: u64) -> Result<&mut ReadWindow> {
        ensure!(
            number != 0 && number <= self.source.lines,
            "Source line is outside the snapshot"
        );
        let index = self.source.index_start + u64::from(number - 1) * 8;
        let mut bytes = [0; 16];
        self.offsets
            .range(index..index + 16)?
            .read_exact(&mut bytes)?;
        let start = u64::from_le_bytes(bytes[..8].try_into()?);
        let end = u64::from_le_bytes(bytes[8..].try_into()?);
        ensure!(
            start <= end && end <= self.source.len,
            "Invalid source line index"
        );
        ensure!(from <= end - start, "Search starts outside the source line");
        self.body
            .range(self.source.body_start + start + from..self.source.body_start + end)
    }
}

struct LineCursor<'a> {
    source: &'a Snapshot,
    offset: u64,
    offsets: BufReader<StoreReader>,
    body: BufReader<StoreReader>,
}

impl LineCursor<'_> {
    fn next(&mut self) -> Result<std::io::Take<&mut BufReader<StoreReader>>> {
        let mut bytes = [0; 8];
        self.offsets.read_exact(&mut bytes)?;
        let end = u64::from_le_bytes(bytes);
        let len = end
            .checked_sub(self.offset)
            .context("Invalid source line index")?;
        self.offset = end;
        Ok(self.body.by_ref().take(len))
    }
}

/// One session's body and index files, shared by all completed snapshots.
pub struct SourceStore {
    body: StoreWriter,
    index: StoreWriter,
}

impl SourceStore {
    pub(crate) fn named_sharing(store: &ByteStore) -> Result<(Self, tempfile::TempPath)> {
        let (body, path) = StoreWriter::named_sharing(store)?;
        Ok((
            Self {
                body,
                index: StoreWriter::sharing(store)?,
            },
            path,
        ))
    }
    pub fn sharing(store: &ByteStore) -> Result<Self> {
        Ok(Self {
            body: StoreWriter::sharing(store)?,
            index: StoreWriter::sharing(store)?,
        })
    }

    pub fn bytes(&self) -> u64 {
        self.body.store().len() + self.index.store().len()
    }

    pub fn capture(
        &mut self,
        input: impl Read,
        check: &impl Fn() -> Result<()>,
    ) -> Result<Arc<Snapshot>> {
        let before = self.checkpoint();
        let result = self.capture_inner(input, check);
        if result.is_err() {
            self.rollback(before)?;
        }
        result
    }

    pub(crate) fn checkpoint(&self) -> (u64, u64) {
        (self.body.store().len(), self.index.store().len())
    }

    pub(crate) fn rollback(&mut self, position: (u64, u64)) -> Result<()> {
        self.body.rollback(position.0)?;
        self.index.rollback(position.1)
    }

    fn capture_inner(
        &mut self,
        mut input: impl Read,
        check: &impl Fn() -> Result<()>,
    ) -> Result<Arc<Snapshot>> {
        let (body_start, index_start) = self.checkpoint();
        let mut offsets = std::io::BufWriter::with_capacity(CHUNK, &mut self.index);
        offsets.write_all(&0u64.to_le_bytes())?;
        let mut buf = [0; CHUNK];
        let (mut len, mut lines, mut final_newline) = (0u64, 0u32, false);
        loop {
            check()?;
            let count = input.read(&mut buf)?;
            if count == 0 {
                break;
            }
            self.body.append(&buf[..count])?;
            for (i, byte) in buf[..count].iter().enumerate() {
                if *byte == b'\n' {
                    lines = lines
                        .checked_add(1)
                        .context("Source line count exceeds the supported range")?;
                    offsets.write_all(&(len + i as u64 + 1).to_le_bytes())?;
                }
            }
            len = len
                .checked_add(count as u64)
                .context("Source length overflow")?;
            final_newline = buf[count - 1] == b'\n';
        }
        if len != 0 && !final_newline {
            lines = lines
                .checked_add(1)
                .context("Source line count exceeds the supported range")?;
            offsets.write_all(&len.to_le_bytes())?;
        }
        offsets.flush()?;
        drop(offsets);
        check()?;
        Ok(Arc::new(Snapshot {
            body: self.body.store(),
            index: self.index.store(),
            body_start,
            index_start,
            len,
            lines,
            final_newline,
        }))
    }
}

struct ByteComparison<'a> {
    a: Vec<u8>,
    b: Vec<u8>,
    check: &'a dyn Fn() -> Result<()>,
}

impl<'a> ByteComparison<'a> {
    fn new(check: &'a dyn Fn() -> Result<()>) -> Self {
        Self {
            a: vec![0; CHUNK],
            b: vec![0; CHUNK],
            check,
        }
    }

    fn equal(&mut self, mut left: impl Read, mut right: impl Read) -> Result<()> {
        let Self { a, b, check } = self;
        loop {
            check()?;
            let count = left.read(a)?;
            if count == 0 {
                ensure!(
                    right.read(&mut b[..1])? == 0,
                    "Source does not match this diff: extra bytes"
                );
                return Ok(());
            }
            right
                .read_exact(&mut b[..count])
                .context("Source does not match this diff: missing bytes")?;
            ensure!(
                a[..count] == b[..count],
                "Source does not match this diff: different bytes"
            );
        }
    }
}
