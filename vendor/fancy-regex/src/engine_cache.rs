use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use crate::{RaRegex, RegexOptions, Result};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Settings {
    case_insensitive: bool,
    multi_line: bool,
    dot_matches_new_line: bool,
    crlf: bool,
    line_terminator: u8,
    swap_greed: bool,
    ignore_whitespace: bool,
    unicode: bool,
    utf8: bool,
    nest_limit: u32,
    octal: bool,
    size_limit: Option<usize>,
    dfa_size_limit: Option<usize>,
}

impl From<&RegexOptions> for Settings {
    fn from(options: &RegexOptions) -> Self {
        let syntax = options.syntaxc;
        Self {
            case_insensitive: syntax.get_case_insensitive(),
            multi_line: syntax.get_multi_line(),
            dot_matches_new_line: syntax.get_dot_matches_new_line(),
            crlf: syntax.get_crlf(),
            line_terminator: syntax.get_line_terminator(),
            swap_greed: syntax.get_swap_greed(),
            ignore_whitespace: syntax.get_ignore_whitespace(),
            unicode: syntax.get_unicode(),
            utf8: syntax.get_utf8(),
            nest_limit: syntax.get_nest_limit(),
            octal: syntax.get_octal(),
            size_limit: options.delegate_size_limit,
            dfa_size_limit: options.delegate_dfa_size_limit,
        }
    }
}

type Patterns = HashMap<Settings, HashMap<String, Weak<RaRegex>>>;

/// Caller-scoped sharing of exact compiled fragments and their search-buffer pools.
///
/// Keys and weak allocation headers live until replacement or cache release.
/// Engines are released when their final strong handle is dropped. Compilation
/// occurs outside the lock; failed compilations never enter the cache.
#[derive(Clone, Debug, Default)]
pub struct EngineCache {
    patterns: Arc<Mutex<Patterns>>,
}

impl EngineCache {
    pub(crate) fn compile(
        &self,
        pattern: &str,
        options: &RegexOptions,
        build: impl FnOnce() -> Result<InnerRegex>,
    ) -> Result<InnerRegex> {
        let settings = Settings::from(options);
        let cached = self
            .patterns
            .lock()
            .unwrap()
            .get(&settings)
            .and_then(|patterns| patterns.get(pattern))
            .and_then(Weak::upgrade);
        if let Some(engine) = cached {
            return Ok(InnerRegex(engine));
        }
        let compiled = build()?;
        let mut cache = self.patterns.lock().unwrap();
        let patterns = cache.entry(settings).or_default();
        if let Some(engine) = patterns.get(pattern).and_then(Weak::upgrade) {
            return Ok(InnerRegex(engine));
        }
        patterns.insert(pattern.to_owned(), Arc::downgrade(&compiled.0));
        Ok(compiled)
    }
}

/// An engine handle whose ordinary clone preserves an independent search pool.
pub struct InnerRegex(Arc<RaRegex>);

impl From<RaRegex> for InnerRegex {
    fn from(regex: RaRegex) -> Self {
        Self(Arc::new(regex))
    }
}

impl Clone for InnerRegex {
    fn clone(&self) -> Self {
        Self(Arc::new(self.0.as_ref().clone()))
    }
}

impl core::ops::Deref for InnerRegex {
    type Target = RaRegex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::fmt::Debug for InnerRegex {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile::compile_inner_cached, RegexBuilder, RegexImpl};

    #[test]
    fn cached_engines_release_without_releasing_the_family() {
        let cache = EngineCache::default();
        let build = || {
            RegexBuilder::new("([a-z]+)")
                .build_with_cache(&cache)
                .unwrap()
        };
        let first = build();
        let second = build();
        let cloned = first.clone();
        let engine = |regex: &crate::Regex| match &regex.inner {
            RegexImpl::Wrap { inner, .. } => Arc::downgrade(&inner.0),
            _ => panic!("expected wrapped expression"),
        };
        let weak = engine(&first);
        assert!(Weak::ptr_eq(&weak, &engine(&second)));
        assert!(!Weak::ptr_eq(&weak, &engine(&cloned)));
        drop((first, second));
        assert!(weak.upgrade().is_none());
        assert_eq!(
            cloned
                .captures("123abc")
                .unwrap()
                .unwrap()
                .get(1)
                .unwrap()
                .range(),
            3..6
        );
        let replacement = build();
        assert!(!Weak::ptr_eq(&weak, &engine(&replacement)));
        assert_eq!(replacement.find("123abc").unwrap().unwrap().range(), 3..6);
        assert_eq!(
            cache
                .patterns
                .lock()
                .unwrap()
                .values()
                .map(HashMap::len)
                .sum::<usize>(),
            1
        );
    }

    #[test]
    fn compiler_settings_and_failures_are_isolated() {
        let cache = EngineCache::default();
        let normal = RegexOptions::default();
        let compile = |pattern: &str, options: &RegexOptions| {
            compile_inner_cached(pattern, options, Some(&cache))
        };
        let first = compile("(aaa)", &normal).unwrap();
        let mut insensitive = normal.clone();
        insensitive.syntaxc = insensitive.syntaxc.case_insensitive(true);
        let second = compile("(aaa)", &insensitive).unwrap();
        assert!(!Arc::ptr_eq(&first.0, &second.0));
        assert!(!first.is_match("AAA"));
        assert!(second.is_match("AAA"));
        let mut constrained = normal.clone();
        constrained.delegate_size_limit = Some(1);
        assert!(compile("(aaa)", &constrained).is_err());
        assert!(compile("(aaa)", &constrained).is_err());
        assert_eq!(cache.patterns.lock().unwrap().len(), 2);
        assert!(compile("(", &normal).is_err());
        assert_eq!(
            cache
                .patterns
                .lock()
                .unwrap()
                .values()
                .map(HashMap::len)
                .sum::<usize>(),
            2
        );

        let changes = [
            normal.syntaxc.multi_line(true),
            normal.syntaxc.dot_matches_new_line(true),
            normal.syntaxc.crlf(true),
            normal.syntaxc.line_terminator(b'!'),
            normal.syntaxc.swap_greed(true),
            normal.syntaxc.ignore_whitespace(true),
            normal.syntaxc.unicode(false),
            normal.syntaxc.utf8(false),
            normal.syntaxc.nest_limit(200),
            normal.syntaxc.octal(true),
        ];
        let mut distinct = vec![first, second];
        for syntax in changes {
            let mut options = normal.clone();
            options.syntaxc = syntax;
            distinct.push(compile("(aaa)", &options).unwrap());
        }
        for dfa in [false, true] {
            let mut options = normal.clone();
            if dfa {
                options.delegate_dfa_size_limit = Some(1 << 20);
            } else {
                options.delegate_size_limit = Some(1 << 20);
            }
            distinct.push(compile("(aaa)", &options).unwrap());
        }
        for (index, engine) in distinct.iter().enumerate() {
            assert_eq!(engine.find("zaaa").unwrap().range(), 1..4);
            assert!(distinct[..index]
                .iter()
                .all(|other| !Arc::ptr_eq(&engine.0, &other.0)));
        }
    }
}
