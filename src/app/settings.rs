//! Tab width, inline-change emphasis and whitespace hiding, changed while a review is open.

use std::sync::Arc;

use anyhow::{Context, Result};

use super::{App, Position};
use crate::{
    engine::{InlineChanges, TabWidth, Whitespace},
    highlight::Cache,
    input::SourceState,
    model::RowKind,
    storage::{Document, FileIndex, PatchScan},
    view::LineId,
};

/// A row with no source line, found again by what it is: the `ordinal`-th row of its kind in
/// its file. Whitespace never adds or removes such rows, only the lines between them.
#[derive(Clone, Copy)]
struct Landmark {
    file: usize,
    kind: Kind,
    ordinal: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Header,
    Hunk,
    Gap,
    Metadata,
    Other,
}

impl Kind {
    fn of(kind: RowKind<LineId>) -> Self {
        match kind {
            RowKind::Header => Self::Header,
            RowKind::Hunk(_) => Self::Hunk,
            RowKind::Gap => Self::Gap,
            RowKind::Metadata => Self::Metadata,
            _ => Self::Other,
        }
    }
}

impl App {
    /// Draws tabs `width` cells wide. True when it changed. The source line under the cursor
    /// keeps its screen row and its text stays in view.
    pub(crate) fn set_tab_width(&mut self, width: TabWidth) -> Result<bool> {
        let cells = usize::from(width.get());
        if cells == self.geometry.tab_width() {
            return Ok(false);
        }
        self.remember_position(None)?;
        self.geometry.set_tab_width(cells);
        self.reflow(self.layout, self.width, self.height)?;
        Ok(true)
    }

    /// Emphasizes changed text by `mode`. True when it changed. Styles are colored again, as
    /// after a theme change.
    pub(crate) fn set_inline_changes(&mut self, mode: InlineChanges) -> bool {
        if mode == self.inline_changes {
            return false;
        }
        self.inline_changes = mode;
        self.worker.set_inline_changes(mode);
        self.epoch += 1;
        self.cache = Cache::default();
        true
    }

    /// Adds a file the input published, as the current whitespace setting shows it.
    pub(crate) fn append_file(&mut self, file: Arc<FileIndex>) -> Result<()> {
        let Some(patch) = &mut self.patch else {
            return self.doc.append(file);
        };
        patch.append(file.clone())?;
        let shown = file.with_whitespace(
            self.whitespace,
            &mut self.scan,
            self.doc.row_len(true),
            self.doc.row_len(false),
        )?;
        self.doc.append(shown)
    }

    /// The files as the patch has them, whatever the whitespace setting.
    pub(crate) fn patch_files(&self) -> &[Arc<FileIndex>] {
        &self.patch.as_ref().unwrap_or(&self.doc).files
    }

    /// Checked source for file `file` of the patch, pointed at the file as it is shown.
    pub(crate) fn shown_source(&self, file: usize, state: SourceState) -> SourceState {
        match state {
            SourceState::Checked(pair)
                if self
                    .doc
                    .files
                    .get(file)
                    .is_some_and(|shown| !Arc::ptr_eq(shown, pair.file())) =>
            {
                SourceState::Checked(Arc::new(pair.with_file(self.doc.files[file].clone())))
            }
            state => state,
        }
    }

    /// The row that shows `id`: its own row, or, for a line of a hunk hidden for whitespace,
    /// the hunk's heading.
    pub(crate) fn shown_row(&self, id: LineId) -> Option<usize> {
        let split = self.split();
        self.view
            .source_row(&self.doc, id, split)
            .or_else(|| match id {
                LineId::Patch(id) if id < self.doc.source_len() => {
                    let file = self.doc.file_for_source(id);
                    file.hidden(id).then(|| {
                        self.view
                            .patch_row(&self.doc, file.row_for_source(id, split), split)
                    })
                }
                _ => None,
            })
    }

    /// Shows or hides whitespace-only changes. True when it changed. The cursor keeps its
    /// source line on its screen row; a line no longer on a row of its own is found on the row
    /// that shows it now.
    pub(crate) fn set_whitespace(&mut self, whitespace: Whitespace) -> Result<bool> {
        if whitespace == self.whitespace {
            return Ok(false);
        }
        let screen_row = self.screen_row();
        self.remember_position(None)?;
        let landmark = if self.reading_anchor.is_some() {
            None
        } else {
            self.landmark()
        };
        // Build the new files before changing anything, so a read failure leaves the review
        // as it was.
        let shown = if whitespace == Whitespace::Show {
            None
        } else {
            let mut doc = Document::default();
            let mut scan = PatchScan::default();
            for file in self.patch_files() {
                doc.append(file.with_whitespace(
                    whitespace,
                    &mut scan,
                    doc.row_len(true),
                    doc.row_len(false),
                )?)?;
            }
            Some(doc)
        };
        match shown {
            None => {
                self.doc = self
                    .patch
                    .take()
                    .context("Whitespace was hidden without the patch's files")?;
            }
            Some(doc) => {
                let patch = std::mem::replace(&mut self.doc, doc);
                self.patch.get_or_insert(patch);
            }
        }
        self.whitespace = whitespace;
        if let Some(mut sources) = self.sources.take() {
            for (file, state) in sources.iter_mut().enumerate() {
                let current = std::mem::replace(state, SourceState::Pending);
                *state = self.shown_source(file, current);
            }
            self.sources = Some(sources);
        }
        self.view.rebuild(&self.doc)?;
        // Pairs change which lines are compared for emphasis.
        self.epoch += 1;
        self.cache = Cache::default();
        self.search_match = None;
        let row = match (self.reading_anchor, landmark) {
            (Some(anchor), _) => self.shown_row(anchor.source),
            (None, Some(landmark)) => self.find_landmark(landmark),
            (None, None) => None,
        };
        self.cursor = Position {
            row: row.unwrap_or(self.cursor.row),
            part: 0,
        };
        self.cursor.row = self.cursor.row.min(self.row_len().saturating_sub(1));
        self.top = self.cursor;
        self.visible.clear();
        self.reflow_from(self.layout, self.width, self.height, Some(screen_row))?;
        Ok(true)
    }

    /// What the cursor's row is, when it has no source line.
    fn landmark(&self) -> Option<Landmark> {
        let row = self.row(self.cursor.row)?;
        let kind = Kind::of(row.kind);
        if kind == Kind::Other {
            return None;
        }
        let first = self.view.file_row(&self.doc, row.file, self.split());
        let ordinal = (first..self.cursor.row)
            .filter(|&n| self.row(n).is_some_and(|r| Kind::of(r.kind) == kind))
            .count();
        Some(Landmark {
            file: row.file,
            kind,
            ordinal,
        })
    }

    fn find_landmark(&self, landmark: Landmark) -> Option<usize> {
        let first = self.view.file_row(&self.doc, landmark.file, self.split());
        let mut seen = 0;
        for n in first..self.row_len() {
            let row = self.row(n)?;
            if row.file != landmark.file {
                break;
            }
            if Kind::of(row.kind) == landmark.kind {
                if seen == landmark.ordinal {
                    return Some(n);
                }
                seen += 1;
            }
        }
        Some(first)
    }
}
