use std::collections::{HashMap, HashSet};

use syntect::parsing::syntax_definition::Pattern;

fn main() {
    let builder = two_face::syntax::extra_newlines().into_builder();
    let mut all = HashMap::<&str, usize>::new();
    let mut syntaxes = Vec::new();
    for syntax in builder.syntaxes() {
        let mut unique = HashSet::new();
        let mut count = 0;
        for context in syntax.contexts.values() {
            for pattern in &context.patterns {
                if let Pattern::Match(pattern) = pattern {
                    let text = pattern.regex.regex_str();
                    *all.entry(text).or_default() += 1;
                    unique.insert(text);
                    count += 1;
                }
            }
        }
        syntaxes.push(
            serde_json::json!({"name": syntax.name, "extensions": syntax.file_extensions,
                                      "patterns": count, "unique_patterns": unique.len()}),
        );
    }
    println!(
        "{}",
        serde_json::json!({
            "scope": "Static grammar inventory. Does not compile regexes or predict live memory savings.",
            "patterns": all.values().sum::<usize>(), "unique_patterns": all.len(), "syntaxes": syntaxes,
        })
    );
}
