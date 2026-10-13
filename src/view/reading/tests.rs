use super::*;
use crate::{
    source::{SourceStore, check_pair},
    storage::{self, Document},
};

fn read(patch: &[u8]) -> Document {
    let mut doc = Document::default();
    storage::read(patch, |file| doc.append(file)).unwrap();
    doc
}

fn sources(order: &SourceOrder) -> Vec<LineId> {
    (0..order.len())
        .filter_map(|row| {
            let source = order.source(row)?;
            assert_eq!(order.row_for_source(source), Some(row));
            Some(source)
        })
        .collect()
}

#[test]
fn unified_order_deduplicates_context_and_reorders_interleaved_replacements() {
    let doc = read(b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1,4 +1,4 @@\n context\n-old one\n+new one\n-old two\n+new two\n tail\n\\ No newline at end of file\ndiff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-again old\n+again new\n");
    assert_eq!(doc.files.len(), 2);
    let view = View::default();
    let first = view.source_order(doc.files[0].clone());
    assert!(!first.has_context());
    assert_eq!(sources(&first), [0, 1, 3, 2, 4, 5].map(LineId::Patch));
    assert_eq!(first.source(0), None);
    assert_eq!(first.source(1), None);
    assert_eq!(first.source(first.len() - 1), None);
    assert_eq!(first.source(first.len()), None);
    assert_eq!(first.row_for_source(LineId::Patch(6)), None);
    assert_eq!(
        first.row_for_source(LineId::context(0, 1, 1).unwrap()),
        None
    );
    let second = view.source_order(doc.files[1].clone());
    assert_eq!(sources(&second), [6, 7].map(LineId::Patch));
    assert_eq!(second.row_for_source(LineId::Patch(0)), None);
    let source_bytes: Vec<_> = sources(&first)
        .into_iter()
        .map(|id| doc.bytes(id.patch().unwrap()).unwrap())
        .collect();
    assert_eq!(
        source_bytes,
        [
            b"context\n".as_slice(),
            b"old one\n",
            b"old two\n",
            b"new one\n",
            b"new two\n",
            b"tail"
        ]
    );
}

#[test]
fn captured_order_contains_only_exposed_context_and_survives_view_changes() {
    let doc = read(b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -26 +26 @@\n-old\n+new\n");
    let mut old = Vec::new();
    let mut new = Vec::new();
    for line in 1..=52 {
        if line == 26 {
            old.extend_from_slice(b"old\n");
            new.extend_from_slice(b"new\n");
        } else {
            let bytes = format!("context {line}{}", if line == 52 { "" } else { "\n" });
            old.extend_from_slice(bytes.as_bytes());
            new.extend_from_slice(bytes.as_bytes());
        }
    }
    let mut store = SourceStore::sharing(&doc.files[0].spool).unwrap();
    let old = store.capture(old.as_slice(), &|| Ok(())).unwrap();
    let new = store.capture(new.as_slice(), &|| Ok(())).unwrap();
    let pair = check_pair(doc.files[0].clone(), Some(old), Some(new), &|| Ok(())).unwrap();
    let mut view = View::default();
    view.set(&doc, 0, Some(&pair)).unwrap();
    let hidden = view.source_order(doc.files[0].clone());
    assert!(!hidden.has_context());
    assert_eq!(sources(&hidden), [0, 1].map(LineId::Patch));
    view.expand(&doc, 0, 0).unwrap();
    view.expand(&doc, 0, 1).unwrap();
    let partial = view.source_order(doc.files[0].clone());
    assert!(partial.has_context());
    let context = |line| LineId::context(0, line, line).unwrap();
    let expected: Vec<_> = (6..26)
        .map(context)
        .chain([LineId::Patch(0), LineId::Patch(1)])
        .chain((27..47).map(context))
        .collect();
    assert_eq!(sources(&partial), expected);
    assert_eq!(partial.row_for_source(context(5)), None);
    assert_eq!(partial.row_for_source(context(47)), None);
    assert_eq!(
        partial.row_for_source(LineId::context(0, 6, 7).unwrap()),
        None
    );
    for row in 0..partial.len() {
        let visible = view.row(&doc, row, false).and_then(|row| match row.kind {
            RowKind::Code { left, right } => left.or(right),
            _ => None,
        });
        assert_eq!(partial.source(row), visible);
    }
    view.expand(&doc, 0, 0).unwrap();
    view.expand(&doc, 0, 1).unwrap();
    let expanded = view.source_order(doc.files[0].clone());
    let complete: Vec<_> = (1..26)
        .map(context)
        .chain([LineId::Patch(0), LineId::Patch(1)])
        .chain((27..=52).map(context))
        .collect();
    assert_eq!(sources(&expanded), complete);
    assert_eq!(expanded.source(expanded.len() - 1), None);
    view.set(&doc, 0, None).unwrap();
    assert_eq!(sources(&partial), expected);
    assert_eq!(sources(&expanded), complete);
    assert_eq!(
        sources(&view.source_order(doc.files[0].clone())),
        [0, 1].map(LineId::Patch)
    );
    assert_eq!(sources(&hidden), [0, 1].map(LineId::Patch));
}
