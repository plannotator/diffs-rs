#![cfg(feature = "regex-census")]

use serde_json::Value;
use std::{sync::Barrier, thread};
use syntect::parsing::{
    ParseState, Scope, ScopeStackOp, SyntaxDefinition, SyntaxReference, SyntaxSet, SyntaxSetBuilder,
};

fn grammars(names: &[&str]) -> SyntaxSet {
    let mut builder = SyntaxSetBuilder::new();
    for name in names {
        let yaml = format!(
            r#"name: {name}
scope: source.census.{name}
first_line_match: '^#!{name}$'
contexts:
  main:
    - match: 'a'
      scope: keyword.first
      push: second
  second:
    - match: 'a'
      scope: string.second
      pop: true
  dormant:
    - match: 'never_reached'
      scope: invalid.unused
"#
        );
        builder.add(SyntaxDefinition::load_from_str(&yaml, true, None).unwrap());
    }
    builder.build()
}

fn census(set: &SyntaxSet) -> Value {
    let rules = serde_json::to_value(set.regex_census()).unwrap();
    #[cfg(feature = "owner-census")]
    {
        let owners = serde_json::to_value(set.owner_census()).unwrap();
        assert_eq!(owners["rules"], rules);
        assert_eq!(serde_json::to_value(set.owner_census()).unwrap(), owners);
        let expected: std::collections::BTreeSet<_> = rules["loaded_syntaxes"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|syntax| syntax["patterns"].as_array().unwrap())
            .chain(rules["first_line_patterns"].as_array().unwrap())
            .filter_map(|rule| rule["compiled_owner"].as_u64())
            .collect();
        let compiled = owners["compiled"].as_object().unwrap();
        assert_eq!(
            compiled
                .keys()
                .map(|key| key.parse::<u64>().unwrap())
                .collect::<std::collections::BTreeSet<_>>(),
            expected
        );
        let mut instances = std::collections::BTreeMap::new();
        for engines in compiled.values() {
            for engine in engines.as_array().unwrap() {
                let identity = engine["instance"].as_u64().unwrap();
                let signature = (
                    &engine["pattern"],
                    &engine["syntax_config"],
                    &engine["engine_config"],
                    &engine["approximate_strategy_bytes"],
                );
                if let Some(previous) = instances.insert(identity, signature) {
                    assert_eq!(previous, signature);
                }
                assert!(engine["scratch_bytes"].is_null());
            }
        }
        assert_eq!(serde_json::to_value(set.regex_census()).unwrap(), rules);
    }
    rules
}

#[cfg(feature = "owner-census")]
#[test]
fn delegated_engine_inspection_preserves_real_capture_mapping() {
    let mut builder = SyntaxSetBuilder::new();
    builder.add(
        SyntaxDefinition::load_from_str(
            r#"name: delegated
scope: source.delegated
contexts:
  main:
    - match: '([a-z]+)(?=cd)cd\1'
      scope: keyword.delegated
      captures:
        1: entity.name.delegated
    - match: '[0-9]+'
      scope: constant.numeric.delegated
  dormant:
    - match: 'never_reached'
      scope: invalid.unused
"#,
            true,
            None,
        )
        .unwrap(),
    );
    let set = builder.build();
    assert!(
        serde_json::to_value(set.owner_census()).unwrap()["compiled"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    let syntax = set.find_syntax_by_name("delegated").unwrap();
    let mut parser = ParseState::new(syntax);
    census(&set);
    let expected = parser.parse_line("abbcdabb 123\n", &set).unwrap();
    for (offset, scope) in [
        (0, "keyword.delegated"),
        (0, "entity.name.delegated"),
        (9, "constant.numeric.delegated"),
    ] {
        assert!(expected.contains(&(offset, ScopeStackOp::Push(Scope::new(scope).unwrap()))));
    }
    assert!(
        expected.contains(&(3, ScopeStackOp::Pop(1))),
        "Capture must end before lookahead"
    );
    let before = serde_json::to_value(set.owner_census()).unwrap();
    let compiled = before["compiled"].as_object().unwrap();
    let engines: Vec<_> = compiled
        .values()
        .flat_map(|value| value.as_array().unwrap())
        .collect();
    assert!(
        engines
            .iter()
            .any(|engine| engine["instruction"].is_number() && engine["capture_groups"].is_array())
    );
    assert!(engines.iter().any(|engine| engine["instruction"].is_null()));
    assert!(
        engines
            .iter()
            .any(|engine| engine["approximate_strategy_bytes"].as_u64().unwrap() > 0)
    );
    assert_eq!(
        ParseState::new(syntax)
            .parse_line("abbcdabb 123\n", &set)
            .unwrap(),
        expected
    );
    assert_eq!(serde_json::to_value(set.owner_census()).unwrap(), before);
    census(&set);
    assert!(
        rules(loaded(&before["rules"], "delegated"), "never_reached")[0]["compiled_owner"]
            .is_null()
    );
}

#[cfg(feature = "owner-census")]
#[test]
fn configured_wrap_and_vm_report_their_actual_compiler_settings() {
    for (pattern, delegated) in [("([a-z]+)", false), (r"([a-z]+)(?=12)12\1", true)] {
        let regex = fancy_regex::RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .unwrap();
        let text = if delegated { "AbC12AbC" } else { "AbC" };
        let captures = || {
            let found = regex.captures(text).unwrap().unwrap();
            (0..found.len())
                .map(|index| found.get(index).map(|value| (value.start(), value.end())))
                .collect::<Vec<_>>()
        };
        let expected = captures();
        assert_eq!(expected[0], Some((0, text.len())));
        assert_eq!(expected[1], Some((0, 3)));
        let inspect = || {
            let mut engines = Vec::new();
            regex.visit_engines(|engine| {
                assert_eq!(engine.instruction.is_some(), delegated);
                assert_eq!(engine.syntax.get_case_insensitive(), !delegated);
                engines.push((
                    engine.inner as *const _ as usize,
                    engine.pattern.to_owned(),
                    engine.inner.memory_usage(),
                ));
            });
            assert!(!engines.is_empty());
            engines
        };
        let before = inspect();
        assert_eq!(captures(), expected);
        assert_eq!(inspect(), before);
    }
}

fn loaded<'a>(snapshot: &'a Value, name: &str) -> &'a Value {
    snapshot["loaded_syntaxes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|syntax| syntax["name"] == name)
        .unwrap()
}

fn rules<'a>(syntax: &'a Value, pattern: &str) -> Vec<&'a Value> {
    syntax["patterns"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|rule| rule["pattern"] == pattern)
        .collect()
}

fn shared_owner(syntax: &Value) -> u64 {
    let matching = rules(syntax, "a");
    assert_eq!(matching.len(), 2);
    assert_ne!(matching[0]["context"], matching[1]["context"]);
    for rule in &matching {
        assert!(rule["context"].as_u64().unwrap() < syntax["contexts"].as_u64().unwrap());
        assert_eq!(rule["rule"], 0);
    }
    let first = matching[0]["compiled_owner"].as_u64().unwrap();
    let second = matching[1]["compiled_owner"].as_u64().unwrap();
    assert_ne!(first, 0);
    assert_eq!(
        first, second,
        "identical rules must share one compiled cell"
    );
    first
}

const CAPTURE_PATTERN: &str = "([a-z]+)([0-9]+)";

fn action_grammars() -> SyntaxSet {
    let mut builder = SyntaxSetBuilder::new();
    for name in ["alpha", "beta", "cold"] {
        let operation = if name == "alpha" {
            "      push: body\n"
        } else {
            ""
        };
        let yaml = format!(
            r#"name: {name}
scope: source.census.{name}
contexts:
  main:
    - match: '{CAPTURE_PATTERN}'
      scope: keyword.{name}
      captures:
        1: entity.name.{name}
        2: constant.numeric.{name}
{operation}  body:
    - match: '!'
      scope: punctuation.body.{name}
      pop: true
"#
        );
        builder.add(SyntaxDefinition::load_from_str(&yaml, true, None).unwrap());
    }
    builder.build()
}

fn action_line(set: &SyntaxSet, name: &str) -> Vec<(usize, ScopeStackOp)> {
    ParseState::new(set.find_syntax_by_name(name).unwrap())
        .parse_line(&format!("{name}123!\n"), set)
        .unwrap()
}

fn check_action(name: &str, operations: &[(usize, ScopeStackOp)]) {
    for (offset, scope) in [
        (0, format!("keyword.{name}")),
        (0, format!("entity.name.{name}")),
        (name.len(), format!("constant.numeric.{name}")),
    ] {
        assert!(operations.contains(&(offset, ScopeStackOp::Push(Scope::new(&scope).unwrap()))));
    }
    let body = ScopeStackOp::Push(Scope::new(&format!("punctuation.body.{name}")).unwrap());
    assert_eq!(
        operations.contains(&(name.len() + 3, body)),
        name == "alpha",
        "only alpha's rule pushes the punctuation context"
    );
}

fn capture_owner(syntax: &Value) -> u64 {
    let matching = rules(syntax, CAPTURE_PATTERN);
    assert_eq!(matching.len(), 1);
    let owner = matching[0]["compiled_owner"].as_u64().unwrap();
    assert_ne!(owner, 0);
    owner
}

#[test]
fn census_separates_cold_rules_shared_cells_and_independent_live_owners() {
    let set = grammars(&["alpha", "beta", "cold"]);
    let cold = census(&set);
    assert_eq!(cold["syntax_count"], 3);
    assert_eq!(cold["loaded_syntaxes"].as_array().unwrap().len(), 0);
    assert_eq!(cold["first_line_cache_initialized"], false);
    assert_eq!(cold["first_line_patterns"].as_array().unwrap().len(), 0);
    assert_eq!(census(&set), cold);

    let mut alpha = ParseState::new(set.find_syntax_by_name("alpha").unwrap());
    let uncompiled = census(&set);
    assert_eq!(uncompiled["loaded_syntaxes"].as_array().unwrap().len(), 1);
    assert_eq!(loaded(&uncompiled, "alpha")["id"], 0);
    assert_eq!(rules(loaded(&uncompiled, "alpha"), "a").len(), 2);
    for rule in loaded(&uncompiled, "alpha")["patterns"].as_array().unwrap() {
        assert!(rule.get("compiled_owner").unwrap().is_null());
    }
    assert_eq!(census(&set), uncompiled);

    let operations = alpha.parse_line("aa\n", &set).unwrap();
    assert!(operations.contains(&(0, ScopeStackOp::Push(Scope::new("keyword.first").unwrap()))));
    assert!(operations.contains(&(1, ScopeStackOp::Push(Scope::new("string.second").unwrap()))));
    let first = census(&set);
    assert_eq!(first["loaded_syntaxes"].as_array().unwrap().len(), 1);
    let alpha_owner = shared_owner(loaded(&first, "alpha"));
    let dormant = rules(loaded(&first, "alpha"), "never_reached");
    assert_eq!(dormant.len(), 1);
    assert!(dormant[0].get("compiled_owner").unwrap().is_null());
    assert_eq!(census(&set), first);

    let mut beta = ParseState::new(set.find_syntax_by_name("beta").unwrap());
    beta.parse_line("aa\n", &set).unwrap();
    let second = census(&set);
    assert_eq!(second["loaded_syntaxes"].as_array().unwrap().len(), 2);
    assert_eq!(loaded(&second, "beta")["id"], 1);
    assert_eq!(shared_owner(loaded(&second, "alpha")), alpha_owner);
    assert_eq!(shared_owner(loaded(&second, "beta")), alpha_owner);
    assert_eq!(second["first_line_cache_initialized"], false);
    assert_eq!(second["first_line_patterns"].as_array().unwrap().len(), 0);
    assert_eq!(census(&set), second);

    let unrelated = grammars(&["alpha", "beta", "cold"]);
    ParseState::new(unrelated.find_syntax_by_name("alpha").unwrap())
        .parse_line("aa\n", &unrelated)
        .unwrap();
    assert_ne!(
        shared_owner(loaded(&census(&unrelated), "alpha")),
        alpha_owner
    );
    assert_eq!(census(&set), second);
}

#[test]
fn cold_and_partially_loaded_clones_share_concurrent_grammar_initialization() {
    for prewarm in [false, true] {
        let family = action_grammars();
        let before_loading = family.clone();
        if prewarm {
            check_action("alpha", &action_line(&family, "alpha"));
        }
        let after_loading = family.clone();
        assert!(
            census(&before_loading)["loaded_syntaxes"]
                .as_array()
                .unwrap()
                .is_empty()
        );

        let barrier = Barrier::new(2);
        let (alpha, beta) = thread::scope(|threads| {
            let alpha = threads.spawn(|| {
                barrier.wait();
                action_line(&before_loading, "alpha")
            });
            let beta = threads.spawn(|| {
                barrier.wait();
                action_line(&after_loading, "beta")
            });
            (alpha.join().unwrap(), beta.join().unwrap())
        });
        check_action("alpha", &alpha);
        check_action("beta", &beta);

        let before = census(&before_loading);
        let after = census(&after_loading);
        assert_eq!(before["loaded_syntaxes"].as_array().unwrap().len(), 1);
        assert_eq!(
            after["loaded_syntaxes"].as_array().unwrap().len(),
            if prewarm { 2 } else { 1 }
        );
        let owner = capture_owner(loaded(&before, "alpha"));
        assert_eq!(capture_owner(loaded(&after, "beta")), owner);
        let original = census(&family);
        assert_eq!(
            original["loaded_syntaxes"].as_array().unwrap().len(),
            usize::from(prewarm)
        );
        if prewarm {
            assert_eq!(capture_owner(loaded(&original, "alpha")), owner);
            assert_eq!(capture_owner(loaded(&after, "alpha")), owner);
        }
        for snapshot in [&before, &after, &original] {
            assert!(
                snapshot["loaded_syntaxes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|syntax| syntax["name"] != "cold")
            );
            assert_eq!(snapshot["first_line_cache_initialized"], false);
        }

        drop(family);
        assert_eq!(action_line(&before_loading, "alpha"), alpha);
        assert_eq!(action_line(&after_loading, "beta"), beta);
        assert_eq!(
            capture_owner(loaded(&census(&after_loading), "beta")),
            owner
        );
    }
}

#[test]
fn serialization_and_rebuilding_create_cold_independent_families() {
    let original = grammars(&["alpha", "beta", "cold"]);
    let cold_bytes = syntect::dumps::dump_binary(&original);
    let mut expected = Vec::new();
    for name in ["alpha", "beta"] {
        expected.push(
            ParseState::new(original.find_syntax_by_name(name).unwrap())
                .parse_line("aa\n", &original)
                .unwrap(),
        );
    }
    original.find_syntax_by_first_line("#!alpha").unwrap();
    assert_eq!(syntect::dumps::dump_binary(&original), cold_bytes);
    let original_snapshot = census(&original);
    let original_owner = shared_owner(loaded(&original_snapshot, "alpha"));
    assert_eq!(
        shared_owner(loaded(&original_snapshot, "beta")),
        original_owner
    );

    let restored: SyntaxSet = syntect::dumps::from_binary(&cold_bytes);
    let rebuilt = original.clone().into_builder().build();
    let mut owners = vec![original_owner];
    for fresh in [&restored, &rebuilt] {
        let cold = census(fresh);
        assert!(cold["loaded_syntaxes"].as_array().unwrap().is_empty());
        assert_eq!(cold["first_line_cache_initialized"], false);
        let serialized = syntect::dumps::dump_binary(fresh);
        for (name, operations) in ["alpha", "beta"].into_iter().zip(&expected) {
            assert_eq!(
                ParseState::new(fresh.find_syntax_by_name(name).unwrap())
                    .parse_line("aa\n", fresh)
                    .unwrap(),
                *operations
            );
        }
        let snapshot = census(fresh);
        assert_eq!(snapshot["loaded_syntaxes"].as_array().unwrap().len(), 2);
        let owner = shared_owner(loaded(&snapshot, "alpha"));
        assert_eq!(shared_owner(loaded(&snapshot, "beta")), owner);
        assert!(
            !owners.contains(&owner),
            "fresh sets must not join an existing family"
        );
        owners.push(owner);
        assert_eq!(syntect::dumps::dump_binary(fresh), serialized);
    }
    assert_eq!(census(&original), original_snapshot);
}

#[test]
fn standalone_reference_deserialization_keeps_lazy_loading_available() {
    let set = grammars(&["alpha"]);
    let syntax = set.find_syntax_by_name("alpha").unwrap();
    let bytes = syntect::dumps::dump_binary(syntax);
    let standalone: SyntaxReference = syntect::dumps::from_binary(&bytes);
    // This initializes the standalone reference through its local-cache fallback.
    // ParseState stores context IDs, so no foreign set is used to parse this reference.
    let state = ParseState::new(&standalone);
    assert!(
        census(&set)["loaded_syntaxes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(state, ParseState::new(syntax));
    assert_eq!(syntect::dumps::dump_binary(&standalone), bytes);
    assert_eq!(syntect::dumps::dump_binary(syntax), bytes);
}

#[test]
fn repeated_inspection_preserves_multiline_parser_operations_and_state() {
    let inspected = grammars(&["alpha", "cold"]);
    let control = grammars(&["alpha", "cold"]);
    let mut observed = ParseState::new(inspected.find_syntax_by_name("alpha").unwrap());
    let mut expected = ParseState::new(control.find_syntax_by_name("alpha").unwrap());

    for line in ["a\n", "a\n", "aa\n", "a\n", "a\n"] {
        let before = census(&inspected);
        assert_eq!(census(&inspected), before);
        assert_eq!(
            observed.parse_line(line, &inspected).unwrap(),
            expected.parse_line(line, &control).unwrap(),
            "inspection changed parse operations for {line:?}"
        );
        assert_eq!(observed, expected);
        let after = census(&inspected);
        assert_eq!(after["loaded_syntaxes"].as_array().unwrap().len(), 1);
        assert_eq!(after["first_line_cache_initialized"], false);
        assert_eq!(census(&inspected), after);
    }
}

#[test]
fn census_only_observes_an_already_initialized_first_line_cache() {
    let set = grammars(&["alpha", "beta"]);
    let before = census(&set);
    assert_eq!(before["first_line_cache_initialized"], false);
    assert_eq!(before["first_line_patterns"].as_array().unwrap().len(), 0);
    assert_eq!(census(&set), before);

    assert_eq!(
        set.find_syntax_by_first_line("#!beta").unwrap().name,
        "beta"
    );
    let partial = census(&set);
    assert_eq!(partial["first_line_cache_initialized"], true);
    assert_eq!(partial["loaded_syntaxes"].as_array().unwrap().len(), 0);
    let patterns = partial["first_line_patterns"].as_array().unwrap();
    assert_eq!(patterns.len(), 2);
    assert_eq!(patterns[0]["syntax"], 0);
    assert_eq!(patterns[0]["pattern"], "^#!alpha$");
    assert!(patterns[0].get("compiled_owner").unwrap().is_null());
    assert_eq!(patterns[1]["syntax"], 1);
    assert_eq!(patterns[1]["pattern"], "^#!beta$");
    let beta_owner = patterns[1]["compiled_owner"].as_u64().unwrap();
    assert_ne!(beta_owner, 0);
    assert_eq!(census(&set), partial);

    assert_eq!(
        set.find_syntax_by_first_line("#!alpha").unwrap().name,
        "alpha"
    );
    let complete = census(&set);
    assert_eq!(complete["loaded_syntaxes"].as_array().unwrap().len(), 0);
    let patterns = complete["first_line_patterns"].as_array().unwrap();
    let alpha_owner = patterns[0]["compiled_owner"].as_u64().unwrap();
    assert_ne!(alpha_owner, 0);
    assert_ne!(alpha_owner, beta_owner);
    assert_eq!(patterns[1]["compiled_owner"], beta_owner);
    assert_eq!(census(&set), complete);
}
