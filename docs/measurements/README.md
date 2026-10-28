# Measurements

This page summarizes the large-patch measurements. The linked reports have the full data,
method, and limits.

## Workload

Five patches pinned by SHA-256 in [corpus.json](../../fixtures/diffshub/corpus.json). They are
the cached inputs of Pierre's DiffsHub. The patch bodies are not in this repository.

| Patch | Size | Files | Hunks |
| --- | ---: | ---: | ---: |
| Pierre commit `0800fb` | 0.6 MB | 73 | 109 |
| Ghostty PR 12291 | 4.2 MB | 25 | 130 |
| Node PR 59805 | 21.5 MB | 3,420 | 19,198 |
| Bun PR 30412 | 43.3 MB | 2,188 | 2,129 |
| Linux `v6.0...v7.0` | 710.9 MB | 76,872 | 421,819 |

Source: [LARGE_DIFFS.md](../design/LARGE_DIFFS.md).

## Conditions

- Apple M5 Max, 48 GiB RAM, macOS 26.3, Rust 1.96.0, release build.
- The viewer runs in a real PTY at 160 x 50, split layout, dark theme. A harness parses the
  terminal output and compares every checked screen with independently computed cells.
- Warm file cache. Each patch gets five fresh processes for timing and one process that browses
  the same route three times for memory.
- Times are the moment the correct output reaches the PTY. They do not include terminal paint.

These numbers come from pre-release builds (September and October 2026). Release builds have
not been timed again.

## Results

Opening and navigation, medians in milliseconds, from the
[2026-09-23 campaign](../history/SIDEBAR_MEASUREMENT_REPORT.md):

| Patch | First text | First colors | Full index | Worst navigation p95 |
| --- | ---: | ---: | ---: | ---: |
| Pierre | 10.3 | 44.3 | 50.1 | 43.3 |
| Ghostty | 8.2 | 38.2 | 44.1 | 21.7 |
| Node | 8.5 | 33.1 | 144.1 | 14.5 |
| Bun | 8.5 | 34.4 | 200.6 | 19.9 |
| Linux | 11.8 | 35.5 | 4,650.5 | 8.6 |

"Full index" is when the whole patch has been indexed in the background. The first screen is
usable long before that.

Memory on the Linux patch, from the final 2026-10-03 cohort recorded in
[CURRENT_STATE.md](../history/CURRENT_STATE.md#current-integrated-candidate): 594 MiB peak RSS
and 543 MiB settled. The original prototype reached 2,590 MiB on the same patch before its
parser rejected it ([baseline report](../history/BASELINE_REPORT.md)).

The memory goals (128 MiB settled, 192 MiB peak) are still missed by Node, Bun, and Linux.

## Forked dependencies

Measured changes from the patches in `crates/`:

- Sharing compiled regex engines between identical patterns across grammars lowered live heap on
  a syntax-heavy trace from 275 MiB to 213 MiB
  ([report](../history/CROSS_GRAMMAR_REPORT.md)).
- Sharing identical patterns within one grammar lowered live heap by 11.4%
  ([report](../history/REGEX_MEMORY.md)).
- Skipping regex searches whose required bytes are absent from the line cut the median time to
  color a deep, first-visited file from 1,494 ms to 752 ms
  ([summary](../history/CURRENT_STATE.md#historical-performance-evidence-and-accepted-tradeoff)).

## Method documents

- [MEASUREMENT.md](MEASUREMENT.md): the content-grid timing protocol.
- [MEASUREMENT_POLICY.md](MEASUREMENT_POLICY.md): targets and how results are reported.
- [FIVE_PATCH_ACCEPTANCE.md](FIVE_PATCH_ACCEPTANCE.md): the five-patch correctness and timing plan.
- [SIDEBAR_CORPUS_PLAN.md](SIDEBAR_CORPUS_PLAN.md): the corpus plan with the sidebar enabled.
- [REGEX_CENSUS_PLAN.md](REGEX_CENSUS_PLAN.md): the regex ownership census.
- [TERMINAL_PRESENTATION_CHECK.md](TERMINAL_PRESENTATION_CHECK.md): the manual terminal check.

The scripts are in [`tools/`](../../tools/). They read the patch bodies and outputs from an
ignored `evidence/` directory.
