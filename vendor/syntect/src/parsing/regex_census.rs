use super::{Pattern, SyntaxSet};
use serde_derive::Serialize;

#[cfg(all(feature = "owner-census", not(feature = "regex-onig")))]
#[path = "owner_census.rs"]
mod owners;

/// Diagnostic references to static patterns, not a measurement of engine bytes.
#[derive(Serialize)]
pub struct RegexCensus<'a> {
    pub syntax_count: usize,
    pub loaded_syntaxes: Vec<LoadedSyntax<'a>>,
    pub first_line_cache_initialized: bool,
    pub first_line_patterns: Vec<FirstLinePattern<'a>>,
}

#[derive(Serialize)]
pub struct LoadedSyntax<'a> {
    pub id: usize,
    pub name: &'a str,
    pub contexts: usize,
    pub patterns: Vec<RegexRule<'a>>,
}

#[derive(Serialize)]
pub struct RegexRule<'a> {
    pub context: usize,
    pub rule: usize,
    pub pattern: &'a str,
    pub compiled_owner: Option<usize>,
}

#[derive(Serialize)]
pub struct FirstLinePattern<'a> {
    pub syntax: usize,
    pub pattern: &'a str,
    pub compiled_owner: Option<usize>,
}

impl SyntaxSet {
    /// Inspects only initialized cells without loading or compiling anything.
    ///
    /// Call while parsing is quiescent. Identities are local to this snapshot;
    /// no compiled owners are cloned. Temporary backreference regexes are excluded.
    pub fn regex_census(&self) -> RegexCensus<'_> {
        let loaded_syntaxes =
            self.syntaxes
                .iter()
                .enumerate()
                .filter_map(|(id, syntax)| {
                    let contexts = syntax.lazy_contexts.get()?;
                    let patterns =
                        contexts
                            .contexts
                            .iter()
                            .enumerate()
                            .flat_map(|(context, value)| {
                                value.patterns.iter().enumerate().filter_map(
                                    move |(rule, value)| {
                                        let Pattern::Match(value) = value else {
                                            return None;
                                        };
                                        Some(RegexRule {
                                            context,
                                            rule,
                                            pattern: value.regex.regex_str(),
                                            compiled_owner: value.regex.compiled_owner(),
                                        })
                                    },
                                )
                            })
                            .collect();
                    Some(LoadedSyntax {
                        id,
                        name: &syntax.name,
                        contexts: contexts.contexts.len(),
                        patterns,
                    })
                })
                .collect();
        let first_line_cache = self.first_line_cache.get();
        let first_line_patterns = first_line_cache
            .into_iter()
            .flat_map(|cache| {
                cache
                    .regexes
                    .iter()
                    .map(|(regex, syntax)| FirstLinePattern {
                        syntax: *syntax,
                        pattern: regex.regex_str(),
                        compiled_owner: regex.compiled_owner(),
                    })
            })
            .collect();
        RegexCensus {
            syntax_count: self.syntaxes.len(),
            loaded_syntaxes,
            first_line_cache_initialized: first_line_cache.is_some(),
            first_line_patterns,
        }
    }
}
