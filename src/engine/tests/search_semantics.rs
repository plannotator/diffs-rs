//! Pins literal search results against an independent brute-force oracle.
//! The oracle reads every searchable source line through the frozen order and lists each
//! occurrence, so a faster scan cannot change which match, offset, or wrap state is reported.

use std::io::Read;

use super::{search::review, *};
use crate::{
    input::{Cancellation, Cancelled},
    reading::{
        domain::Domain,
        literal::Direction,
        search::{Hit, Position},
    },
    view::LineId,
};

type Expected = (Position, LineId, std::ops::Range<u64>);

fn line_bytes(domain: &Domain, file: usize, source: LineId) -> Vec<u8> {
    let file = &domain.files[file];
    match source {
        LineId::Patch(id) => file.order.file.bytes(id).unwrap(),
        LineId::Context { new, .. } => {
            let mut bytes = Vec::new();
            file.context
                .as_ref()
                .unwrap()
                .uncached_lines()
                .line(new.get(), 0)
                .unwrap()
                .read_to_end(&mut bytes)
                .unwrap();
            bytes
        }
    }
}

/// Every occurrence in frozen unified order, overlapping ones included.
fn occurrences(domain: &Domain, query: &str) -> Vec<Expected> {
    let needle = query.as_bytes();
    let mut hits = Vec::new();
    for (file_id, file) in domain.files.iter().enumerate() {
        for row in 0..file.order.len() {
            let Some(source) = file.order.source(row) else {
                continue;
            };
            let bytes = line_bytes(domain, file_id, source);
            for start in 0..bytes.len() {
                if bytes[start..].starts_with(needle) {
                    let start = start as u64;
                    hits.push((
                        Position {
                            file: file_id,
                            row,
                            byte: start,
                        },
                        source,
                        start..start + needle.len() as u64,
                    ));
                }
            }
        }
    }
    hits
}

fn expected(hits: &[Expected], origin: Position, direction: Direction) -> Option<(Expected, bool)> {
    match direction {
        Direction::Forward => hits
            .iter()
            .find(|hit| hit.0 >= origin)
            .map(|hit| (hit.clone(), false))
            .or_else(|| hits.first().map(|hit| (hit.clone(), true))),
        Direction::Backward => hits
            .iter()
            .rev()
            .find(|hit| hit.0 < origin)
            .map(|hit| (hit.clone(), false))
            .or_else(|| hits.last().map(|hit| (hit.clone(), true))),
    }
}

fn actual(hit: Option<Hit>) -> Option<(Expected, bool)> {
    hit.map(|hit| ((hit.position, hit.source, hit.bytes), hit.wrapped))
}

/// Origins at every `hit_stride`-th match, one byte after it, the rows around it, both ends,
/// and every row of every `stride`-th file.
fn origins(domain: &Domain, hits: &[Expected], stride: usize, hit_stride: usize) -> Vec<Position> {
    let mut origins = vec![
        Position::default(),
        Position {
            file: domain.files.len(),
            ..Position::default()
        },
    ];
    for (file_id, file) in domain.files.iter().enumerate().step_by(stride) {
        for row in 0..=file.order.len() {
            origins.push(Position {
                file: file_id,
                row,
                byte: 0,
            });
        }
    }
    for (position, _, bytes) in hits.iter().step_by(hit_stride) {
        origins.push(*position);
        origins.push(Position {
            byte: bytes.start + 1,
            ..*position
        });
        for row in [position.row.saturating_sub(1), position.row + 1] {
            origins.push(Position {
                row,
                byte: 0,
                ..*position
            });
        }
    }
    origins.sort();
    origins.dedup();
    origins
}

fn assert_matches_oracle(domain: &Domain, query: &str) -> usize {
    assert_matches_oracle_from(domain, query, 1, 1)
}

fn assert_matches_oracle_from(
    domain: &Domain,
    query: &str,
    stride: usize,
    hit_stride: usize,
) -> usize {
    let hits = occurrences(domain, query);
    for origin in origins(domain, &hits, stride, hit_stride) {
        for direction in [Direction::Forward, Direction::Backward] {
            let found = domain
                .find(query, origin, direction, &|| Ok(()))
                .unwrap_or_else(|error| panic!("{query:?} from {origin:?}: {error:#}"));
            assert_eq!(
                actual(found),
                expected(&hits, origin, direction),
                "{query:?} {direction:?} from {origin:?}"
            );
        }
    }
    hits.len()
}

const STRUCTURE: &[u8] = b"diff --git a/needle_path.c b/needle_path.c
index 1111111..2222222 100644
--- a/needle_path.c
+++ b/needle_path.c
@@ -1,6 +1,6 @@ needle_function(void)
 context needle
-removed needle Needle
+added needle
 needleneedle
-a needle
+b needle
-c needle
+d needle
 tail
diff --git a/needle.bin b/needle.bin
new file mode 100644
index 0000000..3333333
Binary files /dev/null and b/needle.bin differ
diff --git a/old needle b/new needle
similarity index 100%
rename from old needle
rename to new needle
diff --git a/signs b/signs
--- a/signs
+++ b/signs
@@ -10,4 +10,4 @@ needle
 x
-+needle
++-needle
 y needle\r
-z
\\ No newline at end of file
+z needle
\\ No newline at end of file
";

#[test]
fn search_semantics_match_the_oracle_for_structure_signs_case_and_order() {
    let (mut engine, _directory) = review(STRUCTURE);
    let domain = engine.search_domain().unwrap();
    // Context, both sides of a change, interleaved changes, and overlapping hits.
    assert_eq!(assert_matches_oracle(&domain, "needle"), 13);
    // Case-sensitive: one capitalised occurrence only.
    assert_eq!(assert_matches_oracle(&domain, "Needle"), 1);
    // Signs are not source text: these only match the source that begins with them.
    assert_eq!(assert_matches_oracle(&domain, "+needle"), 1);
    assert_eq!(assert_matches_oracle(&domain, "-needle"), 1);
    assert_eq!(assert_matches_oracle(&domain, " context"), 0);
    assert_eq!(assert_matches_oracle(&domain, "context"), 1);
    // Headers, paths, hunk function context, binary and rename records, markers: no matches.
    for query in [
        "needle_path",
        "needle_function",
        "diff --git",
        "index 1111111",
        "@@",
        "Binary files",
        "rename from",
        "similarity",
        "No newline",
        "\\",
        "needle.bin",
        "new needle\n",
    ] {
        if query.contains('\n') {
            assert!(
                domain
                    .find(query, Position::default(), Direction::Forward, &|| Ok(()))
                    .is_err()
            );
            continue;
        }
        assert_eq!(assert_matches_oracle(&domain, query), 0, "{query:?}");
    }
    // Overlapping occurrences inside one line, and text that spans a removed sign.
    assert_eq!(assert_matches_oracle(&domain, "needleneedle"), 1);
    assert_eq!(assert_matches_oracle(&domain, "eneed"), 1);
    assert_eq!(assert_matches_oracle(&domain, "e"), 46);
    assert_eq!(assert_matches_oracle(&domain, "z"), 2);
    assert_eq!(assert_matches_oracle(&domain, "y needle"), 1);
    // Unified order lists a replacement block's removed lines before its added lines.
    // Source records keep their line ending; queries cannot contain one.
    let hits = occurrences(&domain, "needle");
    let lines: Vec<_> = hits
        .iter()
        .map(|(position, source, _)| {
            String::from_utf8(line_bytes(&domain, position.file, *source)).unwrap()
        })
        .collect();
    assert_eq!(
        lines,
        [
            "context needle\n",
            "removed needle Needle\n",
            "added needle\n",
            "needleneedle\n",
            "needleneedle\n",
            "a needle\n",
            "c needle\n",
            "b needle\n",
            "d needle\n",
            "+needle\n",
            "+-needle\n",
            "y needle\r\n",
            "z needle",
        ]
    );
    assert_eq!(
        hits.iter().map(|hit| hit.0.file).collect::<Vec<_>>(),
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 3, 3, 3]
    );
    engine.close().unwrap();
}

/// Many small files, a few hits, long lines across read-window boundaries.
fn large_patch() -> (Vec<u8>, String) {
    let query = "zq_needle_\u{65e5}";
    let mut patch = Vec::new();
    let mut state = 0x2545_f491_u32;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state
    };
    for file in 0..2500 {
        patch.extend_from_slice(
            format!("diff --git a/f{file} b/f{file}\n--- a/f{file}\n+++ b/f{file}\n").as_bytes(),
        );
        let lines = 1 + next() as usize % 6;
        patch.extend_from_slice(format!("@@ -1,{lines} +1,{lines} @@ {query}\n").as_bytes());
        for line in 0..lines {
            let mut text = format!("{file} {line} filler text");
            match next() % 400 {
                0 => text.push_str(query),
                1 => text.insert_str(0, query),
                2 => text = format!("{query}{query}"),
                _ => {}
            }
            match next() % 3 {
                0 => patch.extend_from_slice(format!(" {text}\n").as_bytes()),
                _ => patch.extend_from_slice(format!("-{text}\n+{text} new\n").as_bytes()),
            }
        }
        if file % 500 == 250 {
            // Long lines whose hits straddle 64 KiB and 1 MiB read boundaries.
            let mut long = vec![b'.'; 3 * 1024 * 1024 + 17];
            for boundary in [64 * 1024, 1024 * 1024, 2 * 1024 * 1024, 3 * 1024 * 1024] {
                let at = boundary - 5 - file % 7;
                long[at..at + query.len()].copy_from_slice(query.as_bytes());
            }
            patch.extend_from_slice(b"@@ -100 +100 @@\n-");
            patch.extend_from_slice(&long);
            patch.extend_from_slice(b"\n+");
            patch.extend_from_slice(&long[..long.len() - 9]);
            patch.push(b'\n');
            // Correct the hunk header counts by adding this as its own hunk.
        }
    }
    (patch, query.to_owned())
}

#[test]
fn search_semantics_match_the_oracle_across_many_files_and_long_lines() {
    let (patch, query) = large_patch();
    let (mut engine, _directory) = review(&patch);
    let domain = engine.search_domain().unwrap();
    assert_eq!(domain.files.len(), 2500);
    let hits = assert_matches_oracle_from(&domain, &query, 97, 1);
    assert!(hits > 40, "{hits}");
    assert_eq!(assert_matches_oracle_from(&domain, "absent text", 97, 1), 0);
    // Common text: the first hit is near, but the scan must still honour order and bounds.
    assert_matches_oracle_from(&domain, "filler text new", 499, 211);
    engine.close().unwrap();
}

#[test]
fn search_semantics_include_exposed_context_in_order() {
    let (mut engine, _directory, pairs) = checked_review(b"needle one needle\n");
    install(&mut engine, &pairs);
    {
        let app = engine.app.as_mut().unwrap();
        app.view.expand(&app.doc, 0, 0).unwrap();
    }
    let domain = engine.search_domain().unwrap();
    assert!(domain.files[0].order.has_context());
    assert_eq!(assert_matches_oracle(&domain, "needle"), 2);
    assert_eq!(assert_matches_oracle(&domain, "e"), 13);
    engine.close().unwrap();
}

#[test]
fn search_without_a_match_or_after_cancellation_reports_none_or_cancelled() {
    let (patch, _) = large_patch();
    let (mut engine, _directory) = review(&patch);
    let domain = engine.search_domain().unwrap();
    for direction in [Direction::Forward, Direction::Backward] {
        assert!(
            domain
                .find("absent text", Position::default(), direction, &|| Ok(()))
                .unwrap()
                .is_none()
        );
        let cancel = Cancellation::default();
        let checks = std::cell::Cell::new(0);
        let result = domain.find("absent text", Position::default(), direction, &|| {
            checks.set(checks.get() + 1);
            if checks.get() == 5 {
                cancel.stop();
            }
            cancel.check()
        });
        assert!(result.unwrap_err().is::<Cancelled>());
        assert_eq!(checks.get(), 5);
    }
    engine.close().unwrap();
}
