use std::collections::VecDeque;

use syntect::{
    highlighting::{HighlightState, Highlighter},
    parsing::{ParseState, ScopeStack},
};

use super::{Continuation, Side, SyntaxIdentity, SyntaxKey};

pub(super) const INTERVAL: usize = 256;
const MAX_ENTRIES: usize = 64;
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_ENTRY_BYTES: usize = 256 * 1024;

struct SavedSide {
    parse: ParseState,
    scopes: ScopeStack,
    fallback: Option<super::Fallback>,
}

impl SavedSide {
    fn capture(side: &Side) -> Self {
        Self {
            parse: side.parse.clone(),
            scopes: side.highlight.path.clone(),
            fallback: side.fallback,
        }
    }

    fn restore(&self, highlighter: &Highlighter<'_>) -> Side {
        let mut highlight = HighlightState::new(highlighter, ScopeStack::new());
        highlight.path = self.scopes.clone();
        Side {
            parse: self.parse.clone(),
            highlight,
            needs_colors: true,
            fallback: self.fallback,
        }
    }
}

fn owned_heap(parse: &ParseState, scopes: &ScopeStack) -> Option<usize> {
    parse
        .owned_heap_bytes()?
        .checked_add(scopes.owned_heap_bytes()?)
}

pub(super) struct Checkpoint {
    key: SyntaxKey,
    pub(super) next: usize,
    old: SavedSide,
    new: Option<SavedSide>,
    heap_bytes: usize,
    protected: bool,
}

impl Checkpoint {
    pub(super) fn restore(
        &self,
        identity: SyntaxIdentity,
        highlighter: &Highlighter<'_>,
    ) -> Option<Continuation> {
        if self.key != identity.key() {
            return None;
        }
        #[cfg(feature = "delivery-trace")]
        let started = std::time::Instant::now();
        let state = Continuation {
            identity,
            next: self.next,
            old: self.old.restore(highlighter),
            new: self.new.as_ref()?.restore(highlighter),
        };
        crate::delivery_event!(
            "checkpoint_restore",
            started.elapsed().as_nanos(),
            self.next
        );
        Some(state)
    }

    pub(super) fn restore_source(
        &self,
        identity: SyntaxIdentity,
        highlighter: &Highlighter<'_>,
    ) -> Option<super::full_source::State> {
        if self.key != identity.key() || self.new.is_some() {
            return None;
        }
        Some(super::full_source::State {
            identity,
            next: self.next,
            side: self.old.restore(highlighter),
        })
    }
}

struct Capture<'a> {
    identity: SyntaxIdentity,
    next: usize,
    start: usize,
    old: &'a Side,
    new: Option<&'a Side>,
}

pub(super) struct Checkpoints {
    entries: VecDeque<Checkpoint>,
    heap_bytes: usize,
}

pub(super) struct Scope<'a> {
    cache: &'a mut Checkpoints,
}

impl Scope<'_> {
    pub(super) fn cache(&mut self) -> &mut Checkpoints {
        self.cache
    }
}

impl Drop for Scope<'_> {
    fn drop(&mut self) {
        for entry in &mut self.cache.entries {
            entry.protected = false;
        }
    }
}

impl Checkpoints {
    #[cfg(feature = "owner-census")]
    pub(super) fn census(&self) -> serde_json::Value {
        let recomputed = self.entries.iter().try_fold(0usize, |total, entry| {
            let old = owned_heap(&entry.old.parse, &entry.old.scopes)?;
            let new = match &entry.new {
                Some(side) => owned_heap(&side.parse, &side.scopes)?,
                None => 0,
            };
            total.checked_add(old)?.checked_add(new)
        });
        serde_json::json!({
            "entries": self.entries.len(), "capacity": self.entries.capacity(),
            "entry_size_bytes": size_of::<Checkpoint>(), "inline_size_bytes": size_of::<Self>(),
            "heap_counter_bytes": self.heap_bytes, "recomputed_heap_bytes": recomputed,
            "retained_charge_bytes": self.retained_bytes(),
            "retained_limit_bytes": MAX_BYTES, "entry_limit": MAX_ENTRIES,
            "protected_entries": self.entries.iter().filter(|entry| entry.protected).count(),
        })
    }

    pub(super) fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(MAX_ENTRIES),
            heap_bytes: 0,
        }
    }

    fn retained_bytes(&self) -> usize {
        size_of::<Self>() + self.entries.capacity() * size_of::<Checkpoint>() + self.heap_bytes
    }

    pub(super) fn scope(&mut self) -> Scope<'_> {
        debug_assert!(self.entries.iter().all(|entry| !entry.protected));
        Scope { cache: self }
    }

    pub(super) fn protect(&mut self, identity: SyntaxIdentity, first: usize) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .filter(|entry| entry.key == identity.key() && entry.next <= first)
            .max_by_key(|entry| entry.next)
        {
            entry.protected = true;
        }
    }

    fn make_room(&mut self, additional: usize) -> bool {
        let mut count = self.entries.len();
        let mut bytes = self.retained_bytes();
        let fits = |count, bytes| count < MAX_ENTRIES && bytes + additional <= MAX_BYTES;
        let mut remove = 0;
        for entry in self.entries.iter().filter(|entry| !entry.protected) {
            if fits(count, bytes) {
                break;
            }
            count -= 1;
            bytes -= entry.heap_bytes;
            remove += 1;
        }
        if !fits(count, bytes) {
            return false;
        }
        for _ in 0..remove {
            let index = self
                .entries
                .iter()
                .position(|entry| !entry.protected)
                .unwrap();
            let entry = self.entries.remove(index).unwrap();
            self.heap_bytes -= entry.heap_bytes;
            crate::delivery_event!("checkpoint_evict", entry.next, entry.heap_bytes);
        }
        true
    }

    pub(super) fn nearest(&self, identity: SyntaxIdentity, first: usize) -> Option<&Checkpoint> {
        self.entries
            .iter()
            .filter(|entry| entry.key == identity.key() && entry.next <= first)
            .max_by_key(|entry| entry.next)
    }

    pub(super) fn capture(
        &mut self,
        state: &Continuation,
        start: usize,
        current: impl Fn() -> bool,
    ) -> bool {
        self.capture_parts(
            Capture {
                identity: state.identity,
                next: state.next,
                start,
                old: &state.old,
                new: Some(&state.new),
            },
            current,
        )
    }

    pub(super) fn capture_source(
        &mut self,
        state: &super::full_source::State,
        current: impl Fn() -> bool,
    ) -> bool {
        self.capture_parts(
            Capture {
                identity: state.identity,
                next: state.next,
                start: 0,
                old: &state.side,
                new: None,
            },
            current,
        )
    }

    fn capture_parts(&mut self, state: Capture<'_>, current: impl Fn() -> bool) -> bool {
        if !(state.next - state.start).is_multiple_of(INTERVAL)
            || self
                .entries
                .iter()
                .any(|entry| entry.key == state.identity.key() && entry.next == state.next)
        {
            return true;
        }
        if !current() {
            return false;
        }
        let estimate = owned_heap(&state.old.parse, &state.old.highlight.path).and_then(|old| {
            old.checked_add(match state.new {
                Some(new) => owned_heap(&new.parse, &new.highlight.path)?,
                None => 0,
            })
        });
        let Some(estimate) = estimate.filter(|heap| {
            heap.checked_add(size_of::<Checkpoint>())
                .is_some_and(|bytes| bytes <= MAX_ENTRY_BYTES)
        }) else {
            crate::delivery_event!(
                "checkpoint_reject",
                state.next,
                estimate
                    .and_then(|heap| heap.checked_add(size_of::<Checkpoint>()))
                    .unwrap_or(usize::MAX)
            );
            return current();
        };
        if !self.make_room(estimate) {
            crate::delivery_event!("checkpoint_admission_declined", state.next, estimate);
            return current();
        }
        #[cfg(feature = "delivery-trace")]
        let started = std::time::Instant::now();
        let old = SavedSide::capture(state.old);
        let new = state.new.map(SavedSide::capture);
        crate::delivery_event!(
            "checkpoint_capture",
            started.elapsed().as_nanos(),
            state.next
        );
        if !current() {
            return false;
        }
        let actual = owned_heap(&old.parse, &old.scopes).and_then(|old| {
            old.checked_add(match &new {
                Some(new) => owned_heap(&new.parse, &new.scopes)?,
                None => 0,
            })
        });
        if let Some(actual) = actual.filter(|actual| *actual <= estimate) {
            self.heap_bytes += actual;
            self.entries.push_back(Checkpoint {
                key: state.identity.key(),
                next: state.next,
                old,
                new,
                heap_bytes: actual,
                protected: false,
            });
            crate::delivery_event!(
                "checkpoint_cache",
                self.entries.len(),
                self.retained_bytes()
            );
        } else {
            crate::delivery_event!("checkpoint_reject", state.next, usize::MAX);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        highlight::Token,
        theme::{self, Appearance},
    };
    use syntect::{
        easy::HighlightLines,
        parsing::{SyntaxDefinition, SyntaxSet, SyntaxSetBuilder},
    };

    fn advance(
        side: &mut Side,
        line: &str,
        syntaxes: &SyntaxSet,
        highlighter: &Highlighter<'_>,
        visible: bool,
    ) -> Vec<Token> {
        side.advance(
            line,
            syntaxes,
            highlighter,
            visible,
            #[cfg(feature = "delivery-trace")]
            &mut crate::delivery_trace::RequestWork::new(0, 0, &[]),
            #[cfg(feature = "delivery-trace")]
            (crate::view::LineId::Patch(0), true),
        )
        .unwrap()
    }

    #[test]
    fn restore_matches_sequential_colors_through_private_scope_and_capture_state() {
        let definition = SyntaxDefinition::load_from_str(
            r#"name: Nested scopes
scope: source.test
contexts:
  main:
    - match: '^BEGIN ([A-Z]+)'
      push: outer
    - match: '\breturn\b'
      scope: keyword.control.test
  outer:
    - meta_scope: string.quoted.test
    - match: '^END-\1'
      pop: true
    - match: '^EMBED'
      push: inner
  inner:
    - clear_scopes: true
    - meta_scope: source.embedded.test
    - match: '^STOP'
      pop: true
    - match: '\breturn\b'
      scope: keyword.control.test
"#,
            true,
            None,
        )
        .unwrap();
        let mut builder = SyntaxSetBuilder::new();
        builder.add(definition);
        let custom = builder.build();
        let bundled = two_face::syntax::extra_newlines();
        for (syntaxes, extension) in [(&custom, "test"), (&bundled, "html")] {
            let syntax = if extension == "test" {
                &syntaxes.syntaxes()[0]
            } else {
                syntaxes.find_syntax_by_extension(extension).unwrap()
            };
            for appearance in [Appearance::Dark, Appearance::Light] {
                let (_, theme) = theme::load(appearance).unwrap();
                let highlighter = Highlighter::new(&theme);
                let previous = match appearance {
                    Appearance::Dark => Appearance::Light,
                    Appearance::Light => Appearance::Dark,
                };
                let (_, previous_theme) = theme::load(previous).unwrap();
                let previous_highlighter = Highlighter::new(&previous_theme);
                let identity = SyntaxIdentity {
                    origin: crate::highlight::Origin::patch(0, 1),
                    file: 1,
                    scope: super::super::SyntaxScope::PatchHunk(2),
                    epoch: 0,
                    appearance: previous,
                };
                let start = 19;
                let mut state = Continuation {
                    identity,
                    next: start,
                    old: Side::new(syntax, &previous_highlighter),
                    new: Side::new(syntax, &previous_highlighter),
                };
                let mut cache = Checkpoints::new();
                let mut old_reference = HighlightLines::new(syntax, &theme);
                let mut new_reference = HighlightLines::new(syntax, &theme);
                for n in 0..360 {
                    let (old, new) = if extension == "test" {
                        match n {
                            0 => ("BEGIN OLD\n", "BEGIN NEW\n"),
                            1 => ("EMBED\n", "EMBED\n"),
                            300 => ("STOP\n", "STOP\n"),
                            320 => ("END-OLD\n", "END-OLD\n"),
                            340 => ("END-NEW\n", "END-NEW\n"),
                            _ => ("return 1\n", "return 1\n"),
                        }
                    } else {
                        match n {
                            0 => ("<script>/*\n", "<script>\n"),
                            320 => ("*/\n", "// next\n"),
                            340 => ("</script>\n", "</script>\n"),
                            _ => ("return 1;\n", "return 1;\n"),
                        }
                    };
                    let expected_old =
                        super::super::tokens(old_reference.highlight_line(old, syntaxes).unwrap());
                    let expected_new =
                        super::super::tokens(new_reference.highlight_line(new, syntaxes).unwrap());
                    if n == 256 {
                        #[cfg(feature = "owner-census")]
                        {
                            let before =
                                (cache.entries.as_slices().0.as_ptr(), cache.retained_bytes());
                            let census = cache.census();
                            assert_eq!(census["entries"], 1);
                            assert_eq!(
                                census["heap_counter_bytes"],
                                census["recomputed_heap_bytes"]
                            );
                            assert_eq!(cache.census(), census);
                            assert_eq!(
                                (cache.entries.as_slices().0.as_ptr(), cache.retained_bytes()),
                                before
                            );
                            let live = super::super::census::side(&state.old);
                            assert!(live["parse_heap_bytes"].as_u64().unwrap() > 0);
                            assert!(live["highlight_heap_bytes"].as_u64().unwrap() > 0);
                            assert_eq!(super::super::census::side(&state.old), live);
                        }
                        let identity = SyntaxIdentity {
                            epoch: 1,
                            appearance,
                            ..identity
                        };
                        let saved = cache.nearest(identity, start + n).unwrap();
                        assert_eq!(saved.next, start + n);
                        let restored = saved.restore(identity, &highlighter).unwrap();
                        assert!(restored.identity == identity);
                        let foreign = SyntaxIdentity {
                            origin: crate::highlight::Origin::patch(1, 1),
                            ..identity
                        };
                        assert!(saved.restore(foreign, &highlighter).is_none());
                        assert!(cache.nearest(foreign, start + n).is_none());
                        assert_eq!(restored.old.parse, state.old.parse);
                        assert_eq!(restored.new.parse, state.new.parse);
                        assert_eq!(restored.old.highlight.path, state.old.highlight.path);
                        assert_eq!(restored.new.highlight.path, state.new.highlight.path);
                        state = restored;
                        let before = cache.retained_bytes();
                        assert!(cache.capture(&state, start, || true));
                        assert_eq!(cache.retained_bytes(), before);
                    }
                    let visible = n >= 290;
                    let current = if n < 256 {
                        &previous_highlighter
                    } else {
                        &highlighter
                    };
                    let actual_old = advance(&mut state.old, old, syntaxes, current, visible);
                    let actual_new = advance(&mut state.new, new, syntaxes, current, visible);
                    if visible {
                        assert_eq!(actual_old, expected_old, "old {extension} {n}");
                        assert_eq!(actual_new, expected_new, "new {extension} {n}");
                    }
                    state.next += 1;
                    assert!(cache.capture(&state, start, || true));
                }
                assert_eq!(cache.entries.len(), 1);
            }
        }
    }

    #[test]
    fn normal_limits_evict_old_positions_and_decline_large_real_parser_states() {
        let definition = SyntaxDefinition::load_from_str(
            r#"name: Nested captures
scope: source.test
contexts:
  main:
    - match: '^BEGIN ([A-Z]+)'
      push: body
  body:
    - match: '^END-\1'
      pop: true
    - include: main
"#,
            true,
            None,
        )
        .unwrap();
        let mut builder = SyntaxSetBuilder::new();
        builder.add(definition);
        let syntaxes = builder.build();
        let syntax = &syntaxes.syntaxes()[0];
        let (_, theme) = theme::load(Appearance::Dark).unwrap();
        let highlighter = Highlighter::new(&theme);
        let identity = SyntaxIdentity {
            origin: crate::highlight::Origin::patch(0, 0),
            file: 0,
            scope: super::super::SyntaxScope::PatchHunk(0),
            epoch: 0,
            appearance: Appearance::Dark,
        };
        let mut state = Continuation {
            identity,
            next: 0,
            old: Side::new(syntax, &highlighter),
            new: Side::new(syntax, &highlighter),
        };
        let mut cache = Checkpoints::new();
        for n in 0..(MAX_ENTRIES + 2) * INTERVAL {
            advance(&mut state.old, "return 1\n", &syntaxes, &highlighter, false);
            advance(&mut state.new, "return 1\n", &syntaxes, &highlighter, false);
            state.next = n + 1;
            assert!(cache.capture(&state, 0, || true));
            assert!(cache.entries.len() <= MAX_ENTRIES);
            assert!(cache.retained_bytes() <= MAX_BYTES);
        }
        assert_eq!(cache.entries.len(), MAX_ENTRIES);
        assert!(cache.nearest(identity, 2 * INTERVAL).is_none());
        assert_eq!(
            cache.nearest(identity, 3 * INTERVAL).unwrap().next,
            3 * INTERVAL
        );
        let positions: Vec<_> = cache.entries.iter().map(|entry| entry.next).collect();
        for _ in 0..INTERVAL {
            advance(&mut state.old, "return 1\n", &syntaxes, &highlighter, false);
            advance(&mut state.new, "return 1\n", &syntaxes, &highlighter, false);
            state.next += 1;
        }
        {
            let mut scope = cache.scope();
            let cache = scope.cache();
            for &next in &positions {
                cache.protect(identity, next);
            }
            assert!(cache.capture(&state, 0, || true));
            assert_eq!(
                cache
                    .entries
                    .iter()
                    .map(|entry| entry.next)
                    .collect::<Vec<_>>(),
                positions
            );
            assert!(!cache.capture(&state, 0, || false));
            assert!(cache.retained_bytes() <= MAX_BYTES);
        }
        assert!(cache.entries.iter().all(|entry| !entry.protected));
        assert!(cache.capture(&state, 0, || true));
        assert_eq!(cache.entries.len(), MAX_ENTRIES);
        assert_eq!(
            cache.nearest(identity, state.next).unwrap().next,
            state.next
        );
        assert!(cache.nearest(identity, positions[0]).is_none());
        let before = cache.retained_bytes();
        assert!(cache.capture(&state, 0, || true));
        assert_eq!(cache.retained_bytes(), before);
        let delimiter = "STOP".repeat(1024);
        let opening = format!("BEGIN {delimiter}\n");
        for n in 0..INTERVAL {
            let line = if n < 64 { &opening } else { "return 1\n" };
            advance(&mut state.old, line, &syntaxes, &highlighter, false);
            advance(&mut state.new, line, &syntaxes, &highlighter, false);
            state.next += 1;
        }
        assert!(owned_heap(&state.old.parse, &state.old.highlight.path).unwrap() > MAX_ENTRY_BYTES);
        assert!(cache.capture(&state, 0, || true));
        assert_eq!(cache.retained_bytes(), before);
        assert!(cache.nearest(identity, state.next).unwrap().next < state.next);
        for _ in 0..64 {
            let closing = format!("END-{delimiter}\n");
            advance(&mut state.old, &closing, &syntaxes, &highlighter, false);
            advance(&mut state.new, &closing, &syntaxes, &highlighter, false);
        }
        assert_eq!(state.old.highlight.path, state.new.highlight.path);
        let mut sequential = HighlightLines::new(syntax, &theme);
        for side in [&mut state.old, &mut state.new] {
            assert!(side.fallback.is_none());
            assert_eq!(
                advance(side, "return 1\n", &syntaxes, &highlighter, true),
                super::super::tokens(sequential.highlight_line("return 1\n", &syntaxes).unwrap())
            );
        }
    }
}
