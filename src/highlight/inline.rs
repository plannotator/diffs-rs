use std::ops::Range;

use similar::{ChangeTag, TextDiff};
use unicode_segmentation::UnicodeSegmentation;

use crate::engine::InlineChanges;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Word,
    Whitespace,
    Symbol,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeSide {
    Old,
    New,
}

fn tokens(text: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = 0;
    let mut previous = Kind::Symbol;
    for (byte, grapheme) in text.grapheme_indices(true) {
        let first = grapheme.chars().next().expect("nonempty grapheme");
        let kind = if first.is_alphanumeric() || first == '_' {
            Kind::Word
        } else if first.is_whitespace() {
            Kind::Whitespace
        } else {
            Kind::Symbol
        };
        if byte > start && (kind != previous || kind == Kind::Symbol) {
            tokens.push(&text[start..byte]);
            start = byte;
        }
        previous = kind;
    }
    if start < text.len() {
        tokens.push(&text[start..]);
    }
    tokens
}

/// Each grapheme of `text`, for `InlineChanges::Char`.
fn graphemes(text: &str) -> Vec<&str> {
    text.graphemes(true).collect()
}

/// In `InlineChanges::Char`, a pair in which either line has more characters than this is
/// compared by word, which bounds the comparison's work.
const CHAR_LIMIT: usize = 1000;

/// The byte ranges of `side`'s text that changed between `old` and `new`, by word.
pub fn changed_spans(old: &str, new: &str, side: ChangeSide) -> Vec<Range<usize>> {
    spans(&tokens(old), &tokens(new), side)
}

/// The byte ranges of `side`'s text that changed between `old` and `new`, as `mode` compares
/// them. `InlineChanges::Word` is [`changed_spans`].
pub(crate) fn changed_spans_by(
    old: &str,
    new: &str,
    side: ChangeSide,
    mode: InlineChanges,
) -> Vec<Range<usize>> {
    match mode {
        InlineChanges::None => Vec::new(),
        InlineChanges::Char
            if old.chars().nth(CHAR_LIMIT).is_none() && new.chars().nth(CHAR_LIMIT).is_none() =>
        {
            spans(&graphemes(old), &graphemes(new), side)
        }
        InlineChanges::WordAlt => {
            let text = match side {
                ChangeSide::Old => old,
                ChangeSide::New => new,
            };
            join_single_gaps(text, changed_spans(old, new, side))
        }
        _ => changed_spans(old, new, side),
    }
}

/// Joins two changes separated by exactly one unchanged grapheme, as Pierre's `word-alt` does.
fn join_single_gaps(text: &str, spans: Vec<Range<usize>>) -> Vec<Range<usize>> {
    let mut joined: Vec<Range<usize>> = Vec::with_capacity(spans.len());
    for span in spans {
        if let Some(last) = joined.last_mut()
            && text[last.end..span.start].graphemes(true).nth(1).is_none()
        {
            last.end = span.end;
        } else {
            joined.push(span);
        }
    }
    joined
}

fn spans(old: &[&str], new: &[&str], side: ChangeSide) -> Vec<Range<usize>> {
    let diff = TextDiff::from_slices(old, new);
    let (changed_tag, skipped_tag) = match side {
        ChangeSide::Old => (ChangeTag::Delete, ChangeTag::Insert),
        ChangeSide::New => (ChangeTag::Insert, ChangeTag::Delete),
    };
    let mut offset = 0;
    let mut changed: Vec<Range<usize>> = Vec::new();
    for part in diff.iter_all_changes() {
        if part.tag() == skipped_tag {
            continue;
        }
        let end = offset + part.value().len();
        if part.tag() == changed_tag {
            if let Some(last) = changed.last_mut().filter(|last| last.end == offset) {
                last.end = end;
            } else {
                changed.push(offset..end);
            }
        }
        offset = end;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marked(text: &str, spans: &[Range<usize>]) -> String {
        let mut out = String::new();
        let mut at = 0;
        for span in spans {
            out.push_str(&text[at..span.start]);
            out.push('[');
            out.push_str(&text[span.clone()]);
            out.push(']');
            at = span.end;
        }
        out.push_str(&text[at..]);
        out
    }

    fn both(old: &str, new: &str, mode: InlineChanges) -> (String, String) {
        (
            marked(old, &changed_spans_by(old, new, ChangeSide::Old, mode)),
            marked(new, &changed_spans_by(old, new, ChangeSide::New, mode)),
        )
    }

    #[test]
    fn word_is_the_word_comparison_as_it_was() {
        for (old, new) in [
            ("old-14", "new-14"),
            ("let retries = 3;", "let retries = 5;"),
            ("foo(bar, baz);", "foo(bar,  qux);"),
            ("emoji 👩‍💻 here", "emoji 👩‍💻 there"),
        ] {
            for side in [ChangeSide::Old, ChangeSide::New] {
                assert_eq!(
                    changed_spans_by(old, new, side, InlineChanges::Word),
                    changed_spans(old, new, side)
                );
            }
        }
        assert_eq!(
            both("old-14", "new-14", InlineChanges::Word),
            ("[old]-14".into(), "[new]-14".into())
        );
    }

    #[test]
    fn word_alt_joins_changes_one_character_apart_and_nothing_else() {
        assert_eq!(
            both("old value here", "new thing here", InlineChanges::Word),
            ("[old] [value] here".into(), "[new] [thing] here".into())
        );
        assert_eq!(
            both("old value here", "new thing here", InlineChanges::WordAlt),
            ("[old value] here".into(), "[new thing] here".into())
        );
        // Two unchanged characters between changes keep them apart.
        assert_eq!(
            both("a, b", "x, y", InlineChanges::WordAlt),
            ("[a], [b]".into(), "[x], [y]".into())
        );
        // The single unchanged character can be a grapheme of several bytes.
        assert_eq!(
            both("a👩‍💻b", "x👩‍💻y", InlineChanges::WordAlt),
            ("[a👩‍💻b]".into(), "[x👩‍💻y]".into())
        );
        // A change with no change after it is not extended.
        assert_eq!(
            both("old tail", "new tail", InlineChanges::WordAlt),
            ("[old] tail".into(), "[new] tail".into())
        );
    }

    #[test]
    fn char_compares_graphemes_and_long_lines_by_word() {
        assert_eq!(
            both("let café = 1;", "let cafe = 1;", InlineChanges::Char),
            ("let caf[é] = 1;".into(), "let caf[e] = 1;".into())
        );
        assert_eq!(
            both("retries", "entries", InlineChanges::Char),
            ("[r]etries".into(), "e[n]tries".into())
        );
        assert_eq!(
            both("👩‍💻 x", "👩‍🔬 x", InlineChanges::Char),
            ("[👩‍💻] x".into(), "[👩‍🔬] x".into())
        );
        let long_old = format!("{} old", "a".repeat(CHAR_LIMIT));
        let long_new = format!("{} new", "a".repeat(CHAR_LIMIT));
        for side in [ChangeSide::Old, ChangeSide::New] {
            assert_eq!(
                changed_spans_by(&long_old, &long_new, side, InlineChanges::Char),
                changed_spans(&long_old, &long_new, side)
            );
        }
    }

    #[test]
    fn none_emphasizes_nothing() {
        assert_eq!(
            both("old value", "new value", InlineChanges::None),
            ("old value".into(), "new value".into())
        );
    }
}
