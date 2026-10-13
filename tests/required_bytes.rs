use fancy_regex::{Regex, RegexBuilder, RuntimeError, SearchScratch};

fn present(text: &str) -> u128 {
    text.bytes()
        .filter(u8::is_ascii)
        .fold(0, |bytes, byte| bytes | (1u128 << byte))
}

#[test]
fn necessary_bytes_preserve_real_matches_at_every_character_boundary() {
    let patterns = [
        r"a(?=b)",
        r"(?=ab)",
        r"(?<=a)b",
        r"(?<!a)b",
        r"(?!a)b",
        r"(ab|cb)",
        r"a?|b",
        r"(ab)?c",
        r"(?:ab){1,3}",
        r"(?:ab){0,3}",
        r"(?i:K)é",
        r"(?i:k)b",
        r"(?x) a \# b",
        r"(?s:a.b)",
        r"(?m:^ab$)",
        r"([ab]+)\1c",
        r"(a)?(?(1)b|c)",
        r"([ab]+)\Kc",
        r"\Ga",
        r"(?>ab|a)c",
        r"(?=(a))\1",
        r"[^a]*b",
        r"\b日本(?=:)",
        r"(?<=[日a])(?P<tail>b?)",
        r"(?P<left>ab)?(?P<right>c)",
    ];
    let mut texts = vec![String::new()];
    let alphabet = ['a', 'b', 'c', ':', '\n', '日'];
    let mut layer = vec![String::new()];
    for _ in 0..3 {
        layer = layer
            .iter()
            .flat_map(|text| alphabet.map(|ch| format!("{text}{ch}")))
            .collect();
        texts.extend(layer.iter().cloned());
    }
    texts.extend(["Ké", "Kb", "a#b", "日本:", "🙂 日本:", "aaabbb", "ababc"].map(str::to_owned));
    let mut scratch = SearchScratch::default();
    let mut rejected = 0;
    let mut matched = 0;
    for pattern in patterns {
        let regex = Regex::new(pattern).unwrap();
        for text in &texts {
            for start in text
                .char_indices()
                .map(|(index, _)| index)
                .chain([text.len()])
            {
                let actual = regex
                    .captures_from_pos_with_scratch(text, start, &mut scratch)
                    .unwrap();
                let possible = regex.required_ascii_bytes() & !present(text) == 0;
                if !possible {
                    rejected += 1;
                    assert!(
                        actual.is_none(),
                        "rejected match: {pattern:?} {text:?} {start}"
                    );
                }
                if let Some(captures) = actual {
                    matched += 1;
                    let fresh = regex.captures_from_pos(text, start).unwrap().unwrap();
                    let spans = |caps: &fancy_regex::Captures<'_>| {
                        caps.iter()
                            .map(|m| m.map(|m| (m.start(), m.end())))
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(spans(&captures), spans(&fresh));
                }
            }
        }
    }
    assert!(rejected > 0 && matched > 0);
}

#[test]
fn proven_requirements_and_fallback_keep_public_errors_and_captures() {
    for (pattern, required) in [
        (r"a(?=b)", "ab"),
        (r"(?<=a)b", "ab"),
        (r"(?<!a)b", "b"),
        (r"(ab|cb)", "b"),
        (r"(ab)?c", "c"),
        (r"(?:ab){0,3}", ""),
        (r"(?i:k)b", "b"),
        (r"(a)?(?(1)b|c)", ""),
    ] {
        assert_eq!(
            Regex::new(pattern).unwrap().required_ascii_bytes(),
            present(required)
        );
    }
    for pattern in [r"(?<x>a)\g<x>b", r"a\g<0>?b"] {
        assert!(matches!(
            Regex::new(pattern),
            Err(fancy_regex::Error::CompileError(
                fancy_regex::CompileError::FeatureNotYetSupported(_)
            ))
        ));
    }
    let limited = RegexBuilder::new(r"(a+)+\1z")
        .backtrack_limit(1)
        .build()
        .unwrap();
    let text = "aaaaaaaaaaaa!";
    assert_ne!(limited.required_ascii_bytes() & !present(text), 0);
    assert!(matches!(
        limited.captures(text),
        Err(fancy_regex::Error::RuntimeError(
            RuntimeError::BacktrackLimitExceeded
        ))
    ));
    let regex = Regex::new(r"(?<=a)(b?)(?=日)").unwrap();
    let captures = regex.captures_from_pos("a日", 1).unwrap().unwrap();
    assert_eq!(captures.get(0).map(|m| (m.start(), m.end())), Some((1, 1)));
    assert_eq!(captures.get(1).map(|m| (m.start(), m.end())), Some((1, 1)));
    assert_eq!(regex.required_ascii_bytes(), present("a"));
    assert_eq!(
        RegexBuilder::new("Ab")
            .case_insensitive(true)
            .build()
            .unwrap()
            .required_ascii_bytes(),
        0
    );
}

#[test]
fn long_identifier_requirements_do_not_reject_matching_delimiters() {
    let patterns = [
        r"(?:(?:[\p{L}][_\p{L}\p{N}]*|_[_\p{L}\p{N}]+)\b)(::)",
        r"(?=(?:(?:[\p{L}][_\p{L}\p{N}]*|_[_\p{L}\p{N}]+)\b)::)",
        r"(?:\b(Copy|Send|Sized)\b|[\p{L}][_\p{L}\p{N}]*\b)(?=<)",
    ];
    let identifier = "長いidentifier".repeat(100);
    for pattern in patterns {
        let regex = Regex::new(pattern).unwrap();
        assert_ne!(regex.required_ascii_bytes(), 0);
        assert_ne!(regex.required_ascii_bytes() & !present(&identifier), 0);
        assert!(regex.captures(&identifier).unwrap().is_none());
        for suffix in ["::", "<"] {
            let text = format!("{identifier}{suffix}");
            let captures = regex.captures(&text).unwrap();
            if let Some(captures) = captures {
                assert_eq!(regex.required_ascii_bytes() & !present(&text), 0);
                assert_eq!(captures.get(0).unwrap().start(), 0);
            }
        }
    }
}
