# Line-local search-buffer checkpoint

Date: 2026-09-15. Status: complete and kept as a bounded first-visit highlighting optimization.

## Change

The implementation reuses three temporary regex VM buffers within one parsed source line.
Each search resets its logical state. The line releases the buffers when parsing returns.
Returned captures remain independently owned.
The change does not alter grammar data, source traversal, matching limits, cache budgets, or rendering rules.
It adds no dependency, unsafe code, global pool, or fixture-specific branch.

Four Rust dependency files changed, with two corresponding provenance descriptions.
The application source is unchanged.
Both ordinary executables and complete build inputs are preserved in `evidence/line-search-scratch-v1`.

## Completed checks

The [qualification record](../../evidence/line-search-scratch-v1/QUALIFICATION.md) records the complete command results and failed setup attempts.
The process suite passed 38 contracts, including exact final colors after pending-highlight resizing.
The independent source check matched all 20,847,451 supplied records across the five patches.
Complete source/projection checks and ten pre-change sequential-token comparisons passed.
The actual regex, dependency, concurrency, capture-ownership, and cleanup checks passed.

Three alternating ordinary pairs completed 234 action windows.
Every final source/color view matched its frozen expectations, and saved terminal packets replayed successfully.
All observations remain in the [focused report](../../evidence/line-search-scratch-v1/focused/report.json).

## Focused observations

The following medians use all three original observations per build.
They measure terminal-output receipt, not physical terminal paint.

| Action | Reference colors | Candidate colors |
| --- | ---: | ---: |
| Deep first jump | 1,734.07 ms | 1,543.21 ms |
| Deep backward visit | 29.92 ms | 26.43 ms |
| Deep theme change | 1,689.87 ms | 1,557.99 ms |

Each of these actions colored sooner in all three candidate pairs.
The first-jump reduction is about 0.19 seconds, or 11% of the reference median.
Its text medians were 1.10 and 1.06 ms.
Smaller-action observations were mixed; this is not a general speedup or a latency distribution.

One candidate guard startup took 219.69 ms for text and 284.72 ms for colors.
Two separately declared follow-up pairs passed all 28 windows and did not repeat that delay.
The [startup review](../../evidence/line-search-scratch-v1/STARTUP_REVIEW.md) narrows the interval but does not establish its cause.
The original result is retained, not excluded or pooled with those follow-ups.

## Sustained memory and broader reading checks

All ten ordinary viewers completed the unchanged checked traces, alternating order by patch.
Each made three tours with settled holds, cache revisits, layout changes, and resizing.
The [memory audit](../../evidence/line-search-scratch-v1/memory-bounded-audit.json) checked all 4,324 views, 2,856 holds, 49,942 RSS queries, and process cleanup.
The [focused audit](../../evidence/line-search-scratch-v1/focused-audit.json) separately replayed all 262 original and follow-up action windows and rederived the original timing summary.

| Patch | Reference peak RSS | Candidate peak RSS | Reference third-tour maximum | Candidate third-tour maximum |
| --- | ---: | ---: | ---: | ---: |
| Pierre | 71.14 MiB | 70.34 MiB | 71.11 MiB | 70.31 MiB |
| Ghostty | 60.16 MiB | 59.19 MiB | 60.12 MiB | 59.16 MiB |
| Node | 136.64 MiB | 135.89 MiB | 136.64 MiB | 135.89 MiB |
| Bun | 165.23 MiB | 165.78 MiB | 165.23 MiB | 165.78 MiB |
| Largest patch | 695.02 MiB | 681.55 MiB | 631.52 MiB | 593.20 MiB |

Peak RSS covers the process lifetime. Third-tour maxima use the settled holds.
These are one paired observation per patch on a shared host, not causal allocation measurements.
The largest input's resident memory fell during both processes' later tours.
The reference tour maxima were 691.03, 649.03, and 631.52 MiB; the candidate maxima were 681.55, 630.16, and 593.20 MiB.
Allocator and OS contributions are not partitioned. No general RSS saving is established.

Both builds missed the existing settled-memory goal on Node, Bun, and the largest patch.
The close Bun pair here does not resolve the earlier cohorts' adverse Bun observations.
All targets and workloads remained unchanged.

The memory trace classifies its viewing actions as setup actions, not the five-category latency workload.
Those observations include validation overhead and do not establish a new latency distribution.
Its largest-patch `long-last` view reached maximum text/color milestones of 306.77 ms for the reference and 302.53 ms for the candidate.
The largest input's index-ready upper bounds were 3,745.25 and 3,784.32 ms.
Thus, full indexing and some large-view delays remain open even though first text arrives much sooner.

## Decision and closeout

The [keep decision](../../evidence/line-search-scratch-v1/decision.json) accepts the narrow implementation for its repeated first-visit coloring improvement and completed correctness qualification.
The wider reading checks did not reveal a material new regression.
Bun's 0.55 MiB paired increase and all other observations remain in the evidence.

The normal executable was rebuilt without diagnostic features.
It passed all 38 process contracts and the separate 34-input alignment contract.
The two existing opt-in process exclusions remain unchanged.
The [normal build record](../../evidence/line-search-scratch-v1/normal-final/report.json) records its identity, features, exact source hashes, and test runners.
Performance observations remain bound to the separately preserved measured executables.

Scratch lifetime is bounded by one source-line parse, but that does not guarantee a lower memory peak.
Different searches can leave the sum of their buffers' high-water capacities until that line ends.
No dedicated latency cohort, physical terminal acceptance, or release readiness is claimed.

The first deep visit still requires substantial syntax-prefix replay.
Further work needs a separate scope; theme-independent checkpoint reuse remains a distinct possible improvement for theme changes, not a solution to the first cold visit.
All checkpoint workloads are complete. Do not restart these drivers in their existing output directories.
