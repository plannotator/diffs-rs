# Saved-state reuse: completed local comparison

Date: 2026-09-22. The declared ordinary comparison is complete.
This checkpoint changes no application source, dependency, cache allowance, or ordinary executable.

## Outcome

All eighteen ordinary runs passed **198 exact final views** and all terminal and spool cleanup checks.
Offline replay reproduced every recorded text and complete-cell milestone. The RSS sample boundaries and saved medians also passed their checks.
The series recorded 216 AC checks. All ten observations from the preceding session remain in the result.

The benefit depends on checkpoint retention. The real Rust source case completed theme changes faster in all three pairs.
The 12,000-line case still parses its full hidden prefixes during theme changes. Its lower observed times do not establish a parsing improvement.
Opening, navigation, and expansion observations are mixed. Some actions were consistently slower in this small series.

The implementation retains its correctness evidence. This comparison does **not** establish a general speedup or complete performance qualification.
The next investigation concerns checkpoint retention under pressure and the adverse navigation observations.

## Method and interruption

The series uses three cases: two TypeScript files with 12,000 lines per side, twelve fixed-commit Git files, and a 2,204-line Rust source file.
Each case has three before/after pairs. The middle pair reverses executable order.
All processes use the original frozen views, public actions, terminal dimensions, and two-second action windows.
No expensive screen checks run during capture. The process exits before offline replay starts.

An earlier AC-to-battery transition stopped the eleventh attempt after three actions.
That partial attempt remains untouched in the original evidence packet and contributes no completed sample.
This session resumed the eight unfinished entries in their original order. It did not replace any completed observation.

The resumed captures occupy a separate directory. All 143 files in the original packet passed checksums before and after resumption.
The preserved source-readiness packet also passed its checksums. Both ordinary executable hashes remain unchanged.

The host remained a shared workstation, with different background activity across the two capture sessions.
AC at action boundaries does not establish a continuously isolated host or equal scheduling conditions.
The summary therefore retains every trial, its capture session, and its before/after pair.
The reported medians are descriptive observations, not confidence intervals or causal speedup estimates.

## Presentation timing

Each cell contains the median and full range in milliseconds: `median [minimum–maximum]`.
Each role has three samples. “Complete cells” includes the entire expected text, colors, styles, dimensions, title, and footer.
The endpoint is receipt of terminal bytes, not physical paint or perceived smoothness.

| Case and action | Before: complete cells, ms | After: complete cells, ms |
| --- | ---: | ---: |
| Two files: open | 770.33 [746.23–825.38] | 778.87 [697.04–1145.24] |
| Two files: light theme | 789.96 [521.32–1110.29] | 545.41 [512.44–588.53] |
| Two files: dark theme | 671.82 [531.30–974.73] | 570.33 [529.27–653.88] |
| Fixed-commit Git: open | 344.84 [294.57–362.95] | 364.57 [323.82–370.54] |
| Fixed-commit Git: light theme | 29.57 [27.05–35.22] | 27.50 [19.40–29.51] |
| Fixed-commit Git: dark theme | 28.36 [28.22–34.15] | 28.15 [19.80–31.08] |
| Real Rust source: open | 208.59 [197.64–238.84] | 203.57 [170.90–301.58] |
| Real Rust source: light theme | 53.47 [50.54–68.10] | 20.95 [20.44–31.48] |
| Real Rust source: dark theme | 54.58 [52.89–63.65] | 20.58 [20.03–27.23] |

The real-source theme result is consistent across all three pairs. Its cold-opening result is mixed.
The final two-file pair is nearly unchanged for theme completion: 521.32→512.44 ms and 531.30→529.27 ms.
The diagnostic work counts explain why the larger aggregate timing difference needs caution.

Adverse observations remain part of the result:

| Case and action | Before median, ms | After median, ms | Paired direction |
| --- | ---: | ---: | --- |
| Git: expand context | 2.33 | 3.20 | After slower in all three pairs |
| Git: last file and source | 64.60 | 82.04 | After slower in all three pairs |
| Rust: actual page scroll, complete cells | 28.83 | 37.78 | Mixed |
| Rust: expand context | 17.17 | 20.26 | After slower in all three pairs |
| Rust: unified layout | 4.61 | 9.29 | After slower in all three pairs |
| Rust: widen terminal | 6.63 | 10.48 | After slower in all three pairs |

These differences are observations, not an established attribution to the implementation.
The current trace does not fully attribute source acquisition, expansion, or main-thread layout work.
The complete summary retains text milestones, all action samples, process CPU time, exit observations, and pair differences.

Only the real-source `page-scroll` action establishes actual viewport movement.
The preceding page action and the prepared cases' page actions change selection without scrolling the viewport.
Width changes occur after prior work settles. This sequence does not qualify rapid resizing during active coloring.

## Diagnostic work counts

A separate `delivery-trace` build ran each case once after the ordinary sequence.
All **33 diagnostic views** matched the same expectations. All three processes passed cleanup checks.
No trace records were dropped. No requested style used syntax fallback.
Superseded requests during source readiness remain in the raw evidence.
The diagnostic durations and memory values are not accepted ordinary measurements.

| Case | Earlier theme request: parsed lines | Current theme request: parsed lines | Current cache observation |
| --- | ---: | ---: | --- |
| Two 12,000-line files | 23,922 | 23,922 | 64-entry limit reached, 212 evictions over the sequence |
| Fixed-commit Git | 1,522 | 998 | Maximum 14 entries, no eviction |
| Real Rust source | No earlier diagnostic for this fixture | 890 | Maximum 14 entries, no eviction |

Both theme transitions have the same count within each row. Counts include old and new source lines.
The earlier counts come from the preserved [coloring investigation](LOCAL_SOURCE_COLORING_REPORT.md), with the same prepared inputs and public actions.

The two-file diagnostic captured 276 checkpoints and evicted 212. Its maximum accounted checkpoint storage was 31,376 bytes.
It reached the entry-count limit, not the 16 MiB byte allowance. No checkpoint was rejected for size.
The opening and theme requests each capture 92 checkpoints, while only 64 entries survive.
The FIFO policy evicts older entries as replay advances. The subsequent sparse requests obtain no reduction in parsed lines.

This identifies a retention problem to investigate. It does not justify increasing limits, skipping source, or selecting a policy for one fixture.
The earlier 6,000-line correctness fixture still proves reuse when useful checkpoints survive: 11,266→2,056 parsed lines with exact tokens.
That result is not a guarantee for inputs beyond the cache's retained coverage.

## Memory observations

| Small local workload | Before median OS peak RSS | After median OS peak RSS |
| --- | ---: | ---: |
| Two files | 29.05 MiB | 29.00 MiB |
| Fixed-commit Git | 33.45 MiB | 33.95 MiB |
| Real Rust source | 16.70 MiB | 16.77 MiB |

All eighteen saved RSS series passed sample-containment, median, peak, and sampler-shutdown checks.
These are short workloads, not the large Linux patch or the five-patch sustained-memory campaign.
The small differences do not establish a leak, a plateau, or a total-memory limit.
The larger current-build memory qualification remains open.

## Identity and evidence

The ordinary application remains `target/release/native-diff`, SHA-256 `3f87c079518ee654053658a7015e56319a5c377a05ce568f45a767b456be5a15`.
The reference remains SHA-256 `c3b337c3549efab30d7fee3f5089d4f9af733a143e28c091cbbd45a21b462074`.
All 2,106 checked runtime, test, vendor, and manifest files match the preceding source archive.
No application rebuild replaced the ordinary executable. The attribution build used a separate Cargo target directory.

- [Complete raw-value summary](evidence/local-source-replay-completion-v1/summary.json)
- [Diagnostic counts](evidence/local-source-replay-completion-v1/diagnostic-counts.json)
- [Resumption declaration](evidence/local-source-replay-completion-v1/resumption.json)
- [Original implementation report](LOCAL_SOURCE_REPLAY_REPORT.md)
- [Next investigation](LOCAL_SOURCE_RETENTION_PLAN.md)

The existing correctness suite remains the preceding 115 ordinary checks and 133 combined-feature checks.
This turn repeated actual terminal views and saved-output checks, not the unchanged full source suite.
Physical terminal inspection, remote acquisition, and release qualification remain separate.
