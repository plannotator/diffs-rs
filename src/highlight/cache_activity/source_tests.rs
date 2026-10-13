use std::{
    fs::File,
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use super::Point;
use crate::{
    app::{App, Layout},
    highlight::{Cache, FileSource, Reply, Request},
    input::SourceState,
    source::{CheckedPair, SourceStore, check_pair},
    storage::{self, FileIndex},
    theme::Appearance,
    view::LineId,
};

fn record(cache: &Cache, label: &str, records: &mut Vec<Value>) {
    crate::highlight::cache_tests::assert_storage(cache);
    let census = cache.census();
    assert_eq!(cache.census(), census);
    let a = &cache.activity;
    assert_eq!(
        a.admissions - a.evictions - a.invalidated_entries,
        cache.lines.len()
    );
    assert_eq!(
        a.admitted_payload_bytes - a.evicted_payload_bytes - a.invalidated_payload_bytes,
        cache.bytes
    );
    assert_eq!(
        a.insertion_capacity_gained + a.removal_capacity_gained + a.invalidation_capacity_gained,
        cache.lines.capacity()
            + a.insertion_capacity_lost
            + a.removal_capacity_lost
            + a.invalidation_capacity_lost
    );
    assert_eq!(
        cache.bytes,
        cache
            .lines
            .values()
            .map(|s| s.cache_weight())
            .sum::<usize>()
    );
    assert_eq!(cache.order.len(), cache.lines.len());
    assert!(cache.order.iter().all(|id| cache.lines.contains_key(id)));
    records.push(json!({"label": label, "cache": census}));
}

fn source_change(app: &mut App, file: usize, state: SourceState) {
    let range = app.doc.files[file].source_base..app.doc.files[file].source_end();
    let removed = |id: &LineId| match id {
        LineId::Patch(id) => range.contains(id),
        LineId::Context { file: owner, .. } => *owner == file,
    };
    let expected_order: Vec<_> = app
        .cache
        .order
        .iter()
        .copied()
        .filter(|id| !removed(id))
        .collect();
    let survivors: Vec<_> = app
        .cache
        .lines
        .iter()
        .filter(|(id, _)| !removed(id))
        .map(|(id, style)| (*id, style.clone(), style.cache_weight()))
        .collect();
    let removed_bytes: usize = app
        .cache
        .lines
        .iter()
        .filter(|(id, _)| removed(id))
        .map(|(_, style)| style.cache_weight())
        .sum();
    let removed_count = app.cache.lines.len() - survivors.len();
    let before = Point::observe(&app.cache);
    let activity = serde_json::to_value(&app.cache.activity).unwrap();
    app.set_source(file, state).unwrap();
    assert_eq!(
        app.cache.order.iter().copied().collect::<Vec<_>>(),
        expected_order
    );
    assert_eq!(app.cache.bytes, before.payload_bytes - removed_bytes);
    assert_eq!(app.cache.lines.len(), survivors.len());
    for (id, style, weight) in survivors {
        let actual = app.cache.get(id).unwrap();
        assert_eq!(actual.tokens, style.tokens);
        assert_eq!(actual.old_tokens, style.old_tokens);
        assert_eq!(actual.changed, style.changed);
        assert_eq!(actual.cache_weight(), weight);
    }
    let a = &app.cache.activity;
    assert_eq!(
        a.invalidations as u64,
        activity["invalidations"].as_u64().unwrap() + 1
    );
    assert_eq!(
        a.invalidated_entries as u64,
        activity["invalidated_entries"].as_u64().unwrap() + removed_count as u64
    );
    assert_eq!(
        a.invalidated_payload_bytes as u64,
        activity["invalidated_payload_bytes"].as_u64().unwrap() + removed_bytes as u64
    );
    assert_eq!(a.evictions as u64, activity["evictions"].as_u64().unwrap());
    assert_eq!(
        a.admissions as u64,
        activity["admissions"].as_u64().unwrap()
    );
}

fn replies(app: &App, source: FileSource, ids: Vec<LineId>) -> Vec<Reply> {
    let count = ids.len();
    app.worker.request(Request {
        epoch: app.epoch,
        appearance: app.appearance,
        ids,
        files: vec![source],
    });
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut received = Vec::new();
    while received.len() < count {
        match app.worker.try_reply() {
            Ok(reply) => {
                let reply = reply.unwrap();
                assert!(reply.source_error.is_none());
                received.push(reply);
            }
            Err(mpsc::TryRecvError::Empty) => {
                assert!(Instant::now() < deadline, "Highlighting did not complete");
                thread::sleep(Duration::from_millis(1));
            }
            Err(error) => panic!("Highlight worker disconnected: {error}"),
        }
    }
    received
}

fn fill(app: &mut App, pair: &Arc<CheckedPair>) {
    let file = pair.file();
    let ids = (file.source_base..file.source_end())
        .map(LineId::Patch)
        .chain([LineId::context(file.id, 10, 10).unwrap()])
        .collect();
    for reply in replies(app, FileSource::Complete(pair.clone()), ids) {
        assert!(!reply.style.tokens.is_empty());
        assert!(app.accept_highlight(reply).unwrap());
    }
}

fn pair(store: &mut SourceStore, file: &Arc<FileIndex>, comment: bool) -> Arc<CheckedPair> {
    let mut old = if comment { "/*\n" } else { "// full source\n" }.to_owned();
    for n in 2..40 {
        old.push_str(&format!("const value{n} = {n};\n"));
    }
    old.push_str(if comment { "*/\n" } else { "// end\n" });
    let new = old.replace("const value20 = 20;", "const value20 = 999;");
    let a = store.capture(old.as_bytes(), &|| Ok(())).unwrap();
    let b = store.capture(new.as_bytes(), &|| Ok(())).unwrap();
    Arc::new(check_pair(file.clone(), Some(a), Some(b), &|| Ok(())).unwrap())
}

fn canonical(app: &App) -> Vec<u8> {
    let mut bytes = Vec::new();
    for file in &app.doc.files {
        file.write_canonical(&mut bytes).unwrap();
    }
    bytes
}

#[test]
fn real_source_replacement_revocation_and_reset_reconcile_cache_ownership() {
    let mut patch = String::new();
    for name in ["alpha.ts", "dir/beta.ts"] {
        patch.push_str(&format!("diff --git a/{name} b/{name}\n--- a/{name}\n+++ b/{name}\n@@ -17,7 +17,7 @@\n const value17 = 17;\n const value18 = 18;\n const value19 = 19;\n-const value20 = 20;\n+const value20 = 999;\n const value21 = 21;\n const value22 = 22;\n const value23 = 23;\n"));
    }
    let mut doc = storage::Document::default();
    let spool = storage::read(patch.as_bytes(), |file| doc.append(file)).unwrap();
    let mut store = SourceStore::sharing(&spool).unwrap();
    let pairs: Vec<_> = doc
        .files
        .iter()
        .map(|file| pair(&mut store, file, true))
        .collect();
    let replacement = pair(&mut store, &doc.files[0], false);
    let weak: Vec<_> = pairs
        .iter()
        .chain([&replacement])
        .map(Arc::downgrade)
        .collect();
    let (wake, _notified) = mpsc::sync_channel(64);
    let mut app = App::new(
        doc,
        "source accounting".into(),
        Layout::Split,
        Appearance::Dark,
        wake,
    )
    .unwrap();
    app.loading = false;
    app.sources = Some(vec![SourceState::Pending; 2]);
    let original = canonical(&app);
    let mut records = Vec::new();
    record(&app.cache, "empty", &mut records);
    for file in app.doc.files.clone() {
        let ids = (file.source_base..file.source_end())
            .map(LineId::Patch)
            .collect();
        for reply in replies(&app, FileSource::Plain(file), ids) {
            assert!(app.accept_highlight(reply).unwrap());
        }
    }
    record(&app.cache, "plain", &mut records);
    for (file, pair) in pairs.iter().enumerate() {
        source_change(&mut app, file, SourceState::Checked(pair.clone()));
        fill(&mut app, pair);
    }
    record(&app.cache, "checked", &mut records);
    let id = LineId::Patch(app.doc.files[0].source_base);
    let old_tokens = app.cache.get(id).unwrap().tokens.clone();
    let stale = replies(&app, FileSource::Complete(pairs[0].clone()), vec![id])
        .pop()
        .unwrap();
    source_change(&mut app, 0, SourceState::Checked(replacement.clone()));
    record(&app.cache, "replacement-invalidated", &mut records);
    let before_stale = app.cache.census();
    assert!(!app.accept_highlight(stale).unwrap());
    assert_eq!(app.cache.census(), before_stale);
    fill(&mut app, &replacement);
    assert_ne!(app.cache.get(id).unwrap().tokens, old_tokens);
    record(&app.cache, "replacement-colored", &mut records);
    let stale = replies(&app, FileSource::Complete(replacement.clone()), vec![id])
        .pop()
        .unwrap();
    source_change(
        &mut app,
        0,
        SourceState::Unavailable("source revoked".into()),
    );
    record(&app.cache, "revoked", &mut records);
    let before_stale = app.cache.census();
    assert!(!app.accept_highlight(stale).unwrap());
    assert_eq!(app.cache.census(), before_stale);
    source_change(
        &mut app,
        0,
        SourceState::Unavailable("still unavailable".into()),
    );
    record(&app.cache, "empty-revocation", &mut records);
    source_change(&mut app, 0, SourceState::Checked(replacement.clone()));
    fill(&mut app, &replacement);
    record(&app.cache, "recovered", &mut records);
    let stale = replies(&app, FileSource::Complete(replacement.clone()), vec![id])
        .pop()
        .unwrap();
    app.toggle_theme().unwrap();
    assert!(!app.accept_highlight(stale).unwrap());
    assert_eq!(app.cache.census(), Cache::default().census());
    record(&app.cache, "reset", &mut records);
    fill(&mut app, &replacement);
    record(&app.cache, "new-instance", &mut records);
    assert_eq!(canonical(&app), original);
    drop(app);
    drop(replacement);
    drop(pairs);
    assert!(weak.iter().all(|pair| pair.upgrade().is_none()));
    if let Some(path) = std::env::var_os("NATIVE_DIFF_CACHE_ACTIVITY_TEST_OUTPUT") {
        serde_json::to_writer_pretty(File::create_new(path).unwrap(), &records).unwrap();
    }
}
