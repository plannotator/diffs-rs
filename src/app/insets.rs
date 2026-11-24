//! Host insets in the viewer state: the rows they keep under the rows of the diff, where the
//! cursor stops, and how the view stays put when the host changes them.
//!
//! An inset's rows are screen rows of the row it hangs under, after that row's own text: a row
//! of height `text + insets` has positions `text..` on its insets. So everything that walks
//! the screen row by row (scrolling, paging, the last page, the sticky heading, reveals)
//! counts inset rows as it counts wrapped parts, and nothing stores row numbers for them.

use anyhow::Result;

use super::{App, Position};
use crate::{
    engine::insets::{InsetAnchor, Insets, Keys},
    model::RowKind,
};

impl App {
    /// What row `index` can carry insets for: the lines it shows, by number, or its file when
    /// it is a heading. A line whose number cannot be read carries none.
    fn inset_keys(&self, index: usize) -> Keys {
        let Some(row) = self.row(index) else {
            return Keys::None;
        };
        match row.kind {
            RowKind::Header => Keys::Heading(row.file),
            RowKind::Code { left, right } => {
                let info =
                    |id: Option<crate::view::LineId>| id.and_then(|id| self.line_info(id).ok());
                // The numbers `Engine::cursor_row` reports for the row.
                let (old, new) = if self.split() {
                    (
                        info(left).and_then(|i| i.old),
                        info(right).and_then(|i| i.new),
                    )
                } else {
                    let id = right.or(left);
                    (
                        info(left.or(id)).and_then(|i| i.old),
                        info(id).and_then(|i| i.new),
                    )
                };
                Keys::Lines {
                    file: row.file,
                    old,
                    new,
                }
            }
            _ => Keys::None,
        }
    }

    /// The screen rows the insets under row `index` keep.
    pub(crate) fn inset_rows(&self, index: usize) -> usize {
        if self.insets.is_empty() {
            return 0;
        }
        self.insets.rows(self.inset_keys(index))
    }

    /// The insets under row `index`, top to bottom, as indexes into the set.
    pub(crate) fn row_insets(&self, index: usize) -> Vec<usize> {
        if self.insets.is_empty() {
            return Vec::new();
        }
        self.insets.members(self.inset_keys(index))
    }

    /// The inset `p` is on, as its index in the set, and which of its rows.
    pub(crate) fn inset_at(&self, p: Position) -> Result<Option<(usize, usize)>> {
        if self.insets.is_empty() || p.part == 0 {
            return Ok(None);
        }
        let text = crate::render::text_height(self, p.row)?;
        let Some(mut offset) = p.part.checked_sub(text) else {
            return Ok(None);
        };
        for index in self.row_insets(p.row) {
            let height = usize::from(self.insets.get(index).height);
            if offset < height {
                return Ok(Some((index, offset)));
            }
            offset -= height;
        }
        Ok(None)
    }

    /// The inset `p` is on, by id, and which of its rows.
    pub(crate) fn inset_id_at(&self, p: Position) -> Result<Option<(u64, usize)>> {
        Ok(self
            .inset_at(p)?
            .map(|(index, offset)| (self.insets.get(index).id, offset)))
    }

    /// The row inset `index` hangs under and the position of its first screen row. `None`
    /// when its line or file is not displayed, or it keeps no rows.
    pub(crate) fn inset_place(&self, index: usize) -> Result<Option<Position>> {
        let inset = *self.insets.get(index);
        if inset.height == 0 {
            return Ok(None);
        }
        let row = match inset.anchor {
            InsetAnchor::Line { side, line } => self
                .displayed_source(inset.file, side == crate::engine::Side::Old, line)
                .and_then(|id| self.view.source_row(&self.doc, id, self.split())),
            InsetAnchor::FileHeading => (inset.file < self.doc.files.len())
                .then(|| self.view.file_row(&self.doc, inset.file, self.split()))
                .filter(|&row| self.row(row).is_some_and(|r| r.kind == RowKind::Header)),
        };
        let Some(row) = row else {
            return Ok(None);
        };
        let mut part = crate::render::text_height(self, row)?;
        for member in self.row_insets(row) {
            if member == index {
                return Ok(Some(Position { row, part }));
            }
            part += usize::from(self.insets.get(member).height);
        }
        Ok(None)
    }

    /// The position `offset` rows into the inset with `id`, kept within it, if it is shown.
    pub(crate) fn inset_position(&self, id: u64, offset: usize) -> Result<Option<Position>> {
        let Some(index) = self.insets.index(id) else {
            return Ok(None);
        };
        let height = usize::from(self.insets.get(index).height);
        Ok(self.inset_place(index)?.map(|start| Position {
            row: start.row,
            part: start.part + offset.min(height.saturating_sub(1)),
        }))
    }

    /// Whether inset `index` hangs under a displayed row. One of height 0 counts as shown when
    /// its line is.
    pub(crate) fn inset_shown(&self, index: usize) -> bool {
        let inset = *self.insets.get(index);
        match inset.anchor {
            InsetAnchor::Line { side, line } => self
                .displayed_source(inset.file, side == crate::engine::Side::Old, line)
                .is_some(),
            InsetAnchor::FileHeading => inset.file < self.doc.files.len(),
        }
    }

    /// Puts the cursor back on the inset it was on before the rows changed, when it is still
    /// shown.
    pub(super) fn restore_inset(&mut self, inset: Option<(u64, usize)>) -> Result<()> {
        if let Some((id, offset)) = inset
            && let Some(position) = self.inset_position(id, offset)?
        {
            self.cursor = position;
        }
        Ok(())
    }

    /// The first position of the inset `p` is on, or `p`.
    fn inset_start(&self, p: Position) -> Result<Position> {
        Ok(match self.inset_at(p)? {
            Some((_, offset)) => Position {
                row: p.row,
                part: p.part - offset,
            },
            None => p,
        })
    }

    /// The last position of the inset `p` is on, or `p`.
    fn inset_end(&self, p: Position) -> Result<Position> {
        Ok(match self.inset_at(p)? {
            Some((index, offset)) => Position {
                row: p.row,
                part: p.part + usize::from(self.insets.get(index).height) - 1 - offset,
            },
            None => p,
        })
    }

    /// The next place the cursor stops after `p`: the next screen row, past the rest of the
    /// inset `p` is on, and past every inset when insets are not stops. `p` when there is none.
    pub(super) fn next_stop(&self, p: Position) -> Result<Position> {
        let mut at = p;
        loop {
            let end = self.inset_end(at)?;
            let next = self.next(end)?;
            if next == end {
                return Ok(p);
            }
            if self.inset_stops || self.inset_at(next)?.is_none() {
                return Ok(next);
            }
            at = next;
        }
    }

    /// The place the cursor stops before `p`, as `next_stop` going up. Coming up onto an
    /// inset it stops on its last row, the nearest.
    pub(super) fn previous_stop(&self, p: Position) -> Result<Position> {
        let mut at = p;
        loop {
            let start = self.inset_start(at)?;
            let previous = self.previous(start)?;
            if previous == start {
                return Ok(p);
            }
            if self.inset_stops || self.inset_at(previous)?.is_none() {
                return Ok(previous);
            }
            at = previous;
        }
    }

    /// When insets are not stops and `p` is on one: the nearest row in `from..=to` that is
    /// not an inset, looking `forward` first. `p` when there is none.
    pub(super) fn off_inset(
        &self,
        p: Position,
        forward: bool,
        (from, to): (Position, Position),
    ) -> Result<Position> {
        if self.inset_stops || self.inset_at(p)?.is_none() {
            return Ok(p);
        }
        for forward in [forward, !forward] {
            let found = if forward {
                self.next_stop(p)?
            } else {
                self.previous_stop(p)?
            };
            if found != p && (from..=to).contains(&found) {
                return Ok(found);
            }
        }
        Ok(p)
    }

    /// Insets become stops or stop being stops. Turned off with the cursor on an inset, the
    /// cursor goes to the row above it, which does not move.
    pub(crate) fn set_inset_stops(&mut self, on: bool) -> Result<()> {
        self.inset_stops = on;
        if on || self.inset_at(self.cursor)?.is_none() {
            return Ok(());
        }
        let above = self.previous_stop(self.cursor)?;
        let shown = self.laid_out_row(above)?.is_some();
        self.cursor = above;
        self.reading_anchor = None;
        if !shown {
            self.reveal()?;
        }
        self.visible.clear();
        Ok(())
    }

    /// Replaces the insets without moving what the person is reading: the source line on the
    /// cursor's screen row stays on that row and the horizontal scroll is kept. On an error
    /// nothing changed.
    pub(crate) fn replace_insets(&mut self, insets: Insets) -> Result<()> {
        let saved = (self.cursor, self.top, self.reading_anchor, self.unwrapped);
        // Read with the old insets in place.
        let before = self.inset_view()?;
        let old = std::mem::replace(&mut self.insets, insets);
        if let Err(error) = self.keep_view(before) {
            self.insets = old;
            (self.cursor, self.top, self.reading_anchor, self.unwrapped) = saved;
            return Err(error);
        }
        // The wrapped view that turning wrapping back on would return to had the old rows.
        self.unwrapped = None;
        self.visible.clear();
        Ok(())
    }

    /// Where the cursor and the top are among the current insets.
    fn inset_view(&self) -> Result<InsetView> {
        let top_inset = self.inset_at(self.top)?;
        let below_top = match top_inset {
            Some((index, _)) => {
                let members = self.row_insets(self.top.row);
                let at = members.iter().position(|&m| m == index).unwrap_or(0);
                members[at + 1..]
                    .iter()
                    .map(|&m| self.insets.get(m).id)
                    .collect()
            }
            None => Vec::new(),
        };
        Ok(InsetView {
            screen_row: self.laid_out_row(self.cursor)?,
            cursor_inset: self.inset_id_at(self.cursor)?,
            text: crate::render::text_height(self, self.cursor.row)?,
            top_inset: top_inset.map(|(index, offset)| (self.insets.get(index).id, offset)),
            below_top,
        })
    }

    /// With the new insets in place, puts the cursor and the top where `before` says the
    /// person was reading.
    fn keep_view(&mut self, before: InsetView) -> Result<()> {
        let mut screen_row = before.screen_row.map(usize::from);
        if let Some((id, offset)) = before.cursor_inset {
            match self.inset_position(id, offset)? {
                Some(position) if position.row == self.cursor.row => self.cursor = position,
                _ => {
                    // The inset is gone from under the row: back to the row's last screen row
                    // of text, which stays where it was.
                    let last = before.text - 1;
                    let up = self.cursor.part - last;
                    screen_row = screen_row.map(|y| y.saturating_sub(up).max(1));
                    self.cursor.part = last;
                    self.reading_anchor = None;
                }
            }
        }
        if let Some(y) = screen_row {
            self.top = self.top_for(self.cursor, y)?;
            return Ok(());
        }
        // Off screen: the row at the top stays there.
        if let Some((id, offset)) = before.top_inset {
            self.top = match self.inset_position(id, offset)? {
                Some(position) if position.row == self.top.row => position,
                _ => self.under_removed(self.top.row, &before.below_top)?,
            };
        }
        Ok(())
    }

    /// What follows a removed inset under row `row`: the first of the insets that were below
    /// it that is still there, or the next row.
    fn under_removed(&self, row: usize, below: &[u64]) -> Result<Position> {
        for &id in below {
            if let Some(position) = self.inset_position(id, 0)?
                && position.row == row
            {
                return Ok(position);
            }
        }
        let last = Position {
            row,
            part: self.row_height(row)?.saturating_sub(1),
        };
        self.next(last)
    }
}

/// Where the cursor and the top were among the insets before they changed.
struct InsetView {
    /// The cursor's screen row, when it is on screen.
    screen_row: Option<u16>,
    /// The inset the cursor is on, by id, and which of its rows.
    cursor_inset: Option<(u64, usize)>,
    /// The screen rows of text of the cursor's row.
    text: usize,
    /// The inset the top is on, by id, and which of its rows.
    top_inset: Option<(u64, usize)>,
    /// The insets below the top's under its row, by id.
    below_top: Vec<u64>,
}
