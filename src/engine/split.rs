//! How a split row's width is shared between the old side on the left and the new side on the
//! right, and where the line between them is drawn.

use anyhow::Result;

use super::{Engine, InputEffect, Layout};

/// The share of a split row's width that the old side (left of the line between the sides)
/// takes, in thousandths of the width left beside that line. The new side takes the rest.
///
/// [`SplitRatio::EVEN`] is the default and draws exactly as an engine that never set a ratio.
/// A ratio is kept between [`SplitRatio::MIN`] and [`SplitRatio::MAX`], and when drawn each
/// side keeps at least [`SplitRatio::MIN_SIDE_WIDTH`] cells; a pane too narrow for two such
/// sides is split evenly. The ratio itself stays as it was set, so a pane made narrower for a
/// while draws the sides with the ratio again when it widens.
///
/// ```
/// use diffs_rs::engine::SplitRatio;
///
/// assert_eq!(SplitRatio::default(), SplitRatio::EVEN);
/// assert_eq!(SplitRatio::from_permille(620).permille(), 620);
/// assert_eq!(SplitRatio::from_permille(990), SplitRatio::MAX);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SplitRatio {
    old: u16,
}

impl SplitRatio {
    /// Both sides the same width, as the engine has always drawn them. On an even number of
    /// cells beside the line the new side is one cell wider.
    pub const EVEN: Self = Self { old: 500 };
    /// The smallest share of the old side: one tenth.
    pub const MIN: Self = Self { old: 100 };
    /// The largest share of the old side: nine tenths.
    pub const MAX: Self = Self { old: 900 };
    /// The fewest cells either side is drawn with, its gutter included.
    pub const MIN_SIDE_WIDTH: u16 = 20;

    /// The old side's share in thousandths, kept between [`Self::MIN`] and [`Self::MAX`].
    pub const fn from_permille(old: u16) -> Self {
        let old = if old < Self::MIN.old {
            Self::MIN.old
        } else if old > Self::MAX.old {
            Self::MAX.old
        } else {
            old
        };
        Self { old }
    }

    /// The old side's share in thousandths.
    pub const fn permille(self) -> u16 {
        self.old
    }
}

impl Default for SplitRatio {
    fn default() -> Self {
        Self::EVEN
    }
}

/// The widths of the old and the new side of a split row `width` cells wide, the line between
/// them not counted.
pub(crate) fn sides(width: u16, ratio: SplitRatio) -> (usize, usize) {
    let room = usize::from(width.saturating_sub(1));
    let least = usize::from(SplitRatio::MIN_SIDE_WIDTH);
    let old = if room < 2 * least {
        room / 2
    } else {
        (room * usize::from(ratio.old) / 1000).clamp(least, room - least)
    };
    (old, room - old)
}

/// Whether the cell `offset` cells into a split row is on the old side. The line between the
/// sides counts toward the narrower side, and toward the new side when both are as wide, as it
/// always has at the even ratio.
pub(crate) fn old_at(width: u16, ratio: SplitRatio, offset: u16) -> bool {
    let (old, new) = sides(width, ratio);
    let offset = usize::from(offset);
    offset < old || (offset == old && old < new)
}

/// The ratio that draws the line `offset` cells into a split row `width` cells wide, or as near
/// as the limits allow. `None` when the row is too narrow for the line to move.
fn ratio_at(width: u16, offset: u16) -> Option<SplitRatio> {
    let room = usize::from(width.saturating_sub(1));
    let least = usize::from(SplitRatio::MIN_SIDE_WIDTH);
    if room < 2 * least {
        return None;
    }
    let target = usize::from(offset).clamp(least, room - least);
    if target == room / 2 {
        return Some(SplitRatio::EVEN);
    }
    // The smallest share whose rounding down lands on the target.
    let old = (target * 1000).div_ceil(room);
    Some(SplitRatio::from_permille(
        u16::try_from(old).unwrap_or(u16::MAX),
    ))
}

impl Engine {
    /// Shares a split row's width between the old and the new side, moving the line between
    /// them. The default is [`SplitRatio::EVEN`]; a host that never calls this draws as before.
    ///
    /// The source line under the cursor stays where it is on the screen, as it does when the
    /// pane is resized, and the view does not scroll sideways. Wrapped rows are measured again
    /// at the new widths. The
    /// ratio applies whenever the diff is split, it can be set before the first file arrives
    /// or while the diff is unified, and it stays set across [`Engine::replace`].
    pub fn set_split_ratio(&mut self, ratio: SplitRatio) -> Result<()> {
        self.split_ratio = ratio;
        let changed = match &mut self.app {
            Some(app) => app.set_split_ratio(ratio)?,
            None => false,
        };
        if changed {
            self.changed(InputEffect::Highlight);
        }
        Ok(())
    }

    /// The ratio set by [`Engine::set_split_ratio`], as it was set: a narrow pane can draw the
    /// sides nearer even than it says.
    pub fn split_ratio(&self) -> SplitRatio {
        self.split_ratio
    }

    /// The screen column of the line between the sides of a split diff, for the size of the
    /// last frame and the ratio set now. `None` before the first draw, while the diff is
    /// unified, and while the pane is too small to show the diff.
    ///
    /// The line is drawn on rows of code; headings, hunk headings and gaps run across it.
    pub fn split_line(&self) -> Option<u16> {
        let app = self.app.as_ref()?;
        let area = app.sidebar.diff;
        if area.is_empty() || !app.split() {
            return None;
        }
        let (old, _) = sides(app.width, app.split_ratio);
        Some(area.x + u16::try_from(old).ok()?)
    }

    /// The ratio that draws the line between the sides at screen column `column` of the last
    /// frame, or as near to it as the limits allow: what a host sets while the person drags
    /// that line. `None` when [`Engine::split_line`] is, and when the pane is too narrow for
    /// the line to move.
    ///
    /// A ratio is kept in thousandths, so on a row wider than about a thousand cells not every
    /// column can hold the line: the ratio returned then draws it at the nearest column that
    /// can, a cell or so from `column`.
    pub fn split_ratio_at(&self, column: u16) -> Option<SplitRatio> {
        self.split_line()?;
        let app = self.app.as_ref()?;
        ratio_at(app.width, column.saturating_sub(app.sidebar.diff.x))
    }

    /// Shows the diff split, unified, or split only when the pane is 120 columns wide or
    /// wider (`Layout::Auto`), keeping the source line under the cursor where it is on the
    /// screen, as [`Command::ToggleLayout`](crate::engine::Command::ToggleLayout) does.
    /// [`Engine::options`] reports the layout set.
    pub fn set_layout(&mut self, layout: Layout) -> Result<()> {
        self.options.layout = layout;
        if let Some(app) = &mut self.app
            && app.layout != layout
        {
            app.reflow(layout, app.width, app.height)?;
            self.changed(InputEffect::Highlight);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_even_ratio_draws_the_sides_as_they_always_were() {
        for width in 0..=400u16 {
            let left = width.saturating_sub(1) as usize / 2;
            assert_eq!(
                sides(width, SplitRatio::EVEN),
                (left, usize::from(width.saturating_sub(1)) - left),
                "{width}"
            );
            for offset in 0..width {
                assert_eq!(
                    old_at(width, SplitRatio::EVEN, offset),
                    offset < width / 2,
                    "{width} {offset}"
                );
            }
        }
    }

    #[test]
    fn each_side_keeps_its_least_width_and_a_narrow_row_splits_evenly() {
        for width in 0..=400u16 {
            for permille in [0, 100, 250, 500, 618, 900, 1000] {
                let (old, new) = sides(width, SplitRatio::from_permille(permille));
                assert_eq!(old + new, usize::from(width.saturating_sub(1)));
                let least = usize::from(SplitRatio::MIN_SIDE_WIDTH);
                if usize::from(width.saturating_sub(1)) >= 2 * least {
                    assert!(old >= least && new >= least, "{width} {permille}");
                } else {
                    assert_eq!(old, usize::from(width.saturating_sub(1)) / 2);
                }
            }
        }
    }

    #[test]
    fn the_ratio_at_a_column_draws_the_line_there_within_the_limits() {
        for width in [41u16, 60, 81, 120, 121, 160, 333] {
            let room = width - 1;
            for offset in 0..width {
                let ratio = ratio_at(width, offset).unwrap();
                let (old, _) = sides(width, ratio);
                let least = SplitRatio::MIN_SIDE_WIDTH;
                let lowest = sides(width, SplitRatio::MIN).0 as u16;
                let highest = sides(width, SplitRatio::MAX).0 as u16;
                let expected = offset.clamp(least.max(lowest), (room - least).min(highest));
                assert_eq!(old as u16, expected, "{width} {offset}");
            }
            assert_eq!(ratio_at(width, room / 2), Some(SplitRatio::EVEN));
        }
        assert_eq!(ratio_at(40, 10), None, "too narrow for the line to move");
    }
}
