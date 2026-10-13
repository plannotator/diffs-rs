#![cfg(feature = "owner-census")]

use fancy_regex::{EngineCache, Regex, RegexBuilder};
use std::{sync::Barrier, thread};

fn captures(regex: &Regex, text: &str, start: usize) -> Vec<Option<(usize, usize)>> {
    let found = regex.captures_from_pos(text, start).unwrap().unwrap();
    (0..found.len())
        .map(|i| found.get(i).map(|m| (m.start(), m.end())))
        .collect()
}

type EngineView = (usize, String, Option<(usize, usize)>);

fn engines(regex: &Regex) -> Vec<EngineView> {
    let mut values = Vec::new();
    regex.visit_engines(|engine| {
        values.push((
            engine.inner as *const _ as usize,
            engine.pattern.to_owned(),
            engine.capture_groups,
        ));
    });
    values
}

#[test]
fn shared_vm_fragments_preserve_outer_capture_offsets() {
    let cache = EngineCache::default();
    let mut retained = Vec::new();
    for (pattern, text, expected) in [
        (
            r"([a-z]+)(?=12)12\1",
            "日本 abc12abc",
            vec![Some((7, 15)), Some((7, 10))],
        ),
        (
            r"(z)(?<=z)([a-z]+)(?=12)12\2",
            "日本 zabc12abc",
            vec![Some((7, 16)), Some((7, 8)), Some((8, 11))],
        ),
        (
            r"(?<=:)(\p{L}+)(?=12)12\1",
            "prefix:日本12日本",
            vec![Some((7, 21)), Some((7, 13))],
        ),
    ] {
        let shared = RegexBuilder::new(pattern).build_with_cache(&cache).unwrap();
        let plain = Regex::new(pattern).unwrap();
        assert_eq!(captures(&shared, text, 7), expected);
        assert_eq!(captures(&plain, text, 7), expected);
        assert_eq!(captures(&shared.clone(), text, 7), expected);
        retained.push(shared);
    }
    let a = engines(&retained[0]);
    let b = engines(&retained[1]);
    assert!(
        a.iter().any(|first| b
            .iter()
            .any(|second| first.0 == second.0 && first.2 != second.2)),
        "expected shared fragment with distinct outer mappings: {a:?} {b:?}"
    );

    for pattern in ["([a-z]+)", r"([a-z]+)(?=12)12\1"] {
        let build = |cache: &EngineCache| {
            RegexBuilder::new(pattern)
                .case_insensitive(true)
                .build_with_cache(cache)
                .unwrap()
        };
        let shared = build(&cache);
        let separate = build(&EngineCache::default());
        let text = if pattern.contains("12") {
            "日本 AbC12AbC"
        } else {
            "日本 AbC"
        };
        assert_eq!(captures(&shared, text, 7), captures(&separate, text, 7));
        shared.visit_engines(|engine| {
            assert_eq!(
                engine.syntax.get_case_insensitive(),
                engine.instruction.is_none()
            )
        });
        assert!(
            engines(&shared)
                .iter()
                .all(|a| engines(&separate).iter().all(|b| a.0 != b.0))
        );
    }
}

#[test]
fn concurrent_compilation_and_search_keep_results_private() {
    let cache = EngineCache::default();
    let start = Barrier::new(8);
    let ready = Barrier::new(8);
    thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|index| {
                let (cache, start, ready) = (&cache, &start, &ready);
                scope.spawn(move || {
                    start.wait();
                    let pattern = r"(?<=:)([a-z]+)(?=12)12\1";
                    let regex = RegexBuilder::new(pattern).build_with_cache(cache).unwrap();
                    ready.wait();
                    let word = "a".repeat(index + 1);
                    let text = format!("日本 :{word}12{word}");
                    let expected = vec![Some((8, text.len())), Some((8, 8 + word.len()))];
                    for _ in 0..100 {
                        assert_eq!(captures(&regex, &text, 7), expected);
                        assert!(!regex.is_match("no match").unwrap());
                    }
                    regex
                })
            })
            .collect();
        let regexes: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        let first = engines(&regexes[0]);
        for regex in &regexes[1..] {
            assert_eq!(engines(regex), first);
        }
    });
}
