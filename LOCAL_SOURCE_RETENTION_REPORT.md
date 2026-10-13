# Checkpoint retention investigation

Date: 2026-09-23. Status: investigation complete. No production policy change.

## Decision

The large sparse source view loses useful saved parser states during replay.
This is a cache-admission problem, not evidence that the process needs a larger memory allowance.
The next experiment will preserve reusable states for the current request within the existing cache limits.
That experiment is not implemented or qualified.

The navigation observations need a separate diagnostic partition.
They do not establish a regression in rendering, expansion, or acquisition.

The ordinary executable remains unchanged:
`3f87c079518ee654053658a7015e56319a5c377a05ce568f45a767b456be5a15`.
All 2,148 archived source, test, vendor, example, tool, and manifest files matched the product files.
The diagnostic changes stayed in an isolated copy.

## P2: replay evicts states that it can reuse later

The cache stores exact parser states every 256 lines.
It admits at most 64 entries, 16 MiB of accounted storage, and 256 KiB per entry.
Admission removes the oldest entries first.
These limits and that policy were unchanged during this investigation.

New diagnostic events identify source origin, old/new side, saved position, lookup target, admission, restoration, and eviction.
The existing worker-start events identify request boundaries in these single-worker traces.
The event buffer and request buffer remain bounded. Neither buffer lost records.
The diagnostic schema is version 4. Normal builds exclude this instrumentation.

An independent replay of each trace reconstructed the retained set.
Every recorded lookup matched the nearest compatible entry in that set.
The checks require one summary for each measured request.
The extra worker request that flushes the preceding summary is explicitly excluded from measured results.

In the 12,000-line terminal case, opening admits 92 checkpoints and evicts 28.
Each theme change then admits another 92 checkpoints and evicts 92.
Each theme change parses 23,922 source lines and restores no saved checkpoint.
The maximum accounted checkpoint storage is 31,376 bytes. This is not total process memory.

The saved state after line 11,776 exists for both sides when each theme request starts.
Earlier replay evicts both states before the corresponding lookup near line 11,957.
At that lookup, the nearest retained position is only 5,888.
The repeated parsing and absent restorations confirm the loss of reuse in this request.

The trace also records three lookups per theme request whose request-start coverage disappeared before use.
This count is not an exact estimate of avoidable parsing.
A live continuation can sometimes be farther ahead than a saved state.

Evidence: `evidence/local-source-retention-v1/terminal-counts.json` and `terminal/deep-files/trace.json` within that packet.
Relevant source: `src/highlight/checkpoint.rs`, `src/highlight/full_source.rs`.

## Different request patterns behave differently

The probes use the real source capture, source correspondence check, syntax worker, and bundled grammar.
Independent sequential highlighting supplies the expected old/new token lists.
Numbers below count parsed source lines, not milliseconds.

| Request pattern | Opening | Later requests, in order |
|---|---:|---|
| Sparse, within entry capacity | 11,266 | 8, 8, 8 |
| Sparse, beyond entry capacity | 23,810 | 23,810, 23,810, 23,810 |
| Dense final view, beyond entry capacity | 23,838 | 286, 286, 286 |
| Backward travel through six destinations | 23,810 | 2, 5,634, 2, 2, 258 |
| Two source origins, then revisits | 23,810 | 23,810, 23,810, 23,810, 23,810 |
| Unequal old/new lengths | 24,323 | 24,323, 24,323, 24,323 |

The first, second, third, and sixth probes revisit in dark mode, switch to light mode, then return to dark mode.
The two-origin probe inserts another source request before that sequence.
It uses distinct captured sources, not simultaneous files in one review.
The separate Git terminal case covers an actual multi-file review.

These probes passed 27 measured requests, 260 source records, and 520 old/new token-list comparisons.
Their requested destinations do not cover the fixture's divergent old/new syntax interval.
They establish retention behavior and exact tokens at those destinations, not complete pressure-case syntax qualification.

The existing richer suite separately passed nine source-color checks.
These checks include divergent hidden syntax, multiple languages, theme changes, source replacement, expanded context, and queued stale styles.
Two existing checkpoint checks passed exact restoration and real oversized-parser rejection.
Independent saturation of the aggregate byte limit was not exercised.

## P2: navigation measurements combine different operations

A read-only review separated three paths in the preceding diagnostic evidence.
The ordinary before/after source archives contain identical navigation, layout, rendering, and source-acquisition implementations.
Indirect cache, allocation, or scheduling effects remain possible.

### Context expansion

The frozen action sends four keys: `g`, `k`, `k`, then Enter.
The application renders after each input event.
The recorded action includes navigation and intermediate frames, not only expansion.

In the preceding real-source diagnostic, about 20 ms precedes the final expansion coloring request.
That request parses 64 source lines and supplies 20 style replies in about 1.3 ms.
These instrumented observations do not qualify ordinary latency.

Expansion also rebuilds compact projection segments and preserves the reading anchor.
Its isolated cost remains unmeasured.
Relevant source: `src/main.rs:448`, `src/main.rs:503`, `src/view.rs:490`, `src/app.rs:400`.

### Layout and width changes

The preceding real-source unified, narrow, and wide windows start no coloring request.
Visible styles are already cached.
Checkpoint replay cannot directly explain those windows.

The remaining path includes event handling, anchor-preserving reflow, rendering, terminal output, and process scheduling.
Existing events do not separate those costs.
Relevant source: `src/app.rs:170`, `src/app/sidebar.rs:173`, `src/main.rs:448`.

### Git endpoint

The first endpoint visit acquires source bytes for newly visible files.
The preceding trace contains plain-source replies, a superseded complete-source request, and a completed replacement request.

Its first visible response arrives near 1.15 ms with a source-preparation status.
Final expected text arrives near 27.52 ms. Complete expected cells arrive near 65.57 ms.
The existing `text_ms` checks final expected interior rows, not the first readable response.
Source readiness can change gap presentation, so those milestones differ.

The new diagnostic has different intermediate completion outcomes at this endpoint, while all final screens still match.
This variation is another reason not to infer a regression from one diagnostic process.

Git child completion uses a 10 ms polling interval.
That interval can delay detection, but the existing traces do not measure its contribution.
It was unchanged between the ordinary builds.
Relevant source: `src/main.rs:472`, `src/input/worker.rs:61`, `src/input.rs:330`, `src/input/git.rs:123`.

## Checks and evidence

All three frozen terminal protocols passed: 33 exact final screens, clean exits, restored terminal modes, and empty application scratch directories.
Raw PTY capture stayed separate from expensive screen checks.
These were diagnostic captures, not accepted timing, RSS, sustained-memory, or physical-terminal measurements.

Formatting and application Clippy passed for the diagnostic library and source-color tests.
The existing vendored Syntect lifetime warning remains unchanged.
A read-only reviewer checked instrumentation, retained-set reconstruction, and claim limits.
The review led to explicit measured-request completeness checks and sentinel exclusion.

The packet preserves unsuccessful preparation attempts:

- The source-only archive omitted embedded theme assets. The isolated build copied the unchanged product assets before retrying.
- The first worker-directory check rejected Apple's `xcrun_db` helper cache. The revised check records it separately from application temporary files.
- The sandbox blocked the first terminal attempt's process-stat query. The capture reran with approved process-stat access.

These attempts are not accepted measurements and remain separate from successful results.

Packet: `evidence/local-source-retention-v1/`.
Main checks: `checked-evidence.json`, `worker-counts.json`, `terminal-counts.json`, and the test logs.
The packet includes the instrumented source archive, changed files, diagnostic binaries, driver scripts, raw traces, raw terminal receipts, and SHA-256 manifest.
The scripts and launchers retain machine-local provenance paths. This packet is evidence, not an installable release.

The [retention plan](LOCAL_SOURCE_RETENTION_PLAN.md) owns the next experiment and its acceptance gates.
