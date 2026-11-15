use std::{
    io::{self, Read},
    ops::Range,
    sync::Arc,
};

use anyhow::{Result, ensure};

use super::{ByteStore, FileIndex};
use crate::source_spans::SpanReader;

const CHUNK: usize = 64 * 1024;

/// Job-local scratch for uncached reads in either logical source order.
pub(crate) struct ReadWindow {
    store: Arc<ByteStore>,
    bounds: Range<u64>,
    buffer: Vec<u8>,
    start: u64,
    position: u64,
    end: u64,
}

impl ReadWindow {
    pub(crate) fn new(store: Arc<ByteStore>) -> Self {
        let end = store.len();
        Self::within(store, 0..end)
    }

    pub(crate) fn within(store: Arc<ByteStore>, bounds: Range<u64>) -> Self {
        Self {
            bounds,
            store,
            buffer: Vec::new(),
            start: 0,
            position: 0,
            end: 0,
        }
    }

    pub(crate) fn range(&mut self, range: Range<u64>) -> Result<&mut Self> {
        ensure!(
            self.bounds.start <= range.start
                && range.start <= range.end
                && range.end <= self.bounds.end,
            "Invalid source byte range"
        );
        self.position = range.start;
        self.end = range.end;
        Ok(self)
    }
}

impl Read for ReadWindow {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() || self.position == self.end {
            return Ok(0);
        }
        if self.position < self.start || self.position - self.start >= self.buffer.len() as u64 {
            let start = (self.position / CHUNK as u64 * CHUNK as u64).max(self.bounds.start);
            self.buffer
                .resize((self.bounds.end - start).min(CHUNK as u64) as usize, 0);
            let read = self
                .store
                .reader(start, self.buffer.len() as u64)
                .map_err(io::Error::other)
                .and_then(|mut reader| reader.read_exact(&mut self.buffer));
            if let Err(error) = read {
                self.buffer.clear();
                return Err(error);
            }
            self.start = start;
        }
        let offset = (self.position - self.start) as usize;
        let count = output
            .len()
            .min(self.buffer.len() - offset)
            .min((self.end - self.position).min(usize::MAX as u64) as usize);
        output[..count].copy_from_slice(&self.buffer[offset..offset + count]);
        self.position += count as u64;
        Ok(count)
    }
}

pub(crate) struct PatchReader<'a> {
    file: &'a FileIndex,
    spans: SpanReader<'a>,
    body: ReadWindow,
}

impl FileIndex {
    pub(crate) fn uncached_lines(&self) -> PatchReader<'_> {
        PatchReader {
            file: self,
            spans: self.source.reader(),
            body: ReadWindow::new(self.spool.clone()),
        }
    }
}

impl PatchReader<'_> {
    pub(crate) fn line(&mut self, id: usize, from: u64) -> Result<&mut ReadWindow> {
        self.file.source_id(id)?;
        let (offset, len) = self.spans.get(id - self.file.source_base)?;
        ensure!(from <= len as u64, "Search starts outside the source line");
        self.body.range(offset + from..offset + len as u64)
    }
}

const SCAN_CHUNK: usize = 1024 * 1024;

/// Job-local scratch for scanning patch bytes of many files in spool order.
/// Like [`ReadWindow`], reads bypass the shared page cache.
#[derive(Default)]
pub(crate) struct PatchScan {
    pub(crate) bytes: SpoolWindow,
    pub(crate) spans: SpanWindow,
}

/// One reusable window of spool bytes. Adjacent files usually share a window.
#[derive(Default)]
pub(crate) struct SpoolWindow {
    store: Option<Arc<ByteStore>>,
    buffer: Vec<u8>,
    start: u64,
    len: usize,
}

/// Reusable scratch for source spans of files that share one index store.
#[derive(Default)]
pub(crate) struct SpanWindow(Option<(Arc<ByteStore>, ReadWindow)>);

impl SpanWindow {
    /// The spool offset and length of one patch source record.
    pub(crate) fn get(&mut self, file: &FileIndex, id: usize) -> Result<(u64, usize)> {
        file.source_id(id)?;
        let index = file.source.index();
        let current = self.0.as_ref().is_some_and(|(store, window)| {
            Arc::ptr_eq(store, index) && window.bounds.end == index.len()
        });
        if !current {
            self.0 = Some((index.clone(), ReadWindow::new(index.clone())));
        }
        let (_, window) = self.0.as_mut().expect("The span window was just installed");
        file.source.get_with(window, id - file.source_base)
    }
}

impl SpoolWindow {
    fn holds(&self, store: &Arc<ByteStore>, range: Range<u64>) -> bool {
        self.store
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, store))
            && self.start <= range.start
            && range.end <= self.start + self.len as u64
    }

    fn fill(&mut self, store: &Arc<ByteStore>, start: u64, len: usize) -> Result<()> {
        self.len = 0;
        if self.buffer.len() < len {
            self.buffer = vec![0; SCAN_CHUNK.max(len)];
        }
        self.store = Some(store.clone());
        store
            .reader(start, len as u64)?
            .read_exact(&mut self.buffer[..len])?;
        self.start = start;
        self.len = len;
        Ok(())
    }

    /// Bytes from `position` toward `end`, at least `min` of them unless `end` comes first.
    pub(crate) fn after(
        &mut self,
        store: &Arc<ByteStore>,
        position: u64,
        end: u64,
        min: usize,
    ) -> Result<&[u8]> {
        ensure!(
            position <= end && end <= store.len(),
            "Invalid source byte range"
        );
        let needed = (end - position).min(min as u64);
        if !self.holds(store, position..position + needed) {
            let len = (store.len() - position).min(SCAN_CHUNK as u64) as usize;
            self.fill(store, position, len.max(needed as usize))?;
        }
        let from = (position - self.start) as usize;
        let to = (end.min(self.start + self.len as u64) - self.start) as usize;
        Ok(&self.buffer[from..to])
    }

    /// Bytes that end at `end` and begin no earlier than `floor`, with their start offset.
    /// At least `min` bytes are returned unless `floor` comes first.
    pub(crate) fn before(
        &mut self,
        store: &Arc<ByteStore>,
        floor: u64,
        end: u64,
        min: usize,
    ) -> Result<(u64, &[u8])> {
        ensure!(
            floor <= end && end <= store.len(),
            "Invalid source byte range"
        );
        let needed = (end - floor).min(min as u64);
        if !self.holds(store, end - needed..end) {
            let len = end.min(SCAN_CHUNK as u64).max(needed);
            self.fill(store, end - len, len as usize)?;
        }
        let start = floor.max(self.start);
        Ok((
            start,
            &self.buffer[(start - self.start) as usize..(end - self.start) as usize],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::StoreWriter;

    #[test]
    fn private_read_windows_preserve_ranges_and_do_not_populate_the_shared_cache() {
        let bytes: Vec<_> = (0..3 * CHUNK + 7).map(|n| (n % 251) as u8).collect();
        let mut writer = StoreWriter::new().unwrap();
        writer.append(&bytes).unwrap();
        let store = writer.finish();
        let mut window = ReadWindow::new(store.clone());
        for range in [
            CHUNK - 3..CHUNK + 11,
            2 * CHUNK..bytes.len(),
            0..17,
            CHUNK + 1..CHUNK + 3,
            bytes.len()..bytes.len(),
        ] {
            let mut actual = Vec::new();
            let reader = window.range(range.start as u64..range.end as u64).unwrap();
            assert_eq!(reader.read(&mut []).unwrap(), 0);
            reader.read_to_end(&mut actual).unwrap();
            assert_eq!(actual, bytes[range]);
            assert!(window.buffer.capacity() <= CHUNK);
        }
        assert_eq!(store.cached_bytes(), 0);
        assert!(window.range(0..bytes.len() as u64 + 1).is_err());
        assert!(window.range(Range { start: 3, end: 2 }).is_err());
    }

    #[test]
    fn a_truncated_private_file_is_not_cached_as_successful_short_input() {
        let root = StoreWriter::new().unwrap();
        let (mut writer, path) = StoreWriter::named_sharing(&root.store()).unwrap();
        writer.append(&vec![b'x'; 3 * CHUNK]).unwrap();
        let mut window = ReadWindow::new(writer.finish());
        let damaged = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        damaged.set_len(CHUNK as u64 + 7).unwrap();
        for _ in 0..2 {
            let error = window
                .range(CHUNK as u64..(CHUNK + 20) as u64)
                .unwrap()
                .read_exact(&mut [0; 20])
                .unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
            assert!(window.buffer.is_empty());
        }
    }

    #[test]
    fn rejected_reader_construction_never_leaves_readable_scratch() {
        let mut writer = StoreWriter::new().unwrap();
        writer.append(&vec![b'x'; 3 * CHUNK]).unwrap();
        let mut window = ReadWindow::new(writer.store());
        window.range(0..1).unwrap().read_exact(&mut [0]).unwrap();
        writer.rollback(CHUNK as u64).unwrap();
        for _ in 0..2 {
            assert!(window.range(1..2).unwrap().read_exact(&mut [0]).is_ok());
            assert!(
                window
                    .range(CHUNK as u64..CHUNK as u64 + 1)
                    .unwrap()
                    .read_exact(&mut [0])
                    .is_err()
            );
            assert!(window.buffer.is_empty());
        }
        let mut fresh = ReadWindow::new(writer.store());
        writer.rollback(1).unwrap();
        for _ in 0..2 {
            let mut actual = [b'?'];
            assert!(fresh.range(0..1).unwrap().read_exact(&mut actual).is_err());
            assert_eq!(actual, [b'?']);
        }
    }
}
