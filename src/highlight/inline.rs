use std::ops::Range;

use similar::{ChangeTag, TextDiff};
use unicode_segmentation::UnicodeSegmentation;

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

pub fn changed_spans(old: &str, new: &str, side: ChangeSide) -> Vec<Range<usize>> {
    let diff = TextDiff::from_slices(&tokens(old), &tokens(new));
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
