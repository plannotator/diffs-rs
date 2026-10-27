use std::collections::{BTreeSet, HashMap};

use diffs_rs::view::LineId;

#[test]
fn source_identities_keep_full_ranges_and_distinct_coordinates() {
    for (old, new) in [(0, 0), (0, 1), (1, 0), (0, u32::MAX), (u32::MAX, 0)] {
        assert_eq!(LineId::context(usize::MAX, old, new), None);
    }
    let mut expected: Vec<_> = [0, 1, usize::MAX].into_iter().map(LineId::Patch).collect();
    for file in [0, 1, usize::MAX] {
        for old in [1, 2, u32::MAX] {
            for new in [1, 2, u32::MAX] {
                let id = LineId::context(file, old, new).unwrap();
                let LineId::Context {
                    file: actual_file,
                    old: actual_old,
                    new: actual_new,
                } = id
                else {
                    panic!("Context identity became a patch identity");
                };
                assert_eq!(
                    (actual_file, actual_old.get(), actual_new.get()),
                    (file, old, new)
                );
                expected.push(id);
            }
        }
    }
    let ordered: BTreeSet<_> = expected.iter().rev().copied().collect();
    assert_eq!(ordered.into_iter().collect::<Vec<_>>(), expected);
    let indexed: HashMap<_, _> = expected.iter().copied().zip(0..).collect();
    assert_eq!(indexed.len(), expected.len());
    for (index, id) in expected.into_iter().enumerate() {
        assert_eq!(indexed[&id], index);
    }
}

#[test]
#[cfg(feature = "delivery-trace")]
fn diagnostic_identities_remain_numeric() {
    assert_eq!(
        serde_json::to_value(LineId::Patch(usize::MAX)).unwrap(),
        serde_json::json!(usize::MAX)
    );
    assert_eq!(
        serde_json::to_value(LineId::context(usize::MAX, 1, u32::MAX).unwrap()).unwrap(),
        serde_json::json!({"file": usize::MAX, "old": 1, "new": u32::MAX}),
    );
}
