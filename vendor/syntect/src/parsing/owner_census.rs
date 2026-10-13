use super::{Pattern, RegexCensus};
use crate::parsing::{Regex, SyntaxSet};
use serde_derive::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub struct OwnerCensus<'a> {
    pub rules: RegexCensus<'a>,
    pub compiled: BTreeMap<usize, Vec<InnerEngine<'a>>>,
}

#[derive(Serialize)]
pub struct InnerEngine<'a> {
    pub instance: usize,
    pub pattern: &'a str,
    pub syntax_config: String,
    pub engine_config: EngineConfig,
    pub instruction: Option<usize>,
    pub capture_groups: Option<(usize, usize)>,
    pub explicit_capture_group_0: bool,
    pub approximate_strategy_bytes: usize,
    pub scratch_bytes: Option<usize>,
}

#[derive(Serialize)]
pub struct EngineConfig {
    pub match_kind: String,
    pub utf8_empty: bool,
    pub auto_prefilter: bool,
    pub custom_prefilter_present: bool,
    pub which_captures: String,
    pub nfa_size_limit: Option<usize>,
    pub onepass_size_limit: Option<usize>,
    pub hybrid_cache_capacity: usize,
    pub dfa_size_limit: Option<usize>,
    pub dfa_state_limit: Option<usize>,
    pub byte_classes: bool,
    pub line_terminator: u8,
    pub pool_capacity: usize,
    pub hybrid: bool,
    pub dfa: bool,
    pub onepass: bool,
    pub backtrack: bool,
}

impl SyntaxSet {
    /// Inspects initialized static owners without loading syntax or acquiring search scratch.
    ///
    /// Call between parse requests. Different references can name the same inner instance;
    /// deduplicate that identity before summing strategy estimates. The application does not
    /// clone compiled inner handles separately. Estimates exclude pooled scratch, outer VM
    /// storage, strings, and allocator overhead; they are not a heap census.
    pub fn owner_census<'a>(&'a self) -> OwnerCensus<'a> {
        let mut compiled = BTreeMap::new();
        let mut inspect = |regex: &'a Regex| {
            if let Some(owner) = regex.compiled_owner() {
                compiled.entry(owner).or_insert_with(|| {
                    let mut engines = Vec::new();
                    regex.visit_engines(|engine| {
                        let config = engine.inner.get_config();
                        engines.push(InnerEngine {
                            instance: engine.inner as *const _ as usize,
                            pattern: engine.pattern,
                            syntax_config: format!("{:?}", engine.syntax),
                            engine_config: EngineConfig {
                                match_kind: format!("{:?}", config.get_match_kind()),
                                utf8_empty: config.get_utf8_empty(),
                                auto_prefilter: config.get_auto_prefilter(),
                                custom_prefilter_present: config.get_prefilter().is_some(),
                                which_captures: format!("{:?}", config.get_which_captures()),
                                nfa_size_limit: config.get_nfa_size_limit(),
                                onepass_size_limit: config.get_onepass_size_limit(),
                                hybrid_cache_capacity: config.get_hybrid_cache_capacity(),
                                dfa_size_limit: config.get_dfa_size_limit(),
                                dfa_state_limit: config.get_dfa_state_limit(),
                                byte_classes: config.get_byte_classes(),
                                line_terminator: config.get_line_terminator(),
                                pool_capacity: config.get_pool_capacity(),
                                hybrid: config.get_hybrid(),
                                dfa: config.get_dfa(),
                                onepass: config.get_onepass(),
                                backtrack: config.get_backtrack(),
                            },
                            instruction: engine.instruction,
                            capture_groups: engine.capture_groups,
                            explicit_capture_group_0: engine.explicit_capture_group_0,
                            approximate_strategy_bytes: engine.inner.memory_usage(),
                            scratch_bytes: None,
                        });
                    });
                    engines
                });
            }
        };
        for syntax in &self.syntaxes {
            if let Some(contexts) = syntax.lazy_contexts.get() {
                for context in &contexts.contexts {
                    for pattern in &context.patterns {
                        if let Pattern::Match(rule) = pattern {
                            inspect(&rule.regex);
                        }
                    }
                }
            }
        }
        if let Some(cache) = self.first_line_cache.get() {
            for (regex, _) in &cache.regexes {
                inspect(regex);
            }
        }
        OwnerCensus {
            rules: self.regex_census(),
            compiled,
        }
    }
}
