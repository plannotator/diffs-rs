use std::{
    collections::{HashMap, VecDeque},
    fs::File,
    io::{self, Read, Write},
    os::unix::fs::{FileExt, PermissionsExt},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::{Context, Result, ensure};

const PAGE: usize = 64 * 1024;
const SOURCE_CACHE: usize = 32 * 1024 * 1024;
static NEXT_STORE: AtomicU64 = AtomicU64::new(1);

/// Creates an unnamed temporary file readable and writable only by its owner.
///
/// On Linux, `tempfile()` uses `O_TMPFILE`, which applies the process umask to mode 0666
/// (usually 0644) because the file has no name. Setting the mode keeps private data
/// owner-only on every platform, including through `/proc/<pid>/fd`.
pub(crate) fn private_tempfile() -> std::io::Result<File> {
    let file = tempfile::tempfile()?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    Ok(file)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct StoreId(u64);

/// A checked range of published bytes, tied to its backing store.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceRef {
    store: StoreId,
    offset: u64,
    len: usize,
}

#[derive(Debug)]
struct Page {
    bytes: Box<[u8]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PageKey {
    store: StoreId,
    number: u64,
}

#[derive(Default, Debug)]
struct Pages {
    entries: HashMap<PageKey, Arc<Page>>,
    order: VecDeque<PageKey>,
    bytes: usize,
}

#[derive(Debug)]
pub struct ByteStore {
    id: StoreId,
    file: File,
    len: AtomicU64,
    pages: Arc<Mutex<Pages>>,
}

/// The sole writer for a store. Appends never modify published bytes.
pub struct StoreWriter {
    store: Arc<ByteStore>,
}

impl StoreWriter {
    pub fn new() -> Result<Self> {
        Self::with_pages(Arc::new(Mutex::new(Pages::default())))
    }

    /// Create a distinct private store within the existing store's cache budget.
    pub fn sharing(store: &ByteStore) -> Result<Self> {
        Self::with_pages(store.pages.clone())
    }

    fn with_pages(pages: Arc<Mutex<Pages>>) -> Result<Self> {
        let file = private_tempfile().context("Cannot create the private source store")?;
        Self::with_file(file, pages)
    }

    pub(crate) fn named_sharing(store: &ByteStore) -> Result<(Self, tempfile::TempPath)> {
        let (file, path) = tempfile::NamedTempFile::new()?.into_parts();
        Ok((Self::with_file(file, store.pages.clone())?, path))
    }

    fn with_file(file: File, pages: Arc<Mutex<Pages>>) -> Result<Self> {
        let id = NEXT_STORE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map(StoreId)
            .map_err(|_| anyhow::anyhow!("Source store identity exhausted"))?;
        Ok(Self {
            store: Arc::new(ByteStore {
                id,
                file,
                len: AtomicU64::new(0),
                pages,
            }),
        })
    }

    pub fn store(&self) -> Arc<ByteStore> {
        self.store.clone()
    }

    pub fn append(&mut self, bytes: &[u8]) -> Result<u64> {
        let base = self.store.len();
        let len = u64::try_from(bytes.len()).context("Source length overflow")?;
        let end = base
            .checked_add(len)
            .context("Source store offset overflow")?;
        self.store
            .file
            .write_all_at(bytes, base)
            .context("Cannot write the source store")?;
        self.store.len.store(end, Ordering::Release);
        Ok(base)
    }

    pub fn finish(self) -> Arc<ByteStore> {
        self.store
    }

    /// Reclaim a source job's tail before its snapshot descriptors are published.
    pub(crate) fn rollback(&mut self, len: u64) -> Result<()> {
        ensure!(len <= self.store.len(), "Invalid source rollback boundary");
        let mut pages = self
            .store
            .pages
            .lock()
            .map_err(|_| anyhow::anyhow!("Source cache lock failed"))?;
        pages
            .entries
            .retain(|key, _| key.store != self.store.id || key.number < len / PAGE as u64);
        pages
            .order
            .retain(|key| key.store != self.store.id || key.number < len / PAGE as u64);
        pages.bytes = pages.entries.values().map(|page| page.bytes.len()).sum();
        self.store.file.set_len(len)?;
        self.store.len.store(len, Ordering::Release);
        Ok(())
    }
}

/// A bounded sequential reader that does not populate the visible-page cache.
pub struct StoreReader {
    store: Arc<ByteStore>,
    position: u64,
    end: u64,
}

impl StoreReader {
    pub(crate) fn remaining(&self) -> u64 {
        self.end - self.position
    }
}

impl Read for StoreReader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let count = (self.end - self.position).min(output.len() as u64) as usize;
        if count == 0 {
            return Ok(0);
        }
        let count = self
            .store
            .file
            .read_at(&mut output[..count], self.position)?;
        if count == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        self.position += count as u64;
        Ok(count)
    }
}

impl Write for StoreWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.append(bytes).map_err(io::Error::other)?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl ByteStore {
    #[cfg(feature = "owner-census")]
    pub(crate) fn cache_census(&self) -> Result<serde_json::Value> {
        let pages = self
            .pages
            .lock()
            .map_err(|_| anyhow::anyhow!("Source cache lock failed"))?;
        Ok(serde_json::json!({
            "entries": pages.entries.len(), "map_capacity": pages.entries.capacity(),
            "payload_counter_bytes": pages.bytes,
            "recomputed_payload_bytes": pages.entries.values().map(|page| page.bytes.len()).sum::<usize>(),
            "payload_limit_bytes": SOURCE_CACHE,
            "page_value_bytes": pages.entries.len() * size_of::<Page>(),
            "occupied_map_element_bytes": pages.entries.len() * size_of::<(PageKey, Arc<Page>)>(),
            "fifo_length": pages.order.len(), "fifo_capacity": pages.order.capacity(),
            "fifo_element_size_bytes": size_of::<PageKey>(),
        }))
    }

    pub(crate) fn identity(&self) -> u64 {
        self.id.0
    }
    pub fn reader(self: &Arc<Self>, offset: u64, len: u64) -> Result<StoreReader> {
        let end = offset.checked_add(len).context("Source range overflow")?;
        ensure!(
            end <= self.len(),
            "Source range is outside the published bytes"
        );
        Ok(StoreReader {
            store: self.clone(),
            position: offset,
            end,
        })
    }

    pub fn source_reader(self: &Arc<Self>, source: SourceRef) -> Result<StoreReader> {
        ensure!(
            source.store == self.id,
            "Source reference belongs to another store"
        );
        self.reader(source.offset, source.len as u64)
    }
    pub(super) fn id(&self) -> StoreId {
        self.id
    }

    pub fn len(&self) -> u64 {
        self.len.load(Ordering::Acquire)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn source_ref(&self, offset: u64, len: usize) -> Result<SourceRef> {
        let length = u64::try_from(len).context("Source length overflow")?;
        let end = offset
            .checked_add(length)
            .context("Source range overflow")?;
        ensure!(
            end <= self.len(),
            "Source range is outside the published bytes"
        );
        Ok(SourceRef {
            store: self.id,
            offset,
            len,
        })
    }

    pub fn read_source(&self, source: SourceRef) -> Result<Vec<u8>> {
        ensure!(
            source.store == self.id,
            "Source reference belongs to another store"
        );
        let end = source.offset + source.len as u64;
        let mut bytes = Vec::with_capacity(source.len);
        let mut position = source.offset;
        while position < end {
            let within = (position % PAGE as u64) as usize;
            let take = (end - position).min((PAGE - within) as u64) as usize;
            let page = self.page(position / PAGE as u64, within + take)?;
            bytes.extend_from_slice(&page.bytes[within..within + take]);
            position += take as u64;
        }
        Ok(bytes)
    }

    pub(crate) fn read_into(&self, offset: u64, output: &mut [u8]) -> Result<()> {
        self.source_ref(offset, output.len())?;
        let mut position = offset;
        let mut remaining = output;
        while !remaining.is_empty() {
            let within = (position % PAGE as u64) as usize;
            let take = remaining.len().min(PAGE - within);
            let page = self.page(position / PAGE as u64, within + take)?;
            remaining[..take].copy_from_slice(&page.bytes[within..within + take]);
            remaining = &mut remaining[take..];
            position += take as u64;
        }
        Ok(())
    }

    fn page(&self, number: u64, required: usize) -> Result<Arc<Page>> {
        let key = PageKey {
            store: self.id,
            number,
        };
        let mut pages = self
            .pages
            .lock()
            .map_err(|_| anyhow::anyhow!("Source cache lock failed"))?;
        if let Some(page) = pages
            .entries
            .get(&key)
            .filter(|page| page.bytes.len() >= required)
        {
            return Ok(page.clone());
        }
        let offset = number
            .checked_mul(PAGE as u64)
            .context("Source page offset overflow")?;
        let len = self
            .len()
            .checked_sub(offset)
            .context("Source page is outside the store")?
            .min(PAGE as u64) as usize;
        ensure!(
            required <= len,
            "Source range is outside the published bytes"
        );
        let mut bytes = vec![0; len].into_boxed_slice();
        self.file
            .read_exact_at(&mut bytes, offset)
            .context("Cannot read the source store")?;
        let page = Arc::new(Page { bytes });
        if let Some(previous) = pages.entries.remove(&key) {
            pages.bytes -= previous.bytes.len();
            pages.order.retain(|entry| *entry != key);
        }
        while pages.bytes + len > SOURCE_CACHE {
            if let Some(old) = pages.order.pop_front()
                && let Some(page) = pages.entries.remove(&old)
            {
                pages.bytes -= page.bytes.len();
            }
        }
        pages.bytes += len;
        pages.order.push_back(key);
        pages.entries.insert(key, page.clone());
        Ok(page)
    }

    pub fn read(&self, offset: u64, len: usize) -> Result<Vec<u8>> {
        self.read_source(self.source_ref(offset, len)?)
    }

    /// Cached payload across this store and every store created with `StoreWriter::sharing`.
    pub fn cached_bytes(&self) -> usize {
        self.pages
            .lock()
            .map(|pages| pages.bytes)
            .unwrap_or_default()
    }
}

impl Drop for ByteStore {
    fn drop(&mut self) {
        if let Ok(mut pages) = self.pages.lock() {
            let mut removed = 0;
            pages.entries.retain(|key, page| {
                if key.store == self.id {
                    removed += page.bytes.len();
                    false
                } else {
                    true
                }
            });
            pages.bytes -= removed;
            pages.order.retain(|key| key.store != self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Seek, SeekFrom},
        sync::mpsc,
        thread,
    };

    #[test]
    fn source_pages_preserve_bytes_and_readers_through_refresh_and_eviction() {
        let mut writer = StoreWriter::new().unwrap();
        let spool = writer.store();
        let prefix = b"partial page\t\xff\r\n";
        assert_eq!(writer.append(prefix).unwrap(), 0);
        let short = spool.page(0, prefix.len()).unwrap();
        let short_weak = Arc::downgrade(&short);
        assert_eq!(short.bytes.as_ref(), prefix);
        assert!(Arc::ptr_eq(&short, &spool.page(0, 1).unwrap()));
        assert!(spool.read(prefix.len() as u64, 1).is_err());

        let expected: Vec<u8> = (0..PAGE * 2 + 19).map(|n| (n % 251) as u8).collect();
        writer.append(&expected[prefix.len()..]).unwrap();
        let full = spool.page(0, PAGE).unwrap();
        let full_weak = Arc::downgrade(&full);
        assert!(!Arc::ptr_eq(&short, &full));
        assert_eq!(short.bytes.as_ref(), prefix);
        assert_eq!(&full.bytes[..prefix.len()], prefix);
        assert_eq!(&full.bytes[prefix.len()..], &expected[prefix.len()..PAGE]);
        assert_eq!(
            spool.read((PAGE - 7) as u64, 23).unwrap(),
            expected[PAGE - 7..PAGE + 16]
        );
        assert_eq!(
            spool.read((PAGE * 2) as u64, 19).unwrap(),
            expected[PAGE * 2..]
        );
        assert!(spool.read(spool.len(), 0).unwrap().is_empty());
        assert!(spool.read(u64::MAX, 2).is_err());
        drop(short);
        assert!(short_weak.upgrade().is_none());

        let page_bytes: Vec<u8> = (0..PAGE).map(|n| (n % 239) as u8).collect();
        let padding = PAGE - (spool.len() as usize % PAGE);
        writer.append(&page_bytes[..padding]).unwrap();
        for _ in 0..=SOURCE_CACHE / PAGE {
            let offset = writer.append(&page_bytes).unwrap();
            assert_eq!(spool.read(offset, PAGE).unwrap(), page_bytes);
        }
        {
            let pages = spool.pages.lock().unwrap();
            assert_eq!(pages.bytes, SOURCE_CACHE);
            assert_eq!(pages.entries.len(), SOURCE_CACHE / PAGE);
            assert!(!pages.entries.contains_key(&PageKey {
                store: spool.id,
                number: 0
            }));
        }
        assert_eq!(&full.bytes[..prefix.len()], prefix);
        assert_eq!(spool.read(0, prefix.len()).unwrap(), prefix);
        assert!(!Arc::ptr_eq(&full, &spool.page(0, PAGE).unwrap()));
        drop(full);
        assert!(full_weak.upgrade().is_none());
        let last = spool.page(0, PAGE).unwrap();
        let last_weak = Arc::downgrade(&last);
        drop(writer);
        drop(spool);
        assert_eq!(&last.bytes[..prefix.len()], prefix);
        drop(last);
        assert!(last_weak.upgrade().is_none());
    }

    #[test]
    fn patch_and_full_source_stores_share_budget_without_aliasing_or_retained_dead_pages() {
        let mut files = Vec::new();
        let patch = crate::storage::read(
            include_bytes!("../../fixtures/review.patch").as_slice(),
            |file| {
                files.push(file);
                Ok(())
            },
        )
        .unwrap();
        let first = &files[0];
        let reference = first.source_ref(first.source_base).unwrap();
        let original = patch.read_source(reference).unwrap();
        let patch_page = patch.page(0, 1).unwrap();
        let mut writers = [
            StoreWriter::sharing(&patch).unwrap(),
            StoreWriter::sharing(&patch).unwrap(),
        ];
        let mut sources = Vec::new();
        let mut expected = Vec::new();
        for (side, writer) in writers.iter_mut().enumerate() {
            let bytes: Vec<u8> = (0..PAGE).map(|n| ((n + side) % 251) as u8).collect();
            let mut input = tempfile::tempfile().unwrap();
            for _ in 0..=SOURCE_CACHE / PAGE / 2 {
                input.write_all(&bytes).unwrap();
            }
            input.seek(SeekFrom::Start(0)).unwrap();
            assert_eq!(
                io::copy(&mut input, writer).unwrap(),
                input.metadata().unwrap().len()
            );
            sources.push(writer.store());
            expected.push(bytes);
        }
        for number in 0..=SOURCE_CACHE / PAGE / 2 {
            for (store, bytes) in sources.iter().zip(&expected) {
                assert_eq!(store.read((number * PAGE) as u64, PAGE).unwrap(), *bytes);
                assert_eq!(store.cached_bytes(), patch.cached_bytes());
                assert!(store.cached_bytes() <= SOURCE_CACHE);
            }
        }
        assert_eq!(patch.cached_bytes(), SOURCE_CACHE);
        #[cfg(feature = "owner-census")]
        {
            let pointers = || {
                let pages = patch.pages.lock().unwrap();
                let mut values: Vec<_> = pages
                    .entries
                    .iter()
                    .map(|(key, page)| {
                        (
                            key.store.0,
                            key.number,
                            Arc::as_ptr(page),
                            Arc::strong_count(page),
                        )
                    })
                    .collect();
                values.sort_unstable();
                (values, pages.order.clone(), Arc::strong_count(&patch.pages))
            };
            let before = pointers();
            let census = patch.cache_census().unwrap();
            assert_eq!(census["payload_counter_bytes"], SOURCE_CACHE);
            assert_eq!(census["recomputed_payload_bytes"], SOURCE_CACHE);
            for store in &sources {
                assert_eq!(store.cache_census().unwrap(), census);
            }
            assert_eq!(patch.cache_census().unwrap(), census);
            assert_eq!(pointers(), before);
        }
        let old = sources[0].source_ref(0, PAGE).unwrap();
        let new = sources[1].source_ref(0, PAGE).unwrap();
        assert_ne!(old, new);
        assert!(sources[0].read_source(new).is_err());
        assert!(patch.read_source(old).is_err());
        assert_eq!(patch.read_source(reference).unwrap(), original);
        assert_eq!(first.source_ref(first.source_base).unwrap(), reference);
        assert_eq!(
            patch_page.bytes[..original.len()],
            patch.read(0, original.len()).unwrap()
        );

        drop(writers);
        let old_store = sources.remove(0);
        let old_id = old_store.id;
        let retained = old_store.page(0, PAGE).unwrap();
        let weak = Arc::downgrade(&old_store);
        drop(old_store);
        assert!(weak.upgrade().is_none());
        {
            let pages = patch.pages.lock().unwrap();
            assert!(pages.entries.keys().all(|key| key.store != old_id));
            assert!(pages.order.iter().all(|key| key.store != old_id));
            assert_eq!(
                pages.bytes,
                pages
                    .entries
                    .values()
                    .map(|page| page.bytes.len())
                    .sum::<usize>()
            );
        }
        assert_eq!(retained.bytes.as_ref(), expected[0]);
        assert_eq!(sources[0].read_source(new).unwrap(), expected[1]);
        let replacement = StoreWriter::sharing(&patch).unwrap();
        assert_ne!(replacement.store.id, old_id);
        assert!(replacement.store.read_source(old).is_err());
        let cache = Arc::downgrade(&patch.pages);
        drop(replacement);
        drop(sources);
        drop(files);
        drop(patch);
        assert!(cache.upgrade().is_none());
        assert_eq!(retained.bytes.as_ref(), expected[0]);
    }

    #[test]
    fn published_references_survive_concurrent_append_and_writer_completion() {
        let mut writer = StoreWriter::new().unwrap();
        let store = writer.store();
        assert!(store.is_empty());
        let empty = store.source_ref(0, 0).unwrap();
        assert!(store.source_ref(1, 0).is_err());
        assert!(store.source_ref(u64::MAX, 2).is_err());
        let payload = b"\tconst VALUE = \"caf\xc3\xa9 \xe6\x97\xa5\";\xff\r\n";
        let input: Vec<u8> = payload
            .iter()
            .copied()
            .cycle()
            .take(PAGE * 3 + 11)
            .collect();
        let mut input_file = tempfile::tempfile().unwrap();
        input_file.write_all(&input).unwrap();
        input_file.seek(SeekFrom::Start(0)).unwrap();
        let (send, receive) = mpsc::sync_channel(1);
        thread::scope(|scope| {
            let writer_task = scope.spawn(move || {
                let mut bytes = [0; 997];
                loop {
                    let n = input_file.read(&mut bytes).unwrap();
                    if n == 0 {
                        break;
                    }
                    let offset = writer.append(&bytes[..n]).unwrap();
                    send.send(writer.store.source_ref(offset, n).unwrap())
                        .unwrap();
                }
                writer.finish()
            });
            let mut references = Vec::new();
            for reference in receive {
                references.push(reference);
                let offset = reference.offset as usize;
                assert_eq!(
                    store.read_source(reference).unwrap(),
                    input[offset..offset + reference.len]
                );
                if let Some(&first) = references.first() {
                    assert_eq!(store.read_source(first).unwrap(), input[..first.len]);
                }
            }
            let completed = writer_task.join().unwrap();
            assert!(Arc::ptr_eq(&store, &completed));
            assert_eq!(store.read(0, input.len()).unwrap(), input);
            assert!(store.read_source(empty).unwrap().is_empty());
            for reference in references {
                let offset = reference.offset as usize;
                assert_eq!(
                    completed.read_source(reference).unwrap(),
                    input[offset..offset + reference.len]
                );
            }
        });
    }
}
