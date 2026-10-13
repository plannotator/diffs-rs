use std::{
    io::{self, BufRead, BufReader, Read, Write},
    ops::{Deref, Range, RangeInclusive},
    sync::Arc,
};

use anyhow::{Context, Result, ensure};

use crate::{
    model::{Document as Parsed, Kind, Row, RowKind},
    projection::{Projection, Rows},
    source_spans::SourceSpans,
};

mod metadata;
pub use metadata::{FileMetadata, HunkMetadata};

mod bytes;
pub use bytes::{ByteStore, SourceRef, StoreReader, StoreWriter};
mod reader;
pub(crate) use reader::ReadWindow;

#[cfg(test)]
pub(crate) mod tests;

/// An original patch record, independent of its current screen row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceId {
    store: bytes::StoreId,
    record: usize,
}

struct Spooling<R> {
    input: R,
    writer: StoreWriter,
}
impl<R: Read> Read for Spooling<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let n = self.input.read(out)?;
        self.writer.append(&out[..n]).map_err(io::Error::other)?;
        Ok(n)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LineInfo {
    pub kind: Kind,
    pub old: Option<u32>,
    pub new: Option<u32>,
    pub no_newline: bool,
}

#[derive(Debug)]
pub struct FileIndex {
    pub id: usize,
    pub source_base: usize,
    pub split_base: usize,
    pub unified_base: usize,
    pub meta: FileMetadata,
    pub spool: Arc<ByteStore>,
    source: SourceSpans,
    projection: Projection,
}

impl Deref for FileIndex {
    type Target = FileMetadata;
    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl FileIndex {
    pub(crate) fn hunk_row(&self, index: usize, split: bool) -> usize {
        self.projection.hunk_row(index, split)
    }
    pub fn source_end(&self) -> usize {
        self.source_base + self.source.len()
    }
    pub fn row_base(&self, split: bool) -> usize {
        if split {
            self.split_base
        } else {
            self.unified_base
        }
    }
    pub fn rows(&self, split: bool) -> Rows<'_> {
        self.projection.rows(split, self.id, self.source_base)
    }
    pub fn row_end(&self, split: bool) -> usize {
        self.row_base(split) + self.rows(split).len()
    }
    pub fn row_for_source(&self, id: usize, split: bool) -> usize {
        self.row_base(split) + self.projection.row_for_source(id - self.source_base, split)
    }
    pub fn info(&self, id: usize) -> LineInfo {
        let n = id - self.source_base;
        let (kind, old, new) = self.projection.info(n);
        LineInfo {
            kind,
            old,
            new,
            no_newline: self.source.no_newline(n),
        }
    }
    pub fn bytes(&self, id: usize) -> Result<Vec<u8>> {
        self.spool.read_source(self.source_ref(id)?)
    }
    pub fn source_id(&self, id: usize) -> Result<SourceId> {
        ensure!(
            (self.source_base..self.source_end()).contains(&id),
            "Source line is outside this file"
        );
        Ok(SourceId {
            store: self.spool.id(),
            record: id,
        })
    }
    pub fn source_ref(&self, id: usize) -> Result<SourceRef> {
        self.source_id(id)?;
        let (offset, len) = self.source.range(id - self.source_base)?;
        self.spool.source_ref(offset, len)
    }
    pub(crate) fn source_refs(
        &self,
        range: Range<usize>,
    ) -> Result<impl Iterator<Item = Result<SourceRef>> + '_> {
        ensure!(
            range.start >= self.source_base
                && range.start <= range.end
                && range.end <= self.source_end(),
            "Source range is outside this file"
        );
        Ok(self
            .source
            .cursor(range.start - self.source_base..range.end - self.source_base)?
            .map(|span| {
                let (offset, len) = span?;
                self.spool.source_ref(offset, len)
            }))
    }
    pub fn partner(&self, id: usize) -> Option<usize> {
        let row = self
            .rows(true)
            .get(self.row_for_source(id, true) - self.split_base)?;
        let RowKind::Code {
            left: Some(left),
            right: Some(right),
        } = row.kind
        else {
            return None;
        };
        if left == right {
            None
        } else {
            Some(if id == left { right } else { left })
        }
    }
    pub fn source_index_bytes(&self) -> usize {
        self.source.bytes()
    }
    pub fn source_index_stored_bytes(&self) -> u64 {
        self.source.stored_bytes()
    }
    pub fn projection_bytes(&self) -> usize {
        self.projection.bytes()
    }
    pub fn source_run_bytes(&self) -> usize {
        self.projection.source_run_bytes()
    }
    pub fn shared_block_bytes(&self) -> usize {
        self.projection.block_bytes()
    }
    pub fn side_index_bytes(&self) -> usize {
        self.projection.side_index_bytes()
    }
    pub fn marker_index_bytes(&self) -> usize {
        self.projection.marker_bytes()
    }
    pub fn boundary_index_bytes(&self) -> usize {
        self.projection.boundary_bytes()
    }
    pub fn metadata_bytes(&self) -> usize {
        self.raw_path.len()
            + self.meta.hunks.len() * size_of::<HunkMetadata>()
            + self
                .meta
                .hunks
                .iter()
                .map(|h| h.header.len())
                .sum::<usize>()
            + [
                &self.path,
                &self.old_path,
                &self.status,
                &self.old_mode,
                &self.new_mode,
            ]
            .iter()
            .map(|s| s.len())
            .sum::<usize>()
    }

    fn selected_intervals<'a>(
        &'a self,
        old: bool,
        lines: RangeInclusive<u32>,
        check: &'a impl Fn() -> Result<()>,
    ) -> impl Iterator<Item = Result<(Range<usize>, u64)>> + 'a {
        let first = u64::from(*lines.start());
        let limit = u64::from(*lines.end()) + 1;
        self.projection
            .source_intervals(old)
            .filter_map(move |(ids, line)| {
                if let Err(error) = check() {
                    return Some(Err(error));
                }
                let line = u64::from(line?);
                let start = first.max(line);
                let end = limit.min(line + ids.len() as u64);
                (start < end).then(|| {
                    let base = self.source_base + ids.start;
                    Ok((
                        base + (start - line) as usize..base + (end - line) as usize,
                        start,
                    ))
                })
            })
    }

    pub(crate) fn write_source_lines(
        &self,
        old: bool,
        lines: RangeInclusive<u32>,
        output: &mut impl Write,
        check: &impl Fn() -> Result<()>,
    ) -> Result<u64> {
        ensure!(
            *lines.start() != 0 && lines.start() <= lines.end(),
            "Invalid source line range"
        );
        let limit = u64::from(*lines.end()) + 1;
        let mut next = u64::from(*lines.start());
        let mut bounds: Option<Range<usize>> = None;
        for interval in self.selected_intervals(old, lines.clone(), check) {
            let (ids, line) = interval?;
            check()?;
            ensure!(
                line == next,
                "Requested source range contains lines missing from the patch"
            );
            next += ids.len() as u64;
            if let Some(bounds) = &mut bounds {
                bounds.end = ids.end;
            } else {
                bounds = Some(ids);
            }
            if next == limit {
                break;
            }
        }
        check()?;
        ensure!(
            next == limit,
            "Requested source range contains lines missing from the patch"
        );
        let bounds = bounds.context("The requested side has no supplied source lines")?;
        let first = bounds.start - self.source_base;
        let last = bounds.end - self.source_base - 1;
        let (offset, _) = self
            .source
            .cursor(first..first + 1)?
            .next()
            .context("Missing first source span")??;
        let (end, len) = self
            .source
            .cursor(last..last + 1)?
            .next()
            .context("Missing last source span")??;
        let end = end
            .checked_add(len as u64)
            .context("Source span overflow")?;
        let len = end
            .checked_sub(offset)
            .context("Source spans are out of order")?;
        let mut body = BufReader::with_capacity(64 * 1024, self.spool.reader(offset, len)?);
        let mut position = offset;
        let mut written = 0u64;
        let mut intervals = self.selected_intervals(old, lines, check);
        let mut selected = intervals
            .next()
            .context("Missing selected source interval")??
            .0;
        let spans = self.source.cursor(first..last + 1)?;
        for (id, span) in bounds.zip(spans) {
            check()?;
            let (offset, len) = span?;
            if id >= selected.end {
                selected = intervals
                    .next()
                    .context("Missing selected source interval")??
                    .0;
            }
            if id < selected.start {
                continue;
            }
            let skip = offset
                .checked_sub(position)
                .context("Source spans overlap or are out of order")?;
            if skip > body.buffer().len() as u64 {
                let remaining = end
                    .checked_sub(offset)
                    .context("Source span exceeds selected range")?;
                body.consume(body.buffer().len());
                *body.get_mut() = self.spool.reader(offset, remaining)?;
            } else {
                body.consume(skip as usize);
            }
            copy_checked(&mut body, output, len as u64, check)?;
            position = offset
                .checked_add(len as u64)
                .context("Source span overflow")?;
            written = written
                .checked_add(len as u64)
                .context("Source output length overflow")?;
        }
        check()?;
        Ok(written)
    }

    pub fn write_canonical(&self, output: &mut impl Write) -> Result<()> {
        output.write_all(b"F")?;
        output.write_all(&(self.id as u64).to_le_bytes())?;
        let mut body = None;
        let mut position = 0;
        for hunk in &self.hunks {
            output.write_all(b"H")?;
            output.write_all(&u64::from(hunk.old_start).to_le_bytes())?;
            output.write_all(&u64::from(hunk.new_start).to_le_bytes())?;
            let spans = self
                .source
                .cursor(hunk.lines.start - self.source_base..hunk.lines.end - self.source_base)?;
            for (id, span) in hunk.lines.clone().zip(spans) {
                let info = self.info(id);
                let sign = match info.kind {
                    Kind::Context => b' ',
                    Kind::Addition => b'+',
                    Kind::Deletion => b'-',
                };
                let (offset, len) = span?;
                if body.is_none() {
                    let last = self.source.len() - 1;
                    let (end, len) = self
                        .source
                        .cursor(last..last + 1)?
                        .next()
                        .context("Missing final source span")??;
                    let length = (end + len as u64)
                        .checked_sub(offset)
                        .context("Patch source spans are out of order")?;
                    body = Some(BufReader::with_capacity(
                        64 * 1024,
                        self.spool.reader(offset, length)?,
                    ));
                    position = offset;
                }
                let body = body.as_mut().context("Missing canonical source reader")?;
                let skip = offset
                    .checked_sub(position)
                    .context("Patch source spans overlap or are out of order")?;
                copy_exact(body, &mut io::sink(), skip)
                    .context("Cannot advance to the next patch source span")?;
                output.write_all(&[b'L', sign])?;
                output.write_all(&u64::from(info.old.unwrap_or(0)).to_le_bytes())?;
                output.write_all(&u64::from(info.new.unwrap_or(0)).to_le_bytes())?;
                output.write_all(&[u8::from(info.no_newline)])?;
                output.write_all(&(len as u64).to_le_bytes())?;
                copy_exact(body, output, len as u64)
                    .context("Cannot copy the patch source span")?;
                position = offset + len as u64;
            }
        }
        Ok(())
    }
}

fn copy_exact(input: &mut impl BufRead, output: &mut impl Write, len: u64) -> Result<()> {
    copy_checked(input, output, len, &|| Ok(()))
}

pub(crate) fn copy_checked(
    input: &mut impl BufRead,
    output: &mut impl Write,
    mut len: u64,
    check: &impl Fn() -> Result<()>,
) -> Result<()> {
    while len != 0 {
        check()?;
        let bytes = match input.fill_buf() {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        };
        if bytes.is_empty() {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
        }
        let count = len.min(bytes.len() as u64) as usize;
        output.write_all(&bytes[..count])?;
        input.consume(count);
        len -= count as u64;
    }
    Ok(())
}

#[derive(Default, Debug)]
pub struct Document {
    pub files: Vec<Arc<FileIndex>>,
}

impl Document {
    pub fn append(&mut self, file: Arc<FileIndex>) -> Result<()> {
        ensure!(
            file.id == self.files.len()
                && file.source_base == self.source_len()
                && file.split_base == self.row_len(true)
                && file.unified_base == self.row_len(false),
            "Input worker published files out of order"
        );
        self.files.push(file);
        Ok(())
    }
    pub fn source_len(&self) -> usize {
        self.files.last().map_or(0, |f| f.source_end())
    }
    pub fn row_len(&self, split: bool) -> usize {
        self.files.last().map_or(0, |f| f.row_end(split))
    }
    pub fn file_for_source(&self, id: usize) -> &Arc<FileIndex> {
        &self.files[self.files.partition_point(|f| f.source_end() <= id)]
    }
    pub fn info(&self, id: usize) -> LineInfo {
        self.file_for_source(id).info(id)
    }
    pub fn bytes(&self, id: usize) -> Result<Vec<u8>> {
        self.file_for_source(id).bytes(id)
    }
    pub fn row(&self, index: usize, split: bool) -> Option<Row> {
        let f = self
            .files
            .get(self.files.partition_point(|f| f.row_end(split) <= index))?;
        f.rows(split).get(index - f.row_base(split))
    }
    pub fn row_for_source(&self, id: usize, split: bool) -> usize {
        self.file_for_source(id).row_for_source(id, split)
    }
    pub fn boundary(&self, row: usize, split: bool, forward: bool, file: bool) -> Option<usize> {
        let current = self.files.partition_point(|f| f.row_end(split) <= row);
        let f = self.files.get(current)?;
        if let Some(target) = f
            .rows(split)
            .boundary(row - f.row_base(split), forward, file)
        {
            return Some(f.row_base(split) + target);
        }
        if forward {
            self.files[current + 1..].iter().find_map(|f| {
                f.rows(split)
                    .first_boundary(true, file)
                    .map(|r| f.row_base(split) + r)
            })
        } else {
            self.files[..current].iter().rev().find_map(|f| {
                f.rows(split)
                    .first_boundary(false, file)
                    .map(|r| f.row_base(split) + r)
            })
        }
    }
}

pub fn read(
    input: impl Read,
    mut publish: impl FnMut(Arc<FileIndex>) -> Result<()>,
) -> Result<Arc<ByteStore>> {
    let writer = StoreWriter::new()?;
    read_with(input, writer, &|| Ok(()), |file, _| publish(file))
}

pub(crate) fn read_with(
    input: impl Read,
    writer: StoreWriter,
    check: &impl Fn() -> Result<()>,
    mut publish: impl FnMut(Arc<FileIndex>, crate::model::PatchIdentity) -> Result<()>,
) -> Result<Arc<ByteStore>> {
    let spool = writer.store();
    let mut index = StoreWriter::sharing(&spool)?;
    let input = io::BufReader::new(Spooling { input, writer });
    let (mut file_id, mut source_base, mut split_base, mut unified_base) = (0, 0, 0, 0);
    // Text between file patches, such as commit headers, is skipped. Each unit carries
    // its own offset in the original stream, which the spool retains unchanged.
    let total = crate::ingest::visit_files(input, |unit_base, unit| {
        check()?;
        let mut parsed = Parsed::parse_file(unit)?;
        for mut meta in std::mem::take(&mut parsed.files) {
            let identity = std::mem::take(&mut meta.identity);
            let projection = Projection::new(&parsed, &meta);
            let range = meta.hunks.first().map_or(0, |h| h.lines.start)
                ..meta.hunks.last().map_or(0, |h| h.lines.end);
            let source = SourceSpans::new(
                &mut index,
                unit_base,
                parsed.lines[range.clone()]
                    .iter()
                    .map(|line| (line.offset as u64, line.bytes.len(), line.no_newline)),
                spool.len(),
                check,
            )?;
            for hunk in &mut meta.hunks {
                hunk.lines = (hunk.lines.start - range.start + source_base)
                    ..(hunk.lines.end - range.start + source_base);
            }
            let file = Arc::new(FileIndex {
                id: file_id,
                source_base,
                split_base,
                unified_base,
                meta: meta.into(),
                spool: spool.clone(),
                source,
                projection,
            });
            source_base = file.source_end();
            split_base = file.row_end(true);
            unified_base = file.row_end(false);
            file_id += 1;
            publish(file, identity)?;
        }
        Ok(())
    })?;
    ensure!(
        total == spool.len(),
        "Spool and parsed patch lengths disagree"
    );
    Ok(spool)
}
