use super::*;

fn tokens(ranges: &[(usize, usize, Color)]) -> Vec<Token> {
    ranges
        .iter()
        .map(|&(start, end, color)| Token {
            range: start..end,
            color,
        })
        .collect()
}

#[test]
fn coverage_respects_partitions_gaps_empty_spans_and_invalid_ranges() {
    let red = Color::Red;
    let blue = Color::Blue;
    let a = tokens(&[(0, 2, red), (2, 2, blue), (2, 5, red), (7, 9, red)]);
    let b = tokens(&[(0, 5, red), (7, 9, red)]);
    assert!(ordered(&a) && ordered(&b));
    let mut counts = Tokens::default();
    assert!(counts.observe(&a, a.capacity()));
    assert_eq!(counts.empty_ranges, 1);
    assert_eq!(counts.eligible_nonempty_tokens, 3);
    assert_eq!(counts.hypothetical_merged_tokens, 2);
    assert!(runs(&a).eq(runs(&b)));
    assert_eq!(runs(&a).collect::<Vec<_>>(), vec![(0..5, red), (7..9, red)]);
    for other in [
        tokens(&[(0, 9, red)]),
        tokens(&[(1, 5, red), (7, 9, red)]),
        tokens(&[(0, 5, red), (7, 10, red)]),
        tokens(&[(0, 5, blue), (7, 9, red)]),
        Vec::new(),
    ] {
        assert!(ordered(&other));
        assert!(!runs(&a).eq(runs(&other)));
    }
    assert!(runs(&tokens(&[(9, 9, blue)])).eq(runs(&[])));
    for invalid in [
        tokens(&[(2, 1, red)]),
        tokens(&[(0, 3, red), (2, 5, red)]),
        tokens(&[(7, 9, red), (0, 5, red)]),
    ] {
        let mut counts = Tokens::default();
        assert!(!counts.observe(&invalid, invalid.capacity()));
        assert_eq!(counts.invalid_vectors, 1);
        assert_eq!(counts.eligible_nonempty_tokens, 0);
        assert_eq!(counts.hypothetical_merged_tokens, 0);
    }
    let end = usize::MAX;
    let large = tokens(&[(end - 4, end - 2, red), (end - 2, end, red)]);
    assert!(ordered(&large));
    assert_eq!(runs(&large).collect::<Vec<_>>(), vec![(end - 4..end, red)]);
}

#[test]
fn cache_accounting_is_disjoint_and_inspection_preserves_storage() {
    let mut primary = Vec::with_capacity(11);
    primary.extend(tokens(&[(0, 2, Color::Red), (2, 5, Color::Red)]));
    let mut old = Vec::with_capacity(7);
    old.extend(tokens(&[(0, 5, Color::Red)]));
    let mut changed = Vec::with_capacity(5);
    changed.push(1..3);
    let styles = [
        LineStyle {
            tokens: primary,
            old_tokens: Some(old),
            changed,
            fallback: false,
            fallback_reason: None,
        },
        LineStyle {
            tokens: tokens(&[(0, 1, Color::Blue)]),
            old_tokens: Some(Vec::new()),
            ..LineStyle::default()
        },
        LineStyle::default(),
    ];
    let expected_dynamic: usize = styles
        .iter()
        .map(|style| {
            (style.tokens.capacity() + style.old_tokens.as_ref().map_or(0, Vec::capacity))
                * size_of::<Token>()
                + style.changed.capacity() * size_of::<Range<usize>>()
        })
        .sum();
    let expected_old_capacity =
        styles[0].old_tokens.as_ref().unwrap().capacity() * size_of::<Token>();
    let mut cache = Cache::default();
    for (id, style) in styles.into_iter().enumerate() {
        cache.insert(id, style);
    }
    let values = cache.lines.clone();
    let addresses: Vec<_> = cache
        .lines
        .iter()
        .map(|(&id, style)| {
            (
                id,
                style.tokens.as_ptr(),
                style.tokens.capacity(),
                style.changed.as_ptr(),
                style.changed.capacity(),
                style
                    .old_tokens
                    .as_ref()
                    .map(|old| (old.as_ptr(), old.capacity())),
            )
        })
        .collect();
    let order = cache.order.clone();
    let before = (cache.lines.capacity(), cache.order.capacity(), cache.bytes);
    let census = cache.census();
    crate::highlight::cache_tests::assert_storage(&cache);
    assert_eq!(cache.census(), census);
    assert_eq!(census["fifo_accounting"], "recomputed-from-styles");
    assert_eq!(
        census["fifo_element_size_bytes"],
        size_of::<crate::view::LineId>()
    );
    assert_eq!(
        census["fifo_capacity_bytes"],
        cache.order.capacity() * size_of::<crate::view::LineId>()
    );
    assert_eq!(
        census["fifo_accounted_bytes"],
        expected_dynamic + 3 * size_of::<LineStyle>()
    );
    let observed = &census["style_storage"];
    assert_eq!(observed["dynamic_capacity_bytes"], expected_dynamic);
    let used = 4 * size_of::<Token>() + size_of::<Range<usize>>();
    assert_eq!(observed["dynamic_used_bytes"], used);
    assert_eq!(observed["dynamic_spare_bytes"], expected_dynamic - used);
    assert_eq!(observed["inline_value_bytes"], 3 * size_of::<LineStyle>());
    assert_eq!(
        census["payload_counter_bytes"],
        expected_dynamic + 3 * size_of::<LineStyle>()
    );
    assert_eq!(observed["primary"]["vectors"]["count"], 3);
    assert_eq!(observed["primary"]["vectors"]["length"], 3);
    assert_eq!(observed["primary"]["hypothetical_merged_tokens"], 2);
    assert_eq!(observed["old"]["vectors"]["count"], 2);
    assert_eq!(observed["old"]["vectors"]["length"], 1);
    assert_eq!(observed["equivalent_old_vectors"], 1);
    assert_eq!(
        observed["equivalent_old_capacity_bytes"],
        expected_old_capacity
    );
    assert_eq!(observed["identical_old_vectors"], 0);
    assert_eq!(cache.lines.len(), values.len());
    for (id, expected) in values {
        let actual = cache.lines.get(&id).unwrap();
        assert_eq!(actual.tokens, expected.tokens);
        assert_eq!(actual.old_tokens, expected.old_tokens);
        assert_eq!(actual.changed, expected.changed);
        assert_eq!(actual.fallback, expected.fallback);
    }
    assert_eq!(cache.order, order);
    assert_eq!(
        (cache.lines.capacity(), cache.order.capacity(), cache.bytes),
        before
    );
    for (id, pointer, capacity, changed_pointer, changed_capacity, old) in addresses {
        let style = cache.lines.get(&id).unwrap();
        assert_eq!(
            (style.tokens.as_ptr(), style.tokens.capacity()),
            (pointer, capacity)
        );
        assert_eq!(
            (style.changed.as_ptr(), style.changed.capacity()),
            (changed_pointer, changed_capacity)
        );
        assert_eq!(
            style
                .old_tokens
                .as_ref()
                .map(|value| (value.as_ptr(), value.capacity())),
            old
        );
    }
}
