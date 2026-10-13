use std::{iter, ops::Range};

use ratatui::style::Color;
use serde::Serialize;

use super::{Cache, LineStyle, Token};

pub(super) fn side(side: &super::Side) -> serde_json::Value {
    serde_json::json!({
        "parse_heap_bytes": side.parse.owned_heap_bytes(),
        "highlight_heap_bytes": side.highlight.owned_heap_bytes(),
    })
}

pub(super) fn worker(
    continuation: &Option<super::Continuation>,
    full: &super::full_source::Replay,
    checkpoints: &super::Checkpoints,
) -> serde_json::Value {
    serde_json::json!({
        "schema": 1,
        "patch": {
            "inline_size_bytes": size_of::<Option<super::Continuation>>(),
            "state": continuation.as_ref().map(|state| serde_json::json!({
                "next": state.next, "old": side(&state.old), "new": side(&state.new),
            })),
        },
        "full_source": full.census(), "checkpoints": checkpoints.census(),
    })
}

#[derive(Default, Serialize)]
struct Vectors {
    count: usize,
    length: usize,
    capacity: usize,
    max_length: usize,
    max_capacity: usize,
}

impl Vectors {
    fn observe(&mut self, length: usize, capacity: usize) {
        self.count += 1;
        self.length += length;
        self.capacity += capacity;
        self.max_length = self.max_length.max(length);
        self.max_capacity = self.max_capacity.max(capacity);
    }
}

#[derive(Default, Serialize)]
struct Tokens {
    vectors: Vectors,
    empty_ranges: usize,
    invalid_vectors: usize,
    eligible_nonempty_tokens: usize,
    hypothetical_merged_tokens: usize,
}

impl Tokens {
    fn observe(&mut self, tokens: &[Token], capacity: usize) -> bool {
        self.vectors.observe(tokens.len(), capacity);
        let empty = tokens
            .iter()
            .filter(|t| t.range.start == t.range.end)
            .count();
        self.empty_ranges += empty;
        let valid = ordered(tokens);
        if valid {
            self.eligible_nonempty_tokens += tokens.len() - empty;
            self.hypothetical_merged_tokens += runs(tokens).count();
        } else {
            self.invalid_vectors += 1;
        }
        valid
    }
}

fn ordered(tokens: &[Token]) -> bool {
    tokens.iter().all(|t| t.range.start <= t.range.end)
        && tokens
            .windows(2)
            .all(|pair| pair[0].range.end <= pair[1].range.start)
}

fn runs(tokens: &[Token]) -> impl Iterator<Item = (Range<usize>, Color)> + '_ {
    let mut tokens = tokens.iter().filter(|t| !t.range.is_empty()).peekable();
    iter::from_fn(move || {
        let first = tokens.next()?;
        let mut range = first.range.clone();
        while let Some(next) = tokens.peek()
            && next.range.start == range.end
            && next.color == first.color
        {
            range.end = next.range.end;
            tokens.next();
        }
        Some((range, first.color))
    })
}

#[derive(Default)]
struct Storage {
    primary: Tokens,
    old: Tokens,
    changed: Vectors,
    old_comparison_ineligible: usize,
    equivalent_old_vectors: usize,
    equivalent_old_capacity_bytes: usize,
    identical_old_vectors: usize,
}

impl Cache {
    pub(crate) fn census(&self) -> serde_json::Value {
        let mut storage = Storage::default();
        for style in self.lines.values() {
            let primary_valid = storage
                .primary
                .observe(&style.tokens, style.tokens.capacity());
            if let Some(old) = &style.old_tokens {
                let old_valid = storage.old.observe(old, old.capacity());
                if primary_valid && old_valid {
                    if runs(old).eq(runs(&style.tokens)) {
                        storage.equivalent_old_vectors += 1;
                        storage.equivalent_old_capacity_bytes +=
                            old.capacity() * size_of::<Token>();
                    }
                    storage.identical_old_vectors += usize::from(*old == style.tokens);
                } else {
                    storage.old_comparison_ineligible += 1;
                }
            }
            storage
                .changed
                .observe(style.changed.len(), style.changed.capacity());
        }
        let inline = self.lines.len() * size_of::<LineStyle>();
        let dynamic_capacity = (storage.primary.vectors.capacity + storage.old.vectors.capacity)
            * size_of::<Token>()
            + storage.changed.capacity * size_of::<Range<usize>>();
        let dynamic_used = (storage.primary.vectors.length + storage.old.vectors.length)
            * size_of::<Token>()
            + storage.changed.length * size_of::<Range<usize>>();
        let table_allocation = self.lines.allocation_size();
        let fifo_capacity = self.order.capacity() * size_of::<super::LineId>();
        let visible_capacity = self.visible.capacity() * size_of::<super::LineId>();
        let visible_bytes: usize = self
            .visible
            .iter()
            .filter_map(|id| self.lines.get(id))
            .map(LineStyle::cache_weight)
            .sum();
        let reported_heap = table_allocation + dynamic_capacity + fifo_capacity + visible_capacity;
        serde_json::json!({
            "entries": self.lines.len(), "payload_counter_bytes": self.bytes,
            "key_size_bytes": size_of::<super::LineId>(),
            "occupied_key_bytes": self.lines.len() * size_of::<super::LineId>(),
            "recomputed_payload_bytes": inline + dynamic_capacity, "map_capacity": self.lines.capacity(),
            "fifo_length": self.order.len(), "fifo_capacity": self.order.capacity(),
            "fifo_accounting": "recomputed-from-styles",
            "fifo_element_size_bytes": size_of::<super::LineId>(),
            "fifo_capacity_bytes": self.order.capacity() * size_of::<super::LineId>(),
            "fifo_accounted_bytes": self.order.iter().map(|id| {
                self.lines.get(id).expect("Queued style is missing").cache_weight()
            }).sum::<usize>(),
            "visible_protection": {
                "schema": 1, "retained_target_bytes": 32 * 1024 * 1024,
                "ids": self.visible.len(), "capacity": self.visible.capacity(),
                "capacity_bytes": visible_capacity,
                "cached_entries": self.visible.iter().filter(|id| self.lines.contains_key(*id)).count(),
                "payload_bytes": visible_bytes,
            },
            "activity": {
                "schema": 3, "scope": "current-cache-instance",
                "counter_storage_bytes": size_of::<super::cache_activity::Activity>(),
                "observations": self.activity,
            },
            "container_storage": {
                "schema": 2,
                "scope": "retained-requested-storage-excluding-allocator-overhead",
                "table_provider": "hashbrown-0.16.1-allocation_size",
                "table_allocation_bytes": table_allocation,
                "vector_capacity_bytes": dynamic_capacity,
                "fifo_capacity_bytes": fifo_capacity,
                "visible_capacity_bytes": visible_capacity,
                "reported_heap_bytes": reported_heap,
                "inline_scope": "observed-owner-census-build",
                "inline_cache_bytes": size_of::<Cache>(),
                "diagnostic_inline_bytes": size_of::<super::cache_activity::Activity>(),
                "reported_value_and_heap_bytes": reported_heap + size_of::<Cache>(),
            },
            "style_storage": {
                "schema": 1, "line_style_size_bytes": size_of::<LineStyle>(),
                "token_size_bytes": size_of::<Token>(), "changed_range_size_bytes": size_of::<Range<usize>>(),
                "inline_value_bytes": inline, "dynamic_capacity_bytes": dynamic_capacity,
                "dynamic_used_bytes": dynamic_used, "dynamic_spare_bytes": dynamic_capacity - dynamic_used,
                "primary": storage.primary, "old": storage.old, "changed": storage.changed,
                "old_comparison_ineligible": storage.old_comparison_ineligible,
                "equivalent_old_vectors": storage.equivalent_old_vectors,
                "equivalent_old_capacity_bytes": storage.equivalent_old_capacity_bytes,
                "identical_old_vectors": storage.identical_old_vectors,
            },
        })
    }
}

#[cfg(test)]
mod tests;
