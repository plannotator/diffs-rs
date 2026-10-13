#![cfg(feature = "delivery-trace")]

use std::{panic::AssertUnwindSafe, time::Instant};

use syntect::parsing::{ParseState, Regex, syntax_timing::Window};

#[test]
fn real_grammar_initialization_is_separate_from_warm_matching() {
    let syntaxes = two_face::syntax::extra_newlines();
    let syntax = syntaxes.find_syntax_by_extension("rs").unwrap();
    let setup = Window::request();
    let mut parser = ParseState::new(syntax);
    let setup = setup.finish();
    assert_eq!(setup.grammar.calls, 1);
    assert_eq!(setup.outer_regex.calls, 0);
    assert!(!setup.invalid);

    let text = "let answer = 42; // a real syntax search\n";
    let request = Window::request();
    let parse = Window::parser_call();
    let started = Instant::now();
    let first = parser.parse_line(text, &syntaxes).unwrap();
    let elapsed = started.elapsed().as_nanos();
    let cold = parse.finish();
    let total = request.finish();
    assert!(!first.is_empty());
    assert!(!cold.invalid && !total.invalid);
    assert!(cold.outer_regex.calls > 0);
    assert_eq!(cold.outer_regex.calls, total.outer_regex.calls);
    assert_eq!(cold.outer_regex.total_ns, total.outer_regex.total_ns);
    assert_eq!(cold.outer_regex.max_ns, total.outer_regex.max_ns);
    assert!(u128::from(cold.grammar.total_ns + cold.outer_regex.total_ns) <= elapsed);

    let request = Window::request();
    let mut parser = ParseState::new(syntax);
    let parse = Window::parser_call();
    let second = parser.parse_line(text, &syntaxes).unwrap();
    let warm = parse.finish();
    let total = request.finish();
    assert_eq!(first, second);
    assert!(!warm.invalid && !total.invalid);
    assert_eq!(warm.grammar.calls, 0);
    assert_eq!(warm.outer_regex.calls, 0);
    assert_eq!(total.grammar.calls, 0);
    assert_eq!(total.outer_regex.calls, 0);
}

#[test]
fn captured_delimiters_repeat_initialization_with_exact_interval_maxima() {
    let syntaxes = two_face::syntax::extra_newlines();
    let syntax = syntaxes.find_syntax_by_extension("rb").unwrap();
    let mut parser = ParseState::new(syntax);
    parser.parse_line("value = <<STOP\n", &syntaxes).unwrap();

    let request = Window::request();
    let mut calls = Vec::new();
    for line in ["return 1 # first\n", "return 2 # second\n", "STOP\n"] {
        let interval = Window::parser_call();
        parser.parse_line(line, &syntaxes).unwrap();
        calls.push(interval.finish());
    }
    let total = request.finish();
    assert!(!total.invalid);
    assert!(
        calls
            .iter()
            .all(|call| !call.invalid && call.source_patterns > 0)
    );
    assert!(calls.iter().all(|call| call.outer_regex.calls > 0));
    assert_eq!(
        total.source_patterns,
        calls.iter().map(|c| c.source_patterns).sum::<u64>()
    );
    assert_eq!(
        total.outer_regex.calls,
        calls.iter().map(|c| c.outer_regex.calls).sum::<u64>()
    );
    assert_eq!(
        total.outer_regex.total_ns,
        calls.iter().map(|c| c.outer_regex.total_ns).sum::<u64>()
    );
    assert_eq!(
        total.outer_regex.max_ns,
        calls.iter().map(|c| c.outer_regex.max_ns).max().unwrap()
    );

    let next = Window::request();
    assert_eq!(next.finish().outer_regex.max_ns, 0);
}

#[test]
fn failed_real_initialization_is_not_reported_as_success() {
    let request = Window::request();
    let invalid = Regex::new("[".to_owned());
    assert!(std::panic::catch_unwind(AssertUnwindSafe(|| invalid.is_match("x"))).is_err());
    let counts = request.finish();
    assert_eq!(counts.outer_regex.calls, 1);
    assert_eq!(counts.outer_regex.failed, 1);

    let request = Window::request();
    let _ = std::panic::catch_unwind(|| {
        let _parse = Window::parser_call();
        Regex::new("[".to_owned()).is_match("x");
    });
    assert!(request.finish().invalid);
}
