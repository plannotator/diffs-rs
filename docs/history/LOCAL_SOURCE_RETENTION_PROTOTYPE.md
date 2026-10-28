# Request-aware checkpoint admission

Date: 2026-09-23. Status: prototype complete, not adopted. The known baseline is restored.

This page preserves the original prototype checkpoint. The later [retention decision](LOCAL_SOURCE_RETENTION_DECISION.md) records adoption and current qualification.

## Decision

The prototype reduces repeated coloring work and passes correctness checks.
Its large-source recoloring observations improve in every pair.
However, Git and Rust-source opening are slower in every pair.
Those adverse observations need attribution before adoption.

The three runtime files and the default executable were restored byte-for-byte.
The new regression tests remain with the candidate, not in the restored baseline suite.
The candidate source, executable, checks, and every observation remain preserved.
This decision does not establish that the policy caused the slower opening.

## Implementation

The candidate coloring worker marks the nearest compatible checkpoint before each requested source coordinate.
Those existing checkpoints stay protected for the current complete-source file job.
Unprotected entries keep FIFO eviction order.
Admission first checks whether enough unprotected storage is available.
If admission cannot fit, it leaves existing entries intact and continues correct parsing without another checkpoint.

A borrowed scope clears every protection flag when the file job ends.
Rust also drops that scope on cancellation, errors, and unwinding.
The scope allocates no storage and copies no parser state.
Its lifetime cannot outlive the borrowed cache.

The entry limit remains 64. The aggregate allowance remains 16 MiB, and the per-entry allowance remains 256 KiB.
The added flags occupy 512 bytes across the preallocated entry slots on this target.
Existing accounting includes those bytes. No allowance increased.

The change uses immutable source identity and requested source coordinates.
It does not inspect filenames, languages, fixture identities, expected timings, or hardware targets.
Parsing, source bytes, syntax restoration, diff pairing, layouts, navigation, and public controls remain unchanged.
No dependency or worker was added.

Protection applies to the current file job, not every file in a multi-file request.
It does not prevent unrelated files from displacing one another between jobs.
It can retain older coverage instead of admitting new coverage, so it does not guarantee faster future navigation.
Protection is conservative. A marked entry can become unnecessary as live parsing advances.
Cold opening still parses unknown source prefixes.

## Correctness and review

The integrated ordinary suite passed 117 checks, including all 61 enabled real-process contracts.
The integrated delivery-trace and owner-census suite passed 136 checks.
Four opt-in contracts remain excluded from each suite.
Formatting and application Clippy passed. The existing vendored Syntect lifetime warning remains unchanged.

New real-worker checks request divergent old/new syntax states under checkpoint pressure.
They compare exact tokens with independent sequential highlighting across revisits and both themes.
Another check fills the reply queue, cancels a protected job, and checks exact colors from a replacement source.
A weak reference proves that the canceled job releases its previous complete source.

The work-reuse regression fails against unchanged FIFO source with the expected repeated-prefix assertion.
The same regression passes after the change.
The existing real-parser cache test now also covers fully protected capacity, declined admission, cancellation, and subsequent release.
Its oversized-parser rejection and exact-restoration checks still pass.

The six diagnostic patterns passed 30 measured requests and 612 old/new token-list comparisons per implementation.
They cover small sparse views, larger sparse views, dense views, backward travel, alternating source origins, and unequal side lengths.
The diagnostic terminal run passed 33 exact final grids and cleanup checks.
No diagnostic events or request summaries were dropped.

The self-review covered ownership, admission preflight, cancellation, bounds, and theme/source compatibility.
The proposed production change affects three runtime files and adds one integration-test file.
Richer schema-4 attribution and the configurable experiment probe remain outside production source.

## Work counts, not timings

| Pattern | FIFO parsed lines by request | Protected admission parsed lines by request |
|---|---|---|
| Sparse within capacity | 11,266; 10; 10; 10 | 11,266; 10; 10; 10 |
| Sparse beyond capacity | 23,810; 23,810; 23,810; 23,810 | 23,810; 7,436; 274; 274 |
| Dense final view | 23,838; 286; 286; 286 | 23,838; 286; 286; 286 |
| Backward travel | 23,810; 2; 2; 6,914; 2; 2; 1,538; 2; 2 | unchanged |
| Alternating source origins | 23,810; 23,810; 23,810; 23,810; 23,810 | 23,810; 23,810; 23,810; 7,436; 274 |
| Unequal side lengths | 24,323; 24,323; 24,323; 24,323 | 24,323; 7,954; 281; 281 |

The sparse sequences open, revisit, switch to light, then return to dark.
The alternating-source sequence inserts another source before revisiting the first.
The final flush request is excluded from every sequence.

In the frozen 12,000-line terminal case, opening still parses 23,926 lines.
Its two theme changes parse 6,591 and 806 lines, compared with 23,922 each in the preceding trace.
The smaller Git and Rust-source theme requests retain their previous parsing counts.
Intermediate source-readiness scheduling remains variable.

These results establish less repeated parser work in the affected patterns, not an equivalent percentage reduction in elapsed time.

## Ordinary comparison

The exact candidate executable was measured through eighteen serial processes: three workloads and three pairs per workload.
The middle trial reversed executable order. All runs stayed on AC power.
Each process used the same frozen 160 by 50 protocol and independent final-screen expectations.
All 198 final grids matched. Offline raw replay reproduced every saved text and complete-cell milestone.
RSS sample containment, saved medians, sampler shutdown, terminal restoration, and scratch cleanup passed.

These are median complete-cell receipt times in milliseconds, not physical terminal paint times:

| Workload and action | Baseline | Candidate | Paired direction |
|---|---:|---:|---|
| 12,000-line source opening | 729.89 | 675.79 | Mixed, including one candidate observation 435 ms slower |
| First recoloring of that source | 530.47 | 142.16 | Candidate faster in all three pairs |
| Following recoloring | 545.83 | 16.46 | Candidate faster in all three pairs |
| Git opening | 272.06 | 334.91 | Candidate slower in all three pairs |
| Git endpoint | 78.00 | 79.34 | Mixed |
| Rust-source opening | 179.66 | 220.24 | Candidate slower in all three pairs |
| Rust-source actual page scroll | 39.99 | 41.44 | Mixed |
| Rust-source expansion sequence | 26.00 | 25.74 | Mixed |
| Rust-source unified layout | 5.00 | 8.07 | Candidate slower in all three pairs |
| Rust-source narrow resize | 6.36 | 9.33 | Candidate slower in all three pairs |
| Rust-source wide resize | 7.79 | 10.91 | Mixed |

Git opening is slower by 24.25 to 62.86 ms within pairs.
Rust-source opening is slower by 7.53 to 40.58 ms within pairs.
Git light recoloring and both Rust-source recoloring actions are also slower in all three pairs.
Their parser-work counts did not improve in the separate diagnostic.
The full per-step observations remain in `ordinary-summary.json`.

Median OS peak RSS, in MiB, is 29.00 to 28.97 for the deep source case.
It is 33.23 to 33.22 for Git, and 16.61 to 16.83 for the Rust source.
The Rust-source increase is retained in the report. These small, short workloads do not qualify sustained memory or the largest patch.

This is a shared-workstation comparison with three pairs, not an isolated causal estimate.
Cold opening still parses the same unknown prefixes.
Final expected text is not always the first readable response, especially while Git source preparation changes gap presentation.
The expansion action still includes `g`, `k`, `k`, and Enter.
These limitations do not justify discarding adverse observations.

## Next checkpoint

The next diagnostic must separate input/source preparation, first frame delivery, and coloring during opening.
It must also measure checkpoint planning directly, with equivalent instrumentation on baseline and candidate.
Existing parser counts alone cannot explain the opening difference.
The restored build reproduces the exact baseline hash, so the current toolchain can reproduce that baseline.

The implementation remains a candidate until its tradeoffs are understood.
Another ordinary capture requires a declared diagnostic question, not a search for better results.

The source-aware sustained-memory campaign and the existing five-patch campaign remain separate.
Physical terminal inspection remains separate from terminal-cell evidence.
Isolated expansion, cached layout costs, and cold versus warm Git acquisition still need narrower attribution.
This checkpoint does not establish a total-memory bound or complete production readiness.

## Provenance

The evidence packet is `evidence/local-source-retention-prototype-v1/` in the project directory.
It includes both executables, complete ordinary and diagnostic candidate source archives, raw captures, checks, and comparison declarations.
The ordinary candidate archive excludes the experimental attribution code.
The evidence scripts retain their original machine-local paths; they are experiment records, not release commands.

The previous ordinary executable is `3f87c079518ee654053658a7015e56319a5c377a05ce568f45a767b456be5a15`.
The measured candidate executable is `f4596aaab48414391999c48bc499df2a798837badcb5e142f96addba88a21508`.
The restored default executable again has the exact baseline hash.

The first expanded fixture reused identical body lines, so Git aligned some ranges differently from the fixture's assumed coordinates.
The unchanged source-correspondence check rejected that fixture before coloring.
The corrected fixture gives source lines distinct comments and passes on both implementations.
The failed preparation and its logs remain preserved and excluded from results.

No historical packet, public repository, global installation, or external project was changed.
