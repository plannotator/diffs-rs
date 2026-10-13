use std::{collections::BTreeMap, num::NonZeroU32, ops::Range};

use anyhow::{Context, Result};

use crate::{
    model::{Row, RowKind},
    source::{CheckedPair, Gap},
    storage::{Document, FileIndex},
};

mod reading;
pub(crate) use reading::SourceOrder;

/// A stable source destination, independent of layout and context expansion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "delivery-trace", derive(serde::Serialize))]
#[cfg_attr(feature = "delivery-trace", serde(untagged))]
pub enum LineId {
    Patch(usize),
    Context {
        file: usize,
        old: NonZeroU32,
        new: NonZeroU32,
    },
}

impl From<usize> for LineId {
    fn from(id: usize) -> Self {
        Self::Patch(id)
    }
}

impl LineId {
    /// Checks positive coordinates; source correspondence is checked separately.
    pub fn context(file: usize, old: u32, new: u32) -> Option<Self> {
        Some(Self::Context {
            file,
            old: NonZeroU32::new(old)?,
            new: NonZeroU32::new(new)?,
        })
    }

    pub(crate) fn patch(self) -> Option<usize> {
        match self {
            Self::Patch(id) => Some(id),
            Self::Context { .. } => None,
        }
    }

    pub(crate) fn belongs_to(self, file: &crate::storage::FileIndex) -> bool {
        match self {
            Self::Patch(id) => (file.source_base..file.source_end()).contains(&id),
            Self::Context { file: id, .. } => id == file.id,
        }
    }
}

#[derive(Clone, Copy)]
enum Part {
    Patch,
    Hidden(usize),
    Context { gap: usize, offset: u32 },
    NoNewline,
}

struct Segment {
    start: usize,
    patch: usize,
    part: Part,
}

#[derive(Default)]
struct Rows {
    segments: Vec<Segment>,
    len: usize,
    hunks: Vec<usize>,
}

impl Rows {
    fn push(&mut self, patch: usize, len: usize, part: Part) -> Result<()> {
        if len != 0 {
            self.segments.push(Segment {
                start: self.len,
                patch,
                part,
            });
            self.len = self
                .len
                .checked_add(len)
                .context("Expanded row count overflow")?;
        }
        Ok(())
    }

    fn segment(&self, row: usize) -> Option<&Segment> {
        if row >= self.len {
            return None;
        }
        self.segments.get(
            self.segments
                .partition_point(|segment| segment.start <= row)
                .checked_sub(1)?,
        )
    }

    fn patch_row(&self, row: usize) -> usize {
        let index = self
            .segments
            .partition_point(|segment| segment.patch <= row)
            .saturating_sub(1);
        let segment = &self.segments[index];
        segment.start
            + if matches!(segment.part, Part::Patch) {
                row - segment.patch
            } else {
                0
            }
    }
}

struct Expansion {
    source: Gap,
    revealed: u32,
}

impl Expansion {
    fn leading(&self) -> bool {
        self.source.before == Some(0)
    }
    fn exposed(&self) -> Range<u32> {
        if self.leading() {
            self.source.len - self.revealed..self.source.len
        } else {
            0..self.revealed
        }
    }
    fn line(&self, file: usize, offset: u32) -> LineId {
        LineId::context(
            file,
            self.source.old + offset + 1,
            self.source.new + offset + 1,
        )
        .expect("Validated source line is positive")
    }
}

struct FileView {
    gaps: Vec<Expansion>,
    rows: [Rows; 2],
}

impl FileView {
    fn insertion(file: &FileIndex, gap: &Gap, split: bool) -> (usize, usize) {
        let end = gap
            .before
            .map_or_else(|| file.rows(split).len(), |h| file.hunk_row(h, split));
        let replaces = end.checked_sub(1).is_some_and(|row| {
            file.rows(split)
                .get(row)
                .is_some_and(|row| row.kind == RowKind::Gap)
        });
        (end - usize::from(replaces), end)
    }

    fn build(&self, file: &FileIndex) -> Result<[Rows; 2]> {
        let mut layouts = [Rows::default(), Rows::default()];
        for (layout, split) in [false, true].into_iter().enumerate() {
            let rows = &mut layouts[layout];
            let mut next = 0;
            for (index, gap) in self.gaps.iter().enumerate() {
                let (start, end) = Self::insertion(file, &gap.source, split);
                rows.push(
                    next,
                    start.checked_sub(next).context("Context gaps overlap")?,
                    Part::Patch,
                )?;
                let hidden = gap.source.len - gap.revealed;
                if gap.leading() && hidden != 0 {
                    rows.push(start, 1, Part::Hidden(index))?;
                }
                rows.push(
                    start,
                    gap.revealed as usize,
                    Part::Context {
                        gap: index,
                        offset: gap.exposed().start,
                    },
                )?;
                if !gap.leading() && hidden != 0 {
                    rows.push(start, 1, Part::Hidden(index))?;
                }
                if gap.source.no_newline && hidden == 0 {
                    rows.push(start, 1, Part::NoNewline)?;
                }
                next = end;
            }
            rows.push(next, file.rows(split).len() - next, Part::Patch)?;
            rows.hunks = (0..file.hunks.len())
                .map(|h| rows.patch_row(file.hunk_row(h, split)))
                .collect();
        }
        Ok(layouts)
    }
}

#[derive(Default)]
pub(crate) struct View {
    files: BTreeMap<usize, FileView>,
    extra: Vec<[usize; 2]>,
    source_revision: reading::Revision,
}

impl View {
    #[cfg(feature = "owner-census")]
    pub(crate) fn census(&self) -> serde_json::Value {
        let mut ranges = 0;
        let mut exposed = 0u64;
        let mut segments = 0;
        let mut allocations = self.extra.capacity() * size_of::<[usize; 2]>();
        for file in self.files.values() {
            ranges += file.gaps.len();
            exposed += file
                .gaps
                .iter()
                .map(|gap| u64::from(gap.revealed))
                .sum::<u64>();
            allocations += file.gaps.capacity() * size_of::<Expansion>();
            for rows in &file.rows {
                segments += rows.segments.len();
                allocations += rows.segments.capacity() * size_of::<Segment>()
                    + rows.hunks.capacity() * size_of::<usize>();
            }
        }
        serde_json::json!({
            "files": self.files.len(), "gap_ranges": ranges, "exposed_lines": exposed,
            "layout_segments": segments, "vector_capacity_bytes": allocations,
            "inline_file_values_bytes": self.files.len() * size_of::<FileView>(),
            "scope": "View vectors and occupied file values only; excludes map-node overhead, source descriptors, caches, and allocator overhead",
        })
    }

    pub(crate) fn row(&self, doc: &Document, row: usize, split: bool) -> Option<Row<LineId>> {
        let file = self.file_at(doc, row, split)?;
        let local = row - self.file_row(doc, file.id, split);
        let Some(view) = self.files.get(&file.id) else {
            return file
                .rows(split)
                .get(local)
                .map(|row| row.map(LineId::Patch));
        };
        let segment = view.rows[usize::from(split)].segment(local)?;
        let kind = match segment.part {
            Part::Patch => {
                return file
                    .rows(split)
                    .get(segment.patch + local - segment.start)
                    .map(|row| row.map(LineId::Patch));
            }
            Part::Hidden(_) => RowKind::Gap,
            Part::Context { gap, offset } => {
                let id = view.gaps[gap].line(file.id, offset + (local - segment.start) as u32);
                RowKind::Code {
                    left: Some(id),
                    right: Some(id),
                }
            }
            Part::NoNewline => RowKind::NoNewline {
                left: true,
                right: true,
            },
        };
        Some(Row {
            file: file.id,
            kind,
        })
    }

    pub(crate) fn len(&self, doc: &Document, split: bool) -> usize {
        doc.row_len(split)
            + self
                .extra
                .last()
                .map_or(0, |extra| extra[usize::from(split)])
    }

    pub(crate) fn source_row(&self, doc: &Document, id: LineId, split: bool) -> Option<usize> {
        match id {
            LineId::Patch(id) => Some(self.patch_row(doc, doc.row_for_source(id, split), split)),
            LineId::Context { file, old, new } => {
                let (old, new) = (old.get(), new.get());
                let view = self.files.get(&file)?;
                let gap = view
                    .gaps
                    .partition_point(|gap| gap.source.old < old)
                    .checked_sub(1)?;
                let source = &view.gaps[gap];
                let offset = old - source.source.old - 1;
                if !source.exposed().contains(&offset)
                    || new.checked_sub(source.source.new) != Some(offset + 1)
                {
                    return None;
                }
                let (slot, _) = FileView::insertion(&doc.files[file], &source.source, split);
                let segments = &view.rows[usize::from(split)].segments;
                let first = segments.partition_point(|segment| segment.patch < slot);
                let segment = segments[first..].iter().take_while(|segment| segment.patch == slot).find(|segment|
                    matches!(segment.part, Part::Context { gap: index, .. } if index == gap))?;
                Some(
                    self.file_row(doc, file, split)
                        + segment.start
                        + (offset - source.exposed().start) as usize,
                )
            }
        }
    }

    pub(crate) fn file_row(&self, doc: &Document, file: usize, split: bool) -> usize {
        doc.files[file].row_base(split)
            + file
                .checked_sub(1)
                .map_or(0, |index| self.extra_at(index, split))
    }

    pub(crate) fn boundary(
        &self,
        doc: &Document,
        row: usize,
        split: bool,
        forward: bool,
    ) -> Option<usize> {
        if self.files.is_empty() {
            return doc.boundary(row, split, forward, false);
        }
        let first = self.file_at(doc, row, split)?.id;
        let mut index = first;
        loop {
            let file = &doc.files[index];
            let base = self.file_row(doc, index, split);
            let found = if let Some(view) = self.files.get(&index) {
                let hunks = &view.rows[usize::from(split)].hunks;
                if index != first {
                    if forward {
                        hunks.first().copied()
                    } else {
                        hunks.last().copied()
                    }
                } else if forward {
                    hunks
                        .get(hunks.partition_point(|h| base + *h <= row))
                        .copied()
                } else {
                    hunks
                        .partition_point(|h| base + *h < row)
                        .checked_sub(1)
                        .map(|i| hunks[i])
                }
            } else if index == first {
                file.rows(split).boundary(row - base, forward, false)
            } else {
                file.rows(split).first_boundary(forward, false)
            };
            if let Some(local) = found {
                return Some(base + local);
            }
            index = if forward {
                index.checked_add(1).filter(|i| *i < doc.files.len())?
            } else {
                index.checked_sub(1)?
            };
        }
    }

    fn extra_at(&self, file: usize, split: bool) -> usize {
        self.extra
            .get(file)
            .or_else(|| self.extra.last())
            .map_or(0, |extra| extra[usize::from(split)])
    }

    fn file_at<'a>(&self, doc: &'a Document, row: usize, split: bool) -> Option<&'a FileIndex> {
        doc.files
            .get(
                doc.files.partition_point(|file| {
                    file.row_end(split) + self.extra_at(file.id, split) <= row
                }),
            )
            .map(|f| f.as_ref())
    }

    pub(crate) fn patch_row(&self, doc: &Document, row: usize, split: bool) -> usize {
        let Some(file) = doc
            .files
            .get(doc.files.partition_point(|f| f.row_end(split) <= row))
        else {
            return row;
        };
        let local = row - file.row_base(split);
        self.file_row(doc, file.id, split)
            + self
                .files
                .get(&file.id)
                .map_or(local, |view| view.rows[usize::from(split)].patch_row(local))
    }

    pub(crate) fn original_row(&self, doc: &Document, row: usize, split: bool) -> usize {
        let Some(file) = self.file_at(doc, row, split) else {
            return row;
        };
        let local = row - self.file_row(doc, file.id, split);
        let original = self
            .files
            .get(&file.id)
            .and_then(|view| view.rows[usize::from(split)].segment(local))
            .map_or(local, |segment| {
                segment.patch
                    + if matches!(segment.part, Part::Patch) {
                        local - segment.start
                    } else {
                        0
                    }
            });
        file.row_base(split) + original.min(file.rows(split).len().saturating_sub(1))
    }

    pub(crate) fn set(
        &mut self,
        doc: &Document,
        file: usize,
        pair: Option<&CheckedPair>,
    ) -> Result<()> {
        let view = pair
            .filter(|pair| !pair.gaps.is_empty())
            .map(|pair| {
                let mut view = FileView {
                    gaps: pair
                        .gaps
                        .iter()
                        .map(|gap| Expansion {
                            source: *gap,
                            revealed: 0,
                        })
                        .collect(),
                    rows: [Rows::default(), Rows::default()],
                };
                view.rows = view.build(&doc.files[file])?;
                Ok::<_, anyhow::Error>(view)
            })
            .transpose()?;
        self.install(doc, file, view)?;
        self.source_revision.invalidate();
        Ok(())
    }

    fn install(&mut self, doc: &Document, file: usize, view: Option<FileView>) -> Result<()> {
        let old: [usize; 2] = std::array::from_fn(|layout| {
            self.files.get(&file).map_or(0, |view| {
                view.rows[layout].len - doc.files[file].rows(layout == 1).len()
            })
        });
        let new: [usize; 2] = std::array::from_fn(|layout| {
            view.as_ref().map_or(0, |view| {
                view.rows[layout].len - doc.files[file].rows(layout == 1).len()
            })
        });
        if self.extra.is_empty() && view.is_none() {
            return Ok(());
        }
        self.extra.resize(
            doc.files.len(),
            self.extra.last().copied().unwrap_or([0; 2]),
        );
        for layout in 0..2 {
            doc.row_len(layout == 1)
                .checked_add(self.extra.last().map_or(0, |extra| extra[layout]) - old[layout])
                .and_then(|n| n.checked_add(new[layout]))
                .context("Expanded document row count overflow")?;
        }
        for extra in &mut self.extra[file..] {
            for layout in 0..2 {
                extra[layout] = extra[layout] - old[layout] + new[layout];
            }
        }
        if let Some(view) = view {
            self.files.insert(file, view);
        } else {
            self.files.remove(&file);
        }
        Ok(())
    }

    pub(crate) fn gap(
        &self,
        doc: &Document,
        row: usize,
        split: bool,
    ) -> Option<(usize, usize, u32)> {
        let file = self.file_at(doc, row, split)?;
        let view = self.files.get(&file.id)?;
        let segment =
            view.rows[usize::from(split)].segment(row - self.file_row(doc, file.id, split))?;
        let Part::Hidden(gap) = segment.part else {
            return None;
        };
        Some((
            file.id,
            gap,
            view.gaps[gap].source.len - view.gaps[gap].revealed,
        ))
    }

    pub(crate) fn expand(&mut self, doc: &Document, file: usize, gap: usize) -> Result<LineId> {
        let view = self
            .files
            .get_mut(&file)
            .context("Context gap is unavailable")?;
        let item = view
            .gaps
            .get_mut(gap)
            .context("Context gap is outside the file")?;
        let before = item.revealed;
        item.revealed += 20.min(item.source.len - item.revealed);
        let offset = if item.leading() {
            item.exposed().start
        } else {
            item.revealed - 1
        };
        let selected = item.line(file, offset);
        let built = view.build(&doc.files[file]);
        let rows = match built {
            Ok(rows) => rows,
            Err(error) => {
                view.gaps[gap].revealed = before;
                return Err(error);
            }
        };
        let old: [usize; 2] = std::array::from_fn(|layout| view.rows[layout].len);
        let growth: [usize; 2] = std::array::from_fn(|layout| rows[layout].len - old[layout]);
        for layout in 0..2 {
            if doc
                .row_len(layout == 1)
                .checked_add(self.extra.last().map_or(0, |e| e[layout]))
                .and_then(|n| n.checked_add(growth[layout]))
                .is_none()
            {
                view.gaps[gap].revealed = before;
                anyhow::bail!("Expanded document row count overflow");
            }
        }
        view.rows = rows;
        self.extra.resize(
            doc.files.len(),
            self.extra.last().copied().unwrap_or([0; 2]),
        );
        for extra in &mut self.extra[file..] {
            for layout in 0..2 {
                extra[layout] += growth[layout];
            }
        }
        self.source_revision.invalidate();
        Ok(selected)
    }
}
