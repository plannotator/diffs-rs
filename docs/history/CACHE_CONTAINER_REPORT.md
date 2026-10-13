# Cache-container ownership

Completed 2026-09-14. Almost all of the later live-heap increase is now attributable to the style lookup table in the rejected build.
The exact earlier ordinary RSS increase remains unpartitioned.
No application code changed, and the boxed-style experiment remains rejected.

## P1: the lookup table grows after repeated navigation

Two preserved executables ran the same three-tour allocation diagnostic, serially.
Both processes matched all 568 expected text/color views and exited cleanly.
Each process produced opening and three tour-end snapshots through the existing native tools.

MiB means 1,048,576 bytes. These figures describe stack-logged live heap, not ordinary RSS.

| Boundary | Reference heap, MiB | Rejected-build heap, MiB | Reference lookup, MiB | Rejected-build lookup, MiB |
| --- | ---: | ---: | ---: | ---: |
| Opening | 513.34 | 513.33 | 0.006 | 0.005 |
| Tour 1 | 701.51 | 726.06 | 22.27 | 32.52 |
| Tour 2 | 706.59 | 731.02 | 22.27 | 32.52 |
| Tour 3 | 706.73 | 763.65 | 22.27 | 65.02 |

The rejected build's tour-two to tour-three heap increase is 34,221,072 bytes.
Its lookup allocation increases by 34,078,720 bytes, or 32.5 MiB.
That difference exactly matches the corresponding hash-table heap-class increase.
Explicitly named engine classes add 145,056 bytes. The remaining classes decrease by 2,704 bytes.
All classes together reconcile to the whole-heap increase.

The full allocation stacks identify `highlight::Cache::insert` as the lookup owner.
The heap reports corroborate one allocation of 34,095,104 bytes, followed by one of 68,173,824 bytes.
Sources: [tour-two stack](../../evidence/cache-container-owners-v1/rejected/tour-2-allocations.txt),
[tour-three stack](../../evidence/cache-container-owners-v1/rejected/tour-3-allocations.txt),
and [tour-three heap](../../evidence/cache-container-owners-v1/rejected/tour-3-heap.txt:125).

The first two tour-end snapshots did not show this growth. The third tour-end snapshot captured it.
This result establishes a retained allocation increase, not its precise admission trigger.
Interior entry peaks and deletion/reinsertion history remain possible causes.
The reference lookup allocation stayed unchanged at all three captured tour endpoints.
Endpoint observations do not rule out all intermediate allocation activity.

The full table contains occupied styles as well as unused slots, keys, and control storage.
The cache's 32 MiB counter already includes occupied inline styles.
Adding that counter to the full table allocation double-counts those values.
The [source review](../../evidence/cache-container-owners-v1/SOURCE_REVIEW.md) explains the ownership and standard-library capacity semantics.

Impact: compact token buffers alone do not control total cache allocation.
In this pair, the rejected build retained a larger lookup allocation despite smaller individual style records.
No table defect or memory leak is established.

## P2: the FIFO retains duplicated bookkeeping

The allocation stacks identify these FIFO allocations at every captured tour endpoint:

| Build | FIFO allocation, bytes | MiB |
| --- | ---: | ---: |
| Reference | 7,946,240 | 7.58 |
| Rejected build | 14,696,448 | 14.02 |

These are allocator-reported allocations, not deque-capacity estimates.
The FIFO did not contribute to the rejected build's tour-two to tour-three heap increase.
Its larger allocation remains a separate cost in this pair.

The [cache](../../src/highlight.rs:637) stores `(source ID, style weight)` in each queue element.
Cached styles cannot change through the public cache API.
The existing map removal returns the style with the same capacities that determined its original weight.
The queued weight is therefore redundant.

The next small implementation can store IDs only and calculate the weight from the removed style.
One private helper can preserve the exact weight formula at admission and removal.
No extra map lookup, custom arena, or eviction-policy change is necessary.
Queue elements then use 8 rather than 16 bytes on this target.
Actual allocation and RSS savings require measurement.

This change adds fixed metadata reads and arithmetic during eviction on the UI thread.
Its scope must preserve duplicate handling, FIFO order, current budget, and insertion-before-eviction behavior.
The diagnostic must acknowledge that FIFO weight becomes recomputed rather than independently stored.

## Allocator retention and remaining uncertainty

The rejected build's final VM snapshot also reports a new 32.5 MiB empty large-allocation region.
That region is resident and dirty in the native report.
This observation is consistent with freed allocator space remaining resident.
The summary does not identify the former owner of that region.
[Raw VM report](../../evidence/cache-container-owners-v1/rejected/tour-3-vmmap.txt).

The VM tool reports 36.7M then 72.3M of dirty-plus-swapped fragmentation across the last two snapshots.
Those are rounded native estimates, not an application-owner partition.
The reports also contain swapped regions. Snapshot commands run sequentially.
Heap, VM, stack-history, and ordinary RSS totals must remain separate.

The saved ordinary run rose by 65.03125 MiB between adjacent held-view maxima late in tour two.
Non-held navigation steps occur between those observations.
The new profile captures table growth on tour three instead.
It corroborates a table-growth mechanism but does not exactly attribute the earlier RSS jump or its timing.
The earlier ordinary peak difference of 35.34375 MiB also remains unpartitioned.

Stack logging changes allocation classes and residency. Observer pauses and asynchronous worker progress can change cache contents.
The allocation difference in this pair is not a universal fixed cost per build.
No rerun was used to obtain a preferred result.

## Next work and correctness boundary

1. Scope and implement IDs-only FIFO on the restored reference as one independent change.
   Preserve the exact weight formula and cache policy.
   Check real spare-capacity styles, old-side distinctions, duplicates, multiple evictions, oversized entries, and identical survivors.
2. Run existing public-terminal source/style, resize, navigation, and revisit checks.
   Measure ordinary responsiveness and sustained memory on preserved builds.
   Reject a change that harms correctness or lacks a defensible overall result.
3. Before a lookup redesign, observe peak live entries and capacity transitions around admission and eviction.
   Keep those observations scalar and off by default.
   Model pre-admission eviction separately, without assuming that it solves retained allocation.

No smaller cache budget, grammar reduction, allocator replacement, forced periodic shrinking, or fixture-specific policy follows from this investigation.
General index storage and actual terminal presentation remain separate open work.

## Evidence and integrity

- [Pre-run scope](../../evidence/cache-container-owners-v1/PLAN.md) and
  [frozen commands and identities](../../evidence/cache-container-owners-v1/freeze.json).
- [Two completed processes](../../evidence/cache-container-owners-v1/run-result.json) and
  [all-class reconciliation](../../evidence/cache-container-owners-v1/analysis.json).
- Every heap snapshot reconciles both allocated bytes and node counts.
  Stack-history totals exceed heap totals by 7,036,688 reference bytes and 7,053,072 rejected-build bytes at each boundary.
  These discrepancies remain unassigned. Stack histories support call-site attribution, not a complete heap partition.
- All 1,136 views, 24 native snapshot commands, terminal restoration, sampler shutdown, and process-group cleanup passed.
  The driver checked frozen identities before and after each process.
- Application sources still match the preserved reference. The local executable remains `67a6ee19…` and received no new ordinary performance qualification.
  No new full Rust suite or full-source corpus scan was needed for this read-only investigation.
- Independent read-only review covered ownership, arithmetic, the driver, and evidence sealing.
  The runs used the shared local host without an acknowledged dedicated-host reservation.

The measured reference is `9b6e45076368f96830e9d64ec58609bfc96ffe6d6e038496094f7609ca4e165d`.
The rejected executable is `d65c2205edc15e9dab04ea9438fe370e52ca6c4b20aacb42072a983286f2502b`.
Their source archives and earlier sealed evidence remain unchanged.
