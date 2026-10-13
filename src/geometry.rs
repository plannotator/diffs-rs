use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use crate::{
    storage::{ByteStore, Document, SourceId, SourceRef},
    text::{Glyph, display_text, glyphs, glyphs_from},
};

const CACHE_BYTES: usize = 16 * 1024 * 1024;
const CACHE_ENTRIES: usize = 1024;
const CHECKPOINT_GLYPHS: usize = 256;

#[derive(Clone, Copy, Default)]
struct Boundary {
    byte: usize,
    column: usize,
}

#[cfg(all(test, feature = "owner-census"))]
mod census_tests {
    use super::*;
    use crate::storage::StoreWriter;

    #[test]
    fn inspection_preserves_wrapped_geometry_and_releases_owners() {
        let mut writer = StoreWriter::new().unwrap();
        let bytes = "\tlet café = \"日本 👩‍💻\";".repeat(400);
        writer.append(bytes.as_bytes()).unwrap();
        let store = writer.finish();
        let source = store.source_ref(0, bytes.len()).unwrap();
        let cache = Cache::default();
        let lines: Vec<_> = [None, Some(24), Some(80)]
            .into_iter()
            .map(|width| cache.get_source(&store, source, width).unwrap())
            .collect();
        let addresses: Vec<_> = lines
            .iter()
            .map(|line| {
                (
                    line.text.as_ptr(),
                    line.starts.as_ptr(),
                    line.checkpoints.as_ptr(),
                    Rc::strong_count(line),
                )
            })
            .collect();
        let pages = store.cache_census().unwrap();
        let census = cache.census().unwrap();
        assert_eq!(census["entries"], 3);
        assert_eq!(
            census["payload_counter_bytes"],
            lines.iter().map(|line| line.bytes()).sum::<usize>()
        );
        assert_eq!(
            census["payload_counter_bytes"],
            census["recomputed_payload_bytes"]
        );
        assert_eq!(cache.census().unwrap(), census);
        assert_eq!(store.cache_census().unwrap(), pages);
        for (line, expected) in lines.iter().zip(addresses) {
            assert_eq!(
                (
                    line.text.as_ptr(),
                    line.starts.as_ptr(),
                    line.checkpoints.as_ptr(),
                    Rc::strong_count(line)
                ),
                expected
            );
        }
        let weak: Vec<_> = lines.iter().map(Rc::downgrade).collect();
        drop(lines);
        drop(cache);
        assert!(weak.iter().all(|line| line.upgrade().is_none()));
    }
}

pub struct Line {
    text: Box<str>,
    columns: usize,
    checkpoints: Vec<Boundary>,
    starts: Vec<Boundary>,
}

impl Line {
    fn new(text: String, width: Option<usize>) -> Self {
        let mut checkpoints = Vec::new();
        let mut starts = vec![Boundary::default()];
        let mut used = 0;
        let mut columns = 0;
        for (index, glyph) in glyphs(&text, 4).enumerate() {
            columns = glyph.column + glyph.width;
            let boundary = Boundary {
                byte: glyph.byte,
                column: glyph.column,
            };
            if index % CHECKPOINT_GLYPHS == 0 {
                checkpoints.push(boundary);
            }
            if let Some(width) = width {
                if used > 0 && used + glyph.width > width {
                    starts.push(boundary);
                    used = 0;
                }
                used += glyph.width;
            }
        }
        Self {
            text: text.into_boxed_str(),
            columns,
            checkpoints,
            starts,
        }
    }

    fn bytes(&self) -> usize {
        size_of::<Self>()
            + self.text.len()
            + (self.checkpoints.capacity() + self.starts.capacity()) * size_of::<Boundary>()
    }

    pub fn height(&self) -> usize {
        self.starts.len()
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    pub fn start_column(&self, part: usize) -> Option<usize> {
        self.starts.get(part).map(|start| start.column)
    }

    pub fn part_for_column(&self, column: usize) -> usize {
        self.starts
            .partition_point(|start| start.column <= column)
            .saturating_sub(1)
    }

    pub fn glyphs_from_column(&self, column: usize) -> impl Iterator<Item = Glyph<'_>> {
        let checkpoint = self
            .checkpoints
            .get(
                self.checkpoints
                    .partition_point(|point| point.column <= column)
                    .saturating_sub(1),
            )
            .copied()
            .unwrap_or_default();
        glyphs_from(&self.text, 4, checkpoint.byte, checkpoint.column)
    }

    pub fn byte_at_column(&self, column: usize) -> usize {
        self.glyphs_from_column(column)
            .find(|glyph| glyph.column + glyph.width > column)
            .map_or(self.text.len(), |glyph| glyph.byte)
    }

    pub fn column_at_byte(&self, byte: usize) -> (usize, usize) {
        let checkpoint = self
            .checkpoints
            .get(
                self.checkpoints
                    .partition_point(|point| point.byte <= byte)
                    .saturating_sub(1),
            )
            .copied()
            .unwrap_or_default();
        glyphs_from(&self.text, 4, checkpoint.byte, checkpoint.column)
            .take_while(|glyph| glyph.byte <= byte)
            .last()
            .map_or((0, 1), |glyph| (glyph.column, glyph.width))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum SourceKey {
    Patch(SourceId),
    Stored(SourceRef),
}

type Key = (SourceKey, Option<usize>);

#[derive(Default)]
struct Entries {
    lines: HashMap<Key, Rc<Line>>,
    order: VecDeque<Key>,
    bytes: usize,
}

#[derive(Default)]
pub struct Cache {
    entries: RefCell<Entries>,
}

impl Cache {
    #[cfg(feature = "owner-census")]
    pub(crate) fn census(&self) -> anyhow::Result<serde_json::Value> {
        let entries = self.entries.try_borrow()?;
        Ok(serde_json::json!({
            "entries": entries.lines.len(), "map_capacity": entries.lines.capacity(),
            "payload_counter_bytes": entries.bytes,
            "recomputed_payload_bytes": entries.lines.values().map(|line| line.bytes()).sum::<usize>(),
            "payload_limit_bytes": CACHE_BYTES, "entry_limit": CACHE_ENTRIES,
            "text_bytes": entries.lines.values().map(|line| line.text.len()).sum::<usize>(),
            "occupied_map_element_bytes": entries.lines.len() * size_of::<(Key, Rc<Line>)>(),
            "fifo_length": entries.order.len(), "fifo_capacity": entries.order.capacity(),
            "fifo_element_size_bytes": size_of::<Key>(),
        }))
    }

    pub fn get(&self, doc: &Document, id: usize, width: Option<usize>) -> anyhow::Result<Rc<Line>> {
        let file = doc.file_for_source(id);
        self.get_or_insert((SourceKey::Patch(file.source_id(id)?), width), || {
            Ok(Line::new(display_text(&file.bytes(id)?), width))
        })
    }

    pub(crate) fn get_source(
        &self,
        store: &ByteStore,
        source: SourceRef,
        width: Option<usize>,
    ) -> anyhow::Result<Rc<Line>> {
        self.get_or_insert((SourceKey::Stored(source), width), || {
            Ok(Line::new(display_text(&store.read_source(source)?), width))
        })
    }

    fn get_or_insert(
        &self,
        key: Key,
        build: impl FnOnce() -> anyhow::Result<Line>,
    ) -> anyhow::Result<Rc<Line>> {
        if let Some(line) = self.entries.borrow().lines.get(&key) {
            return Ok(line.clone());
        }
        let line = Rc::new(build()?);
        let bytes = line.bytes();
        if bytes <= CACHE_BYTES {
            let mut entries = self.entries.borrow_mut();
            while entries.bytes + bytes > CACHE_BYTES || entries.lines.len() >= CACHE_ENTRIES {
                if let Some(old) = entries.order.pop_front()
                    && let Some(line) = entries.lines.remove(&old)
                {
                    entries.bytes -= line.bytes();
                }
            }
            entries.bytes += bytes;
            entries.order.push_back(key);
            entries.lines.insert(key, line.clone());
        }
        Ok(line)
    }
}
