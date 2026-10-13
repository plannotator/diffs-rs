# Local-source coloring investigation

Date: 2026-09-22. This checkpoint diagnoses the delay in [the accepted short series](LOCAL_SOURCE_TIMING_REPORT.md).
It changes no runtime source or production executable.

## Findings

### P2: Hidden-source parsing dominates the measured coloring work

The two-file diagnostic parsed 23,926 source lines across both sides for its opening view.
Only 24 requested style records followed from that work. The hidden prefixes establish the correct parser state for the separated changes.
The hidden parser calls account for 96.11% of the opening request duration.
They account for 98.65% and 98.72% of the two theme-change requests.

| Two-file request | Parsed source lines, both sides | Request duration | Hidden parser calls | Requested parser calls | Visible color construction | Publication |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Opening | 23,926 | 539.56 ms | 518.56 ms | 14.31 ms | 0.15 ms | 0.02 ms |
| Light theme | 23,922 | 522.64 ms | 515.61 ms | 0.78 ms | 0.14 ms | 0.22 ms |
| Dark theme | 23,922 | 519.64 ms | 512.96 ms | 0.75 ms | 0.13 ms | 0.08 ms |

These are single instrumented requests, not accepted user-visible timings.
Initialization is nested inside some parser calls. It must not be added to the parser totals again.
Opening initialization took 20.74 ms in total, including 0.63 ms outside the parser-call timers.
Both theme requests initialized no grammar or regex engine. Repeated engine initialization does not explain their delay.
Unattributed request time remained about 4.4–4.6 ms for these three requests, after known initialization outside the parser.

The ordinary stack sample supports the same conclusion without application tracing.
The coloring thread spent its active samples in `ParseState::parse_line`, regex matching, grammar iteration, and related bookkeeping.
The flat sample summary includes 180 observations in `captures_from_pos_with_scratch`, 28 in grammar iteration, and 27 in `parse_line` itself.
These sample counts are not durations or additive inclusive percentages.
The sample also includes idle time. It does not prove the absence of brief main-thread work or input stalls.

The call path is `Reader::advance` in `src/highlight/full_source.rs:276`, followed by `Side::advance` in `src/highlight.rs:138`.
The parser must process hidden text. Skipping unknown prefixes or treating them as plain text would weaken correctness.
The evidence supports reducing repeated work, not removing necessary syntax interpretation.

### P2: The reader cannot reuse later checkpoints inside a sparse request

`Reader::prepare` consults saved state only before the earliest requested source line (`src/highlight/full_source.rs:230`).
`Reader::advance` then reads each later hidden gap sequentially. It does not look for a compatible checkpoint inside that gap.
An early requested line can therefore force replay through later text that the worker previously parsed.

Theme changes add another restriction. `Checkpoints::set_epoch` clears the shared checkpoint cache (`src/highlight/checkpoint.rs:117`).
The active state also includes the theme and request epoch. All three observed two-file requests consequently parsed nearly the same source prefixes.

Keeping checkpoints across themes alone is insufficient for this sparse view.
Its first requested line precedes the useful later checkpoints. Reuse also requires checkpoint lookup before later hidden gaps.
This is an optimization opportunity, not a reproduced rendering defect.

The [next implementation plan](LOCAL_SOURCE_REPLAY_PLAN.md) separates syntax compatibility from color identity.
It retains the old/new source distinction, immutable source identity, complete parser state, cancellation checks, and existing memory limits.

## Other observations

The multi-file diagnostic also passed all eleven views.
Its light and dark requests parsed 1,522 source lines each and spent 25.25 ms and 25.37 ms in hidden parser calls.
Those requests used about 26.73 ms and 26.99 ms overall.
The smaller replay workload is consistent with the smaller observed delay. It is not a controlled source-size scaling experiment.

One Git opening request was superseded as checked sources arrived. Its partial work remains in the evidence.
No output mismatch or source failure followed. No request or trace event was dropped.

This investigation did not find a reason to change terminal painting, channel capacity, cache budgets, grammar rules, or diff alignment.
The source acquisition path has no dedicated timing here. It is not fully attributed by subtraction from a different process or earlier median.

## Method and checks

The diagnostic build used unchanged source with only `delivery-trace` enabled, in a separate Cargo target directory.
It ran once per prepared case, with the same eleven public actions, two-second windows, dimensions, fixtures, and final expectations.
Raw PTY capture preceded offline screen checks. The patch-only analyzer remained unchanged and continued to reject full-source traces.
The new summary uses only supported request timers, with explicit unassigned time and nested-initialization checks.

All 22 diagnostic final views matched their frozen text, colors, styles, and status.
One ordinary two-file process then ran the same sequence with one one-second stack sample during the light-theme action.
All eleven sampled final views matched too. All three processes restored the terminal and removed their private spool files.
The diagnostic captures recorded AC at each boundary. The ordinary sample recorded AC before and after its sequence.

The six existing full-source worker checks passed with `delivery-trace` enabled.
They use actual files and the production worker, including independent source sides, sequential-color references, checkpoint eviction, source replacement, and fallback.
The existing vendored Syntect lifetime warning remains unchanged.
The full ordinary suite was not repeated because no runtime or test source changed.

Ordinary executable SHA-256:

`c3b337c3549efab30d7fee3f5089d4f9af733a143e28c091cbbd45a21b462074`

Preserved diagnostic executable SHA-256:

`2bf6070ce9f8e42e9355c86334f3b42ed9b744bac1a17ff8cece64c1c005a56d`

## Diagnostic boundaries

- Full-source summaries do not contain patch group counts, reset reasons, or predecessor record counts.
- The longest parser-call location names the requested target, not the exact hidden line under replay.
- Full-source emphasis, source reads, text conversion, cancellation locks, and checkpoint maintenance contribute to unassigned request time.
- Theme checkpoint disposal occurs before the request timer. Source restoration has no dedicated checkpoint event.
- Parser time includes matching, dispatch, allocation, and bookkeeping. It is not a pure regex timer.
- The snapshot stores parser and scope state, not a token stream for every hidden line.
- This diagnostic does not establish first-open improvement, sustained memory, scrolling smoothness, or physical terminal latency.

The independent review confirmed these accounting limits and the proposed checkpoint-reuse boundaries.
It also confirmed that cache eviction prevents a fixed upper bound on replay distance.

## Evidence

`evidence/local-source-coloring-v1/` contains the plan, diagnostic executable, source identities, build log, and six-check log.
Each case retains the raw trace, PTY receipts, capture outcome, and exact-screen check result.
`summary.json` retains every request, including the superseded request and plain-style requests.
`ordinary-sample/` contains the uninstrumented stack sample and its eleven checked views.

The accepted timing evidence and previous source evidence remain unchanged.
No additional instrumentation entered the application, and no speculative optimization was adopted.
