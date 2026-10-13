use std::{
    ops::Range,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

use super::{LineId, Part, View};
use crate::{model::RowKind, storage::FileIndex};

#[derive(Default)]
pub(super) struct Revision(Weak<AtomicBool>);

impl Revision {
    fn capture(&mut self) -> Arc<AtomicBool> {
        if let Some(active) = self.0.upgrade() {
            return active;
        }
        let active = Arc::new(AtomicBool::new(true));
        self.0 = Arc::downgrade(&active);
        active
    }

    pub(super) fn invalidate(&mut self) {
        if let Some(active) = std::mem::take(&mut self.0).upgrade() {
            active.store(false, Ordering::Release);
        }
    }
}

impl Drop for Revision {
    fn drop(&mut self) {
        self.invalidate();
    }
}

enum Content {
    Patch { start: usize },
    Context { old: u32, new: u32 },
}

struct Segment {
    rows: Range<usize>,
    content: Content,
}

/// A file's frozen logical unified rows, without copying source text or per-line IDs.
pub(crate) struct SourceOrder {
    pub(crate) file: Arc<FileIndex>,
    split: bool,
    segments: Option<Box<[Segment]>>,
    len: usize,
}

impl View {
    pub(crate) fn source_revision(&mut self) -> Arc<AtomicBool> {
        self.source_revision.capture()
    }

    pub(crate) fn source_order(&self, file: Arc<FileIndex>) -> SourceOrder {
        self.reading_order(file, false)
    }

    pub(crate) fn reading_order(&self, file: Arc<FileIndex>, split: bool) -> SourceOrder {
        let Some(view) = self.files.get(&file.id) else {
            let len = file.rows(split).len();
            return SourceOrder {
                file,
                split,
                segments: None,
                len,
            };
        };
        let rows = &view.rows[usize::from(split)];
        let segments = rows
            .segments
            .iter()
            .enumerate()
            .filter_map(|(index, segment)| {
                let content = match segment.part {
                    Part::Patch => Content::Patch {
                        start: segment.patch,
                    },
                    Part::Context { gap, offset } => Content::Context {
                        old: view.gaps[gap].source.old + offset + 1,
                        new: view.gaps[gap].source.new + offset + 1,
                    },
                    Part::Hidden(_) | Part::NoNewline => return None,
                };
                Some(Segment {
                    rows: segment.start
                        ..rows
                            .segments
                            .get(index + 1)
                            .map_or(rows.len, |next| next.start),
                    content,
                })
            })
            .collect();
        SourceOrder {
            file,
            split,
            segments: Some(segments),
            len: rows.len,
        }
    }
}

impl SourceOrder {
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn has_context(&self) -> bool {
        self.segments.as_ref().is_some_and(|segments| {
            segments
                .iter()
                .any(|segment| matches!(segment.content, Content::Context { .. }))
        })
    }

    pub(crate) fn source(&self, row: usize) -> Option<LineId> {
        if row >= self.len {
            return None;
        }
        let patch_row = if let Some(segments) = &self.segments {
            let index = segments
                .partition_point(|segment| segment.rows.start <= row)
                .checked_sub(1)?;
            let segment = &segments[index];
            if !segment.rows.contains(&row) {
                return None;
            }
            let offset = row - segment.rows.start;
            match segment.content {
                Content::Patch { start } => start + offset,
                Content::Context { old, new } => {
                    let offset = u32::try_from(offset).ok()?;
                    return LineId::context(
                        self.file.id,
                        old.checked_add(offset)?,
                        new.checked_add(offset)?,
                    );
                }
            }
        } else {
            row
        };
        match self.file.rows(self.split).get(patch_row)?.kind {
            RowKind::Code { left, right } => left.or(right).map(LineId::Patch),
            _ => None,
        }
    }

    pub(crate) fn row_for_source(&self, source: LineId) -> Option<usize> {
        if !source.belongs_to(&self.file) {
            return None;
        }
        match source {
            LineId::Patch(id) => {
                let row = self.file.row_for_source(id, self.split) - self.file.row_base(self.split);
                let Some(segments) = &self.segments else {
                    return Some(row);
                };
                segments.iter().find_map(|segment| {
                    let Content::Patch { start } = segment.content else {
                        return None;
                    };
                    let offset = row.checked_sub(start)?;
                    (offset < segment.rows.len()).then_some(segment.rows.start + offset)
                })
            }
            LineId::Context { old, new, .. } => {
                self.segments.as_ref()?.iter().find_map(|segment| {
                    let Content::Context {
                        old: first,
                        new: second,
                    } = segment.content
                    else {
                        return None;
                    };
                    let offset = old.get().checked_sub(first)?;
                    if new.get().checked_sub(second) != Some(offset)
                        || offset as usize >= segment.rows.len()
                    {
                        return None;
                    }
                    Some(segment.rows.start + offset as usize)
                })
            }
        }
    }
}

#[cfg(test)]
mod tests;
