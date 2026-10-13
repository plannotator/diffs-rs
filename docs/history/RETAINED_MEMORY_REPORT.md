# Retained-memory ownership

Date: 2026-09-13. Candidate: `da5754e8…`. Diagnosis only; no runtime changes.

The new three-tour diagnostic confirms live-allocation growth on the addition-heavy
patch. Directly named highlighting-engine classes explain 50.72 MiB of its
83.44 MiB tour-one to tour-three increase. The style cache's backing table also
grows by 5.56 MiB. These are measured owners to investigate, not recoverable-byte
estimates or proof of a leak.

The preserved ordinary executable matched the opening and all 414 action views,
then exited normally with terminal restoration and clean RSS-sampler shutdown.
The viewer PID and process group are absent. No application source, binary,
grammar, cache limit, workload, or expected screen changed. No new ordinary
performance series ran, and the candidate remains unpromoted.

## Findings

### P1: highlighting engines retain substantial memory and continue growing

The worker owns one syntax set for its lifetime. Each initialized grammar keeps
its contexts; those contexts strongly own shared compiled patterns. The sharing
registry holds weak references, so it is not independently keeping compiled
engines alive. Source: [worker](../../src/highlight.rs:292),
[grammar initialization](../../vendor/syntect/src/parsing/syntax_set.rs:483),
[compiled owner](../../vendor/syntect/src/parsing/regex.rs:16).

One outer pattern can contain several inner matching engines. Each inner engine
retains both immutable matching structures and a pool of reusable search state.
Dropping a search guard returns that state to the pool. A stable outer-pattern
count therefore does not imply stable memory. Conversely, Syntect's per-line
`SearchCache` and source-derived backreference expressions are temporary, not
persistent collections. See [per-line state](../../vendor/syntect/src/parsing/parser.rs:241)
and [backreference construction](../../vendor/syntect/src/parsing/syntax_definition.rs:269).

The new profile uses the unchanged Bun patch and checked three-tour sequence:

| Snapshot | Whole live heap, MiB | Directly named engine classes, MiB |
| --- | ---: | ---: |
| Opening | 24.79 | 4.63 |
| Tour 1 | 229.57 | 118.68 |
| Tour 2 | 295.41 | 163.66 |
| Tour 3 | 313.01 | 169.40 |

The named subtotal uses heap classes containing `regex_automata::` or
`fancy_regex::`; generic allocator classes remain outside it. Its 50.72 MiB
increase accounts for about 61% of the whole-heap increase. NFA construction,
compiled DFA construction, and matching-strategy classes contribute substantial
growth. Separate stacks also show retained search-buffer allocations. This is
not an exact split between immutable engines and scratch, nor attribution to
individual grammar patterns.

The existing syntax-heavy profile uses the same executable and ends at 213.40 MiB
live heap, with 165.80 MiB in that same narrowly defined named subtotal. Its engine
subtotal is unchanged between tours two and three. The earlier report's slightly
broader name match also includes mangled type names; neither subtotal is a
complete engine total.

The current outer-pattern sharing does not deduplicate inner fragments compiled
by different outer expressions. Pinned `fancy-regex 0.16.2`, `src/compile.rs:594`,
constructs each delegate through a fresh inner builder and preserves its capture
mapping. Repeated equivalent inner fragments are an opportunity to measure,
not an established duplication count or justification to implement sharing now.

### P2: the line-color cache retains material container storage beyond its budget

[The style cache](../../src/highlight.rs:603) accounts for `LineStyle` and the capacities
of token/change vectors against 32 MiB. It does not count its hash table or FIFO
deque in that budget. Those containers grow as entries are inserted and can
retain capacity after eviction. Theme changes replace the cache.

The new allocation stacks identify the following live allocations through
`highlight::Cache::insert`:

| Container | Tour 1, bytes | Tour 3, bytes | Increase |
| --- | ---: | ---: | ---: |
| Hash table | 5,849,088 | 11,681,792 | 5.56 MiB |
| FIFO deque | 540,672 | 1,064,960 | 0.50 MiB |

The table difference exactly matches the corresponding heap-class increase.
These stacks corroborate ownership; their values are not added to heap totals.
Source evidence: [tour-one table](../../evidence/retained-owners-v1/profile/tour-1-allocations.txt:34),
[tour-three table](../../evidence/retained-owners-v1/profile/tour-3-allocations.txt:28),
[tour-one deque](../../evidence/retained-owners-v1/profile/tour-1-allocations.txt:110),
[tour-three deque](../../evidence/retained-owners-v1/profile/tour-3-allocations.txt:82).

The `Side::advance` allocation class also grows by 10.35 MiB. That function
produces token vectors later held by the style cache; this is not evidence that
parser checkpoints grew by that amount. Current snapshots do not include cache
entry counts, accounted payload, or map/deque capacities. More cached lines,
different per-line styles, and table capacity behavior remain to be separated.
No tombstone defect or budget violation is established.

## Other retained owners

| Owner | Lifetime and bound | What this review establishes |
| --- | --- | --- |
| Document indexes and metadata | Document-owned, immutable after publication | 9.65 MiB of accounted structures for the addition-heavy patch; 381.34 MiB for the largest patch. Repeated navigation does not rebuild or append them. |
| Source-page cache | 32 MiB page payload; reader Arcs temporarily extend lifetime | Source-page allocation-site totals stay unchanged across all three new tours. Containers and allocator rounding are extra. |
| Geometry cache | 16 MiB payload and 1,024 entries | Text and wrap positions for different widths can coexist. Returned `Rc` owners are temporary in reviewed rendering paths. No unbounded width-history collection found. |
| Parser continuation | One old/new state pair | Active parse/capture/scope storage is separate from cache budgets. No exact current owner-byte snapshot. |
| Parser checkpoints | 64 entries, 16 MiB including containers, 256 KiB per entry | IDs, captured text, and scopes; they do not strongly own compiled engines. Epoch/theme changes clear entries. |
| Worker delivery | One pending and one active request; 16 queued replies plus current publisher reply | File indexes are shared by Arc. No accumulating request queue found. |

Sources: [document and source pages](../../src/storage.rs:163),
[geometry](../../src/geometry.rs:115), [checkpoints](../../src/highlight/checkpoint.rs:11),
[delivery](../../src/highlight.rs:178), [request construction](../../src/app.rs:379).

Structural bytes come from the earlier source-page build's index accounting.
The five structural source files are hash-identical to the current build. The
original binary identity remains explicit; this is not a new current-process
heap measurement. The largest document's 381.34 MiB remains a separate scaling
cost, even though it does not explain growth across tours on the smaller input.

## Measurement limits

- This was one allocation-instrumented process, not a retry or new timing series.
  The ordinary Bun observation of roughly 275, 313, and 337 MiB remains unchanged.
  The diagnostic corroborates live growth but does not quantitatively explain
  those exact RSS samples.
- Stack logging changes allocation classes. Earlier native allocator controls
  show its effect on source-page buffers. Do not treat logged page sizes as a
  regression in the ordinary executable.
- All eight analyzed heap snapshots, including the existing syntax profile,
  reconcile every class's bytes and allocation count to their heap totals.
  Snapshot commands are sequential, not simultaneous.
- Stack-history byte totals exceed heap totals by 7,036,944 bytes in every new
  snapshot and 7,053,328 bytes in every existing syntax snapshot. The discrepancy
  is unassigned. Stack histories provide call-site evidence, not an exact heap
  partition. It is not attributed to instrumentation merely because it is constant.
- Matching scripted actions and settled views do not establish identical internal
  parser work. Cache state, pending work, and cancellation can expose additional
  patterns during intermediate navigation. Three increasing samples establish
  growth, not a leak or eventual plateau.
- No current per-pattern engine bytes, exact scratch capacities, or live
  application-cache counters were collected. No current largest-patch allocation
  profile was added. Its previously observed RSS drops remain unexplained.

## Next bounded task

Add a diagnostic-only owner census at the existing opening/tour-end boundaries:

1. Initialized grammar contexts and live outer compiled-pattern identities.
2. Inner delegate identities and effective settings, with immutable engine
   storage separate from reusable search state. Count shared owners once.
3. Style-cache entry count, accounted payload, hash-table capacity, and deque
   capacity. Keep backing allocations outside the payload subtotal explicit.

Use existing off-by-default diagnostics and the same checked workload. Do not
add a general telemetry system or alter application behavior. Pinned
`regex-automata 0.4.18` reports strategy and cache memory separately; some cache
counters use vector lengths rather than capacities. Label those as approximate,
and reconcile them against allocator evidence rather than calling them exact.

If inner-engine inspection requires a broad dependency rewrite, stop and scope
that separately. Neither engine eviction, smaller cache limits, inner-fragment
sharing, nor engine replacement is authorized by this finding alone. Choose one
runtime change only after the census identifies a redundant or unnecessarily
retained owner. Preserve exact source, colors, capture mappings, reading position,
and cancellation; verify any later change separately from instrumentation.

## Evidence

- [Pre-run declaration](../../evidence/retained-owners-v1/DECLARATION.md)
- [Frozen inputs and command](../../evidence/retained-owners-v1/freeze.json)
- [Real-process completion](../../evidence/retained-owners-v1/run-result.json)
- [Allocation reconciliation and input hashes](../../evidence/retained-owners-v1/analysis.json)
- [Offline reducer](../../evidence/retained-owners-v1/analyze.py)

The performance audit used an independent read-only ownership review, followed
by direct verification of its findings. No runtime files were edited or rebuilt.
