use std::{
    fs::File,
    io::{BufReader, Read},
    sync::mpsc,
    thread,
};

use anyhow::{Context, Result, ensure};
use native_diff::{model::RowKind, storage};
use serde::Serialize;

pub use native_diff::{ingest, text};

#[path = "projection_reference/model.rs"]
#[allow(dead_code)]
mod reference;

#[derive(Default, Serialize)]
struct Counts {
    files: usize,
    source_records: usize,
    rows: usize,
    split_rows: usize,
    unified_rows: usize,
    inverse_lookups: usize,
    partners: usize,
    boundary_probes: usize,
    compact_replacements: usize,
    wide_replacements: usize,
}

fn compare_file(
    file: &storage::FileIndex,
    parsed: &reference::Document,
    f: usize,
    counts: &mut Counts,
) -> Result<()> {
    ensure!(
        file.id == counts.files && file.source_base == counts.source_records,
        "File/source publication order at file {}",
        counts.files
    );
    let old_meta = &parsed.files[f];
    let start = old_meta.hunks.first().map_or(0, |h| h.lines.start);
    let end = old_meta.hunks.last().map_or(0, |h| h.lines.end);
    let replacements: usize = old_meta
        .hunks
        .iter()
        .map(|hunk| {
            parsed.lines[hunk.lines.clone()]
                .split(|line| matches!(line.kind, reference::Kind::Context))
                .filter(|group| !group.is_empty())
                .count()
        })
        .sum();
    let compact = old_meta
        .hunks
        .len()
        .checked_mul(2)
        .and_then(|n| n.checked_add(2))
        .and_then(|n| n.checked_add(end - start))
        .is_some_and(|n| u32::try_from(n).is_ok());
    if compact {
        counts.compact_replacements += replacements;
    } else {
        counts.wide_replacements += replacements;
    }
    let mut expected_meta = serde_json::to_value(old_meta)?;
    for hunk in expected_meta["hunks"].as_array_mut().unwrap() {
        for key in ["start", "end"] {
            hunk["lines"][key] =
                (hunk["lines"][key].as_u64().unwrap() as usize - start + file.source_base).into();
        }
    }
    ensure!(
        serde_json::to_value(&file.meta)? == expected_meta,
        "File {} metadata",
        file.id
    );
    ensure!(
        file.source_end() - file.source_base == end - start,
        "File {} source count",
        file.id
    );
    for split in [true, false] {
        let rows = if split {
            &parsed.split
        } else {
            &parsed.unified
        };
        let locations = if split {
            &parsed.split_locations
        } else {
            &parsed.unified_locations
        };
        let begin = rows.partition_point(|r| r.file < f);
        let finish = rows.partition_point(|r| r.file <= f);
        let rows = &rows[begin..finish];
        let actual = file.rows(split);
        ensure!(
            file.row_base(split)
                == if split {
                    counts.split_rows
                } else {
                    counts.unified_rows
                },
            "File {} layout {split} global row base",
            file.id
        );
        ensure!(
            actual.len() == rows.len(),
            "File {} layout {split} row count",
            file.id
        );
        for (n, expected) in rows.iter().enumerate() {
            let row = actual.get(n).context("Missing row")?;
            let kind = match row.kind {
                RowKind::Header => reference::RowKind::Header,
                RowKind::Hunk(h) => reference::RowKind::Hunk(h),
                RowKind::Gap => reference::RowKind::Gap,
                RowKind::Metadata => reference::RowKind::Metadata,
                RowKind::Code { left, right } => reference::RowKind::Code {
                    left: left.map(|id| id - file.source_base + start),
                    right: right.map(|id| id - file.source_base + start),
                },
                RowKind::NoNewline { left, right } => reference::RowKind::NoNewline { left, right },
            };
            ensure!(
                row.file == file.id && kind == expected.kind,
                "File {} layout {split} row {n}: {kind:?} != {:?}",
                file.id,
                expected.kind
            );
            counts.rows += 1;
        }
        ensure!(actual.get(rows.len()).is_none(), "Row after file end");
        if split {
            counts.split_rows += rows.len();
        } else {
            counts.unified_rows += rows.len();
        }
        for (n, expected) in locations[start..end].iter().enumerate() {
            ensure!(
                file.row_for_source(file.source_base + n, split)
                    == file.row_base(split) + expected - begin,
                "File {} layout {split} inverse source {n}",
                file.id
            );
            counts.inverse_lookups += 1;
        }
        for file_boundary in [true, false] {
            let boundaries: Vec<_> = rows
                .iter()
                .enumerate()
                .filter_map(|(n, row)| {
                    let matches = if file_boundary {
                        row.kind == reference::RowKind::Header
                    } else {
                        matches!(row.kind, reference::RowKind::Hunk(_))
                    };
                    matches.then_some(n)
                })
                .collect();
            let mut probes = vec![0, rows.len() - 1];
            for &n in &boundaries {
                probes.extend(n.saturating_sub(1)..=(n + 1).min(rows.len() - 1));
            }
            probes.sort_unstable();
            probes.dedup();
            for forward in [true, false] {
                ensure!(
                    actual.first_boundary(forward, file_boundary)
                        == if forward {
                            boundaries.first().copied()
                        } else {
                            boundaries.last().copied()
                        },
                    "File {} first boundary",
                    file.id
                );
                for &n in &probes {
                    let expected = if forward {
                        boundaries.iter().copied().find(|b| *b > n)
                    } else {
                        boundaries.iter().rev().copied().find(|b| *b < n)
                    };
                    ensure!(
                        actual.boundary(n, forward, file_boundary) == expected,
                        "File {} layout {split} boundary at {n}",
                        file.id
                    );
                    counts.boundary_probes += 1;
                }
            }
        }
    }
    for (n, line) in parsed.lines[start..end].iter().enumerate() {
        let id = file.source_base + n;
        let info = file.info(id);
        ensure!(
            serde_json::to_value(info.kind)? == serde_json::to_value(line.kind)?
                && info.old == line.old
                && info.new == line.new
                && info.no_newline == line.no_newline
                && file.bytes(id)? == line.bytes,
            "File {} source {n}",
            file.id
        );
        let expected = match parsed.split[parsed.split_locations[start + n]].kind {
            reference::RowKind::Code {
                left: Some(l),
                right: Some(r),
            } if l != r => Some(if l == start + n { r } else { l } - start + file.source_base),
            _ => None,
        };
        ensure!(file.partner(id) == expected, "File {} partner {n}", file.id);
        counts.source_records += 1;
        counts.partners += 1;
    }
    counts.files += 1;
    Ok(())
}

fn compare(actual: impl Read, expected: impl Read + Send) -> Result<Counts> {
    thread::scope(|scope| {
        let (send, recv) = mpsc::sync_channel(1);
        let worker = scope.spawn(move || {
            ingest::visit_files(BufReader::new(expected), |unit| {
                let mut parsed = reference::Document::parse_file(unit)?;
                parsed.index_rows();
                if !parsed.files.is_empty() {
                    send.send(parsed).context("Reference consumer stopped")?;
                }
                Ok(())
            })
        });
        let mut counts = Counts::default();
        let mut parsed = reference::Document::default();
        let mut file = 0;
        let result = storage::read(actual, |indexed| {
            if file == parsed.files.len() {
                parsed = recv.recv().context("Reference ended early")?;
                file = 0;
            }
            compare_file(&indexed, &parsed, file, &mut counts)?;
            file += 1;
            Ok(())
        });
        let no_extra = result.is_ok() && file == parsed.files.len() && recv.recv().is_err();
        drop(recv);
        let reference_result = worker
            .join()
            .map_err(|_| anyhow::anyhow!("Reference worker panicked"))?;
        result?;
        reference_result?;
        ensure!(no_extra, "Reference contains extra files");
        Ok(counts)
    })
}

fn patterns() -> Result<serde_json::Value> {
    let mut input = String::new();
    let (mut accepted, mut rejected) = (0, 0);
    for length in 1..=6 {
        for code in 0..3usize.pow(length) {
            let signs: Vec<_> = (0..length)
                .map(|n| b" -+"[(code / 3usize.pow(n)) % 3])
                .collect();
            if signs.iter().all(|s| *s == b' ') {
                continue;
            }
            let old = signs.iter().filter(|s| **s != b'+').count();
            let new = signs.iter().filter(|s| **s != b'-').count();
            let last_old = signs.iter().rposition(|s| *s != b'+');
            let last_new = signs.iter().rposition(|s| *s != b'-');
            for flags in 0..4 {
                let old_flag = flags & 1 != 0;
                let new_flag = flags & 2 != 0;
                if (last_old == last_new && old_flag != new_flag)
                    || (last_old.is_none() && old_flag)
                    || (last_new.is_none() && new_flag)
                {
                    continue;
                }
                let case = accepted + rejected;
                let mut patch = format!(
                    "diff --git a/case-{case} b/case-{case}\n--- a/case-{case}\n+++ b/case-{case}\n@@ -{},{} +{},{} @@\n",
                    usize::from(old > 0),
                    old,
                    usize::from(new > 0),
                    new
                );
                for (n, sign) in signs.iter().enumerate() {
                    patch.push_str(&format!("{}record-{n}\n", char::from(*sign)));
                    if (old_flag && last_old == Some(n)) || (new_flag && last_new == Some(n)) {
                        patch.push_str("\\ No newline at end of file\n");
                    }
                }
                if reference::Document::parse(patch.as_bytes()).is_err() {
                    ensure!(
                        storage::read(patch.as_bytes(), |_| Ok(())).is_err(),
                        "Parser acceptance changed for case {case}"
                    );
                    rejected += 1;
                } else {
                    input.push_str(&patch);
                    accepted += 1;
                }
            }
        }
    }
    let counts = compare(input.as_bytes(), input.as_bytes())?;
    Ok(
        serde_json::json!({"patterns": "all nontrivial context/deletion/addition sequences of length 1..6 and valid final-side flag combinations",
                         "accepted_cases": accepted, "rejected_by_both_parsers": rejected, "counts": counts, "status": "pass"}),
    )
}

fn main() -> Result<()> {
    for path in std::env::args().skip(1) {
        if path == "--patterns" {
            println!("{}", patterns()?);
            continue;
        }
        let counts = compare(File::open(&path)?, File::open(&path)?)?;
        println!(
            "{}",
            serde_json::json!({"path": path, "status": "pass", "counts": counts})
        );
    }
    Ok(())
}
