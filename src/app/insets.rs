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
    engine::insets::{InsetAnchor, Insets, Keys, Placed},
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
        self.insets.rows(self.inset_keys(index), self.split())
    }

    /// The insets under row `index`, laid out, in the order the cursor stops on them.
    fn row_layout(&self, index: usize) -> Vec<Placed> {
        if self.insets.is_empty() {
            return Vec::new();
        }
        self.insets.layout(self.inset_keys(index), self.split())
    }

    /// The insets drawn on the screen row `p`, with which of their rows each: one, or two side
    /// by side, the old side's first. Empty when `p` is a row of text.
    pub(crate) fn insets_on(&self, p: Position) -> Result<Vec<(Placed, usize)>> {
        if self.insets.is_empty() || p.part == 0 {
            return Ok(Vec::new());
        }
        let text = crate::render::text_height(self, p.row)?;
        let Some(offset) = p.part.checked_sub(text) else {
            return Ok(Vec::new());
        };
        Ok(self
            .row_layout(p.row)
            .into_iter()
            .filter(|placed| placed.contains(offset))
            .map(|placed| (placed, offset - placed.start))
            .collect())
    }

    /// The inset at `p` and which of its rows: of two side by side, the one on the old side
    /// when `old` says so, on the new side when it says not, and the first without it.
    fn placed_at(&self, p: Position, old: Option<bool>) -> Result<Option<(Placed, usize)>> {
        let mut on = self.insets_on(p)?;
        if on.len() > 1 && old == Some(false) {
            return Ok(on.pop());
        }
        Ok(on.into_iter().next())
    }

    /// The inset `p` is on, as its index in the set, and which of its rows. At the cursor, of
    /// two side by side, the one on the cursor's side.
    pub(crate) fn inset_at(&self, p: Position) -> Result<Option<(usize, usize)>> {
        let old = if p == self.cursor {
            self.inset_side
        } else {
            None
        };
        self.inset_at_side(p, old)
    }

    /// `inset_at`, choosing between two side by side by `old` as `placed_at` does.
    pub(crate) fn inset_at_side(
        &self,
        p: Position,
        old: Option<bool>,
    ) -> Result<Option<(usize, usize)>> {
        Ok(self
            .placed_at(p, old)?
            .map(|(placed, offset)| (placed.index, offset)))
    }

    /// The inset `p` is on, by id, and which of its rows.
    pub(crate) fn inset_id_at(&self, p: Position) -> Result<Option<(u64, usize)>> {
        Ok(self
            .inset_at(p)?
            .map(|(index, offset)| (self.insets.get(index).id, offset)))
    }

    /// The side whose column the cursor's inset takes, when it takes one.
    pub(crate) fn cursor_inset_column(&self) -> Option<bool> {
        let (placed, _) = self.placed_at(self.cursor, self.inset_side).ok()??;
        placed.column.old()
    }

    /// The row inset `index` hangs under, laid out there. `None` when its line or file is not
    /// displayed, or it keeps no rows.
    fn inset_place(&self, index: usize) -> Result<Option<(usize, Placed)>> {
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
        Ok(self
            .row_layout(row)
            .into_iter()
            .find(|placed| placed.index == index)
            .map(|placed| (row, placed)))
    }

    /// The position `offset` rows into the inset with `id`, kept within it, if it is shown,
    /// and the side of the code column it takes, when it takes one.
    pub(crate) fn inset_position(
        &self,
        id: u64,
        offset: usize,
    ) -> Result<Option<(Position, Option<bool>)>> {
        let Some(index) = self.insets.index(id) else {
            return Ok(None);
        };
        let Some((row, placed)) = self.inset_place(index)? else {
            return Ok(None);
        };
        let text = crate::render::text_height(self, row)?;
        let part = text + placed.start + offset.min(placed.height.saturating_sub(1));
        Ok(Some((Position { row, part }, placed.column.old())))
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

    /// Puts the cursor on `position`, an inset's row, and on the inset of two side by side
    /// that takes the column of `old`'s side. The code's side does not change.
    fn put_cursor(&mut self, (position, old): (Position, Option<bool>)) {
        self.cursor = position;
        self.inset_side = old;
    }

    /// The inset the cursor is on, to put it back there after the rows change.
    pub(crate) fn inset_spot(&self) -> Result<Option<InsetSpot>> {
        let Some((index, offset)) = self.inset_at(self.cursor)? else {
            return Ok(None);
        };
        let line = self.line_above(self.cursor.row)?;
        Ok(Some(InsetSpot {
            id: self.insets.get(index).id,
            offset,
            up: self.cursor.part - line.part,
            line_shown: self.laid_out_row(line)?.is_some(),
        }))
    }

    /// The last screen row of row `row`'s own text, which its insets hang under.
    fn line_above(&self, row: usize) -> Result<Position> {
        let part = crate::render::text_height(self, row)? - 1;
        Ok(Position { row, part })
    }

    /// Puts the cursor back on the inset it was on before the rows changed, `offset` rows
    /// into it as far as it is still tall, when it is still shown. Returns the last screen
    /// row of the line the inset hangs under and how far above the cursor's row it was, when
    /// it was on screen: the row to keep where it was, so that the code above the inset does
    /// not move whatever the insets' heights now.
    pub(super) fn restore_inset(
        &mut self,
        spot: Option<InsetSpot>,
    ) -> Result<Option<(Position, usize)>> {
        let Some(spot) = spot else {
            return Ok(None);
        };
        let Some(place) = self.inset_position(spot.id, spot.offset)? else {
            return Ok(None);
        };
        self.put_cursor(place);
        if !spot.line_shown {
            return Ok(None);
        }
        Ok(Some((self.line_above(place.0.row)?, spot.up)))
    }

    /// The next place the cursor stops after `p`, and the side of the inset there when it
    /// takes one: the next screen row of text, the next inset under the row (one stop
    /// however tall it is), and past every inset when insets are not stops. `p` when there is
    /// none.
    pub(super) fn next_stop(&self, p: Position) -> Result<(Position, Option<bool>)> {
        let text = crate::render::text_height(self, p.row)?;
        let layout = self.row_layout(p.row);
        if self.inset_stops && !layout.is_empty() {
            let next = if p.part + 1 < text {
                None
            } else if p.part < text {
                Some(0)
            } else {
                let index = self.inset_at(p)?.map(|(index, _)| index);
                layout
                    .iter()
                    .position(|placed| Some(placed.index) == index)
                    .map(|at| at + 1)
            };
            if let Some(placed) = next.and_then(|at| layout.get(at)) {
                let part = text + placed.start;
                return Ok((Position { row: p.row, part }, placed.column.old()));
            }
        }
        let from = if p.part + 1 < text {
            p
        } else {
            Position {
                row: p.row,
                part: self.row_height(p.row)? - 1,
            }
        };
        let next = self.next(from)?;
        Ok((if next == from { p } else { next }, None))
    }

    /// The place the cursor stops before `p`, as `next_stop` going up. Coming up onto an
    /// inset it stops on its last row, the nearest.
    pub(super) fn previous_stop(&self, p: Position) -> Result<(Position, Option<bool>)> {
        let text = crate::render::text_height(self, p.row)?;
        if p.part >= text {
            // On an inset: the one before it under the row, or the row's last row of text.
            if self.inset_stops {
                let layout = self.row_layout(p.row);
                let index = self.inset_at(p)?.map(|(index, _)| index);
                if let Some(at) = layout.iter().position(|placed| Some(placed.index) == index)
                    && let Some(placed) = at.checked_sub(1).and_then(|at| layout.get(at))
                {
                    let part = text + placed.start + placed.height - 1;
                    return Ok((Position { row: p.row, part }, placed.column.old()));
                }
            }
            let part = text - 1;
            return Ok((Position { row: p.row, part }, None));
        }
        if p.part > 0 {
            let part = p.part - 1;
            return Ok((Position { row: p.row, part }, None));
        }
        let Some(row) = p.row.checked_sub(1) else {
            return Ok((p, None));
        };
        let text = crate::render::text_height(self, row)?;
        if self.inset_stops
            && let Some(placed) = self.row_layout(row).last()
        {
            let part = text + placed.start + placed.height - 1;
            return Ok((Position { row, part }, placed.column.old()));
        }
        Ok((
            Position {
                row,
                part: text - 1,
            },
            None,
        ))
    }

    /// After a move of one stop onto an inset: shows all of the inset when it fits in the
    /// pane, and otherwise as much of it as fits with the cursor's row. Off an inset, `reveal`.
    pub(super) fn reveal_inset(&mut self) -> Result<()> {
        let Some((index, offset)) = self.inset_at(self.cursor)? else {
            return self.reveal();
        };
        if self.width < 40 || self.height < 10 {
            return self.reveal();
        }
        let start = Position {
            row: self.cursor.row,
            part: self.cursor.part - offset,
        };
        let end = Position {
            row: start.row,
            part: start.part + usize::from(self.insets.get(index).height) - 1,
        };
        let bottom = usize::from(self.height.saturating_sub(2));
        // Its first row, then its last, then the cursor's, which wins when it does not fit.
        if start < self.top {
            self.top = start;
        }
        if self.laid_out_row(end)?.is_none() {
            self.top = self.top_for(end, bottom)?;
        }
        if self.cursor < self.top {
            self.top = self.cursor;
        } else if self.laid_out_row(self.cursor)?.is_none() {
            self.top = self.top_for(self.cursor, bottom)?;
        }
        Ok(())
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
            let (found, _) = if forward {
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
        let (above, _) = self.previous_stop(self.cursor)?;
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
        // The screen row a reflow keeps the cursor on is where it is now: the old one can be
        // a row of an inset that has since got shorter, and the line above it would move.
        if let Some(y) = self.laid_out_row(self.cursor).ok().flatten()
            && let Some(anchor) = &mut self.reading_anchor
        {
            anchor.screen_row = Some(usize::from(y));
        }
        // The wrapped view that turning wrapping back on would return to had the old rows.
        self.unwrapped = None;
        self.visible.clear();
        Ok(())
    }

    /// Where the cursor and the top are among the current insets.
    fn inset_view(&self) -> Result<InsetView> {
        let top_on = self.insets_on(self.top)?;
        let below_top = match top_on.first() {
            Some(&(placed, offset)) => {
                let at = placed.start + offset;
                let mut below: Vec<_> = self
                    .row_layout(self.top.row)
                    .into_iter()
                    .filter(|placed| placed.start > at)
                    .collect();
                below.sort_by_key(|placed| placed.start);
                below
                    .into_iter()
                    .map(|placed| self.insets.get(placed.index).id)
                    .collect()
            }
            None => Vec::new(),
        };
        Ok(InsetView {
            cursor: self.cursor,
            top: self.top,
            screen_row: self.laid_out_row(self.cursor)?,
            cursor_inset: self.inset_spot()?,
            at_start: self.top == Position::default(),
            top_on: top_on
                .into_iter()
                .map(|(placed, offset)| (self.insets.get(placed.index).id, offset))
                .collect(),
            below_top,
        })
    }

    /// With the new insets in place, puts the cursor and the top where `before` says the
    /// person was reading.
    fn keep_view(&mut self, before: InsetView) -> Result<()> {
        // The screen row a call just before could not keep, while nothing has moved since.
        let wish = self
            .inset_wish
            .take()
            .filter(|&(cursor, top, _)| (cursor, top) == (before.cursor, before.top))
            .map(|(_, _, y)| y);
        // What stays on its screen row: the cursor's row, or the first row of the inset the
        // cursor is on when that was on screen.
        let mut kept = before.screen_row.map(|y| (self.cursor, usize::from(y)));
        if let Some(spot) = before.cursor_inset {
            let line = self.line_above(self.cursor.row)?;
            match self.inset_position(spot.id, spot.offset)? {
                Some(place) if place.0.row == self.cursor.row => self.put_cursor(place),
                _ => {
                    // The inset is gone from under the row: back to the line it hung under.
                    self.cursor = line;
                    self.inset_side = None;
                    self.reading_anchor = None;
                }
            }
            // The line keeps its screen row when it was on screen or the cursor is back on it;
            // otherwise the cursor's row on the inset is kept.
            let on_line = spot.line_shown || self.cursor == line;
            kept = kept.map(|(_, y)| match on_line {
                true => (line, y.saturating_sub(spot.up).max(1)),
                false => (self.cursor, y),
            });
        }
        if before.at_start && wish.is_none() {
            // The view at the very start stays there, insets under the first rows included,
            // unless that puts the cursor's row, which was on screen, under the last.
            self.top = Position::default();
            if let Some((_, y)) = kept
                && self.laid_out_row(self.cursor)?.is_none()
            {
                let bottom = usize::from(self.height.saturating_sub(2));
                self.top = self.top_for(self.cursor, bottom)?;
                self.inset_wish = Some((self.cursor, self.top, y));
            }
            return Ok(());
        }
        if let Some((at, y)) = kept {
            let y = wish.unwrap_or(y);
            self.top = self.top_for(at, y)?;
            // Too few rows above to keep it there: the next call tries again.
            if self.laid_out_row(at)? != u16::try_from(y).ok() {
                self.inset_wish = Some((self.cursor, self.top, y));
            }
            return Ok(());
        }
        // Off screen: the row at the top stays there.
        if !before.top_on.is_empty() {
            let mut kept = None;
            for &(id, offset) in &before.top_on {
                if let Some((position, _)) = self.inset_position(id, offset)?
                    && position.row == self.top.row
                {
                    kept = Some(position);
                    break;
                }
            }
            self.top = match kept {
                Some(position) => position,
                None => self.under_removed(self.top.row, &before.below_top)?,
            };
        }
        Ok(())
    }

    /// What follows a removed inset under row `row`: the first of the insets that were below
    /// it that is still there, or the next row.
    fn under_removed(&self, row: usize, below: &[u64]) -> Result<Position> {
        let mut first: Option<Position> = None;
        for &id in below {
            if let Some((position, _)) = self.inset_position(id, 0)?
                && position.row == row
            {
                first = Some(first.map_or(position, |first| first.min(position)));
            }
        }
        if let Some(position) = first {
            return Ok(position);
        }
        let last = Position {
            row,
            part: self.row_height(row)?.saturating_sub(1),
        };
        self.next(last)
    }
}

/// The inset the cursor is on, by id, which of its rows, how far below the last screen row
/// of the line it hangs under, and whether that row was on screen.
#[derive(Clone, Copy, Debug)]
pub(crate) struct InsetSpot {
    id: u64,
    offset: usize,
    up: usize,
    line_shown: bool,
}

/// Where the cursor and the top were among the insets before they changed.
struct InsetView {
    cursor: Position,
    top: Position,
    /// The cursor's screen row, when it is on screen.
    screen_row: Option<u16>,
    /// The inset the cursor is on.
    cursor_inset: Option<InsetSpot>,
    /// The top is the first row of the diff.
    at_start: bool,
    /// The insets the top is on, by id, and which of their rows: one, or two side by side.
    top_on: Vec<(u64, usize)>,
    /// The insets that started below the top under its row, by id.
    below_top: Vec<u64>,
}
