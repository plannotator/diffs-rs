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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    /// The number of screen rows to keep free. An inset of height 0 keeps none: it is never
    /// drawn, reported or stopped on, but it is not hidden either.
    pub height: u16,
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

/// Where an [`Inset`] was drawn in the last frame, from [`Engine::inset_areas`]. Only the
/// rows on screen are reported: a host draws its content's rows from `clipped_top` on, as
/// many as `area.height`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct InsetArea {
    /// The [`Inset::id`].
    pub id: u64,
    /// The rows on screen, from the first column of code to the right edge of the diff.
    /// Unified, that is the code column; split, it runs from the old side's code across the
    /// line between the sides and the new side's gutter to the right edge.
    pub area: Rect,
    /// The same rows over the old side's code column only. Unified, equal to `area`.
    pub left: Rect,
    /// The same rows over the new side's code column only. Unified, equal to `area`.
    pub right: Rect,
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
}

impl Group {
    fn add(&mut self, index: usize, height: u16) {
        self.members.push(index);
        self.rows += usize::from(height);
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
                    .add(index, inset.height),
                InsetAnchor::FileHeading => headings
                    .entry(inset.file)
                    .or_default()
                    .add(index, inset.height),
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

    /// The rows the insets under a row with `keys` keep.
    pub(crate) fn rows(&self, keys: Keys) -> usize {
        self.groups(keys).iter().flatten().map(|g| g.rows).sum()
    }

    /// The insets under a row with `keys`, top to bottom: in the order the host gave them.
    pub(crate) fn members(&self, keys: Keys) -> Vec<usize> {
        match self.groups(keys) {
            [Some(a), Some(b)] => {
                let (mut i, mut j) = (0, 0);
                let mut merged = Vec::with_capacity(a.members.len() + b.members.len());
                while i < a.members.len() || j < b.members.len() {
                    if j == b.members.len() || (i < a.members.len() && a.members[i] < b.members[j])
                    {
                        merged.push(a.members[i]);
                        i += 1;
                    } else {
                        merged.push(b.members[j]);
                        j += 1;
                    }
                }
                merged
            }
            [Some(g), None] | [None, Some(g)] => g.members.clone(),
            [None, None] => Vec::new(),
        }
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
    /// stack in the order of the slice; in the split layout that includes insets on the other
    /// side's line of the row.
    ///
    /// The view does not move. The source line on the cursor's screen row stays on that row
    /// and the horizontal scroll is kept; rows below it move. With the cursor on an inset that
    /// is kept, the inset stays on its screen row; with the cursor on an inset that is
    /// removed, the cursor goes to the line the inset was under, which stays where it was.
    /// When the cursor is off screen, the row at the top stays at the top. The one exception
    /// is the start of the diff: with too few rows above to keep the cursor's line where it
    /// was, it moves up.
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
    /// use diffs_rs::engine::{Engine, Side};
    /// use ratatui::{Frame, layout::Rect, widgets::{Block, Borders, Paragraph, Wrap}};
    ///
    /// struct Note { id: u64, side: Side, text: String }
    ///
    /// /// Draws each note in its inset, boxed, on its side of a split diff.
    /// fn draw_notes(frame: &mut Frame<'_>, engine: &Engine, notes: &[Note]) {
    ///     for found in engine.inset_areas() {
    ///         let Some(note) = notes.iter().find(|note| note.id == found.id) else { continue };
    ///         let column = if note.side == Side::Old { found.left } else { found.right };
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
    ///         frame.render_widget(text, Rect { height: found.area.height, ..column });
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

    /// The width an inset in `file` has to draw in, for the size of the last frame: the code
    /// column, as [`InsetArea::area`] reports it unified, and the column of `side` split, as
    /// [`InsetArea::left`] or [`InsetArea::right`]. Use it to wrap an inset's content before
    /// setting its height. A pane resize, [`Engine::set_layout`], [`Engine::set_split_ratio`]
    /// and line numbers change it, and so can checked source arriving for a local review,
    /// which can widen the line numbers.
    ///
    /// `None` before the first draw, while the pane is too small to show the diff and for a
    /// file not in the review.
    pub fn inset_width(&self, file: usize, side: Side) -> Option<u16> {
        let app = self.app.as_ref()?;
        if app.sidebar.diff.is_empty() || file >= app.doc.files.len() {
            return None;
        }
        u16::try_from(crate::render::inset_width(app, file, side == Side::Old)).ok()
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
            height,
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
        assert_eq!(insets.members(row), [0, 1, 2]);
        assert_eq!(insets.rows(row), 6);
        let old = Keys::Lines {
            file: 0,
            old: Some(3),
            new: None,
        };
        assert_eq!(insets.members(old), [1]);
        // Height 0 keeps no row and is no member.
        let zero = Keys::Lines {
            file: 0,
            old: Some(9),
            new: None,
        };
        assert_eq!((insets.rows(zero), insets.members(zero).len()), (0, 0));
        assert_eq!(insets.members(Keys::Heading(0)), [4]);
        assert_eq!(insets.rows(Keys::Heading(1)), 0);
        assert_eq!(insets.index(12), Some(2));
        assert!(!insets.is_empty());
        assert!(Insets::new(&[inset(1, Side::New, 1, 0)]).is_empty());
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
