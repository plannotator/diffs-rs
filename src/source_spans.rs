use std::{
    io::{BufReader, BufWriter, Read, Write},
    ops::Range,
    sync::Arc,
};

use anyhow::{Context, Result, ensure};

use crate::storage::{ByteStore, ReadWindow, StoreWriter};

const BUFFER: usize = 64 * 1024;

#[derive(Debug)]
pub(crate) struct ReadFailure;

impl std::fmt::Display for ReadFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Cannot read the patch source index")
    }
}

impl std::error::Error for ReadFailure {}

#[derive(Clone, Copy, Debug)]
enum Encoding {
    Relative(u64),
    Absolute,
}

impl Encoding {
    fn width(self) -> usize {
        match self {
            Self::Relative(_) => 8,
            Self::Absolute => 12,
        }
    }
}

#[derive(Debug)]
pub(crate) struct SourceSpans {
    index: Arc<ByteStore>,
    start: u64,
    encoding: Encoding,
    count: usize,
    flags: Box<[u8]>,
    spool_len: u64,
}

impl SourceSpans {
    pub(crate) fn reader(&self) -> SpanReader<'_> {
        SpanReader {
            source: self,
            input: ReadWindow::new(self.index.clone()),
        }
    }

    pub fn new(
        writer: &mut StoreWriter,
        base: u64,
        records: impl ExactSizeIterator<Item = (u64, usize, bool)> + Clone,
        spool_len: u64,
        check: &impl Fn() -> Result<()>,
    ) -> Result<Self> {
        check()?;
        let mut encoding = Encoding::Relative(base);
        for (n, (offset, _, _)) in records.clone().enumerate() {
            if n % 1024 == 0 {
                check()?;
            }
            if u32::try_from(offset).is_err() {
                encoding = Encoding::Absolute;
                break;
            }
        }
        let index = writer.store();
        let start = index.len();
        let count = records.len();
        let encoded_len = u64::try_from(count)?
            .checked_mul(encoding.width() as u64)
            .context("Source index length overflow")?;
        let end = start
            .checked_add(encoded_len)
            .context("Source index offset overflow")?;
        let mut flags = vec![0; count.div_ceil(8)];
        let mut output = BufWriter::with_capacity(BUFFER, writer);
        for (n, (relative, len, no_newline)) in records.enumerate() {
            if n % 1024 == 0 {
                check()?;
            }
            let offset = base
                .checked_add(relative)
                .context("Source offset overflow")?;
            let len = u32::try_from(len).context("Source line exceeds 4 GiB")?;
            let source_end = offset
                .checked_add(u64::from(len))
                .context("Source end overflow")?;
            ensure!(
                source_end <= spool_len,
                "Source range exceeds the patch spool"
            );
            match encoding {
                Encoding::Relative(_) => {
                    output.write_all(&u32::try_from(relative)?.to_le_bytes())?
                }
                Encoding::Absolute => output.write_all(&offset.to_le_bytes())?,
            }
            output.write_all(&len.to_le_bytes())?;
            flags[n / 8] |= u8::from(no_newline) << (n % 8);
        }
        output.flush()?;
        check()?;
        ensure!(
            index.len() == end,
            "Source index length disagrees with its records"
        );
        Ok(Self {
            index,
            start,
            encoding,
            count,
            flags: flags.into_boxed_slice(),
            spool_len,
        })
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn range(&self, n: usize) -> Result<(u64, usize)> {
        let read = || {
            ensure!(n < self.count, "Source line is outside the index");
            let mut bytes = [0; 12];
            self.index.read_into(
                self.start + n as u64 * self.encoding.width() as u64,
                &mut bytes[..self.encoding.width()],
            )?;
            self.decode(&bytes)
        };
        read().context(ReadFailure)
    }

    pub fn cursor(
        &self,
        range: Range<usize>,
    ) -> Result<impl Iterator<Item = Result<(u64, usize)>> + '_> {
        let reader = || {
            ensure!(
                range.start <= range.end && range.end <= self.count,
                "Source range is outside the index"
            );
            self.index.reader(
                self.start + range.start as u64 * self.encoding.width() as u64,
                range.len() as u64 * self.encoding.width() as u64,
            )
        };
        let mut input = BufReader::with_capacity(BUFFER, reader().context(ReadFailure)?);
        Ok(range.map(move |_| {
            let mut bytes = [0; 12];
            input
                .read_exact(&mut bytes[..self.encoding.width()])
                .map_err(anyhow::Error::from)
                .and_then(|()| self.decode(&bytes))
                .context(ReadFailure)
        }))
    }

    fn decode(&self, bytes: &[u8; 12]) -> Result<(u64, usize)> {
        let (offset, len) = match self.encoding {
            Encoding::Relative(base) => (
                base.checked_add(u64::from(u32::from_le_bytes(bytes[..4].try_into()?)))
                    .context("Source offset overflow")?,
                u32::from_le_bytes(bytes[4..8].try_into()?),
            ),
            Encoding::Absolute => (
                u64::from_le_bytes(bytes[..8].try_into()?),
                u32::from_le_bytes(bytes[8..].try_into()?),
            ),
        };
        ensure!(
            offset
                .checked_add(u64::from(len))
                .is_some_and(|end| end <= self.spool_len),
            "Source range exceeds the patch spool"
        );
        Ok((offset, len as usize))
    }

    pub fn no_newline(&self, n: usize) -> bool {
        self.flags[n / 8] & (1 << (n % 8)) != 0
    }

    pub fn bytes(&self) -> usize {
        self.flags.len()
    }

    pub fn stored_bytes(&self) -> u64 {
        self.count as u64 * self.encoding.width() as u64
    }
}

pub(crate) struct SpanReader<'a> {
    source: &'a SourceSpans,
    input: ReadWindow,
}

impl SpanReader<'_> {
    pub(crate) fn get(&mut self, n: usize) -> Result<(u64, usize)> {
        let mut read = || {
            ensure!(n < self.source.count, "Source line is outside the index");
            let width = self.source.encoding.width();
            let start = self.source.start + n as u64 * width as u64;
            let mut bytes = [0; 12];
            self.input
                .range(start..start + width as u64)?
                .read_exact(&mut bytes[..width])?;
            self.source.decode(&bytes)
        };
        read().context(ReadFailure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_boundaries_preserve_full_length_domain_and_flag_ordinals() {
        let mut writer = StoreWriter::new().unwrap();
        let max = u64::from(u32::MAX);
        let records: Vec<_> = (0..17)
            .map(|n| (max - n, u32::MAX as usize, n % 3 == 1))
            .collect();
        let source = SourceSpans::new(
            &mut writer,
            max + 100,
            records.iter().copied(),
            3 * max + 100,
            &|| Ok(()),
        )
        .unwrap();
        assert!(matches!(source.encoding, Encoding::Relative(_)));
        assert_eq!(source.bytes(), 3);
        assert_eq!(source.stored_bytes(), 8 * 17);
        let sequential = source
            .cursor(0..17)
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();
        let mut uncached = source.reader();
        for (n, (offset, len, flag)) in records.into_iter().enumerate() {
            assert_eq!(source.range(n).unwrap(), (max + 100 + offset, len));
            assert_eq!(sequential[n], (max + 100 + offset, len));
            assert_eq!(uncached.get(n).unwrap(), (max + 100 + offset, len));
            assert_eq!(source.no_newline(n), flag);
        }
    }

    #[test]
    fn wide_fallback_and_invalid_ranges_are_checked_before_publication() {
        let mut writer = StoreWriter::new().unwrap();
        let max = u64::from(u32::MAX);
        let source = SourceSpans::new(
            &mut writer,
            100,
            [(0, 1, false), (max + 1, 4, true)].into_iter(),
            max + 105,
            &|| Ok(()),
        )
        .unwrap();
        assert!(matches!(source.encoding, Encoding::Absolute));
        assert_eq!(source.bytes(), 1);
        assert_eq!(source.stored_bytes(), 24);
        assert_eq!(source.range(0).unwrap(), (100, 1));
        assert_eq!(source.range(1).unwrap(), (max + 101, 4));
        assert!(!source.no_newline(0));
        assert!(source.no_newline(1));
        for (base, offset, len, spool) in [
            (u64::MAX, 1, 0, u64::MAX),
            (u64::MAX, 0, 1, u64::MAX),
            (0, 0, max as usize + 1, u64::MAX),
            (0, max + 1, 4, max + 4),
            (0, 1, 4, 4),
        ] {
            assert!(
                SourceSpans::new(
                    &mut writer,
                    base,
                    [(offset, len, false)].into_iter(),
                    spool,
                    &|| Ok(())
                )
                .is_err()
            );
        }
        let empty = SourceSpans::new(&mut writer, 0, [].into_iter(), 0, &|| Ok(())).unwrap();
        assert_eq!(empty.len(), 0);
        assert_eq!(empty.bytes(), 0);
        assert!(empty.cursor(0..0).unwrap().next().is_none());
    }

    #[test]
    fn append_page_boundaries_eviction_and_truncation_preserve_published_slices() {
        let mut writer = StoreWriter::new().unwrap();
        let first =
            SourceSpans::new(&mut writer, 0, [(3, 7, true)].into_iter(), 10, &|| Ok(())).unwrap();
        assert_eq!(first.range(0).unwrap(), (3, 7));
        let max = u64::from(u32::MAX);
        let records: Vec<_> = (0..6000)
            .map(|n| (max + n + 1, n as usize % 23, n % 8 == 3))
            .collect();
        let wide = SourceSpans::new(
            &mut writer,
            17,
            records.iter().copied(),
            max + 7000,
            &|| Ok(()),
        )
        .unwrap();
        for n in [0, 1, 5460, 5461, 5999] {
            assert_eq!(wide.range(n).unwrap(), (17 + records[n].0, records[n].1));
        }
        let mut uncached = wide.reader();
        for n in [5999, 5461, 5460, 1, 0] {
            assert_eq!(uncached.get(n).unwrap(), (17 + records[n].0, records[n].1));
        }
        assert_eq!(first.range(0).unwrap(), (3, 7));
        assert!(first.range(1).unwrap_err().is::<ReadFailure>());
        assert!(first.cursor(0..2).is_err());
        assert_eq!(
            wide.cursor(5459..5464)
                .unwrap()
                .collect::<Result<Vec<_>>>()
                .unwrap(),
            records[5459..5464]
                .iter()
                .map(|&(offset, len, _)| (17 + offset, len))
                .collect::<Vec<_>>()
        );
        let mut shared = StoreWriter::sharing(&writer.store()).unwrap();
        let page = vec![b'x'; BUFFER];
        for _ in 0..513 {
            let offset = shared.append(&page).unwrap();
            shared.store().read_into(offset, &mut [0]).unwrap();
        }
        assert!(writer.store().cached_bytes() <= 32 * 1024 * 1024);
        assert_eq!(first.range(0).unwrap(), (3, 7));
        assert_eq!(
            wide.range(5460).unwrap(),
            (17 + records[5460].0, records[5460].1)
        );
        let mut cursor = wide.cursor(5999..6000).unwrap();
        writer.rollback(first.stored_bytes()).unwrap();
        assert_eq!(first.range(0).unwrap(), (3, 7));
        assert!(first.no_newline(0));
        assert!(wide.range(0).unwrap_err().is::<ReadFailure>());
        assert!(cursor.next().unwrap().unwrap_err().is::<ReadFailure>());
        drop(shared);
        assert!(writer.store().cached_bytes() <= BUFFER);
    }

    #[test]
    fn cancellation_during_both_passes_keeps_its_type_and_prior_slice() {
        use crate::input::{Cancellation, Cancelled};
        use std::cell::Cell;
        for stop in [3, 9] {
            let mut writer = StoreWriter::new().unwrap();
            let first =
                SourceSpans::new(&mut writer, 0, [(0, 1, false)].into_iter(), 1, &|| Ok(()))
                    .unwrap();
            let cancel = Cancellation::default();
            let calls = Cell::new(0);
            let records: Vec<_> = (0..5000).map(|n| (n, 1, false)).collect();
            let result = SourceSpans::new(&mut writer, 0, records.iter().copied(), 5000, &|| {
                calls.set(calls.get() + 1);
                if calls.get() == stop {
                    cancel.stop();
                }
                cancel.check()
            });
            assert!(result.unwrap_err().is::<Cancelled>());
            assert_eq!(calls.get(), stop);
            assert_eq!(first.range(0).unwrap(), (0, 1));
            assert!(!first.no_newline(0));
        }
    }

    #[test]
    fn malformed_stored_offsets_fail_cached_and_uncached_decode() {
        let root = StoreWriter::new().unwrap();
        for wide in [false, true] {
            let (mut writer, path) = StoreWriter::named_sharing(&root.store()).unwrap();
            let (base, offset) = if wide {
                (0, u64::from(u32::MAX) + 1)
            } else {
                (u64::MAX - 3, 1)
            };
            let source = SourceSpans::new(
                &mut writer,
                base,
                [(offset, 1, true)].into_iter(),
                u64::MAX,
                &|| Ok(()),
            )
            .unwrap();
            let mut damaged = std::fs::OpenOptions::new().write(true).open(path).unwrap();
            if wide {
                damaged.write_all(&u64::MAX.to_le_bytes()).unwrap();
            } else {
                damaged.write_all(&u32::MAX.to_le_bytes()).unwrap();
            }
            assert_eq!(damaged.metadata().unwrap().len(), source.stored_bytes());
            assert!(source.range(0).unwrap_err().is::<ReadFailure>());
            assert!(source.reader().get(0).unwrap_err().is::<ReadFailure>());
            assert!(
                source
                    .cursor(0..1)
                    .unwrap()
                    .next()
                    .unwrap()
                    .unwrap_err()
                    .is::<ReadFailure>()
            );
            assert!(source.no_newline(0));
        }
    }
}
