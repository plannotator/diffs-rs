//! Read-only answers about the rows the engine lays out: the row under the cursor, and the
//! row drawn at a screen cell. Neither moves the cursor, the view or the selection.

use anyhow::Result;

use super::{Engine, ReviewId, Side};
use crate::{
    app::{App, Position},
    model::RowKind,
    view::LineId,
};

/// One row of the diff: the file it belongs to and what it shows, from
/// [`Engine::cursor_row`] or [`Engine::row_at`].
///
/// Every row belongs to a file, the headings, gaps and notes included.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DiffRow {
    /// The review the row is from, as [`Engine::review_id`] reports it. A row read before
    /// [`Engine::replace`] has the earlier review's id.
    pub review: ReviewId,
    /// The file's index, as [`Engine::file`] takes it.
    pub file: usize,
    /// The file's [`FileMetadata::path`](crate::storage::FileMetadata::path).
    pub path: String,
    /// What the row shows: a heading, a hunk's heading, a gap, a note, code, a
    /// "No newline" mark or a host inset.
    pub kind: DiffRowKind,
    /// The index of the file's hunk the row is part of: on a hunk's heading, on the hunk's
    /// own lines and on its own "No newline" marks. `None` on headings, gaps and notes, and on
    /// context lines a gap revealed and their "No newline" mark, which are in no hunk. On an
    /// inset, the hunk of the line it hangs under.
    pub hunk: Option<usize>,
    /// Which screen row of a wrapped row: 0 for its first, 1 for the next, and so on. On an
    /// inset, which of its rows, from 0.
    pub part: usize,
    /// The original one-based lines the row shows. A split row takes its old line from the
    /// left cell and its new line from the right one; either can be empty. Unified, a context
    /// line has both, an added line only `new_line`, a deleted line only `old_line`. A pair
    /// of lines drawn as unchanged because whitespace is hidden has both, like context. On an
    /// inset, the line it hangs under, on the inset's side only; neither under a heading.
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    /// Whether the row shows changes that are hidden as whitespace only, by the engine's own
    /// rule for the current [`Engine::set_whitespace`] setting: on a code row, a removed and
    /// an added line that differ only in whitespace, drawn as one unchanged line; on a hunk's
    /// heading, a hunk whose every change is such a pair, drawn as its heading alone.
    /// [`Engine::hunk_is_whitespace_only`] answers the same for a hunk without its row.
    /// Always `false` while whitespace is shown.
    pub whitespace_only: bool,
    /// The side of the row.
    ///
    /// - Split: from [`Engine::cursor_row`], the pane the cursor is in; from
    ///   [`Engine::row_at`], the pane of the cell, by the rule a click uses, so the column
    ///   between the panes counts as the side a click there would pick.
    /// - Unified, on a line: the side of the line as [`Engine::selected_source`] reports it.
    ///   On a row with no line, the side the cursor is on, and [`Side::New`] on the sticky
    ///   heading.
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
    /// A row of the host inset with this [`Inset::id`](super::Inset::id), set by
    /// [`Engine::set_insets`]. [`DiffRow::part`] says which of its rows.
    Inset { id: u64 },
}

impl Engine {
    /// The row under the cursor, read from the cursor itself, so it is current after any
    /// command or event, before the next frame. Nothing moves.
    ///
    /// Unlike [`Engine::selected_source`], it answers on rows with no line under the cursor:
    /// a heading, a hunk's heading, a gap, a binary or mode-only file's note, an empty side of
    /// a split row, a "No newline" mark, every wrapped part of a row, and a host inset
    /// ([`DiffRowKind::Inset`]).
    ///
    /// `None` when there is no true answer: before the first file arrives, for a diff with
    /// no files, after [`Engine::replace`] until the new files arrive, and on a revealed
    /// context line whose source was withdrawn.
    ///
    /// ```
    /// use diffs_rs::engine::{DiffRowKind, Engine};
    ///
    /// /// The path of the file the cursor is in, and whether it is on a line.
    /// fn under_cursor(engine: &Engine) -> Option<(String, bool)> {
    ///     let row = engine.cursor_row()?;
    ///     let on_line = matches!(row.kind, DiffRowKind::Code);
    ///     Some((row.path, on_line))
    /// }
    /// ```
    pub fn cursor_row(&self) -> Option<DiffRow> {
        let app = self.app.as_ref()?;
        let old = if app.split() {
            app.old_side
        } else {
            unified_old(app, app.cursor.row)
        };
        describe(self.id, app, app.cursor, RowShown::Row, old, app.inset_side)
            .ok()
            .flatten()
    }

    /// The row drawn at the screen cell (`column`, `row`) in the last frame, as a click there
    /// would find it, or [`DiffRowKind::StickyHeader`] for the heading kept over the top row,
    /// which takes no click. Nothing moves.
    ///
    /// `None` when there is no true answer: before the first draw, while the help is shown,
    /// outside the diff's area, below its last row, for a diff with no files, after
    /// [`Engine::replace`] until the new files are drawn, and on a revealed context line whose
    /// source was withdrawn. Between a move and the next frame it answers for the frame last
    /// drawn, as a click does.
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
                super::split::old_at(app.width, app.split_ratio, column - area.x)
            } else {
                unified_old(app, position.row)
            };
            return describe(
                self.id,
                app,
                *position,
                RowShown::Row,
                old,
                split_old(app, old),
            )
            .ok()
            .flatten();
        }
        // The top row holds the sticky heading of the file the first row drawn is in.
        let (first, at) = app.visible.first()?;
        if y == 1 && *at == 2 {
            let old =
                app.split() && super::split::old_at(app.width, app.split_ratio, column - area.x);
            return describe(self.id, app, *first, RowShown::Sticky, old, None)
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

/// `old` in the split layout, where it picks one of two insets side by side.
fn split_old(app: &App, old: bool) -> Option<bool> {
    app.split().then_some(old)
}

/// The row at `at`, for `old`'s side. Of two side insets beside each other there, the one on
/// the old side when `inset` says so, the new side's when it says not, the first without it.
fn describe(
    review: ReviewId,
    app: &App,
    at: Position,
    shown: RowShown,
    old: bool,
    inset: Option<bool>,
) -> Result<Option<DiffRow>> {
    let Some(row) = app.row(at.row) else {
        return Ok(None);
    };
    let Some(file) = app.doc.files.get(row.file) else {
        return Ok(None);
    };
    if shown == RowShown::Row
        && let Some((index, offset)) = app.inset_at_side(at, inset)?
    {
        let inset = app.insets.get(index);
        let (old_line, new_line) = match inset.anchor {
            super::InsetAnchor::Line {
                side: Side::Old,
                line,
            } => (Some(line), None),
            super::InsetAnchor::Line { line, .. } => (None, Some(line)),
            _ => (None, None),
        };
        let line = Position {
            row: at.row,
            part: 0,
        };
        let hunk = match row.kind {
            RowKind::Code { .. } => {
                describe(review, app, line, shown, old, None)?.and_then(|r| r.hunk)
            }
            _ => None,
        };
        return Ok(Some(DiffRow {
            review,
            file: row.file,
            path: file.meta.path.to_string(),
            kind: DiffRowKind::Inset { id: inset.id },
            hunk,
            part: offset,
            old_line,
            new_line,
            whitespace_only: false,
            // A side inset is on its side of a split diff.
            side: match inset.span {
                super::InsetSpan::Side(side) if app.split() => side,
                _ if old => Side::Old,
                _ => Side::New,
            },
        }));
    }
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
                // The old number is the left line's: a removed line paired with this added
                // one when whitespace is hidden, and otherwise the line itself.
                let id = right.or(left);
                (
                    DiffRowKind::Code,
                    line(left.or(id), true)?,
                    line(id, false)?,
                )
            }
        }
    };
    let hunk = match (shown, row.kind) {
        (RowShown::Sticky, _) => None,
        (_, RowKind::Hunk(index)) => Some(index),
        (_, RowKind::Code { .. } | RowKind::NoNewline { .. })
            if app.view.is_patch_row(&app.doc, at.row, app.split()) =>
        {
            app.view
                .boundary(&app.doc, at.row, app.split(), false)
                .and_then(|heading| app.row(heading))
                .and_then(|heading| match heading.kind {
                    RowKind::Hunk(index) if heading.file == row.file => Some(index),
                    _ => None,
                })
        }
        _ => None,
    };
    let part = if shown == RowShown::Sticky {
        0
    } else {
        at.part
    };
    let whitespace_only = match (shown, row.kind) {
        (RowShown::Row, RowKind::Hunk(index)) => file.hunk_hidden(index),
        (
            RowShown::Row,
            RowKind::Code {
                left: Some(LineId::Patch(id)),
                ..
            },
        ) => file.whitespace_only(id),
        _ => false,
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
        whitespace_only,
        side: if old { Side::Old } else { Side::New },
    }))
}
