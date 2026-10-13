# Immutable source-page ownership

Date: 2026-09-13. Implementation and focused gates complete; the full memory
checkpoint is incomplete. The working candidate remains unpromoted.

## Change and scope

The private source cache now stores `Arc<Page>`, where `Page` owns a
`Box<[u8]>`. The immutable byte buffer is allocated and filled before publishing
the shared owner. This separates the reference-count header from the large
buffer and avoids converting the filled vector into a shared slice.

Only `src/storage.rs` changes at runtime. The 64 KiB page size, 32 MiB payload
limit, insertion-order eviction, locking, partial-page replacement, published
source length, errors, and owned results from `Spool::read` remain unchanged.
There is no allocator-specific branch, unsafe code, pool, dependency, source
concession, or new public interface.

The immediate experimental control is ordinary build `611805a3…`, preserved
under `evidence/delivery-capacity-v1/candidate`. The page candidate is ordinary
build `b749889e…`, preserved under `evidence/source-page-v1/candidate`. Both have
source archives and build manifests. The control already contains the preceding
unaccepted projection/delivery changes; this experiment cannot silently accept
them or attribute their effects to pages.

## Correctness gate

Nine library tests, 32 terminal/CLI process tests, and one worker integration
test pass. Two existing opt-in process tests remain ignored. The added real-file
test covers byte-exact reads, page boundaries, partial-page refresh, eviction,
reader survival, and final strong-owner release. The added process test reads
beyond cache capacity, replaces the input path, invalidates rendering caches by
changing theme, and checks independently specified first/last source lines in
both layouts. It passes against both preserved builds.

Complete source checks match all five pinned patches, including all 19,299,175
supplied records in the largest input. Projection answers match the unchanged
reference on five corpus inputs, five structural challenges, and 2,888 accepted
exhaustive small cases; 716 other cases are rejected by both. Disjoint document
index accounting is unchanged. Formatting and application Clippy pass.

`Session::open` tests use the preserved candidate through
`NATIVE_DIFF_CONTRACT_BINARY`. Tests that construct Cargo's binary directly,
library checks, and example checks use the separate test/example build. These
are not all executions of one binary. The initial process-test gutter assertion
failure and its correction remain recorded in
[preparation notes](../../evidence/source-page-v1/PREPARATION.md). No expected diff
content changed. [The review](../../evidence/source-page-v1/REVIEW.md) records test
binding and review details.

## Allocation observer correction

The declared stack-logged pair completed opening and three tours, with matching
checked views and terminal cleanup. At each full-tour snapshot, both builds have
512 stack-attributed page buffers totaling 40 MiB. Separate shared owners add
24 KiB; the thinner table saves 16 KiB. The reviewed source-cache owner therefore
grows by 8 KiB under this observer. That negative result is retained.

An untimed native allocator control establishes observer interference on this
host. Without stack logging, a 65,536-byte allocation occupies 65,536 bytes and
a 65,552-byte allocation occupies 81,920 bytes. With logging, both report 81,904
usable bytes and occupy the 81,920-byte physical class in the heap report.
Logging pushes the exact-sized candidate buffer into the larger class.

Before another profile, the [observer correction](../../evidence/source-page-v1/UNLOGGED_SCOPE.md)
declared one reference/candidate pair without stack logging. It keeps the same
binaries, workload, expected screens, phases, and native heap/vmmap snapshots;
only logging and its dependent stack-history command are removed. The original
helper and results remain intact. No ordinary speed series was repeated.

Heap classes without stack logging are anonymous. Logical cache occupancy is
not directly sampled. Combine the native allocator control, logged owner counts,
and unlogged class/heap observations; do not label every allocation in a size
class as a page or present this as a fixed-payload process-memory guarantee.
Sequential profiler pauses are not ordinary responsiveness measurements.

Both corrected runs completed successfully. Unlogged live-heap differences
(candidate minus control) are -8,381,888, -8,371,760, and -8,429,744 bytes after
the three tours: approximately 8 MiB less at each phase. Each tour shows 512
fewer 80 KiB blocks, 511 more 64 KiB blocks, and one more 48 KiB block. This fits
the buffer/table change, with small owners and unrelated allocations included
in total heap. It supports the next gate, not exact cache-occupancy attribution.
[Saved reconciliation](../../evidence/source-page-v1/allocation-summary.json) includes
all phases, allocation counts, anonymous classes, and input hashes.

The logged tour-three whole-heap reduction is unrelated to the page saving: its
highlight-cache table is 46,678,016 bytes in the control versus 23,347,200 bytes
in the candidate. That difference accounts for essentially all the logged total
reduction, while its page owner grows by 8 KiB. The table-capacity difference's
cause is not established. The [gate decision](../../evidence/source-page-v1/MEMORY_GATE.md)
keeps it separate and records the independent review.

## Ordinary responsiveness

The capture-first correctness gate passes, including copied-capture corruption
controls and real-child overflow cleanup. Five alternating pairs complete with
40 supported visual sessions, 280 checked actions, disk replay, and clean exits.
Ten joined-emoji cases retain their source-only decoder exclusion; they are not
visual passes.

Continuous-end coloring medians are 62.117 ms control versus 61.972 ms candidate;
maxima are 62.359 versus 63.399 ms. Other results are mixed. Guard dark-theme
coloring is slower at 48.046 versus 50.126 ms median; the Ratatui fixture's
opening coloring is 38.081 versus 35.037 ms. All individual observations and
28 case/action summaries remain in
[the paired results](../../evidence/source-page-v1/paired/summary.json).

The [gate decision](../../evidence/source-page-v1/RESPONSIVENESS_GATE.md) supports
proceeding without declaring equivalence or a general speed improvement. There
is no second ordinary series. Earlier slower comparisons against another build
remain unresolved; this pair isolates the page delta against its immediate
control.

## Full-workload checkpoint

All ten fresh captures pass the original sequential-style oracle, historical
trace semantics, and corruption controls. All 25 ordinary latency processes and
four three-tour memory processes complete with matching checked views and normal
terminal cleanup. The final memory process fails its sampling requirement; the
whole checkpoint remains incomplete. There is no retry in this batch.

The corpus protocol uses inline validation, distinct from capture-first receipts.
These are developer-workload observations, not physical terminal-paint timings.

| Patch | First text maximum | Initial coloring maximum | Complete-index observation maximum |
| --- | ---: | ---: | ---: |
| Pierre showcase | 8.34 ms | 35.77 ms | 35.77 ms |
| Ghostty | 12.65 ms | 76.32 ms | 91.20 ms |
| Node | 9.10 ms | 53.88 ms | 193.41 ms |
| Bun | 8.65 ms | 40.70 ms | 185.67 ms |
| Largest patch | 9.30 ms | 19.91 ms | 4,254.10 ms |

Index observations are upper bounds from received frames. The largest input
misses the unchanged two-second index target. The Pierre showcase's distant
coloring maximum is 226.01 ms, above the 200 ms navigation limit. The other
navigation checks and all 25 normal latency exits meet their shared targets.

Settled RSS below is the maximum within the declared holds in each completed
tour, in MiB. It is not a guarantee of an indefinite plateau.

| Patch | Tour 1 | Tour 2 | Tour 3 | Process peak |
| --- | ---: | ---: | ---: | ---: |
| Pierre showcase | 223.95 | 270.94 | 271.11 | 271.14 |
| Ghostty | 108.14 | 109.00 | 111.25 | 111.28 |
| Node | 231.64 | 241.67 | 243.03 | 243.03 |
| Bun | 280.88 | 336.34 | 344.16 | 344.16 |
| Largest patch | Incomplete | — | — | No complete run |

Only the Ghostty patch meets the shared memory target among completed memory
runs. Growth across tours remains visible. Historical RSS values from different
builds are not substituted for the missing largest-input result.

The largest memory run stops on tour one at `long-page-44`: its one-second hold
collects three RSS samples instead of the required five. The final saved cells
exactly match that view's frozen expectation; 168 earlier steps completed. The
harness terminates the viewer with SIGTERM (exit 143) and restores terminal
settings. That is failure cleanup, not a successful normal quit.

The collector launches a whole-system `ps` query, filters by process group, then
waits 100 ms. Saved completion timestamps show gaps reaching roughly 300–440 ms
near failure. Query time, scheduling, and exact hold boundaries are not separately
recorded, so the precise delay source is not established. The
[failure report](../../evidence/source-page-v1/MEMORY_CAPTURE_FAILURE.md) preserves this
limit and the next bounded task. No sample minimum, hold, workload, or expected
screen was relaxed.

The [offline audit](../../evidence/source-page-v1/final-verification.json) verifies
archives, live source, binaries, frozen dependencies, allocation reconciliation,
and recomputed per-case summaries. Its evidence verification succeeds while
explicitly keeping the checkpoint status failed. All processes from this batch
have stopped.

## Decision and next step

Keep the small page-owner implementation as an unpromoted working candidate.
Correctness and focused allocation/responsiveness evidence support it; the full
ordinary memory requirement is not complete. This is not acceptance of the
preceding combined candidate or resolution of earlier slower observations.

Next, harden RSS collection with a verified process-group query and real-process
checks, preserving descendants, the safety stop, and settled-hold requirements.
Declare any revised measurement protocol before new collection. Do not silently
retry or mix replacement data into this series. Broader compiled-syntax ownership
work follows a reliable measurement path and still requires evidence of actual
live duplication before an implementation change.

The page contribution must be judged against its immediate control. This
full-workload checkpoint characterizes the combined candidate; an unmatched
historical RSS difference is not the page saving. Physical terminal pixels,
input-to-paint latency, smoothness, and visual parity remain unverified here.
The allocation benefit observed on this host is not a guarantee for every
allocator or platform.

## Artifacts

- [Candidate and exact build recipe](../../evidence/source-page-v1/candidate/build.json).
- [Source, tests, tools, and documentation package](../../evidence/source-page-v1/source-page-v1-source.tar.gz).
- [Complete batch checksums](../../evidence/source-page-v1/SHA256SUMS), including the
  retained failed measurements. Generated bytecode caches are excluded.
- [Offline completion/failure audit](../../evidence/source-page-v1/verify_final.py).

The source package excludes build products, local environments, and the large
evidence/corpus cache. The existing fixture manifest identifies the pinned inputs.
