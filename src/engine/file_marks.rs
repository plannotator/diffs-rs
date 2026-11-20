use std::collections::HashMap;

use anyhow::{Result, ensure};
use unicode_width::UnicodeWidthStr;

use super::{Engine, InputEffect};
use crate::tree::Mark;

/// A host decoration for the Files sidebar row of every file whose path is `path`.
///
/// `path` is the file's [`FileMetadata::path`](crate::storage::FileMetadata::path): its new
/// path, or its old path when the file was deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileMark<'a> {
    pub path: &'a str,
    /// Drawn in a column of its own at the right edge of the row.
    pub badge: Option<Badge<'a>>,
    /// Draws the row's name, status and counts muted.
    pub dimmed: bool,
}

/// Text of one or two terminal cells, such as `✓`, in a theme color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Badge<'a> {
    pub text: &'a str,
    pub color: BadgeColor,
}

/// The theme color of a badge, so it follows the dark and light themes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BadgeColor {
    /// The ordinary text color.
    Text,
    /// The muted color of line numbers and zero counts.
    Muted,
    Accent,
    Added,
    Deleted,
    Modified,
    Renamed,
}

/// The host's file marks by path, with each distinct badge stored once.
#[derive(Default)]
pub(crate) struct FileMarks {
    /// The host has set marks, so the sidebar keeps a badge column.
    pub(crate) column: bool,
    paths: HashMap<Box<str>, Mark>,
    badges: Vec<(Box<str>, BadgeColor)>,
}

impl FileMarks {
    /// Later marks for the same path win.
    pub(crate) fn new(marks: &[FileMark<'_>]) -> Self {
        let mut ids: HashMap<(&str, BadgeColor), u32> = HashMap::new();
        let mut badges = Vec::new();
        let mut paths = HashMap::with_capacity(marks.len());
        for mark in marks {
            let badge = mark.badge.map(|badge| {
                *ids.entry((badge.text, badge.color)).or_insert_with(|| {
                    badges.push((Box::from(badge.text), badge.color));
                    // Every mark has at most one badge, and `validate` bounds their number.
                    (badges.len() - 1) as u32
                })
            });
            paths.insert(
                Box::from(mark.path),
                Mark {
                    badge,
                    dimmed: mark.dimmed,
                },
            );
        }
        Self {
            column: true,
            paths,
            badges,
        }
    }

    pub(crate) fn get(&self, path: &str) -> Mark {
        self.paths.get(path).copied().unwrap_or_default()
    }

    pub(crate) fn badge(&self, id: u32) -> Option<(&str, BadgeColor)> {
        self.badges
            .get(id as usize)
            .map(|(text, color)| (&**text, *color))
    }
}

fn validate(marks: &[FileMark<'_>]) -> Result<()> {
    ensure!(u32::try_from(marks.len()).is_ok(), "Too many file marks");
    for badge in marks.iter().filter_map(|mark| mark.badge) {
        ensure!(
            matches!(badge.text.width(), 1 | 2) && !badge.text.chars().any(char::is_control),
            "Badge {:?} must be one or two terminal cells of printable text",
            badge.text
        );
    }
    Ok(())
}

impl Engine {
    /// Replaces every file mark. Marks for paths that are not in the review draw nothing. The
    /// Files sidebar keeps a badge column from the first call until `replace()` or `close()`,
    /// even when the set is empty.
    pub fn set_file_marks(&mut self, marks: &[FileMark<'_>]) -> Result<()> {
        ensure!(self.input.is_some(), "The review is closed");
        validate(marks)?;
        let marks = FileMarks::new(marks);
        match &mut self.app {
            Some(app) => app.set_file_marks(marks),
            None => self.file_marks = marks,
        }
        self.changed(InputEffect::Redraw);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark<'a>(path: &'a str, text: &'a str, color: BadgeColor, dimmed: bool) -> FileMark<'a> {
        FileMark {
            path,
            badge: Some(Badge { text, color }),
            dimmed,
        }
    }

    #[test]
    fn equal_badges_share_one_entry_and_later_paths_win() {
        let marks = FileMarks::new(&[
            mark("a.rs", "✓", BadgeColor::Added, true),
            mark("b.rs", "✓", BadgeColor::Added, false),
            mark("c.rs", "✓", BadgeColor::Accent, false),
            mark("a.rs", "!", BadgeColor::Deleted, false),
            FileMark {
                path: "d.rs",
                badge: None,
                dimmed: true,
            },
        ]);
        assert!(marks.column);
        assert_eq!(marks.badges.len(), 3);
        let a = marks.get("a.rs");
        assert_eq!(
            marks.badge(a.badge.unwrap()),
            Some(("!", BadgeColor::Deleted))
        );
        assert!(!a.dimmed);
        let b = marks.get("b.rs");
        assert_eq!(
            marks.badge(b.badge.unwrap()),
            Some(("✓", BadgeColor::Added))
        );
        assert_ne!(marks.get("c.rs").badge, b.badge);
        assert_eq!(
            marks.get("d.rs"),
            Mark {
                badge: None,
                dimmed: true
            }
        );
        assert_eq!(marks.get("e.rs"), Mark::default());
        assert!(!FileMarks::default().column);
    }

    #[test]
    fn badges_must_be_one_or_two_printable_cells() {
        for text in ["✓", "ok", "日", "👀", "A"] {
            assert!(
                validate(&[mark("a", text, BadgeColor::Text, false)]).is_ok(),
                "{text}"
            );
        }
        for text in ["", "abc", "日本", "\t", "\x1b", "a\n"] {
            assert!(
                validate(&[mark("a", text, BadgeColor::Text, false)]).is_err(),
                "{text:?}"
            );
        }
    }
}
