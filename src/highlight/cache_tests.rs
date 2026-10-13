use super::*;

fn allocated_style(mib: usize, old: Option<bool>) -> LineStyle {
    let mut tokens = Vec::with_capacity(11);
    tokens.push(Token {
        range: 1..4,
        color: Color::Blue,
    });
    let old_tokens = old.map(|nonempty| {
        let mut tokens = Vec::with_capacity(7);
        if nonempty {
            tokens.push(Token {
                range: 2..5,
                color: Color::Red,
            });
        }
        tokens
    });
    let mut changed = Vec::with_capacity(mib * 1024 * 1024 / size_of::<Range<usize>>());
    changed.push(3..4);
    LineStyle {
        tokens,
        old_tokens,
        changed,
        fallback: true,
        fallback_reason: None,
    }
}

fn expected_weight(style: &LineStyle) -> usize {
    size_of::<LineStyle>()
        + style.tokens.capacity() * size_of::<Token>()
        + style.old_tokens.as_ref().map_or(0, Vec::capacity) * size_of::<Token>()
        + style.changed.capacity() * size_of::<Range<usize>>()
}

#[cfg(feature = "owner-census")]
pub(super) fn assert_storage(cache: &Cache) {
    let census = cache.census();
    let storage = &census["container_storage"];
    let vectors: usize = cache
        .lines
        .values()
        .map(|style| expected_weight(style) - size_of::<LineStyle>())
        .sum();
    let table = cache.lines.allocation_size();
    let fifo = cache.order.capacity() * size_of::<LineId>();
    let visible = cache.visible.capacity() * size_of::<LineId>();
    assert_eq!(storage["table_allocation_bytes"], table);
    assert_eq!(storage["vector_capacity_bytes"], vectors);
    assert_eq!(storage["fifo_capacity_bytes"], fifo);
    assert_eq!(storage["visible_capacity_bytes"], visible);
    assert_eq!(
        storage["reported_heap_bytes"],
        table + vectors + fifo + visible
    );
    assert_eq!(storage["inline_cache_bytes"], size_of::<Cache>());
    assert_eq!(
        storage["diagnostic_inline_bytes"],
        size_of::<cache_activity::Activity>()
    );
    assert_eq!(
        storage["reported_value_and_heap_bytes"],
        table + vectors + fifo + visible + size_of::<Cache>()
    );
    assert!(table >= cache.lines.len() * size_of::<(LineId, LineStyle)>());
    assert_eq!(
        cache.bytes,
        vectors + cache.lines.len() * size_of::<LineStyle>()
    );
}

#[test]
fn real_capacity_accounting_preserves_duplicate_styles_and_fifo_survivors() {
    let mut cache = Cache::default();
    assert_eq!(cache.lines.allocation_size(), 0);
    #[cfg(feature = "owner-census")]
    assert_storage(&cache);
    let mut expected = Vec::new();
    for (id, old) in [(7, None), (2, Some(false)), (9, Some(true)), (4, None)] {
        let style = allocated_style(6, old);
        let weight = expected_weight(&style);
        expected.push((id, weight, style.clone(), style.tokens.as_ptr()));
        cache.insert(id, style);
        #[cfg(feature = "owner-census")]
        assert_storage(&cache);
        assert_eq!(cache.bytes, expected.iter().map(|row| row.1).sum::<usize>());
    }
    assert_eq!(cache.get(2).unwrap().tokens_for(true), &[]);
    assert_ne!(
        cache.get(9).unwrap().tokens_for(true),
        cache.get(9).unwrap().tokens_for(false)
    );
    assert_eq!(
        cache.get(7).unwrap().tokens_for(true),
        cache.get(7).unwrap().tokens_for(false)
    );
    let before = (
        cache.bytes,
        cache.order.clone(),
        cache.lines.allocation_size(),
    );
    cache.insert(7, allocated_style(33, Some(true)));
    assert_eq!(
        (
            cache.bytes,
            cache.order.clone(),
            cache.lines.allocation_size()
        ),
        before
    );

    let style = allocated_style(17, Some(false));
    expected.push((
        10,
        expected_weight(&style),
        style.clone(),
        style.tokens.as_ptr(),
    ));
    cache.insert(10, style);
    #[cfg(feature = "owner-census")]
    assert_storage(&cache);
    assert_eq!(
        cache.order.iter().copied().collect::<Vec<_>>(),
        vec![9.into(), 4.into(), 10.into()]
    );
    assert_eq!(cache.lines.len(), 3);
    assert!(cache.get(7).is_none() && cache.get(2).is_none());
    assert_eq!(
        cache.bytes,
        expected[2..].iter().map(|row| row.1).sum::<usize>()
    );
    for (id, weight, expected, pointer) in &expected[2..] {
        let actual = cache.get(*id).unwrap();
        assert_eq!(expected_weight(actual), *weight);
        assert_eq!(actual.tokens.as_ptr(), *pointer);
        assert_eq!(actual.tokens, expected.tokens);
        assert_eq!(actual.old_tokens, expected.old_tokens);
        assert_eq!(actual.changed, expected.changed);
        assert_eq!(actual.fallback, expected.fallback);
    }

    cache.insert(20, allocated_style(33, Some(true)));
    assert!(cache.lines.is_empty() && cache.order.is_empty());
    assert_eq!(cache.bytes, 0);
    assert!(cache.lines.allocation_size() > 0);
    assert!(cache.order.capacity() > 0);
    #[cfg(feature = "owner-census")]
    assert_storage(&cache);
    let style = allocated_style(1, None);
    let weight = expected_weight(&style);
    cache.insert(7, style);
    assert_eq!(cache.bytes, weight);
    assert_eq!(
        cache.order.iter().copied().collect::<Vec<_>>(),
        vec![7.into()]
    );
    assert!(cache.get(7).is_some());
    cache = Cache::default();
    assert_eq!(cache.lines.allocation_size(), 0);
    assert_eq!(cache.order.capacity(), 0);
    #[cfg(feature = "owner-census")]
    assert_storage(&cache);
}

#[test]
fn visible_coloring_completes_under_cache_pressure_without_another_request() {
    use crate::{
        app::{App, Layout},
        model::RowKind,
        render, storage,
    };
    use ratatui::{Terminal, backend::TestBackend};
    use std::time::{Duration, Instant};

    let mut patch = String::from(
        "diff --git a/code.ts b/code.ts\n--- a/code.ts\n+++ b/code.ts\n@@ -0,0 +1,64 @@\n",
    );
    for line in 0..64 {
        patch.push_str(&format!("+return 2; // item {line}\n"));
    }
    let mut doc = storage::Document::default();
    storage::read(patch.as_bytes(), |file| doc.append(file)).unwrap();
    let (wake, ready) = mpsc::sync_channel(1);
    let mut app = App::new(
        doc,
        "cache pressure".into(),
        Layout::Split,
        Appearance::Dark,
        wake,
    )
    .unwrap();
    app.loading = false;
    let mut terminal = Terminal::new(TestBackend::new(160, 12)).unwrap();
    terminal
        .draw(|frame| render::draw(frame, &mut app, None).unwrap())
        .unwrap();
    let ids: Vec<_> = app
        .visible
        .iter()
        .filter_map(|(p, _)| match app.row(p.row)?.kind {
            RowKind::Code { right, .. } => right,
            _ => None,
        })
        .collect();
    assert!(ids.len() > 4 && !ids.contains(&63.into()));

    app.worker.request(Request {
        epoch: 0,
        appearance: Appearance::Dark,
        ids: vec![0, 63],
        files: vec![app.doc.files[0].clone().into()],
    });
    let receive = |app: &App| {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            match app.worker.try_reply() {
                Ok(reply) => break reply.unwrap(),
                Err(mpsc::TryRecvError::Empty) => {
                    assert!(Instant::now() < until, "color worker did not complete");
                    let _ = ready.recv_timeout(Duration::from_millis(10));
                }
                Err(error) => panic!("color worker disconnected: {error}"),
            }
        }
    };
    let first = receive(&app);
    assert_eq!(first.id, ids[0]);
    let first_weight = first.style.cache_weight();
    let first_tokens = first.style.tokens.clone();
    app.accept_highlight(first).unwrap();
    let mut offscreen = receive(&app);
    assert_eq!(offscreen.id, 63.into());
    // Real capacity pressure, without changing the worker's source or style contents.
    let reserve = (32 * 1024 * 1024 - offscreen.style.cache_weight() - 3 * first_weight)
        / size_of::<Range<usize>>();
    offscreen.style.changed.reserve_exact(reserve);
    app.accept_highlight(offscreen).unwrap();
    assert!(app.cache.get(ids[0]).is_some());

    app.request_highlights();
    for &id in &ids[1..] {
        let reply = receive(&app);
        assert_eq!(reply.id, id);
        app.accept_highlight(reply).unwrap();
        terminal
            .draw(|frame| render::draw(frame, &mut app, None).unwrap())
            .unwrap();
    }
    for &id in &ids {
        let style = app
            .cache
            .get(id)
            .expect("visible style was evicted after its request was omitted");
        assert_eq!(style.tokens, first_tokens);
    }
    assert!(app.cache.get(63).is_none());
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(!text.contains("Highlighting visible"));

    app.toggle_theme().unwrap();
    terminal
        .draw(|frame| render::draw(frame, &mut app, None).unwrap())
        .unwrap();
    app.request_highlights();
    for &id in &ids {
        let mut reply = receive(&app);
        assert_eq!(reply.id, id);
        reply
            .style
            .changed
            .reserve_exact(6 * 1024 * 1024 / size_of::<Range<usize>>());
        app.accept_highlight(reply).unwrap();
        terminal
            .draw(|frame| render::draw(frame, &mut app, None).unwrap())
            .unwrap();
    }
    assert!(app.cache.bytes > 32 * 1024 * 1024);
    assert!(ids.iter().all(|id| app.cache.get(*id).is_some()));
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(!text.contains("Highlighting visible"));
    #[cfg(feature = "owner-census")]
    let over_target = app.cache.census();

    terminal.backend_mut().resize(30, 8);
    terminal.autoresize().unwrap();
    terminal
        .draw(|frame| render::draw(frame, &mut app, None).unwrap())
        .unwrap();
    assert!(app.visible.is_empty());
    assert!(app.cache.visible.is_empty());
    assert!(app.cache.bytes <= 32 * 1024 * 1024);
    assert!(ids.iter().any(|id| app.cache.get(*id).is_none()));
    #[cfg(feature = "owner-census")]
    {
        assert_storage(&app.cache);
        if let Some(path) = std::env::var_os("NATIVE_DIFF_VISIBLE_CACHE_TEST_OUTPUT") {
            serde_json::to_writer_pretty(
                std::fs::File::create_new(path).unwrap(),
                &vec![over_target, app.cache.census()],
            )
            .unwrap();
        }
    }
}
