# Style-cache lookup: review and next prototype

Date: 2026-09-27. Status: isolated prototype passed its first correctness checkpoint. Not adopted or performance-qualified.

## Implementation checkpoint

The isolated candidate passed 125 ordinary checks and 147 combined diagnostic/tracing checks.
Its preserved ordinary executable passed all 61 enabled process checks.
The normal-reference comparison matched 96 complete grids and two canonical source streams.
Seven real-process diagnostic controls passed.
The final reader passed ten source-transition records, 32 activity corruptions, and 33 retained records across three activity schemas.
An independent review identified one impossible-peak reader gap. The corrected reader rejected the reproduced case.
Formatting and strict application Clippy passed. The absent Rust 1.88 toolchain leaves current MSRV qualification open.

The normal source, executable, fixtures, and dependencies remain unchanged.
Container-storage schema 2 reports the `std-btree` provider and explicitly unavailable totals.
Activity schema 3 retains semantic counters without invented capacity data.
The checkpoint report (temporary verification record `2026-09-27-tree-cache.bCkChG/report.html`, not retained) records the evidence and retained engineering failures.
The ordered tasks describe the original plan. Tasks 1 and 2 are complete except for the unavailable MSRV check.
Task 3 remains the next gate. This checkpoint does not establish memory savings or responsiveness.

## Finding: P2 retained table storage exceeds occupied entries

The cache stores 80-byte style descriptors and 16-byte source identities in one hash table on the measured arm64 build.
The 32 MiB charge includes occupied styles and their vector capacities, but not all table or FIFO storage.
Repeated navigation therefore has a larger retained footprint than the style charge.
This is an ownership cost, not a demonstrated leak or incorrect diff.

The preserved integrated diagnostic produced these final observations:

| Input | Entries | Table bytes | Occupied keys and values | Table remainder | Style vectors | FIFO bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Bun | 84,997 | 12,713,992 | 8,159,712 | 4,554,280 | 26,753,936 | 2,097,152 |
| Linux | 113,370 | 50,855,944 | 10,883,520 | 39,972,424 | 24,484,744 | 4,194,304 |

The remainder subtracts occupied key and style extents from the complete table allocation.
It includes unoccupied slots, controls, and layout padding. It is not a measured amount that another container can save.
The Linux remainder is about 38.12 MiB. The complete table is 48.50 MiB.
These figures are requested storage, not process RSS, allocator retention, or total application memory.

Linux recorded a capacity high-water increase during an admission that also evicted entries, without a new population peak.
Its table retained 50,855,944 bytes at every end-of-tour snapshot.
The records do not measure allocation bytes at each admission.
The public-entry insertion experiment retained the same table allocations and remains closed without adoption.
Neither input name nor its measurements will control product behavior.

Evidence: [integrated storage report](STYLE_CACHE_STORAGE_REPORT.md#integration-and-bounded-owner-observations),
raw final observations (temporary verification record `2026-09-27-cache-integration.IhX6h6/owner-analysis.json`, not retained),
[cache ownership](../../src/highlight.rs:747), and [charge calculation](../../src/highlight.rs:74).

## Decision

The next isolated candidate uses `std::collections::BTreeMap<LineId, LineStyle>` for lookup.
The existing `VecDeque<LineId>` remains the sole eviction-order authority.
The candidate keeps every style value, source identity, vector capacity, and charge unchanged.
This is a controlled container experiment, not a claim that trees are generally better caches.

Rust documents logarithmic lookup, insertion, and removal for `BTreeMap`, versus expected constant-time operations for `HashMap`.
It also identifies hash maps as the usual choice for caches.
The measured retained owner justifies this exception as an experiment, not an automatic replacement.
See the [standard collection guidance](https://doc.rust-lang.org/std/collections/index.html#use-a-hashmap-when) and
[operation costs](https://doc.rust-lang.org/std/collections/index.html#cost-of-collection-operations).

The tree stores multiple entries in each node.
It changes allocation and traversal behavior without adding a separate style allocation for every entry.
The [public API](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html) supports the required operations.
It supplies no public node-allocation-size counter.

### Alternatives

| Alternative | Decision and reason |
| --- | --- |
| Keep the current map | Accepted production baseline. No change remains a valid experiment outcome. |
| Standard BTreeMap | Selected for an isolated control. It changes one owner and needs no new dependency or custom storage manager. |
| Hash map plus reusable slots | Deferred. It adds slot ownership, vacancy reuse, a free list, dependent reads, and another retained allocation. |
| Hash map plus packed values | Deferred. Each moved value needs its identity and a corrected map index after removal. |
| Split key spaces or compact optional values | Deferred. They add representation cases without evidence that those cases solve the retained-table mechanism. |
| Boxed values/vectors, direct token collection, source-span changes, Entry insertion | Prior decisions remain closed. This review does not reopen them. |
| Smaller budgets, periodic shrink, fixed seeds, custom allocators, input-specific rules | Out of scope. They change policy or add complexity without isolating this question. |

Dense-storage variants also retain hash-table churn.
Slot numbers must not replace stable source identities in the FIFO.
Otherwise, slot reuse introduces stale-handle risks.
No proposed storage can allocate according to the numerical magnitude of a source ID.

## Correctness and ownership contract

1. Keep `LineId` unchanged, including full-width patch/file IDs and positive context coordinates.
2. Keep `LineStyle` unchanged, including exact token ranges, colors, spans, fallback state, and old-side distinctions.
3. Preserve the difference between absent old tokens and present-but-empty old tokens.
4. Keep duplicate rejection before charging, insertion, and FIFO changes.
5. Keep insertion and FIFO append before the existing eviction loop.
6. Keep the 32 MiB threshold and capacity-based weight calculation unchanged.
7. Keep exact FIFO victims, survivors, and oversized-entry behavior.
8. Keep source invalidation, unrelated-file survivors, stale-reply rejection, and complete theme reset unchanged.
9. Keep lookup results borrowed from the cache. Do not add cloning, reference counting, or shared mutable ownership.
10. Keep rendering, scheduling, navigation, wrapping, and source acquisition outside the candidate.

`LineId` already derives equality and ordering together.
The invalidation callback uses membership and charge subtraction, not table iteration order.
The renderer obtains styles by ID, not by traversal order.
A tree's sorted iteration therefore has no intended user-visible meaning.
The implementation must not use key order to choose eviction victims.

Source paths: [identities](../../src/view.rs:11), [invalidation](../../src/highlight.rs:756),
[duplicate and eviction behavior](../../src/highlight.rs:776), [reply rejection](../../src/app/source.rs:152),
[theme reset](../../src/app.rs:439), and [render lookup](../../src/render.rs:593).

## Responsiveness risks

The current UI thread performs cache lookups during rendering, pending-status calculation, and highlight requests.
The tree adds comparisons and node traversal to those paths.
Insertion, removal, invalidation, and reset can also incur different node-maintenance and deallocation costs.
More compact retained storage does not prove a faster viewer.

The first candidate retains `contains_key` followed by `insert`.
It does not combine container replacement with another insertion optimization.
Duplicate lookups in status code also remain unchanged.
This boundary makes an adverse result attributable to a smaller change set.

Sources: [visible requests](../../src/app.rs:447), [status](../../src/render.rs:209),
[complete-source status](../../src/app/source.rs:173), and [style receipt](../../src/app/source.rs:152).

## Diagnostic contract

The existing diagnostics describe hash-table capacity and allocation bytes.
Those meanings cannot transfer to a tree.
The candidate must preserve semantic counters while reporting unavailable representation data explicitly.

The isolated candidate needs a distinct, versioned provider record:

- Container provider: `std-btree`.
- Node-allocation bytes: unavailable, represented by `null`.
- Total retained cache heap and value-plus-heap: unavailable, represented by `null`.
- Vector capacities, FIFO capacity, inline cache size, and diagnostic inline size: measured with existing methods.
- Occupied key and style extents: retained as overlapping value extents, not additional heap allocations.
- Hash capacity, capacity transitions, and capacity high-water data: unavailable, not entry count or zero.

The partial known heap subtotal contains vector and FIFO capacities only.
It must not appear under an existing total-heap field.
Admission, duplicate, eviction, invalidation, population-peak, and charged-byte counters remain exact.
Historical hash-table records keep their current reader and conservation checks.

Before implementation, the candidate schema must name its new versions and required fields.
The reader must reject false zero totals, fabricated capacity, wrong providers, and inconsistent semantic counts.
No generic container trait, runtime backend option, custom allocator, or copied private node layout is necessary.

Sources: [current census](../../src/highlight/census.rs:137),
[capacity-specific activity](../../src/highlight/cache_activity.rs:12), and [reader checks](../../tools/owner_census.py:210).

## Ordered implementation tasks

### 1. Preserve and implement the isolated candidate

Preserve the current source inventory and normal executable.
Use a separate source copy and build directory.
Keep the existing hashbrown dependency declaration during the experiment to avoid unrelated dependency changes.
Replace only the style lookup container and adapt its representation-specific diagnostics.
Finish the ordinary, tracing, and owner-census configurations before the first checkpoint.

### 2. Prove semantic equivalence

Separate hash-specific capacity assertions from container-independent correctness checks.
Retain the reference's original capacity checks in the preserved reference.
Do not weaken survivor, source, color, or terminal expectations.

Use one independent FIFO model with real owned vectors.
Check sequential, reverse, and interleaved admissions, mixed/extreme valid IDs, duplicates, reinsertion, multiple evictions, oversized styles, invalidation, and reset.
Compare survivors, order, charge, vector contents, side distinctions, and fallback state after every operation.
Record original vector capacities separately. A cloned vector does not establish an identical capacity or charge.
Reuse the real source-transition tests for replacement, revocation, recovery, stale replies, and unrelated-file survival.

Run the existing ordinary and diagnostic suites.
Run the preserved ordinary candidate through all enabled process checks with the corrected executable override.
Compare all 96 complete grids and both canonical source streams against the preserved normal executable.
Run formatting, application Clippy, and the declared Rust 1.88 build checks without changing global toolchains.
If that compiler is unavailable, keep its qualification gate open.

This checkpoint ends with a correct isolated candidate and truthful diagnostics.
It does not adopt the candidate or claim a performance improvement.

### 3. Qualify memory and responsiveness before adoption

Declare the reference, candidate, workload, order, and stop conditions before each comparison.
Use the existing five-patch schedules and local complete-source cases.
Preserve opening-text, coloring, navigation, resize, cancellation, exit, and sustained-memory observations separately.
Include warm revisits as well as cold reads. Do not infer interactive cost from container-operation timings.

Use native allocation tools only where the existing traces cannot identify retained node ownership.
Attribute node allocations to the cache insertion path, not every B-tree allocation in the process.
The application already has other B-tree owners.
Keep native live heap, requested component storage, and ordinary RSS distinct.
If attribution fails, retain the uncertainty and do not invent a complete cache total.

Retain every adverse result and incomplete capture.
Record power and host conditions without requiring AC.
Do not repeat completed trials to seek favorable numbers.
Any additional comparison needs a specific unresolved question and a declared scope.

Adoption requires exact correctness and a defensible overall memory/responsiveness tradeoff.
Smaller descriptors or a favorable largest-input result alone are insufficient.
A tree that increases ordinary-case cost without enough benefit remains unadopted.

## Review boundary

A separate read-only reviewer examined ownership, invariants, alternatives, and diagnostic limitations.
The main review checked those findings against the cited source.
No correctness defect was found in the inspected cache paths.
This is not proof for every possible input.

The normal executable remains `9350f499bda0f9442234ce856992ceb3756e0a9324302ee2a6dd62ef04dfa630`.
All 2,085 production files still match the qualified baseline.
No product source, fixture, test expectation, dependency, executable, or cache policy changed during this review.
No new build, process suite, performance capture, or physical-terminal check ran.
The review covers style-cache representation only, not another broad audit of the renderer, parser, network, or release system.
