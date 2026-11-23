//! Read-only answers about the rows the engine lays out: the row under the cursor, and the
//! row drawn at a screen cell. Neither moves the cursor, the view or the selection.

use anyhow::Result;

use super::{Engine, ReviewId, Side};
use crate::{
    app::{App, Position},
    model::RowKind,
    view::LineId,
};

/// One row of the diff: the file it belongs to and what it shows.
///
/// Every row belongs to a file, the headings and gaps included, so a host can always tell
/// which file the cursor is in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffRow {
    pub review: ReviewId,
    /// The file's index, as [`Engine::file`] takes it.
    pub file: usize,
    /// The file's [`FileMetadata::path`](crate::storage::FileMetadata::path).
    pub path: String,
    pub kind: DiffRowKind,
    /// The index of the file's hunk the row is in: on a hunk's heading, its lines and its
    /// "No newline" marks. `None` on headings, gaps and notes.
    pub hunk: Option<usize>,
    /// Which screen row of a wrapped row: 0 for its first, 1 for the next, and so on.
    pub part: usize,
    /// The original one-based lines the row shows. A split row takes its old line from the
    /// left cell and its new line from the right one; either can be empty. Unified, a context
    /// line has both, an added line only `new_line`, a deleted line only `old_line`.
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    /// The side the cursor is on, or the side of the cell: split, the pane; unified, the side
    /// of the row's line as [`Engine::selected_source`] reports it.
    pub side: Side,
}

/// What a [`DiffRow`] shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiffRowKind {
    /// The file's heading.
    Header,
    /// The file's heading drawn over the top row while the file's rows scroll under it. Only
    /// [`Engine::row_at`] returns it; the cursor is never on it.
    StickyHeader,
    /// The heading of the file's hunk with this index.
    Hunk(usize),
    /// Lines between two hunks, or before the first, that the patch does not show.
    Gap,
    /// The note on a file with no textual hunks: binary, or a mode or rename only.
    Metadata,
    /// Lines of code. A split row can be empty on one side.
    Code,
    /// The "No newline at end of file" mark.
    NoNewline,
}

impl Engine {
    /// The row under the cursor. `None` before the first file arrives.
    ///
    /// Unlike [`Engine::selected_source`], this answers on every row: a heading, a hunk's
    /// heading, a gap, a binary file's note, an empty side of a split row, the
    /// "No newline" mark and every wrapped part of a row.
    pub fn cursor_row(&self) -> Option<DiffRow> {
        let app = self.app.as_ref()?;
        let old = if app.split() {
            app.old_side
        } else {
            unified_old(app, app.cursor.row)
        };
        describe(self.id, app, app.cursor, RowShown::Row, old)
            .ok()
            .flatten()
    }

    /// The row drawn at the screen cell (`column`, `row`) in the last frame, as a click there
    /// would find it. `None` outside the diff, below its last row, and before the first draw.
    /// Nothing moves.
    pub fn row_at(&self, column: u16, row: u16) -> Option<DiffRow> {
        let app = self.app.as_ref()?;
        if app.help {
            return None;
        }
        let area = app.sidebar.diff;
        if !area.contains(ratatui::layout::Position::new(column, row)) {
            return None;
        }
        // Screen rows are numbered from 1 at the area's top, as the frame recorded them.
        let y = row - area.y + 1;
        if let Some((position, _)) = app.visible.iter().find(|(_, at)| *at == y) {
            let old = if app.split() {
                column - area.x < app.width / 2
            } else {
                unified_old(app, position.row)
            };
            return describe(self.id, app, *position, RowShown::Row, old)
                .ok()
                .flatten();
        }
        // The top row holds the sticky heading of the file the first row drawn is in.
        let (first, at) = app.visible.first()?;
        if y == 1 && *at == 2 {
            let old = app.split() && column - area.x < app.width / 2;
            return describe(self.id, app, *first, RowShown::Sticky, old)
                .ok()
                .flatten();
        }
        None
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RowShown {
    Row,
    Sticky,
}

/// The side a unified row's line is on, as the cursor would select it.
fn unified_old(app: &App, index: usize) -> bool {
    match app.row(index).map(|row| row.kind) {
        Some(RowKind::Code { left, right: None }) => left.is_some(),
        Some(RowKind::Code { left, right }) => left == right && app.old_side,
        _ => app.old_side,
    }
}

fn describe(
    review: ReviewId,
    app: &App,
    at: Position,
    shown: RowShown,
    old: bool,
) -> Result<Option<DiffRow>> {
    let Some(row) = app.row(at.row) else {
        return Ok(None);
    };
    let Some(file) = app.doc.files.get(row.file) else {
        return Ok(None);
    };
    let line = |id: Option<LineId>, old: bool| -> Result<Option<u32>> {
        let Some(id) = id else { return Ok(None) };
        let info = app.line_info(id)?;
        Ok(if old { info.old } else { info.new })
    };
    let (kind, old_line, new_line) = match (shown, row.kind) {
        (RowShown::Sticky, _) => (DiffRowKind::StickyHeader, None, None),
        (RowShown::Row, RowKind::Header) => (DiffRowKind::Header, None, None),
        (RowShown::Row, RowKind::Hunk(index)) => (DiffRowKind::Hunk(index), None, None),
        (RowShown::Row, RowKind::Gap) => (DiffRowKind::Gap, None, None),
        (RowShown::Row, RowKind::Metadata) => (DiffRowKind::Metadata, None, None),
        (RowShown::Row, RowKind::NoNewline { .. }) => (DiffRowKind::NoNewline, None, None),
        (RowShown::Row, RowKind::Code { left, right }) => {
            if app.split() {
                (DiffRowKind::Code, line(left, true)?, line(right, false)?)
            } else {
                let id = right.or(left);
                (DiffRowKind::Code, line(id, true)?, line(id, false)?)
            }
        }
    };
    let hunk = match (shown, row.kind) {
        (RowShown::Sticky, _) => None,
        (_, RowKind::Hunk(index)) => Some(index),
        (_, RowKind::Code { .. } | RowKind::NoNewline { .. }) => app
            .view
            .boundary(&app.doc, at.row, app.split(), false)
            .and_then(|heading| app.row(heading))
            .and_then(|heading| match heading.kind {
                RowKind::Hunk(index) if heading.file == row.file => Some(index),
                _ => None,
            }),
        _ => None,
    };
    let part = if shown == RowShown::Sticky {
        0
    } else {
        at.part
    };
    Ok(Some(DiffRow {
        review,
        file: row.file,
        path: file.meta.path.to_string(),
        kind,
        hunk,
        part,
        old_line,
        new_line,
        side: if old { Side::Old } else { Side::New },
    }))
}
