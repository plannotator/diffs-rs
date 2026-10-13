# Saved-style storage: measured opportunity

Completed 2026-09-14. [Declared scope](evidence/style-census-v1/PLAN.md),
[audited counters](evidence/style-census-v1/audit.json),
[execution record](evidence/style-census-v1/run-result.json).

Unused vector capacity is the first representation cost to address. The final
snapshots contain 10.28 MiB of it in the three-tour Bun workload and 10.19 MiB in
the largest-input one-tour workload. These are cache-storage observations, not
demonstrated process-memory savings. No runtime optimization was implemented.

## What changed

The existing off-by-default `owner-census` feature now reports borrowed aggregates
for primary tokens, explicitly stored old-side tokens, and changed-word ranges.
It separates used elements, allocated capacity, occupied inline values, and
hypothetical same-color boundary reductions. The new module neither mutates the
cache nor reads source text, clones tokens, sorts ranges, or runs new matching.

Application build-input changes are exactly `src/highlight.rs`,
`src/highlight/census.rs`, and `src/highlight/census/tests.rs`. The former gated
cache census moved into the new module. Token construction, grammar coverage,
publication, rendering, cache contents, budgets, and eviction are unchanged.
There are no dependency changes.

The existing PTY runner gained aggregate checks and a choice of one or three
complete memory tours. It no longer restricts all diagnostic traces to one
fixture's action count. Default three-tour behavior, exact view expectations,
deadlines, safety sampling, input binding, and cleanup remain in place.

## Observations

MiB is 1,048,576 bytes. Dynamic storage below includes token and changed-range
vectors. Accounted cache weight is dynamic capacity plus occupied inline values;
it is not the full cache allocation or process memory.

| Workload boundary | Entries | Accounted weight, MiB | Dynamic used, MiB | Dynamic capacity, MiB | Unused capacity, MiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| Bun opening | 43 | 0.009 | 0.001 | 0.006 | 0.004 |
| Bun tour 1 | 40,750 | 14.74 | 6.22 | 11.63 | 5.41 |
| Bun tour 2 | 63,815 | 23.31 | 9.91 | 18.44 | 8.53 |
| Bun tour 3 | 76,173 | 28.07 | 11.99 | 22.26 | 10.28 |
| Largest opening | 42 | 0.008 | 0.001 | 0.005 | 0.004 |
| Largest tour 1 | 108,593 | 32.00 | 13.52 | 23.71 | 10.19 |

At the Bun endpoint, 46.16% of dynamic capacity is unused: 10,747,152 bytes in
primary tokens and 28,112 bytes in changed ranges. At the largest endpoint,
42.99% is unused: 10,359,336 primary-token bytes, 1,464 old-token bytes, and
329,008 changed-range bytes. These counts exclude allocator rounding/overhead.

Merging exactly contiguous equal-color ranges would reduce Bun's 523,233
nonempty tokens to 447,135, a 14.54% reduction in that count. The largest endpoint
would go from 585,479 to 451,516, a 22.88% reduction. At the current 24-byte token
size, the removed boundaries represent 1.74 and 3.07 MiB of used elements.
These are hypothetical representation changes, not changes made by the census.
Do not add them to unused capacity as a promised memory saving: merging changes
required capacity, and a future cache can retain a different set of entries.

All six corpus snapshots reported zero empty token spans and zero invalid
reversed, overlapping, or out-of-order token vectors. Bun retained no explicit
old-side vectors at these boundaries. The largest endpoint retained 18, with
54 tokens; none had equivalent primary-side coverage. There is no measured case
for adding old/new token sharing here. This does not establish that other inputs
never retain equivalent old-side data.

The largest endpoint's map reports 456,530 usable slots for 108,593 occupied
entries; its FIFO has capacity 262,144. These are container counts, not backing
allocation bytes. The 32 MiB logical weight excludes empty table slots, table
metadata, FIFO backing storage, and allocator overhead. Adding the full map
allocation to logical weight would also double-count occupied inline values.

## Correctness and execution checks

- Formatting and all-target/all-feature Clippy passed.
- Both preserved release builds passed 32 real-process contracts and the worker
  lifecycle contract. They cover side-specific syntax, exact inline emphasis,
  wrapping/Unicode, rapid resizing, anchors, loading failures, and cleanup.
- Diagnostic library checks passed 11 tests, including the two new accounting
  and normalized-coverage checks. Its two inner-engine and eight regex-census
  checks also passed. The ordinary build passed its nine library checks.
- Two pre-existing opt-in process tests remained ignored in each suite: the
  preserved pre-storage comparison and full five-body endpoint check. This
  stage did not rerun all supplied source records or the ordinary timing cohort.
- Seven PTY controls passed: startup observation, eight identical settled
  snapshots with safe ninth rejection, three output-collision boundaries,
  feature-off behavior, and no-output-requested behavior. Observations left
  the checked terminal cells unchanged.
- The declared Bun trace ran once for three tours: 138 actions per tour,
  415 checked views, four snapshots. The largest trace ran once for one tour:
  189 actions, 190 checked views, two snapshots. The offline audit matched the
  complete ordered label sequences, not just their counts.
- All nine processes completed their expected exit paths, restored terminal
  state, stopped the sampler, and left no viewer process group. Counter
  equations reconciled raw snapshots with process records. Two corruptions of
  an actual captured cache record were rejected by the new validator.

The [development record](evidence/style-census-v1/DEVELOPMENT.md) retains a test
compilation correction and a failed offline audit inventory. Only the offline
audit was repeated after its file-filter fix; no successful corpus run was
repeated. [Independent read-only review](evidence/style-census-v1/REVIEW.md)
found no blocking implementation or accounting issue.

## Limits and executable identities

These observations cover cached styles at declared boundaries, not all pending
replies, active worker storage, parser state, indexes, or total retained memory.
The census pauses work and allocates diagnostic JSON. Asynchronous worker
progress can change retained cache counts between processes, even when every
checked view is identical. Do not subtract this process's cache counts from an
earlier process's heap or RSS to claim a disjoint allocation partition.

The runs were serial on the local shared host, without allocation logging.
The inherited `MallocNanoZone=0` setting is recorded in both declarations.
No overlapping viewer/build/profile process was observed before the controls.
Companion-task messaging remains unavailable, so there was no newly acknowledged
host reservation. Diagnostic duration and safety sampling are not an ordinary
responsiveness or sustained-memory qualification. Physical Ghostty/Kitty paint,
scrolling feel, and Pierre visual parity remain unaccepted.

| Executable | SHA-256 | Qualification |
| --- | --- | --- |
| [New ordinary build](evidence/style-census-v1/ordinary/build.json) | `9b6e45076368f96830e9d64ec58609bfc96ffe6d6e038496094f7609ca4e165d` | Release contracts and feature-off control; no new performance cohort |
| [New diagnostic build](evidence/style-census-v1/diagnostic/build.json) | `723c2eb302dd809004034189f3c6d1817b1730f198e4fd2706b4587764afecd8` | Release contracts, seven controls, two census workloads |
| [Previously measured ordinary build](evidence/inner-engine-v1/candidate/build.json) | `9f16c2c92a9efd2d3cf77e5c0a5a41d314124099cafa577ad36116ed83a1b657` | Existing five-patch responsiveness and memory checkpoint |

The new ordinary executable is not byte-identical to the previously measured
one. Earlier performance results, including the 771.36 MiB largest-input peak,
remain bound to that earlier executable.

## Next bounded implementation

Reduce unused capacity at the boundary where completed style data becomes
immutable. First trace the existing temporary-fragment-vector conversion and
final storage; select a small idiomatic ownership change, such as exact-sized
slices, without changing a single token range, foreground color, old-side
distinction, changed range, or fallback answer. Avoid periodic shrinking,
allocator replacement, custom pools, and input-specific policies.

Keep boundary merging and new sharing out of that first change. They are
separate representation decisions with additional coverage proof obligations.

Use truthful accounting for the chosen representation. An unchanged byte budget
can admit more entries after compaction, so assess map/FIFO growth and whole
process memory, not just vector bytes. Keep the existing grammar, workload, and
budgets. Recheck exact source/style views, side distinctions, resize anchors,
repeated cache revisits, and cleanup before a preserved current-build
responsiveness and sustained-memory comparison. Extra finalization work could
hurt coloring latency and must be measured. Revert the candidate if it harms
correctness or fails to provide a defensible overall improvement.

Large-input index storage and first-visit coloring delays remain separate work.
This census neither resolves them nor authorizes a broader index rewrite.
