//! Host insets: blank rows the engine reserves between the rows of the diff, under a line or
//! under a file's heading, for the host to draw into after each frame.

use std::collections::{HashMap, HashSet};

use anyhow::{Result, ensure};
use ratatui::layout::Rect;

use super::{Engine, InputEffect, ReviewId, Side};

/// Rows the engine keeps free under a row of the diff, for the host to draw into: a review
/// comment shown inline, for example. Set with [`Engine::set_insets`].
///
/// The engine draws only the background there, the line between the sides of a split diff,
/// and the cursor's `▎` in the gutter while the cursor is on the inset. After each
/// [`Engine::draw`], [`Engine::inset_areas`] says where every visible inset landed.
///
/// Built with [`Inset::new`], which takes the whole width, and [`Inset::with_span`]:
///
/// ```
/// use diffs_rs::engine::{Engine, Inset, InsetAnchor, InsetSpan, Side};
///
/// fn note_under(engine: &Engine, file: usize, line: u32) -> Inset {
///     let anchor = InsetAnchor::Line { side: Side::New, line };
///     Inset::new(engine.review_id(), 7, file, anchor, 3).with_span(InsetSpan::Side(Side::New))
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Inset {
    /// The review the inset is for, as [`Engine::review_id`] reports it.
    pub review: ReviewId,
    /// The host's name for the inset, reported back by [`Engine::inset_areas`],
    /// [`Engine::hidden_insets`] and [`DiffRowKind::Inset`](super::DiffRowKind::Inset). Each
    /// inset in a set needs its own.
    pub id: u64,
    /// The file's index, as [`Engine::file`] takes it.
    pub file: usize,
    /// Where in the file the inset hangs.
    pub anchor: InsetAnchor,
    /// Which columns of a split diff the inset takes, and so how it shares rows with the other
    /// insets under its row. Unified, every inset takes the code column.
    pub span: InsetSpan,
    /// The number of screen rows to keep free. An inset of height 0 keeps none: it is never
    /// drawn, reported or stopped on, but it is not hidden either.
    pub height: u16,
}

impl Inset {
    /// An inset of `height` rows for `review`, named `id`, hanging at `anchor` in `file`,
    /// across the diff ([`InsetSpan::Full`]).
    #[must_use]
    pub fn new(review: ReviewId, id: u64, file: usize, anchor: InsetAnchor, height: u16) -> Self {
        Self {
            review,
            id,
            file,
            anchor,
            span: InsetSpan::Full,
            height,
        }
    }

    /// The same inset in the columns of `span`.
    #[must_use]
    pub fn with_span(self, span: InsetSpan) -> Self {
        Self { span, ..self }
    }
}

/// Where an [`Inset`] hangs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InsetAnchor {
    /// Under the row that shows original line `line` (one-based) on `side`, below the last
    /// screen row of that row when it wraps. In the split layout that row can show a line of
    /// the other side too; insets on both hang under the same row.
    Line { side: Side, line: u32 },
    /// Under the file's heading.
    FileHeading,
}

/// Which columns of a split diff an [`Inset`] takes, and so how insets under the same row
/// share its rows.
///
/// In the split layout the insets under a row are laid out in the order given, in blocks. A
/// [`InsetSpan::Full`] inset is a block of its own, across both sides. A run of
/// [`InsetSpan::Side`] insets with no `Full` one between them is one block: the old side's
/// insets stack in the left column and the new side's in the right one, both from the block's
/// first row, and the block is as tall as its taller column. Unified, every inset is a block
/// of its own, in the order given, whatever its span.
///
/// ```text
/// [Side(Old) a, Side(New) b, Full c, Side(New) d]
///
///   a a │ b b     a and b side by side; the block is as tall as a,
///   a a │         and the cells beside the end of b are blank
///   c c c c c     c across both sides
///       │ d d     d in the right column, the left one blank
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InsetSpan {
    /// Across the diff, from the old side's code to the right edge. The default.
    #[default]
    Full,
    /// The code column of one side, beside the other side's insets of the same block.
    Side(Side),
}

/// Where an [`Inset`] was drawn in the last frame, from [`Engine::inset_areas`]. Only the
/// rows on screen are reported: a host draws its content's rows from `clipped_top` on, as
/// many as `area.height`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct InsetArea {
    /// The [`Inset::id`].
    pub id: u64,
    /// The inset's rows on screen, over the columns it takes. Unified, the code column. Split,
    /// for [`InsetSpan::Full`], from the old side's code across the line between the sides and
    /// the new side's gutter to the right edge; for [`InsetSpan::Side`], that side's code
    /// column. [`Engine::inset_width`] gives the same width before the inset is drawn.
    pub area: Rect,
    /// How many of the inset's rows are above the top of the pane, or under the file heading
    /// kept over the top row.
    pub clipped_top: u16,
    /// The inset's full height, as set. Rows cut at the bottom of the pane are
    /// `height - clipped_top - area.height`.
    pub height: u16,
    /// The cursor is on the inset.
    pub cursor: bool,
}

#[derive(Default)]
struct Group {
    /// Indexes into the set, in the order the host gave them.
    members: Vec<usize>,
    rows: usize,
    /// Some member takes one side only.
    sides: bool,
}

impl Group {
    fn add(&mut self, index: usize, inset: &Inset) {
        self.members.push(index);
        self.rows += usize::from(inset.height);
        self.sides |= matches!(inset.span, InsetSpan::Side(_));
    }
}

/// What a row of the diff can carry insets for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Keys {
    None,
    Heading(usize),
    Lines {
        file: usize,
        old: Option<u32>,
        new: Option<u32>,
    },
}

/// The columns of a row an inset is drawn in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Column {
    /// The old side's code column, beside the new side's.
    Old,
    /// The new side's code column, beside the old side's.
    New,
    /// Across the diff, or the code column unified.
    Full,
}

impl Column {
    /// `Some(true)` for the old side's column, `Some(false)` for the new side's.
    pub(crate) fn old(self) -> Option<bool> {
        match self {
            Self::Old => Some(true),
            Self::New => Some(false),
            Self::Full => None,
        }
    }
}

/// Where an inset is among the insets under its row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Placed {
    /// The inset's index in the set.
    pub(crate) index: usize,
    pub(crate) column: Column,
    /// The first screen row of its block, and its own, counted from the first row under the
    /// row's text.
    pub(crate) block: usize,
    pub(crate) start: usize,
    pub(crate) height: usize,
}

impl Placed {
    pub(crate) fn contains(&self, offset: usize) -> bool {
        (self.start..self.start + self.height).contains(&offset)
    }
}

/// Lays out insets given in order, as [`InsetSpan`] describes. Calls `place` with each one's
/// column and first row, and returns the rows they take.
fn lay_out(
    members: impl Iterator<Item = (usize, Inset)>,
    split: bool,
    mut place: impl FnMut(Placed),
) -> usize {
    // The first row of the current block, and the rows each column of it has taken.
    let (mut top, mut old, mut new) = (0, 0, 0);
    for (index, inset) in members {
        let height = usize::from(inset.height);
        let column = match inset.span {
            InsetSpan::Side(Side::Old) if split => Column::Old,
            InsetSpan::Side(Side::New) if split => Column::New,
            _ => Column::Full,
        };
        let (block, start) = match column {
            Column::Old => {
                old += height;
                (top, top + old - height)
            }
            Column::New => {
                new += height;
                (top, top + new - height)
            }
            Column::Full => {
                let start = top + old.max(new);
                (top, old, new) = (start + height, 0, 0);
                (start, start)
            }
        };
        place(Placed {
            index,
            column,
            block,
            start,
            height,
        });
    }
    top + old.max(new)
}

/// Two ascending lists of indexes merged into one.
fn merged<'a>(a: &'a [usize], b: &'a [usize]) -> impl Iterator<Item = usize> + 'a {
    let (mut i, mut j) = (0, 0);
    std::iter::from_fn(move || {
        let first = match (a.get(i), b.get(j)) {
            (Some(x), Some(y)) => x < y,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => return None,
        };
        if first {
            i += 1;
            Some(a[i - 1])
        } else {
            j += 1;
            Some(b[j - 1])
        }
    })
}

/// The host's insets, grouped by what they hang under, so finding the insets of a drawn row
/// costs a hash lookup or two however many insets exist. Rows are not stored: a row finds its
/// insets by the lines it shows, so the set stays right across layouts, wrapping, revealed
/// context and hidden whitespace without being rebuilt.
#[derive(Default)]
pub(crate) struct Insets {
    list: Vec<Inset>,
    lines: HashMap<(usize, bool, u32), Group>,
    headings: HashMap<usize, Group>,
    ids: HashMap<u64, usize>,
    /// No inset keeps a row.
    empty: bool,
}

impl Insets {
    fn new(list: &[Inset]) -> Self {
        let mut lines: HashMap<(usize, bool, u32), Group> = HashMap::new();
        let mut headings: HashMap<usize, Group> = HashMap::new();
        for (index, inset) in list.iter().enumerate() {
            if inset.height == 0 {
                continue;
            }
            match inset.anchor {
                InsetAnchor::Line { side, line } => lines
                    .entry((inset.file, side == Side::Old, line))
                    .or_default()
                    .add(index, inset),
                InsetAnchor::FileHeading => {
                    headings.entry(inset.file).or_default().add(index, inset)
                }
            }
        }
        Self {
            ids: list
                .iter()
                .enumerate()
                .map(|(index, inset)| (inset.id, index))
                .collect(),
            empty: lines.is_empty() && headings.is_empty(),
            list: list.to_vec(),
            lines,
            headings,
        }
    }

    /// No inset keeps a row, so every row is as tall as its text.
    pub(crate) fn is_empty(&self) -> bool {
        self.empty
    }

    pub(crate) fn list(&self) -> &[Inset] {
        &self.list
    }

    pub(crate) fn get(&self, index: usize) -> &Inset {
        &self.list[index]
    }

    pub(crate) fn index(&self, id: u64) -> Option<usize> {
        self.ids.get(&id).copied()
    }

    fn groups(&self, keys: Keys) -> [Option<&Group>; 2] {
        match keys {
            Keys::None => [None, None],
            Keys::Heading(file) => [self.headings.get(&file), None],
            Keys::Lines { file, old, new } => [
                old.and_then(|line| self.lines.get(&(file, true, line))),
                new.and_then(|line| self.lines.get(&(file, false, line))),
            ],
        }
    }

    /// The insets under a row with `keys`, in the order given.
    fn members(&self, keys: Keys) -> impl Iterator<Item = (usize, Inset)> + '_ {
        let [a, b] = self.groups(keys);
        let a = a.map_or(&[][..], |g| &g.members[..]);
        let b = b.map_or(&[][..], |g| &g.members[..]);
        merged(a, b).map(|index| (index, self.list[index]))
    }

    /// The screen rows the insets under a row with `keys` take.
    pub(crate) fn rows(&self, keys: Keys, split: bool) -> usize {
        let groups = self.groups(keys);
        if !split || !groups.iter().flatten().any(|g| g.sides) {
            return groups.iter().flatten().map(|g| g.rows).sum();
        }
        lay_out(self.members(keys), split, |_| {})
    }

    /// The insets under a row with `keys`, laid out, in the order the cursor stops on them:
    /// block by block, and in a block of side insets the old side's column top to bottom,
    /// then the new side's.
    pub(crate) fn layout(&self, keys: Keys, split: bool) -> Vec<Placed> {
        let mut placed = Vec::new();
        lay_out(self.members(keys), split, |p| placed.push(p));
        placed.sort_by_key(|p| (p.block, p.column, p.start));
        placed
    }
}

fn validate(review: ReviewId, insets: &[Inset]) -> Result<()> {
    let mut ids = HashSet::with_capacity(insets.len());
    for inset in insets {
        ensure!(inset.review == review, "Inset belongs to another review");
        ensure!(ids.insert(inset.id), "Inset id {} is used twice", inset.id);
        if let InsetAnchor::Line { line, .. } = inset.anchor {
            ensure!(line != 0, "Inset {} is on line 0", inset.id);
        }
    }
    Ok(())
}

impl Engine {
    /// Replaces every inset: blank rows the engine keeps under a line or a file's heading for
    /// the host to draw into. An empty slice removes them. Several insets under the same row
    /// are laid out in the order of the slice, side by side or across as their
    /// [`Inset::span`] says (see [`InsetSpan`]); in the split layout that includes insets on
    /// the other side's line of the row.
    ///
    /// The view does not move. The source line on the cursor's screen row stays on that row
    /// and the horizontal scroll is kept; rows below it move. With the cursor on an inset, the
    /// line the inset hangs under stays on its screen row: the inset keeps its first row and
    /// grows or shrinks at the bottom, and the cursor stays on the same row of it, or its last
    /// row when it got shorter than that. When the inset is removed, the cursor goes to that
    /// line. When the line is above the pane, the cursor's own row is kept instead. When the
    /// cursor is off screen, the row at the top stays at the top.
    ///
    /// The exception is the start of the diff. When the top row is the diff's first row, the
    /// top stays: insets added under the first rows push the cursor's line down rather than
    /// start above the top, and insets removed there pull it up. Only if that would put the
    /// cursor's line, on screen before, under the pane's last row does the view scroll, just
    /// enough to show it on the last row. Likewise, with the top a little below the first
    /// row and too few rows above to keep the cursor's line where it was, it moves up. The
    /// next call puts it back on the row it had, as soon as there are rows enough above and
    /// while nothing else moved the view, so setting and removing the same insets is an exact
    /// round trip.
    ///
    /// Insets on a line that is not displayed take no rows: a line outside the diff, in an
    /// unrevealed gap, in a hunk drawn as its heading alone because whitespace is hidden, or
    /// in a file that has not arrived yet. They appear when the line does.
    /// [`Engine::hidden_insets`] lists them. Insets can be set before the first file arrives;
    /// [`Engine::replace`] and [`Engine::close`] drop them.
    ///
    /// Rejects the whole set, keeping the previous one, if an inset belongs to another review,
    /// two share an id, or one is on line 0. It also fails after [`Engine::close`].
    pub fn set_insets(&mut self, insets: &[Inset]) -> Result<()> {
        ensure!(self.input.is_some(), "The review is closed");
        validate(self.id, insets)?;
        let insets = Insets::new(insets);
        match &mut self.app {
            Some(app) => app.replace_insets(insets)?,
            None => self.insets = insets,
        }
        self.changed(InputEffect::Highlight);
        Ok(())
    }

    /// The insets set by [`Engine::set_insets`], in the order given.
    pub fn insets(&self) -> &[Inset] {
        self.app
            .as_ref()
            .map_or(&self.insets, |app| &app.insets)
            .list()
    }

    /// Where each inset on screen was drawn in the last frame, top to bottom. Empty before the
    /// first draw, while the help is shown and while the pane is too small to show the diff.
    /// A host draws into these after [`Engine::draw`], in the same frame.
    ///
    /// ```
    /// use diffs_rs::engine::Engine;
    /// use ratatui::{Frame, widgets::{Block, Borders, Paragraph, Wrap}};
    ///
    /// struct Note { id: u64, text: String }
    ///
    /// /// Draws each note in its inset, boxed.
    /// fn draw_notes(frame: &mut Frame<'_>, engine: &Engine, notes: &[Note]) {
    ///     for found in engine.inset_areas() {
    ///         let Some(note) = notes.iter().find(|note| note.id == found.id) else { continue };
    ///         // Only the rows on screen are drawn: no border where the box is cut.
    ///         let mut borders = Borders::LEFT | Borders::RIGHT;
    ///         if found.clipped_top == 0 {
    ///             borders |= Borders::TOP;
    ///         }
    ///         if found.clipped_top + found.area.height == found.height {
    ///             borders |= Borders::BOTTOM;
    ///         }
    ///         let text = Paragraph::new(note.text.as_str())
    ///             .wrap(Wrap { trim: false })
    ///             .scroll((found.clipped_top.saturating_sub(1), 0))
    ///             .block(Block::new().borders(borders));
    ///         frame.render_widget(text, found.area);
    ///     }
    /// }
    /// ```
    pub fn inset_areas(&self) -> &[InsetArea] {
        self.app.as_ref().map_or(&[], |app| &app.inset_areas)
    }

    /// The ids of the insets that keep no rows because their line is not displayed, in the
    /// order they were set: see [`Engine::set_insets`]. A host can say "N notes not shown".
    /// Before the first file arrives every inset is listed.
    pub fn hidden_insets(&self) -> Vec<u64> {
        let Some(app) = &self.app else {
            return self.insets.list().iter().map(|inset| inset.id).collect();
        };
        app.insets
            .list()
            .iter()
            .enumerate()
            .filter(|&(index, _)| !app.inset_shown(index))
            .map(|(_, inset)| inset.id)
            .collect()
    }

    /// The width an inset in `file` with `span` has to draw in, as [`InsetArea::area`] will
    /// report it: the settings as they are now (layout, split ratio, line numbers, the file
    /// tree) in the area last given to [`Engine::draw`]. No frame is needed after a setting
    /// changes. Use it to wrap an inset's content before setting its height. A pane resize,
    /// [`Engine::set_layout`], [`Engine::set_split_ratio`] and line numbers change it, and so
    /// can checked source arriving for a local review, which can widen the line numbers.
    ///
    /// `None` before anything was drawn, while the area is too small to show the diff, and
    /// for a file not in the review, or not arrived yet.
    pub fn inset_width(&self, file: usize, span: InsetSpan) -> Option<u16> {
        self.inset_width_in(self.area, file, span)
    }

    /// [`Engine::inset_width`] for a frame drawn into `area`, the area the host will give
    /// [`Engine::draw`]: before the first frame, or for a size to come.
    pub fn inset_width_in(&self, area: Rect, file: usize, span: InsetSpan) -> Option<u16> {
        let app = self.app.as_ref()?;
        let (_, diff) = app.panes(area);
        if diff.is_empty() || file >= app.doc.files.len() {
            return None;
        }
        let (_, width) = crate::render::inset_columns(app, file, span, diff.width);
        u16::try_from(width).ok()
    }

    /// Whether the cursor stops on insets. On (the default), `Command::Down` from the row an
    /// inset hangs under puts the cursor on the inset, as one stop however tall it is, and the
    /// next `Down` leaves it; [`Engine::cursor_row`] then reports
    /// [`DiffRowKind::Inset`](super::DiffRowKind::Inset). Off, line moves pass over insets, a
    /// click on one is ignored, and a page move or wheel scroll that ends on one moves the
    /// cursor to the nearest row on screen that is not an inset, unless an inset fills the
    /// whole pane. Insets are rows to scroll through either way. It stays set across
    /// [`Engine::replace`].
    pub fn set_inset_stops(&mut self, on: bool) -> Result<()> {
        self.inset_stops = on;
        if let Some(app) = &mut self.app {
            app.set_inset_stops(on)?;
        }
        self.changed(InputEffect::Redraw);
        Ok(())
    }

    /// The setting set by [`Engine::set_inset_stops`].
    pub fn inset_stops(&self) -> bool {
        self.inset_stops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inset(id: u64, side: Side, line: u32, height: u16) -> Inset {
        Inset {
            review: ReviewId(1),
            id,
            file: 0,
            anchor: InsetAnchor::Line { side, line },
            span: InsetSpan::Full,
            height,
        }
    }

    fn side(id: u64, side: Side, height: u16) -> Inset {
        Inset {
            span: InsetSpan::Side(side),
            ..inset(id, Side::New, 4, height)
        }
    }

    #[test]
    fn a_row_finds_its_insets_in_the_order_given_on_both_sides() {
        let insets = Insets::new(&[
            inset(10, Side::New, 4, 2),
            inset(11, Side::Old, 3, 1),
            inset(12, Side::New, 4, 3),
            inset(13, Side::Old, 9, 0),
            Inset {
                anchor: InsetAnchor::FileHeading,
                ..inset(14, Side::New, 1, 5)
            },
        ]);
        let row = Keys::Lines {
            file: 0,
            old: Some(3),
            new: Some(4),
        };
        let members = |keys| {
            insets
                .members(keys)
                .map(|(index, _)| index)
                .collect::<Vec<_>>()
        };
        assert_eq!(members(row), [0, 1, 2]);
        assert_eq!(insets.rows(row, false), 6);
        assert_eq!(insets.rows(row, true), 6);
        let old = Keys::Lines {
            file: 0,
            old: Some(3),
            new: None,
        };
        assert_eq!(members(old), [1]);
        // Height 0 keeps no row and is no member.
        let zero = Keys::Lines {
            file: 0,
            old: Some(9),
            new: None,
        };
        assert_eq!((insets.rows(zero, true), members(zero).len()), (0, 0));
        assert_eq!(members(Keys::Heading(0)), [4]);
        assert_eq!(insets.rows(Keys::Heading(1), true), 0);
        assert_eq!(insets.index(12), Some(2));
        assert!(!insets.is_empty());
        assert!(Insets::new(&[inset(1, Side::New, 1, 0)]).is_empty());
    }

    #[test]
    fn side_insets_stack_in_two_columns_between_full_ones() {
        // a and b side by side; c across; d and e in the right column, f in the left.
        let insets = Insets::new(&[
            side(1, Side::Old, 3),
            side(2, Side::New, 2),
            inset(3, Side::New, 4, 1),
            side(4, Side::New, 1),
            side(5, Side::New, 2),
            side(6, Side::Old, 2),
        ]);
        let row = Keys::Lines {
            file: 0,
            old: None,
            new: Some(4),
        };
        let laid: Vec<_> = insets
            .layout(row, true)
            .iter()
            .map(|p| (insets.get(p.index).id, p.column, p.start, p.height))
            .collect();
        // In the order the cursor stops: by block, the left column, then the right one.
        assert_eq!(
            laid,
            [
                (1, Column::Old, 0, 3),
                (2, Column::New, 0, 2),
                (3, Column::Full, 3, 1),
                (6, Column::Old, 4, 2),
                (4, Column::New, 4, 1),
                (5, Column::New, 5, 2),
            ]
        );
        // max(3, 2) + 1 + max(2, 1 + 2)
        assert_eq!(insets.rows(row, true), 7);
        // Unified, everything stacks in the order given.
        assert_eq!(insets.rows(row, false), 11);
        let unified: Vec<_> = insets
            .layout(row, false)
            .iter()
            .map(|p| (insets.get(p.index).id, p.start))
            .collect();
        assert_eq!(unified, [(1, 0), (2, 3), (3, 5), (4, 6), (5, 7), (6, 9)]);
    }

    #[test]
    fn a_set_with_a_repeated_id_or_line_zero_is_rejected() {
        let review = ReviewId(1);
        assert!(
            validate(
                review,
                &[inset(1, Side::New, 1, 1), inset(2, Side::New, 1, 1)]
            )
            .is_ok()
        );
        assert!(
            validate(
                review,
                &[inset(1, Side::New, 1, 1), inset(1, Side::Old, 2, 1)]
            )
            .is_err()
        );
        assert!(validate(review, &[inset(1, Side::New, 0, 1)]).is_err());
        assert!(validate(ReviewId(2), &[inset(1, Side::New, 1, 1)]).is_err());
    }
}
