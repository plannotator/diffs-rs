# Remaining memory owners

Completed 2026-09-13. Diagnosis only; no runtime optimization or release promotion.

The evidence now separates two costs: substantial live document/index storage
on large inputs, and growing saved color/token storage during reading. No cache
budget violation, leak, or new recoverable-byte estimate is established.

The existing post-sharing Bun profile attributes most of its residual growth without a
new run. One additional largest-input diagnostic completed all 190 frozen views,
two allocation snapshots, normal quit, terminal restoration, sampler shutdown,
and process-group cleanup. Application/dependency sources and ordinary executable
`9f16c2c9…` remain unchanged.

## P1: large documents retain material live index storage

The largest input's current instrumented heap is 538,275,136 bytes after completed
opening and 733,462,816 bytes after one fixed navigation tour:

| Snapshot | Whole live heap | Partial engine-named classes |
| --- | ---: | ---: |
| Opening | 513.34 MiB | 0.009 MiB |
| After one tour | 699.48 MiB | 40.87 MiB |

Every heap class and allocation count reconciles to the native tool's totals.
This is live allocated storage under stack logging, not ordinary RSS, physical
footprint, or a measurement of an indefinite steady state.

Two large, disjoint heap classes are identical at both boundaries:

| Allocation class | Bytes at each boundary |
| --- | ---: |
| Projection table finalization | 225,598,688 |
| Main indexed-file construction allocation site | 204,587,472 |

The [projection builder](src/projection/storage.rs:384) finalizes immutable boxed
tables. The [input publication path](src/storage.rs:377) builds source spans and
file owners and publishes them by `Arc`. These classes corroborate persistent
document storage; inlined call-site names are not a complete per-field partition.
They do not show an accumulating layout copy on navigation or resize.

Earlier logical/capacity accounting remains applicable because the seven reviewed
storage, parsing, projection, ingestion, and geometry source files match both
build manifests. The original accounting still has its original executable identity:

| Largest-document owner | Accounted bytes |
| --- | ---: |
| Source spans and final-newline flags | 156,836,180 |
| Projection, including its subindexes | 144,259,332 |
| Metadata allocations | 67,584,706 |
| File descriptors | 30,133,824 |
| Document file-vector capacity | 1,048,576 |
| Total | 399,862,618 (381.34 MiB) |

These categories are disjoint within that accounting. Projection subcategories
must not be added again. The total excludes allocator rounding, owner headers,
other caches, library allocations, and stacks. Do not subtract it from either
heap or RSS to claim an exact unexplained remainder.

The other document totals are 0.22 MiB for the showcase, 1.28 MiB for Ghostty,
13.33 MiB for Node, and 9.65 MiB for Bun. Index cost scales with source records,
changes, and file/metadata counts. It cannot explain Bun's repeated-tour growth.

Impact and direction: the current representation retains a substantial
input-scaled cost even with correct disk-backed source storage. Further general
index/metadata representation work may help, but the observed allocation sizes
do not by themselves justify an arena, custom allocator, or disk-backed-index
rewrite. Any such proposal must preserve the complete source domain, metadata,
split/unified projection, inverse destinations, and reading anchors.

## P2: saved color/token storage explains most residual Bun growth

Reanalysis of the existing current-candidate allocation reports gives this
disjoint tour-one to tour-three heap-class delta:

| Class group | Growth |
| --- | ---: |
| Direct visible-color/token allocation class | 11,064,800 bytes |
| Hash-table allocation classes | 5,832,704 bytes |
| Explicitly engine-named classes | 2,130,192 bytes |
| Remaining classes | 2,079,808 bytes |
| Whole heap growth | 21,107,504 bytes (20.13 MiB) |

The first two groups explain 80.05% of this measured increase.
Supporting stacks identify the style-cache table growing from 5,849,088 to
11,681,792 bytes. Other allocations in its exact heap class remain unchanged.
The table stacks corroborate the class delta; they are not an extra heap total.

The direct token/color allocation stacks also reconcile to their exact heap
class. Do not sum all stacks containing `Side::advance`: that function also
appears above grammar compilation. The reviewed source path is
[visible coloring](src/highlight.rs:113), [token construction](src/highlight.rs:55),
[side-specific style transfer](src/highlight.rs:503), and
[cache insertion](src/highlight.rs:668). Not every byte at that allocation site
is proven to be inside the cache rather than a pending reply or current work item.

The FIFO allocation is 1,064,960 bytes at both boundaries in this profile.
The earlier pre-sharing report's FIFO growth must not be carried into this result.

The separate diagnostic census supports useful cache population:
41,186 to 78,025 entries and 15,630,608 to 30,225,840 bytes of accounted weight.
Unique inner handles rise only from 1,027 to 1,059, with strategy estimates rising
from 31,986,159 to 33,144,365 bytes. These are separate-process observations, not
simultaneous heap counters. They must not be subtracted from the heap profile.

The cache counter includes occupied inline `LineStyle` values and vector
capacities. A full table allocation also includes those inline values. Adding
both whole totals would double-count them. Entry growth is not evidence that the
cached answers are redundant, that the cache is leaking, or that its budget should
be reduced.

Impact and direction: examine the representation of completed styles next.
Their tokens are constructed once and then read through slices, including
separate old-side colors where required. Determine whether unused capacity or
equivalent foreground boundaries are material before changing their storage.

## What the new tour adds, and what it does not

The largest-input tour adds 186.15 MiB of live heap relative to opening.
The engine-named subset grows by 40.86 MiB; source pages, visible-color/token
storage, and cache containers also populate. This first-tour increase does not
establish continuing growth across repeated tours.

At tour end, source-page allocation stacks total 41,943,040 bytes for 512 page
buffers. The application still limits their logical payload to 32 MiB.
The existing [source-page investigation](SOURCE_PAGE_REPORT.md) documents the
stack logger's larger allocation class. This is not a new source-cache regression.

Supporting cache-container stacks identify a 23,347,200-byte table and a
5,849,088-byte FIFO at tour end. This process did not collect logical cache-entry
or token-capacity counters, so do not import counts from Bun or infer unused
capacity from those two allocations alone.

The new profile's `vmmap` reports physical footprint of 564.2M then 806.9M,
with swapped regions present at the second snapshot. Snapshot commands are
sequential, and these VM fields are not heap or ordinary RSS. They remain in
the raw evidence rather than being combined into a causal partition.

The earlier uninstrumented 771.36 MiB lifetime peak and RSS decrease remain
unexplained quantitatively. This diagnostic establishes live owners in its own
process; it does not establish the cause of the earlier RSS decrease.

## Other owners and remaining uncertainty

- Exact patch bytes remain in a private disk spool, not a resident 678 MiB buffer.
  Source-page payload is bounded; readers and container/allocator overhead are extra.
- Geometry retains at most 1,024 entries and 16 MiB of accounted payload.
  Several widths may coexist, but no unbounded width-history owner was found.
  Long-line construction can temporarily exceed the cache admission budget.
- Document indexes are immutable after publication. Requests share file owners.
  Temporary per-file parsing and input-buffer capacities end with their scopes;
  any resulting allocator retention is not a live application collection.
- Parser continuation/checkpoints and queued replies have explicit ownership and
  bounds. Their exact bytes remain outside this profile's complete owner partition.
- Search-buffer pool bytes remain unmeasured separately. Existing engine estimates
  exclude them; another broad dependency change is not justified by that absence.

## Next bounded task

Extend the existing off-by-default style-cache census with borrowed aggregate
counts at its existing observation boundaries:

1. Primary and old-side token lengths, capacities, and spare-capacity bytes,
   plus changed-span lengths/capacities and occupied inline storage.
2. Contiguous equal-foreground token boundaries and their hypothetical merged
   count, without modifying tokens or bridging gaps.
3. Optional old-side storage counts and whether different token partitions give
   identical byte-to-foreground coverage.

Do not clone styles, read source files, reserve maps, resize caches, or change
eviction to obtain these counters. Reuse the checked addition-heavy and
largest-input traces; keep diagnostic pauses outside performance claims.
The outcome can be no change if the proposed redundancy is negligible.

If unused capacity is material, evaluate immutable boxed token storage before
combining it with token coalescing. Boxing can copy/reallocate, changes inline
header size and cache weights, and may admit more lines under the same budget.
It is not automatically a total-memory saving. Coalescing is a separate semantic
decision: preserve exact byte/color coverage, gaps, side separation, and inline
change backgrounds.

Any later implementation must keep the independent source/style oracle unchanged,
pass real-process side/layout/wrap/resize and worker-lifecycle checks, and receive
separate ordinary responsiveness and sustained-memory qualification. Full index
latency, the focused coloring slowdown, and actual terminal presentation remain
open. This investigation does not begin that runtime change.

## Evidence and integrity

- [Pre-run scope](evidence/remaining-owners-v1/PLAN.md),
  [frozen diagnostic inputs](evidence/remaining-owners-v1/profile-freeze.json),
  [completed process](evidence/remaining-owners-v1/run-result.json),
  and [reconciliation](evidence/remaining-owners-v1/analysis.json).
- The [offline reducer](evidence/remaining-owners-v1/analyze.py) reuses the existing
  heap parser, verifies all class/count totals, checks the earlier Bun reduction
  exactly, and records input hashes. It does not detect performance defects by
  scanning source or run another viewer.
- Both Bun and the new largest-input process use the preserved ordinary candidate,
  with allocation logging enabled only for profiling. The separate owner census
  uses the diagnostic build of the same implementation.
- Stack-history totals exceed heap totals by 7,036,944 bytes in the existing Bun
  profile and 7,053,072 bytes in the new profile. These discrepancies remain
  unassigned; stack histories are supporting call sites, not a heap partition.
- The performance audit included independent read-only ownership and arithmetic
  review. No new Rust suite or complete corpus benchmark ran. The additional
  diagnostic passed its opening and every fixed action, then exited cleanly.
- Direct companion-task messaging remains unavailable. No dedicated-host or
  newly acknowledged reservation is claimed. No native workload remains running.

No application code, dependency code, grammar, rendering rule, cache
limit, or previously sealed evidence was changed.
