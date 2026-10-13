use fancy_regex::{Captures, Error, Regex, RegexBuilder, Result, RuntimeError, SearchScratch};
use std::{sync::Barrier, thread};

struct Search(SearchScratch);

impl Search {
    fn new() -> Self {
        Self(SearchScratch::default())
    }

    #[allow(
        clippy::result_large_err,
        reason = "Preserve the regex API's exact error type in this contract adapter"
    )]
    fn captures<'t>(
        &mut self,
        regex: &Regex,
        text: &'t str,
        start: usize,
    ) -> Result<Option<Captures<'t>>> {
        regex.captures_from_pos_with_scratch(text, start, &mut self.0)
    }
}

type Group = Option<(usize, usize)>;

struct Case {
    pattern: &'static str,
    text: &'static str,
    start: usize,
    expected: Option<&'static [Group]>,
}

const CASES: &[Case] = &[
    Case {
        pattern: r"(?P<word>\p{L}+)(?:=(\d+))?",
        text: "🙂 日本=42",
        start: 5,
        expected: Some(&[Some((5, 14)), Some((5, 11)), Some((12, 14))]),
    },
    Case {
        pattern: r"(?<=:)(\p{L}+)(?=12)12\1",
        text: "🙂 :日本12日本",
        start: 5,
        expected: Some(&[Some((6, 20)), Some((6, 12))]),
    },
    Case {
        pattern: r"(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)(p)",
        text: "abcdefghijklmnop",
        start: 0,
        expected: Some(&[
            Some((0, 16)),
            Some((0, 1)),
            Some((1, 2)),
            Some((2, 3)),
            Some((3, 4)),
            Some((4, 5)),
            Some((5, 6)),
            Some((6, 7)),
            Some((7, 8)),
            Some((8, 9)),
            Some((9, 10)),
            Some((10, 11)),
            Some((11, 12)),
            Some((12, 13)),
            Some((13, 14)),
            Some((14, 15)),
            Some((15, 16)),
        ]),
    },
    Case {
        pattern: r"(?P<word>\p{L}+)(?:=(\d+))?",
        text: "---",
        start: 0,
        expected: None,
    },
    Case {
        pattern: r"(?<=:)(\p{L}+)(?=12)12\1",
        text: "🙂 :日本12日月",
        start: 5,
        expected: None,
    },
    Case {
        pattern: r"(?P<word>\p{L}+)(?:=(\d+))?",
        text: "日本",
        start: 0,
        expected: Some(&[Some((0, 6)), Some((0, 6)), None]),
    },
    Case {
        pattern: r#"(?<quote>['"])(?<word>[a-z]+)\k<quote>"#,
        text: "'fox' \"owl\"",
        start: 5,
        expected: Some(&[Some((6, 11)), Some((6, 7)), Some((7, 10))]),
    },
    Case {
        pattern: r"([a-z]+)\K=\1",
        text: "tag=tag",
        start: 0,
        expected: Some(&[Some((3, 7)), Some((0, 3))]),
    },
    Case {
        pattern: r"(foo)(?=!)|(bar)(?=\.)",
        text: "bar.",
        start: 0,
        expected: Some(&[Some((0, 3)), None, Some((0, 3))]),
    },
    Case {
        pattern: r"(foo)(?=!)|(bar)(?=\.)",
        text: "foo!",
        start: 0,
        expected: Some(&[Some((0, 3)), Some((0, 3)), None]),
    },
    Case {
        pattern: r"^(a)(?>bc|b)c$",
        text: "abcc",
        start: 0,
        expected: Some(&[Some((0, 4)), Some((0, 1))]),
    },
    Case {
        pattern: r"^(a)(?>bc|b)c$",
        text: "abc",
        start: 0,
        expected: None,
    },
    Case {
        pattern: r"(?<=x)(y?)",
        text: "x",
        start: 1,
        expected: Some(&[Some((1, 1)), Some((1, 1))]),
    },
    Case {
        pattern: r"(?=(日本))",
        text: "🙂 日本",
        start: 5,
        expected: Some(&[Some((5, 5)), Some((5, 11))]),
    },
    Case {
        pattern: r"(?<=日)$",
        text: "日",
        start: 3,
        expected: Some(&[Some((3, 3))]),
    },
    Case {
        pattern: r"\b(\w)",
        text: "ax",
        start: 1,
        expected: None,
    },
    Case {
        pattern: r"\b(\w)",
        text: ".x",
        start: 1,
        expected: Some(&[Some((1, 2)), Some((1, 2))]),
    },
    Case {
        pattern: r"(?m:^)(\d+)",
        text: "1 test 123\n2 foo",
        start: 7,
        expected: Some(&[Some((11, 12)), Some((11, 12))]),
    },
    Case {
        pattern: r"^((?!x)(?:a|ab))c$",
        text: "abc",
        start: 0,
        expected: Some(&[Some((0, 3)), Some((0, 2))]),
    },
];

fn groups(captures: &Captures<'_>) -> Vec<Group> {
    captures
        .iter()
        .map(|group| group.map(|matched| (matched.start(), matched.end())))
        .collect()
}

#[test]
fn alternating_patterns_preserve_exact_capture_spans() {
    let regexes: Vec<_> = CASES
        .iter()
        .map(|case| Regex::new(case.pattern).unwrap())
        .collect();
    let mut search = Search::new();
    for index in (0..CASES.len()).chain((0..CASES.len()).rev()) {
        let case = &CASES[index];
        let actual = search
            .captures(&regexes[index], case.text, case.start)
            .unwrap();
        assert_eq!(
            actual.as_ref().map(groups).as_deref(),
            case.expected,
            "pattern {:?}, text {:?}, start {}",
            case.pattern,
            case.text,
            case.start
        );
        if let (Some(actual), Some(expected)) = (actual, case.expected) {
            for (index, expected) in expected.iter().enumerate() {
                assert_eq!(
                    actual.get(index).map(|matched| matched.as_str()),
                    expected.map(|(start, end)| &case.text[start..end])
                );
            }
        }
    }
}

#[test]
fn held_captures_survive_other_searches_and_owner_drop() {
    let captures = {
        let mut search = Search::new();
        let wrap = Regex::new(r"(?P<word>\p{L}+)(?:=(\d+))?").unwrap();
        let fancy = Regex::new(r#"(?<quote>['"])(?<word>[a-z]+)\k<quote>"#).unwrap();
        let first = search.captures(&wrap, "日本=42", 0).unwrap().unwrap();
        let second = search
            .captures(&fancy, "'fox' \"owl\"", 5)
            .unwrap()
            .unwrap();
        let third = search.captures(&wrap, "last", 0).unwrap().unwrap();
        assert!(search.captures(&fancy, "none", 0).unwrap().is_none());
        [first, second, third]
    };
    assert_eq!(
        groups(&captures[0]),
        [Some((0, 9)), Some((0, 6)), Some((7, 9))]
    );
    assert_eq!(captures[0].name("word").unwrap().as_str(), "日本");
    assert_eq!(
        groups(&captures[1]),
        [Some((6, 11)), Some((6, 7)), Some((7, 10))]
    );
    assert_eq!(captures[1].name("quote").unwrap().as_str(), "\"");
    assert_eq!(captures[1].name("word").unwrap().as_str(), "owl");
    assert_eq!(groups(&captures[2]), [Some((0, 4)), Some((0, 4)), None]);
    assert_eq!(captures[2].name("word").unwrap().as_str(), "last");
}

#[test]
fn runtime_errors_do_not_leak_state_into_later_searches() {
    let limited = RegexBuilder::new("(x+x+)+(?>y)")
        .backtrack_limit(1)
        .build()
        .unwrap();
    let matching = Regex::new(r"(?<=:)([a-z]+)(?=12)12\1").unwrap();
    let mut search = Search::new();
    for _ in 0..3 {
        let held = search.captures(&matching, ":abc12abc", 0).unwrap().unwrap();
        assert!(matches!(
            search.captures(&limited, "xxxxxxxxxxy", 0),
            Err(Error::RuntimeError(RuntimeError::BacktrackLimitExceeded))
        ));
        assert!(
            search
                .captures(&matching, ":abc12xyz", 0)
                .unwrap()
                .is_none()
        );
        let next = search.captures(&matching, ":z12z", 0).unwrap().unwrap();
        assert_eq!(groups(&next), [Some((1, 5)), Some((1, 2))]);
        assert_eq!(groups(&held), [Some((1, 9)), Some((1, 4))]);
    }
}

#[test]
fn concurrent_callers_keep_search_and_capture_state_private() {
    let fancy = Regex::new(r"(?<=:)([a-z]+)(?=12)12\1").unwrap();
    let wrap = Regex::new(r"(\p{L}+)(\d+)?").unwrap();
    let barrier = Barrier::new(4);
    thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|index| {
                let (fancy, wrap, barrier) = (&fancy, &wrap, &barrier);
                scope.spawn(move || {
                    let mut search = Search::new();
                    let word = "a".repeat(index + 1);
                    let text = format!("日本 :{word}12{word}");
                    let expected = [Some((8, text.len())), Some((8, 8 + word.len()))];
                    barrier.wait();
                    for _ in 0..32 {
                        let held = search.captures(fancy, &text, 7).unwrap().unwrap();
                        let other = search.captures(wrap, "日本12", 0).unwrap().unwrap();
                        assert_eq!(groups(&other), [Some((0, 8)), Some((0, 6)), Some((6, 8))]);
                        assert!(search.captures(fancy, "not a match", 0).unwrap().is_none());
                        assert_eq!(groups(&held), expected);
                        assert_eq!(held.get(1).unwrap().as_str(), word);
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
}

#[test]
fn repeated_vm_search_retains_the_same_workspace_capacity() {
    let regex = Regex::new(r"(?<=:)([a-z]+)(?=12)12\1").unwrap();
    let mut scratch = SearchScratch::default();
    assert_eq!(scratch.heap_capacity_bytes(), Some(0));
    let first = regex
        .captures_from_pos_with_scratch(":abc12abc", 0, &mut scratch)
        .unwrap()
        .unwrap();
    assert_eq!(groups(&first), [Some((1, 9)), Some((1, 4))]);
    let capacity = scratch.heap_capacity_bytes().unwrap();
    assert!(capacity > 0);
    for _ in 0..8 {
        let next = regex
            .captures_from_pos_with_scratch(":abc12abc", 0, &mut scratch)
            .unwrap()
            .unwrap();
        assert_eq!(groups(&next), [Some((1, 9)), Some((1, 4))]);
        assert_eq!(scratch.heap_capacity_bytes(), Some(capacity));
    }
    assert_eq!(groups(&first), [Some((1, 9)), Some((1, 4))]);
}
