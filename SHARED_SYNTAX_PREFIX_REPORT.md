# Shared leading-syntax checkpoint

Date: 2026-09-15. Status: evidence collection complete. Promotion deferred. The candidate is not adopted.

## Change and scope

The candidate parses identical leading old/new history once, with one complete syntax state.
Before the first side-only record, it clones that complete state into independent old/new owners.
This separation precedes parsing and long-line invalidation.
The worker does not join divergent forward histories.
Backward restoration can return to an earlier shared checkpoint before that divergence.

Checkpoints preserve shared or separate ownership and count their actual retained storage.
They keep the existing intervals, entry limits, byte limits, and lazy color reconstruction.
Smaller shared entries can change which checkpoints fit those same byte limits.
This can change reuse and retained storage. It is not an RSS guarantee.
The change preserves source bytes, line pairing, word emphasis, navigation, grammar data, fallback thresholds, and publication guards.
It adds no dependency, unsafe code, global pool, or fixture-specific branch.

Two normal Rust modules changed: `src/highlight.rs` and `src/highlight/checkpoint.rs`.
The optional diagnostic module, `src/delivery_trace.rs`, changed its parser-side accounting.
Dependency implementations are unchanged.

The [preceding investigation](evidence/cold-search-review-v1/REPORT.md) records the sample and structural eligibility, not a predicted speedup.
Long leading context strongly favors the deep fixture.
Leading prefixes account for only 0.14–9.82% of possible two-side record visits across the five larger patches.
These input-wide counts are not measured-path coverage.

## Correctness qualification

Both new worker contracts passed against the unchanged implementation before production edits.
They compare exact tokens with independent sequential highlighting through captured Ruby delimiters, checkpoint boundaries, divergence, and backward revisits.
They also cover old-only and new-only oversized lines after shared TypeScript history.
A real-grammar checkpoint check covers single-owner storage and complete state restoration.

Formatting and application Clippy passed.
The complete release suite passed, including 38 process contracts and the separate 34-input alignment contract.
The two existing opt-in process exclusions remain unchanged.
The process suite includes exact final colors after pending-highlight resizing and theme changes.
All 20,847,451 supplied source records, complete five-patch projections, and ten pre-change sequential-token comparisons matched.
The [qualification record](evidence/shared-syntax-prefix-v1/checks/report.json) retains commands, source identities, and results.

All five diagnostic captures passed exact output checks.
Schema 4 reports one actual parser call when that call serves both histories.
The deep first-end request processed 11,955 records with 11,955 calls, including 11,954 shared calls.
This count demonstrates avoided duplication, not a timing or memory result.

An initial corrupted-trace control failed because the checker rejected an unknown side through an earlier accounting error.
The original failure and checker source remain intact.
Only validation order changed. No replacement viewer ran.
Offline replay passed all 39 saved diagnostic windows.
The corrected reader passed 20 current-schema controls, 15 schema-3 controls, and six schema-2 controls.

## Ordinary focused observations

Three alternating reference/candidate pairs completed all five cases and 234 action windows.
The [focused audit](evidence/shared-syntax-prefix-v1/focused-audit.json) replayed all 30 captures and rederived every per-action value and median.
The medians use all three original observations per build.
They measure receipt of terminal output, not physical terminal paint.

| Deep-case action | Reference colors | Candidate colors |
| --- | ---: | ---: |
| First end jump | 1,522.15 ms | 770.29 ms |
| Backward visit | 25.42 ms | 13.78 ms |
| Theme change | 1,527.47 ms | 794.75 ms |
| Uncached forward visit | 9.62 ms | 7.71 ms |

Each action colored sooner in all three pairs.
First-end text medians remained 0.99 and 1.00 ms.
The observed first-end coloring reduction is about 0.75 seconds for this long-prefix case.
It does not establish a general speedup or a latency distribution.

The smaller cases were mixed.
Guard light-theme readiness medians increased from 93.28 to 102.42 ms, although the candidate was faster in two individual pairs.
Ruby `light-back-3` coloring medians increased from 6.62 to 8.93 ms, with slower candidate values in all three pairs.
All 39 actions and every original value remain in the [focused report](evidence/shared-syntax-prefix-v1/focused/report.json).

One candidate opening received source text at 156.08 ms and colors at 181.10 ms.
Its first loading frame arrived at 5.06 ms.
The [startup review](evidence/shared-syntax-prefix-v1/STARTUP_REVIEW.md) retains that interval and the fixed follow-up scope.
The cause remains unknown. No result is discarded or pooled with a follow-up.
The four follow-up processes passed 20 action windows and saved-stream replay.
Their exact opening text arrived in 7.40–8.61 ms and colors in 32.90–43.44 ms.
The original delay did not recur. This follow-up does not establish tail equivalence.

## Sustained memory

All ten original processes completed three tours with settled holds, revisits, layout changes, and resizing.
The [memory audit](evidence/shared-syntax-prefix-v1/memory-bounded-audit.json) checked 4,324 views, 2,856 holds, 49,067 RSS queries, and absent process groups.
This audit checks recorded view order and raw RSS accounting. It does not replay memory-run terminal bytes offline.

Low-battery hibernation interrupted the Bun reference process, but not its completed candidate partner.
The resumed process passed its views. Its interrupted memory comparison cannot support acceptance.
The [incident record](evidence/shared-syntax-prefix-v1/HOST_INTERRUPTION.md) preserves the power-log evidence and the original result identity.
The other original pairs completed outside that interruption:

| Patch | Reference peak RSS | Candidate peak RSS | Reference third-tour maximum | Candidate third-tour maximum |
| --- | ---: | ---: | ---: | ---: |
| Pierre | 71.14 MiB | 70.61 MiB | 71.11 MiB | 70.58 MiB |
| Ghostty | 59.00 MiB | 60.08 MiB | 58.97 MiB | 60.05 MiB |
| Node | 135.92 MiB | 135.88 MiB | 135.92 MiB | 135.88 MiB |
| Largest patch | 751.27 MiB | 747.48 MiB | 751.27 MiB | 559.91 MiB |

Peak RSS covers each process lifetime. Third-tour maxima use the settled holds.
These are shared-host observations from one process per build and patch, not causal allocation measurements.
Largest-patch tour maxima were 746.86, 750.92, and 751.27 MiB for the reference.
Candidate maxima were 699.98, 622.34, and 559.91 MiB.
The candidate's resident-memory decline is not partitioned into allocator and OS effects or attributed to shared syntax state.
Both builds still missed the memory goal on Node and the largest patch.

The interrupted Bun reference peaked at 160.63 MiB and ended with a third-tour maximum of 96.03 MiB.
Its completed candidate partner peaked and ended at 167.48 MiB.
The reference's apparent settled-memory target pass is not accepted.
A separately declared fresh Bun pair completed the unchanged workload with additional power, clock, and sleep-event checks.
Its audit passed 830 views, 528 holds, 6,905 RSS queries, and process cleanup.
Both processes used AC power and had no recorded sleep event.
Its results remain separate from the interrupted pair:

| Build | Peak RSS | First-tour maximum | Second-tour maximum | Third-tour maximum |
| --- | ---: | ---: | ---: | ---: |
| Reference | 158.80 MiB | 140.91 MiB | 158.61 MiB | 125.83 MiB |
| Candidate | 167.72 MiB | 141.48 MiB | 162.81 MiB | 167.72 MiB |

The adverse peak difference is 8.92 MiB. The third-tour difference is 41.89 MiB.
The reference RSS decreased without hibernation. Its cause remains unpartitioned.
This result triggered the [matched allocation investigation](evidence/shared-syntax-prefix-v1/MEMORY_OWNER_PLAN.md).
The investigation uses both preserved builds and the same Bun trace.
Its diagnostic allocation snapshots cannot replace these ordinary RSS observations.

The matched profiles completed 830 expected views and eight allocation snapshots.
Every heap class byte count and allocation count reconciled to the native totals.
Candidate live heap was slightly lower at all four boundaries, ending at 152.62 MiB versus 153.11 MiB.
The profile did not reproduce a total live-heap increase.
Native VM reports showed substantial swapped pages in the candidate profile, unlike the reference.
This demonstrates different residency during the profiles, not the cause of the earlier ordinary RSS gap.
The [allocation report](evidence/shared-syntax-prefix-v1/MEMORY_OWNER_REPORT.md) retains the full distinctions and limits.

The memory trace's timing observations are setup milestones, not a dedicated latency distribution.
Largest-patch `long-last` maxima were 310.23 ms for the reference and 304.20 ms for the candidate.
Index-ready upper bounds were 9,614.49 and 3,729.60 ms. No indexing code changed, so these do not establish a loading improvement.
Largest-patch quit observations were 50.97 and 122.84 ms. Both met the unchanged quit target, but this is not quit-time equivalence.
Opening observations across the other patches were also mixed.

## Closeout

All declared qualification, recovery, startup, and allocation work is complete.
The [adoption review](evidence/shared-syntax-prefix-v1/ADOPTION_REVIEW.md) defers promotion without rejecting the candidate as defective.
The profile narrows the concern but does not clear the ordinary RSS question under the frozen runbook.
No final normal-build promotion ran. The default executable still contains the prior kept change.
The working production source contains the experimental candidate. Its source and executable remain separately preserved.
A new default Cargo build would compile that experimental source.

Any next investigation must address ordinary-workload resident-memory attribution under a separate declaration.
No result here justifies speculative parser changes or cache tuning.

All earlier adverse memory observations remain separate and unresolved.
Fewer parser calls and smaller checkpoint ownership do not guarantee lower RSS.
Physical Ghostty/Kitty presentation, frame pacing, and release performance acceptance remain open.
