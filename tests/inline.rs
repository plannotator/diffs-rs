use diffs_rs::highlight::{ChangeSide, changed_spans};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

fn marked(input: &str) -> (String, Vec<Range<usize>>) {
    let mut text = String::new();
    let mut spans = Vec::new();
    let mut start = None;
    for c in input.chars() {
        match c {
            '⟦' => assert!(start.replace(text.len()).is_none()),
            '⟧' => spans.push(start.take().unwrap()..text.len()),
            _ => text.push(c),
        }
    }
    assert!(start.is_none());
    (text, spans)
}

#[test]
fn punctuation_aware_spans_match_independent_answers_on_both_sides() {
    for (old, new) in [
        ("⟦old⟧-14", "⟦new⟧-14"),
        ("value-⟦14⟧", "value-⟦15⟧"),
        ("let count = ⟦3⟧;", "let count = ⟦5⟧;"),
        ("call(⟦old⟧);", "call(⟦new⟧);"),
        ("path/⟦old⟧.rs", "path/⟦new⟧.rs"),
        ("call⟦(⟧value⟦)⟧", "call⟦[⟧value⟦]⟧"),
        ("left=right", "left=⟦=⟧right"),
        ("a,b", "a,⟦new,⟧b"),
        ("⟦old_value⟧", "⟦new_value⟧"),
        ("⟦oldValue2⟧", "⟦newValue2⟧"),
        ("⟦old⟧ ⟦red⟧", "⟦new⟧ ⟦blue⟧"),
        ("⟦old⟧.⟦red⟧", "⟦new⟧.⟦blue⟧"),
        ("a⟦\t⟧b", "a⟦  ⟧b"),
        ("a\tb⟦  ⟧", "a\tb⟦ ⟧"),
        ("日本語:⟦旧⟧-14", "日本語:⟦新⟧-14"),
        ("⟦café⟧-14", "⟦cafe\u{301}⟧-14"),
        ("cafe\u{301}:⟦old⟧", "cafe\u{301}:⟦new⟧"),
        ("icon=⟦👩‍💻⟧;", "icon=⟦👨‍💻⟧;"),
        ("tone=⟦👍🏽⟧;", "tone=⟦👍🏻⟧;"),
        ("same,⟦old⟧,same", "same,⟦new⟧,same"),
        ("", "⟦value();⟧"),
        ("unchanged(); 日本語 👩‍💻", "unchanged(); 日本語 👩‍💻"),
    ] {
        let (old, old_spans) = marked(old);
        let (new, new_spans) = marked(new);
        for (text, side, expected) in [
            (&old, ChangeSide::Old, old_spans),
            (&new, ChangeSide::New, new_spans),
        ] {
            let actual = changed_spans(&old, &new, side);
            assert_eq!(actual, expected, "{side:?}: {old:?} compared with {new:?}");
            let boundaries: Vec<_> = text
                .grapheme_indices(true)
                .map(|(byte, _)| byte)
                .chain([text.len()])
                .collect();
            for span in actual {
                assert!(span.start < span.end);
                assert!(boundaries.contains(&span.start));
                assert!(boundaries.contains(&span.end));
            }
        }
    }
}

#[test]
fn punctuation_dense_lines_at_the_existing_limit_keep_exact_spans() {
    let old = "({".repeat(5_000);
    let mut new = old.clone();
    new.replace_range(4_998..5_000, "]}");
    for side in [ChangeSide::Old, ChangeSide::New] {
        assert_eq!(changed_spans(&old, &new, side), vec![4_998..5_000]);
    }

    let new = "]}".repeat(5_000);
    for side in [ChangeSide::Old, ChangeSide::New] {
        assert_eq!(changed_spans(&old, &new, side), vec![0..10_000]);
    }
}
