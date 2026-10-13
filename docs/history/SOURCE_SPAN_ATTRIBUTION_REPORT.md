# Source-span attribution: diagnostic complete

Date: 2026-09-24. The ordinary source and executable remain unchanged.

## Decision

Keep the source-span candidate isolated and unadopted.
Close this attribution checkpoint without another capture or implementation change.
The next product checkpoint is integrated local-source qualification on the normal build.

The smaller representation reduced native live allocations in this diagnostic.
The diagnostic did not reproduce the earlier adverse RSS direction.
It therefore does not establish that the representation caused the earlier increase.
It also does not replace the completed ordinary comparison or resolve its Bun opening delay.

This result supports the storage calculation, not a general performance improvement claim.
No allocator replacement, cache reduction, extra instrumentation framework, or new packing change follows from it.

## What completed

Two small opening checks and two large profiles completed on battery power.
The checks passed 414 exact terminal views, including 412 views in the matched large profiles.
All 18 native commands succeeded across six snapshot boundaries.
Both large profiles used the exact existing first-tour sequence and unchanged screen expectations.
All viewers exited normally, restored terminal state, and stopped their resource samplers.

The driver checked source, executable, input, trace, and helper identities before and after each profile.
The host records contain no sleep event or reported thermal warning.
Battery power did not prevent these correctness and ownership checks.
No timing or ordinary RSS result is accepted from this diagnostic.

The [scope](../../evidence/source-span-attribution-v1/PLAN.md), [capture record](../../evidence/source-span-attribution-v1/capture-001/report.json), and [analysis](../../evidence/source-span-attribution-v1/capture-analysis.json) preserve the evidence.

## Findings

### The smaller representation reduces live allocations in the measured pair

The table reports native live-allocation bytes in MiB, with allocator stack logging active.
These are diagnostic values, not a replacement baseline for normal viewing.

| Boundary | Reference | Candidate | Candidate minus reference |
| --- | ---: | ---: | ---: |
| Completed opening | 529.52 | 494.42 | -35.10 |
| First-tour endpoint | 720.67 | 683.74 | -36.93 |

The existing logical index accounting predicted a 35.05 MiB reduction after descriptor costs.
Native allocation sizes include allocator rounding and are not identical to logical Rust extents.
The document-construction stack group decreased by 36,805,952 bytes at both boundaries.
That group contains more than source spans, so it is not an exact span-only census.

At opening, the candidate had 75,238 additional live allocations.
That count matches the additional array allocation for each eligible nonempty file.
Both constructors allocate their final representation directly. Neither constructor retains an old representation beside a converted copy.

The [candidate constructor](../../evidence/source-span-prototype-v1/source/src/source_spans.rs:49) uses separate offset, length, and flag arrays.
The [normal constructor](../../src/source_spans.rs:43) uses a combined offset/length array and flags.
The [document](../../src/storage.rs:205) retains immutable file indexes while it remains open.
Navigation does not grow or replace those source-span arrays.

### Inspected coloring containers do not explain extra candidate memory in this diagnostic

The style-map and FIFO allocation-site byte totals matched at opening and the first-tour endpoint.
The direct color-output allocation site was 5,600 bytes smaller in the candidate at the endpoint.
The inspected sidebar and projection-table allocation-site totals also matched between roles at each boundary.

These are allocation-site observations, not complete typed owner counts.
They do not distinguish every cached value, queued reply, and current worker value.
The selected stack groups must not be added to the native heap total.

The [reply path](../../src/app/source.rs:150) accepts valid queued styles after a viewport change.
Thus, equal completed screens do not require identical retained caches under different schedules.
The [cache charge](../../src/highlight.rs:74) includes inline style descriptors and vector capacity.
The native map allocation also contains those descriptors, so its entire size is not additional uncharged overhead.

### The earlier ordinary reference run contains a large RSS decrease

The saved first tour changed from 735,969,280 to 137,527,296 RSS bytes between two checked holds.
Those values are approximately 701.88 and 131.16 MiB.
Both holds contain nine successful raw `ps` queries for the same sole process-group member, PID 17426.
Intervening samples also show the decrease.

These are different views: `quantile-24797-unified` and `quantile-27276-split`.
Both expected screens retain the complete 76,872-file inventory and contain no loading indicator.
The saved samples show residency variation within the same process, not evidence that the application discarded its immutable indexes.
They cannot identify which pages, allocations, or host conditions caused that variation.

The [new analysis](../../evidence/source-span-attribution-v1/capture-analysis.json) preserves the raw hold and query records under `prior_ordinary_context`.
The original [comparison](../../evidence/source-span-prototype-v1/COMPARISON_REPORT.md) remains intact.
No observation is excluded or repeated because of this finding.

### The native region observations also favor the candidate, within diagnostic limits

At the first-tour endpoint, `vmmap` reported these rounded allocator totals:

| Native allocator field | Reference | Candidate |
| --- | ---: | ---: |
| Resident zone size | 820.5M | 783.9M |
| Allocated bytes | 720.7M | 683.7M |
| Reported dirty-plus-swap fragmentation size | 66.0M | 48.2M |

These are original native units and labels, not an exact disjoint partition of RSS.
The sampled RSS ranges during the sequential commands were also lower for the candidate.
This pair does not show increased candidate allocator fragmentation at those boundaries.
It cannot establish the allocator state during the earlier ordinary run.

## Limits and integrity

One reference process preceded one candidate process. This order does not remove shared-host effects or establish repeatability.
Stack logging changes allocator behavior. Its timings and memory observations cannot replace ordinary measurements.
The profiler also pauses for native commands and omits the ordinary one-second holds.

All native heap rows reconcile to their reported totals.
Allocation-stack totals exceed heap totals by 7,053,072 bytes at each large-profile boundary.
That discrepancy remains unassigned. It is not labeled allocator overhead or added to the heap.
The native commands run sequentially rather than as an atomic whole-process snapshot.

Independent read-only review checked the ownership paths, capture driver, analyzer, and saved RSS decrease.
The performance-audit method kept measured allocation sites separate from possible causes.
The documentation uses separate names for live allocations, logical storage, physical footprint, and RSS.

The capture driver has two failure-report limitations: an interrupted capture can omit host postlude fields and leave an inner record marked `running`.
The aggregate record still fails, the power monitor stops, and the raw log remains.
Neither limitation occurred in these successful captures or permits false acceptance.

The prior `CURRENT_STATE.md` remains unchanged because the previous completion record and this declaration bind its exact bytes.
This report supplies the newer decision without modifying that historical record.

## Next work

The next checkpoint qualifies complete local-source viewing with the normal build.
It covers actual viewport movement, repeated context expansion, coloring, resizing, cancellation, and cache revisits together.
No new production optimization is selected from this diagnostic alone.
The isolated candidate remains available, but no further capture for it is scheduled.

The normal executable SHA-256 remains `1ab24c0c70c45a5b480b83e111fa909d33ed56f4f4af9510b30dad72558f2e4e`.
The candidate SHA-256 remains `46c69f4974361e4ac4c31ea5ac8516c4c0319fcb0c5a122cb053265477a7a8fa`.
