use once_cell::sync::OnceCell;
use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};
use std::error::Error;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

/// An abstraction for regex patterns.
///
/// * Allows swapping out the regex implementation because it's only in this module.
/// * Makes regexes serializable and deserializable using just the pattern string.
/// * Lazily compiles regexes on first use to improve initialization time.
#[derive(Debug)]
pub struct Regex {
    regex_str: String,
    regex: Arc<OnceCell<regex_impl::Regex>>,
    #[cfg(all(feature = "shared-engines", not(feature = "regex-onig")))]
    engines: Option<fancy_regex::EngineCache>,
}

/// Exact-pattern interning without retaining compiled values.
#[derive(Debug, Default)]
pub(crate) struct RegexCache {
    patterns: HashMap<String, Weak<OnceCell<regex_impl::Regex>>>,
    #[cfg(all(feature = "shared-engines", not(feature = "regex-onig")))]
    engines: fancy_regex::EngineCache,
}

impl RegexCache {
    pub(crate) fn share(&mut self, regex: &mut Regex) {
        #[cfg(all(feature = "shared-engines", not(feature = "regex-onig")))]
        {
            regex.engines = Some(self.engines.clone());
        }
        if let Some(cached) = self.patterns.get(&regex.regex_str).and_then(Weak::upgrade) {
            regex.regex = cached;
        } else {
            self.patterns
                .insert(regex.regex_str.clone(), Arc::downgrade(&regex.regex));
        }
    }
}

/// A region contains text positions for capture groups in a match result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Region {
    region: regex_impl::Region,
}

pub(crate) type SearchScratch = regex_impl::SearchScratch;

impl Regex {
    #[cfg(all(feature = "owner-census", not(feature = "regex-onig")))]
    pub(crate) fn visit_engines<'a>(
        &'a self,
        visit: impl FnMut(fancy_regex::owner_census::Engine<'a>),
    ) {
        if let Some(compiled) = self.regex.get() {
            compiled.visit_engines(visit);
        }
    }

    #[cfg(feature = "regex-census")]
    pub(crate) fn compiled_owner(&self) -> Option<usize> {
        self.regex.get().map(|_| Arc::as_ptr(&self.regex) as usize)
    }

    /// Create a new regex from the pattern string.
    ///
    /// Note that the regex compilation happens on first use, which is why this method does not
    /// return a result.
    pub fn new(regex_str: String) -> Self {
        Self {
            regex_str,
            regex: Arc::new(OnceCell::new()),
            #[cfg(all(feature = "shared-engines", not(feature = "regex-onig")))]
            engines: None,
        }
    }

    /// Check whether the pattern compiles as a valid regex or not.
    pub fn try_compile(regex_str: &str) -> Option<Box<dyn Error + Send + Sync + 'static>> {
        regex_impl::Regex::new(regex_str).err()
    }

    /// Return the regex pattern.
    pub fn regex_str(&self) -> &str {
        &self.regex_str
    }

    /// Check if the regex matches the given text.
    pub fn is_match(&self, text: &str) -> bool {
        self.regex().is_match(text)
    }

    /// Search for the pattern in the given text from begin/end positions.
    ///
    /// If a region is passed, it is used for storing match group positions. The argument allows
    /// the [`Region`] to be reused between searches, which makes a significant performance
    /// difference.
    ///
    /// [`Region`]: struct.Region.html
    pub fn search(
        &self,
        text: &str,
        begin: usize,
        end: usize,
        region: Option<&mut Region>,
    ) -> bool {
        self.regex()
            .search(text, begin, end, region.map(|r| &mut r.region))
    }

    pub(crate) fn search_with_scratch(
        &self,
        text: &str,
        begin: usize,
        end: usize,
        region: &mut Region,
        scratch: &mut SearchScratch,
    ) -> bool {
        self.regex()
            .search_with_scratch(text, begin, end, Some(&mut region.region), scratch)
    }

    pub(crate) fn possible_with_ascii_bytes(&self, present: u128) -> bool {
        #[cfg(not(feature = "regex-onig"))]
        {
            self.regex().required_ascii_bytes() & !present == 0
        }
        #[cfg(feature = "regex-onig")]
        {
            let _ = present;
            true
        }
    }

    fn regex(&self) -> &regex_impl::Regex {
        self.regex.get_or_init(|| {
            #[cfg(feature = "syntax-timing")]
            let _timing =
                super::syntax_timing::Initializer::new(super::syntax_timing::Phase::OuterRegex);
            #[cfg(all(feature = "shared-engines", not(feature = "regex-onig")))]
            if let Some(cache) = &self.engines {
                return regex_impl::Regex::new_cached(&self.regex_str, cache)
                    .expect("regex string should be pre-tested");
            }
            regex_impl::Regex::new(&self.regex_str).expect("regex string should be pre-tested")
        })
    }
}

impl Clone for Regex {
    fn clone(&self) -> Self {
        Regex {
            regex_str: self.regex_str.clone(),
            regex: Arc::clone(&self.regex),
            #[cfg(all(feature = "shared-engines", not(feature = "regex-onig")))]
            engines: self.engines.clone(),
        }
    }
}

impl PartialEq for Regex {
    fn eq(&self, other: &Regex) -> bool {
        self.regex_str == other.regex_str
    }
}

impl Eq for Regex {}

impl Serialize for Regex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.regex_str)
    }
}

impl<'de> Deserialize<'de> for Regex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let regex_str = String::deserialize(deserializer)?;
        Ok(Regex::new(regex_str))
    }
}

impl Region {
    pub(crate) fn owned_heap_bytes(&self) -> Option<usize> {
        regex_impl::region_owned_heap_bytes(&self.region)
    }

    pub fn new() -> Self {
        Self {
            region: regex_impl::new_region(),
        }
    }

    /// Get the start/end positions of the capture group with given index.
    ///
    /// If there is no match for that group or the index does not correspond to a group, `None` is
    /// returned. The index 0 returns the whole match.
    pub fn pos(&self, index: usize) -> Option<(usize, usize)> {
        self.region.pos(index)
    }
}

impl Default for Region {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "regex-onig")]
mod regex_impl {
    pub use onig::Region;
    use onig::{MatchParam, RegexOptions, SearchOptions, Syntax};
    use std::error::Error;

    #[derive(Default)]
    pub(crate) struct SearchScratch;

    #[derive(Debug)]
    pub struct Regex {
        regex: onig::Regex,
    }

    pub fn new_region() -> Region {
        Region::with_capacity(8)
    }

    pub(super) fn region_owned_heap_bytes(_: &Region) -> Option<usize> {
        None
    }

    impl Regex {
        pub fn new(regex_str: &str) -> Result<Regex, Box<dyn Error + Send + Sync + 'static>> {
            let result = onig::Regex::with_options(
                regex_str,
                RegexOptions::REGEX_OPTION_CAPTURE_GROUP,
                Syntax::default(),
            );
            match result {
                Ok(regex) => Ok(Regex { regex }),
                Err(error) => Err(Box::new(error)),
            }
        }

        pub(super) fn search_with_scratch(
            &self,
            text: &str,
            begin: usize,
            end: usize,
            region: Option<&mut Region>,
            _: &mut SearchScratch,
        ) -> bool {
            self.search(text, begin, end, region)
        }

        pub fn is_match(&self, text: &str) -> bool {
            self.regex
                .match_with_options(text, 0, SearchOptions::SEARCH_OPTION_NONE, None)
                .is_some()
        }

        pub fn search(
            &self,
            text: &str,
            begin: usize,
            end: usize,
            region: Option<&mut Region>,
        ) -> bool {
            let matched = self.regex.search_with_param(
                text,
                begin,
                end,
                SearchOptions::SEARCH_OPTION_NONE,
                region,
                MatchParam::default(),
            );

            // If there's an error during search, treat it as non-matching.
            // For example, in case of catastrophic backtracking, onig should
            // fail with a "retry-limit-in-match over" error eventually.
            matches!(matched, Ok(Some(_)))
        }
    }
}

// If both regex-fancy and regex-onig are requested, this condition makes regex-onig win.
#[cfg(all(feature = "regex-fancy", not(feature = "regex-onig")))]
mod regex_impl {
    use std::error::Error;

    #[derive(Debug)]
    pub struct Regex {
        regex: fancy_regex::Regex,
    }

    pub(crate) use fancy_regex::SearchScratch;

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Region {
        positions: Vec<Option<(usize, usize)>>,
    }

    pub fn new_region() -> Region {
        Region {
            positions: Vec::with_capacity(8),
        }
    }

    pub(super) fn region_owned_heap_bytes(region: &Region) -> Option<usize> {
        region
            .positions
            .capacity()
            .checked_mul(std::mem::size_of::<Option<(usize, usize)>>())
    }

    impl Regex {
        pub fn new(regex_str: &str) -> Result<Regex, Box<dyn Error + Send + Sync + 'static>> {
            Self::build(
                regex_str,
                #[cfg(feature = "shared-engines")]
                None,
            )
        }

        #[cfg(feature = "shared-engines")]
        pub(super) fn new_cached(
            regex_str: &str,
            cache: &fancy_regex::EngineCache,
        ) -> Result<Regex, Box<dyn Error + Send + Sync + 'static>> {
            Self::build(regex_str, Some(cache))
        }

        fn build(
            regex_str: &str,
            #[cfg(feature = "shared-engines")] cache: Option<&fancy_regex::EngineCache>,
        ) -> Result<Regex, Box<dyn Error + Send + Sync + 'static>> {
            let mut builder = fancy_regex::RegexBuilder::new(regex_str);
            builder.oniguruma_mode(true);
            #[cfg(feature = "shared-engines")]
            let result = match cache {
                Some(cache) => builder.build_with_cache(cache),
                None => builder.build(),
            };
            #[cfg(not(feature = "shared-engines"))]
            let result = builder.build();
            match result {
                Ok(regex) => Ok(Regex { regex }),
                Err(error) => Err(Box::new(error)),
            }
        }

        #[cfg(feature = "owner-census")]
        pub(super) fn visit_engines<'a>(
            &'a self,
            visit: impl FnMut(fancy_regex::owner_census::Engine<'a>),
        ) {
            self.regex.visit_engines(visit);
        }

        pub fn is_match(&self, text: &str) -> bool {
            // Errors are treated as non-matches
            self.regex.is_match(text).unwrap_or(false)
        }

        pub fn search(
            &self,
            text: &str,
            begin: usize,
            end: usize,
            region: Option<&mut Region>,
        ) -> bool {
            self.search_with_scratch(text, begin, end, region, &mut SearchScratch::default())
        }

        pub(super) fn search_with_scratch(
            &self,
            text: &str,
            begin: usize,
            end: usize,
            region: Option<&mut Region>,
            scratch: &mut SearchScratch,
        ) -> bool {
            // If there's an error during search, treat it as non-matching.
            // For example, in case of catastrophic backtracking, fancy-regex should
            // fail with an error eventually.
            if let Ok(Some(captures)) =
                self.regex
                    .captures_from_pos_with_scratch(&text[..end], begin, scratch)
            {
                if let Some(region) = region {
                    region.init_from_captures(&captures);
                }
                true
            } else {
                false
            }
        }

        pub(super) fn required_ascii_bytes(&self) -> u128 {
            self.regex.required_ascii_bytes()
        }
    }

    impl Region {
        fn init_from_captures(&mut self, captures: &fancy_regex::Captures) {
            self.positions.clear();
            for i in 0..captures.len() {
                let pos = captures.get(i).map(|m| (m.start(), m.end()));
                self.positions.push(pos);
            }
        }

        pub fn pos(&self, i: usize) -> Option<(usize, usize)> {
            if i < self.positions.len() {
                self.positions[i]
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_heap_tracks_capture_growth_and_private_clones() {
        let regex = Regex::new("(a)".repeat(16));
        let text = "a".repeat(16);
        let mut region = Region::new();
        let empty_bytes = region.owned_heap_bytes();
        assert!(regex.search(&text, 0, text.len(), Some(&mut region)));
        let captured_bytes = region.owned_heap_bytes();
        let cloned = region.clone();
        for capture in 0..=16 {
            assert_eq!(cloned.pos(capture), region.pos(capture));
        }
        assert_eq!(cloned.pos(16), Some((15, 16)));
        if cfg!(feature = "regex-onig") {
            assert_eq!(captured_bytes, None);
            assert_eq!(cloned.owned_heap_bytes(), None);
        } else {
            assert!(captured_bytes.unwrap() > empty_bytes.unwrap());
            let minimum = 17 * std::mem::size_of::<Option<(usize, usize)>>();
            assert!(cloned.owned_heap_bytes().unwrap() >= minimum);
        }
        assert!(Regex::new("(b)".into()).search("b", 0, 1, Some(&mut region)));
        assert_eq!(region.pos(1), Some((0, 1)));
        assert_eq!(region.pos(16), None);
        assert_eq!(cloned.pos(16), Some((15, 16)));
        assert_eq!(region.owned_heap_bytes(), captured_bytes);
    }

    #[test]
    fn caches_compiled_regex() {
        let regex = Regex::new(String::from(r"\w+"));

        assert!(regex.regex.get().is_none());
        assert!(regex.is_match("test"));
        assert!(regex.regex.get().is_some());
    }

    #[test]
    fn serde_as_string() {
        let pattern: Regex = serde_json::from_str("\"just a string\"").unwrap();
        assert_eq!(pattern.regex_str(), "just a string");
        let back_to_str = serde_json::to_string(&pattern).unwrap();
        assert_eq!(back_to_str, "\"just a string\"");
    }

    #[cfg(all(feature = "shared-engines", not(feature = "regex-onig")))]
    #[test]
    fn only_static_rules_receive_engine_cache_admission() {
        use crate::parsing::syntax_definition::{MatchOperation, MatchPattern};

        let mut closing = MatchPattern {
            has_captures: true,
            regex: Regex::new(r"^\1$".into()),
            scope: vec![],
            captures: None,
            operation: MatchOperation::None,
            with_prototype: None,
        };
        let mut cache = RegexCache::default();
        cache.share(&mut closing.regex);
        assert!(closing.regex.engines.is_some());
        assert!(closing.regex.clone().engines.is_some());
        let restored: Regex =
            serde_json::from_str(&serde_json::to_string(&closing.regex).unwrap()).unwrap();
        assert!(restored.engines.is_none());
        let opener = Regex::new(r"<<(\w+)".into());
        let mut region = Region::new();
        assert!(opener.search("<<END", 0, 5, Some(&mut region)));
        let derived = closing.regex_with_refs(&region, "<<END");
        assert!(derived.engines.is_none());
        assert!(derived.is_match("END"));
        assert!(!derived.is_match("OTHER"));
        assert!(opener.engines.is_none());
    }

    #[test]
    fn shared_compilation_keeps_captures_private_and_releases_its_owner() {
        let text = r"(?<=prefix:)([a-z]+)([0-9]+)";
        let mut first = Regex::new(text.into());
        let mut second = Regex::new(text.into());
        let independent = Regex::new(text.into());
        let mut cache = RegexCache::default();
        cache.share(&mut first);
        cache.share(&mut second);
        assert!(Arc::ptr_eq(&first.regex, &second.regex));
        assert!(!Arc::ptr_eq(&first.regex, &independent.regex));
        let owner = Arc::downgrade(&first.regex);

        let mut a = Region::new();
        let mut b = Region::new();
        assert!(first.search("prefix:alpha12", 0, 14, Some(&mut a)));
        assert!(second.search("prefix:beta345", 0, 14, Some(&mut b)));
        assert_eq!(a.pos(1), Some((7, 12)));
        assert_eq!(b.pos(1), Some((7, 11)));
        assert_eq!(a.pos(2), Some((12, 14)));
        assert_eq!(b.pos(2), Some((11, 14)));
        assert!(second.regex.get().is_some());
        let cloned = first.clone();
        assert!(Arc::ptr_eq(&first.regex, &cloned.regex));
        std::thread::scope(|threads| {
            for regex in [&first, &second, &cloned] {
                threads.spawn(move || {
                    let mut region = Region::new();
                    for _ in 0..100 {
                        assert!(regex.search("prefix:alpha12", 0, 14, Some(&mut region)));
                        assert_eq!(region.pos(1), Some((7, 12)));
                        assert!(!regex.search("not a match", 0, 11, Some(&mut region)));
                    }
                });
            }
        });
        drop((first, second, cloned));
        assert!(owner.upgrade().is_none());
        assert_eq!(cache.patterns.len(), 1);
        let mut replacement = Regex::new(text.into());
        cache.share(&mut replacement);
        assert!(Arc::ptr_eq(
            &cache.patterns[text].upgrade().unwrap(),
            &replacement.regex,
        ));
        assert!(replacement.regex.get().is_none());
        assert!(replacement.search("prefix:gamma67", 0, 14, Some(&mut a)));
        assert_eq!(a.pos(1), Some((7, 12)));
        assert_eq!(a.pos(2), Some((12, 14)));
    }

    #[cfg(feature = "yaml-load")]
    #[test]
    fn grammar_interning_preserves_rule_actions_clone_and_serialization() {
        use crate::parsing::syntax_definition::Pattern;
        use crate::parsing::{
            ParseState, Scope, ScopeStackOp, SyntaxDefinition, SyntaxSet, SyntaxSetBuilder,
        };

        let definition = SyntaxDefinition::load_from_str(
            "name: Shared\nscope: source.shared\ncontexts:\n  main:\n    - match: 'a'\n      scope: keyword.first\n      push: second\n  second:\n    - match: 'a'\n      scope: string.second\n      pop: true\n",
            true, None,
        ).unwrap();
        let mut builder = SyntaxSetBuilder::new();
        builder.add(definition);
        let set = builder.build();
        let syntax = &set.syntaxes()[0];
        let first = set.get_context(&syntax.context_ids()["main"]).unwrap();
        let second = set.get_context(&syntax.context_ids()["second"]).unwrap();
        let (Pattern::Match(a), Pattern::Match(b)) = (&first.patterns[0], &second.patterns[0])
        else {
            panic!("expected match rules")
        };
        assert!(Arc::ptr_eq(&a.regex.regex, &b.regex.regex));
        let expected = ParseState::new(syntax).parse_line("aa\n", &set).unwrap();
        assert!(expected.contains(&(0, ScopeStackOp::Push(Scope::new("keyword.first").unwrap()))));
        assert!(expected.contains(&(1, ScopeStackOp::Push(Scope::new("string.second").unwrap()))));
        let restored: SyntaxSet = crate::dumps::from_binary(&crate::dumps::dump_binary(&set));
        for set in [set.clone(), restored] {
            let answer = ParseState::new(&set.syntaxes()[0])
                .parse_line("aa\n", &set)
                .unwrap();
            assert_eq!(answer, expected);
        }
    }
}
