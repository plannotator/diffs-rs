# Current allocation attribution

Date: 2026-09-26. The investigation is complete. No production change is adopted.

## Decision

The next candidate is direct collection of color tokens, without an intermediate fragment vector.
The [prototype plan](DIRECT_TOKEN_COLLECTION_PLAN.md) preserves every token boundary, color, side distinction, and fallback.
This is a candidate for evaluation, not a demonstrated memory saving.

The evidence does not justify smaller cache budgets, an allocator replacement, or a grammar-storage change.
The existing boxed-slice and boxed-value experiments remain rejected.

## Current-build observation

One diagnostic process completed the existing three-tour Bun workload.
All 418 exact terminal views, four retained-owner snapshots, twelve native commands, and 1,233 raw RSS queries passed their checks.
The process exited normally, restored the terminal, and left no process group or private files.
Power was battery, from 81% to 80%. The driver did not require AC.

The diagnostic executable is `95b374b9fcdcd92559427b7dc836f3e88ba9cb1ba0b6704f75cd1203bf28d729`.
Its runtime source matches the current source.
The ordinary executable remains `53d28a87bd1669910adfc6f28aedb20063ddb9db270310ad056da5a20a3bd838`.
No runtime source, dependency, cache policy, or executable changed.

MiB means 1,048,576 bytes. These columns have different scopes and must not be added.

| Boundary | Native live allocations, MiB | Charged styles, MiB | Unused style-vector capacity, MiB | Inner-engine estimate, MiB |
| --- | ---: | ---: | ---: | ---: |
| Opening | 23.11 | 0.01 | 0.004 | 6.86 |
| Tour 1 | 134.45 | 15.70 | 5.76 | 28.33 |
| Tour 2 | 158.60 | 25.58 | 9.35 | 31.08 |
| Tour 3 | 165.59 | 31.17 | 11.41 | 31.45 |

Unused vector capacity is already part of the charged styles.
Charged styles exclude some container storage and allocator costs.
Inner-engine values are strategy estimates, not native allocations or total regex storage.

## Findings

### P2: retained color output remains a concrete optimization target

Native live allocations increased by 32,651,536 bytes between tours one and three.
The direct color-output realloc class increased by 16,729,760 bytes.
The matched style-table and FIFO classes increased by 6,356,992 and 2,129,920 bytes.
Together, these disjoint classes account for 77.23% of that diagnostic increase.
Their allocation stacks connect them to color production and style acceptance.
The [table stack](/var/folders/xg/f67qq94s1qsbxl_nlh9hd_xc0000gn/T/pe-verify/native-diff/2026-09-26-current-attribution.JwrTF0/capture-001/tour-3-allocations.txt:28)
and [FIFO stack](/var/folders/xg/f67qq94s1qsbxl_nlh9hd_xc0000gn/T/pe-verify/native-diff/2026-09-26-current-attribution.JwrTF0/capture-001/tour-3-allocations.txt:34)
identify the matching allocation specializations.

The final cache held 83,371 entries with no eviction during this process.
Its population increased across tours. That growth includes useful retained colors, not only waste.
The final vectors retained 11,962,680 unused bytes, measured from their capacities.
These unused bytes are not a promise of equal process-memory savings.

[Color production](../../src/highlight.rs:168) first collects `HighlightIterator` into a fragment vector.
[Token conversion](../../src/highlight.rs:94) consumes that vector into `Vec<Token>`.
A direct iterator conversion can remove the intermediate representation.
The prototype must establish actual allocation behavior, because conversion can reuse the fragment allocation.
There is no evidence that two complete token buffers remain alive in the cache.

Smaller style charges can admit more entries under the same budget.
The rejected [boxed-slice experiment](STYLE_STORAGE_REPORT.md) demonstrates why lower vector storage alone is insufficient for adoption.
The prototype therefore requires whole-cache, responsiveness, and sustained-memory checks.

### Pages and parser state do not explain the repeated-tour increase

Source-page payload stayed at 33,148,575 bytes across all three tours.
The native page-buffer stack stayed at 41,435,136 bytes.
The [existing allocator control](SOURCE_PAGE_REPORT.md#allocation-observer-correction) explains the larger logged allocation class.
The current result does not show duplicate cached pages.

At the final snapshot, geometry charged 200,980 bytes and checkpoints charged 74,160 bytes.
Live parser and highlighter heaps totaled 5,393 bytes, excluding their inline values.
These results do not justify reducing their existing limits.

### Grammar and outer-regex storage remain secondary

The current application creates one worker-owned syntax set.
The review found no second application syntax set or missed opportunity to reapply existing engine sharing.

The pinned serialized grammar asset contains 959,011 bytes for the complete set.
Its content cannot explain a large-patch increase of hundreds of MiB.
This count excludes expanded contexts, spare capacity, and allocator overhead.
Releasing serialized blobs also affects serialization and `into_builder`.

The final allocation-stack grouping assigns 5,140,192 bytes to outer-regex and admission paths.
That grouping excludes stacks with inner-engine frames and is not a complete semantic-owner total.
Outer VM vectors remain unmeasured as direct Rust extents.
No new VM census is selected before the simpler color-output prototype.

### Allocator residency is real, but not an established fix

The final logged malloc-zone report shows 165.6M allocated, 181.1M resident, and 15.5M dirty-plus-swapped fragmentation.
Those rounded native fields overlap. They are not additional owners to add to the heap total.
Stack logging changes allocation classes and residency.
This capture does not establish ordinary allocator behavior or recoverable bytes.

## Integrity and limits

Native commands ran before owner serialization within each phase.
Earlier census allocations can still affect later allocator state.
Native commands and worker/application snapshots cover ordered, separate intervals.
The results do not partition ordinary RSS or measure physical terminal paint.

Heap rows reconcile exactly to each native heap total.
Stack-history totals exceed heap totals by 7,036,592 bytes at every boundary.
That discrepancy remains unassigned. Stack totals are supporting attribution, not extra heap bytes.

All input hashes, source identities, snapshot order, counter equations, and stop-before-quit boundaries passed the offline audit.
Three corrupted native reports were rejected.
One offline reader initially mixed Unix and monotonic clocks. The failed reader and correction remain preserved.
Only the offline reduction repeated. The viewer ran once.

The capture records power before and after, not a continuous power or sleep monitor.
Wall/monotonic disagreement was below one millisecond. This does not prove that no host sleep event occurred.
No new ordinary performance cohort, largest-patch qualification, or complete-source measurement ran.
The native reports identify their tool versions. Native command executable hashes were not frozen before capture.
The host remained shared. No exclusive host reservation is claimed.

## Evidence

- [Capture declaration and scope](/var/folders/xg/f67qq94s1qsbxl_nlh9hd_xc0000gn/T/pe-verify/native-diff/2026-09-26-current-attribution.JwrTF0/PLAN.md).
- [Capture output](/var/folders/xg/f67qq94s1qsbxl_nlh9hd_xc0000gn/T/pe-verify/native-diff/2026-09-26-current-attribution.JwrTF0/capture-001/report.json).
- [Reconciled allocations and owner records](/var/folders/xg/f67qq94s1qsbxl_nlh9hd_xc0000gn/T/pe-verify/native-diff/2026-09-26-current-attribution.JwrTF0/analysis.json).
- [Offline reader correction](/var/folders/xg/f67qq94s1qsbxl_nlh9hd_xc0000gn/T/pe-verify/native-diff/2026-09-26-current-attribution.JwrTF0/DEVELOPMENT.md).

The performance-audit review kept historical profiles separate from the current capture.
It ruled out speculative grammar changes and selected one application-level prototype.
