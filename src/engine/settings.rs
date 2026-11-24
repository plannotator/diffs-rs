//! Display settings a host can change while a review is open: the tab width, how changed text
//! inside a pair of lines is emphasized, and whether whitespace-only changes are shown.

use anyhow::Result;

use super::{Engine, InputEffect};

/// How many cells a tab advances to, measured from the start of the code: a tab moves to the
/// next multiple of the width. It applies wherever code is drawn or measured, on both sides of
/// a split row: wrapping, horizontal scrolling, clicks and [`Engine::row_at`], changed-word
/// emphasis and search matches.
///
/// [`TabWidth::DEFAULT`] (4) is what the engine has always drawn. A width is kept between
/// [`TabWidth::MIN`] and [`TabWidth::MAX`].
///
/// ```
/// use diffs_rs::engine::TabWidth;
///
/// assert_eq!(TabWidth::default(), TabWidth::DEFAULT);
/// assert_eq!(TabWidth::new(8).get(), 8);
/// assert_eq!(TabWidth::new(0), TabWidth::MIN);
/// assert_eq!(TabWidth::new(40), TabWidth::MAX);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TabWidth(u8);

impl TabWidth {
    /// Four cells, as the engine has always drawn tabs.
    pub const DEFAULT: Self = Self(4);
    /// The narrowest width: one cell.
    pub const MIN: Self = Self(1);
    /// The widest width: sixteen cells.
    pub const MAX: Self = Self(16);

    /// A tab `cells` cells wide, kept between [`Self::MIN`] and [`Self::MAX`].
    pub const fn new(cells: u8) -> Self {
        Self(if cells < Self::MIN.0 {
            Self::MIN.0
        } else if cells > Self::MAX.0 {
            Self::MAX.0
        } else {
            cells
        })
    }

    /// The width in cells.
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl Default for TabWidth {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// How the changed text inside a replaced line is emphasized, against the line it is paired
/// with. The names and meanings follow `lineDiffType` in Pierre's `@pierre/diffs`.
///
/// Only the emphasis changes: which lines are added, deleted or paired, their colors and their
/// backgrounds stay the same. Lines longer than 10,000 characters are never compared, in any
/// mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InlineChanges {
    /// Changed words, numbers and punctuation (Pierre's `word`). Each run of letters, digits
    /// and underscores is one unit, each run of whitespace is one, and every other character
    /// is its own. Unchanged spaces and punctuation between two changes are not emphasized.
    /// The engine has always drawn this.
    #[default]
    Word,
    /// As [`InlineChanges::Word`], and two changes separated by a single unchanged character,
    /// such as the space in `old value` → `new thing`, are emphasized as one (Pierre's
    /// `word-alt`, its default). Pierre also extends a change over one unchanged character
    /// that no change follows; this mode does not.
    WordAlt,
    /// Changed characters (Pierre's `char`). Each grapheme, such as `é` or `👩‍💻`, is one
    /// unit. A pair in which either line has more than 1,000 characters is compared by word
    /// instead, to bound the work.
    Char,
    /// No emphasis inside lines (Pierre's `none`); the line backgrounds still mark the change.
    None,
}

/// Whether changes that only add, remove or alter whitespace are shown, as `git diff` shows
/// them, or hidden as `git diff -b` and `git diff -w` (GitHub's "Hide whitespace") hide them.
///
/// The engine reads a patch, not the two files, so it hides whitespace inside each block of
/// removed and added lines that the patch supplies. In each block it pairs a removed line with
/// an added line that is the same apart from whitespace, keeping the order of both sides and
/// pairing as many as it can. A pair is drawn as an unchanged line: both line numbers, no
/// sign, no change background and no emphasis. In the unified layout it shows the new line's
/// text, as Git does. A hunk whose every change is such a pair shows only its heading, marked
/// as whitespace only, and its lines are not displayed.
///
/// What stays as the patch has it:
/// - Removed or added lines with no partner, blank ones included, stay changes.
///   (`git diff -w` keeps them too; `--ignore-blank-lines` is another option.)
/// - Lines are only paired within one block. A line moved across unchanged lines, or into
///   another hunk, keeps showing as removed and added, although `git diff -w` on the files
///   might pair it.
/// - Hunks keep the patch's boundaries and context: a hunk with a real change keeps every
///   line, where `git diff -w` would recount its context.
/// - Files, their order and their counts of added and removed lines stay as in the patch, and
///   a file whose only changes are whitespace keeps its heading.
/// - Search reads the review in unified order, so it finds a pair by its new text, which is
///   what the unified layout shows, and does not reach the lines of a hidden hunk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Whitespace {
    /// Every change, as the patch has it. The engine has always drawn this.
    #[default]
    Show,
    /// Hide changes in the amount of whitespace, as `git diff -b` (`--ignore-space-change`):
    /// whitespace at the end of a line is ignored and any run of whitespace matches any other
    /// run. Whitespace added where there was none, as in `a+b` → `a + b`, still counts.
    IgnoreChange,
    /// Hide every whitespace difference, as `git diff -w` (`--ignore-all-space`) and GitHub's
    /// "Hide whitespace": two lines match when they are equal once all whitespace is removed.
    IgnoreAll,
}

impl Engine {
    /// Draws tabs `width` cells wide. The default is [`TabWidth::DEFAULT`]; a host that never
    /// calls this draws as before.
    ///
    /// The source line under the cursor stays where it is on the screen, as it does when
    /// wrapping is turned on or off, and the text under the cursor stays in view. When
    /// measuring the lines fails, the error is returned and the review and the setting stay as
    /// they were. It can be set before the first file arrives, and it stays set across
    /// [`Engine::replace`].
    pub fn set_tab_width(&mut self, width: TabWidth) -> Result<()> {
        // Kept only once the review shows it, so a failure leaves the setting as it was.
        let changed = match &mut self.app {
            Some(app) => app.set_tab_width(width)?,
            None => false,
        };
        self.tab_width = width;
        if changed {
            self.changed(InputEffect::Highlight);
        }
        Ok(())
    }

    /// The width set by [`Engine::set_tab_width`].
    pub fn tab_width(&self) -> TabWidth {
        self.tab_width
    }

    /// Chooses how changed text inside a replaced line is emphasized. The default is
    /// [`InlineChanges::Word`]; a host that never calls this draws as before.
    ///
    /// Nothing moves: only the emphasis of the visible lines is computed again, as after a
    /// theme change, and appears when it is ready. It can be set before the first file
    /// arrives, and it stays set across [`Engine::replace`].
    ///
    /// It cannot fail today: it only records the mode and asks for the lines to be colored
    /// again. It returns a `Result`, as the other settings do, so that a later version may
    /// do fallible work here without changing the signature.
    pub fn set_inline_changes(&mut self, mode: InlineChanges) -> Result<()> {
        self.inline_changes = mode;
        let changed = match &mut self.app {
            Some(app) => app.set_inline_changes(mode),
            None => false,
        };
        if changed {
            self.changed(InputEffect::Highlight);
        }
        Ok(())
    }

    /// The mode set by [`Engine::set_inline_changes`].
    pub fn inline_changes(&self) -> InlineChanges {
        self.inline_changes
    }

    /// Shows or hides whitespace-only changes; see [`Whitespace`] for exactly what is hidden.
    /// The default is [`Whitespace::Show`]; a host that never calls this draws as before.
    ///
    /// The cursor keeps its source line on its screen row. When that line is no longer drawn
    /// on a row of its own, the cursor goes to the row that shows it now: the unchanged row of
    /// its pair, or the heading of its hidden hunk. A selection is kept, by its line numbers.
    /// A search captured before the change is out of date and its `find` fails; search again.
    ///
    /// Hiding reads the removed and added lines of every file that has arrived once, on the
    /// calling thread, and files that arrive later as they arrive. When reading or measuring
    /// fails, the error is returned and the review and the setting stay as they were. It can be set before
    /// the first file arrives, and it stays set across [`Engine::replace`].
    pub fn set_whitespace(&mut self, whitespace: Whitespace) -> Result<()> {
        // Kept only once the review shows it, so a failure leaves the setting as it was.
        let changed = match &mut self.app {
            Some(app) => app.set_whitespace(whitespace)?,
            None => false,
        };
        self.whitespace = whitespace;
        if changed {
            self.changed(InputEffect::Highlight);
            // Without extending, keeping the selection reads nothing and cannot fail, so the
            // setting is never left half applied.
            self.track_selection(false)?;
        }
        Ok(())
    }

    /// Whether hunk `hunk` of file `file` is drawn as its heading alone because its every
    /// change is whitespace only, by the engine's own rule for the current
    /// [`Engine::set_whitespace`] setting. `hunk` counts the file's hunks from 0, as
    /// [`DiffRow::hunk`](super::DiffRow::hunk) does. `false` while whitespace is shown, before
    /// the file arrives, and for a hunk the file does not have.
    pub fn hunk_is_whitespace_only(&self, file: usize, hunk: usize) -> bool {
        self.app.as_ref().is_some_and(|app| {
            app.doc
                .files
                .get(file)
                .is_some_and(|file| hunk < file.hunks.len() && file.hunk_hidden(hunk))
        })
    }

    /// The setting set by [`Engine::set_whitespace`].
    pub fn whitespace(&self) -> Whitespace {
        self.whitespace
    }
}
