# Current retained-memory owners

Date: 2026-09-23. Diagnosis complete. The ordinary executable and runtime source remain unchanged.

## Findings

Two different owners deserve attention.
The largest patch retains 361.82 MiB of counted document indexes and metadata.
The new Bun allocation profile instead locates most repeated-tour growth in color-token output and style-cache containers.
Neither observation proves a leak or identifies dispensable data.

The next bounded task is a [source-span representation census](SOURCE_SPAN_CENSUS_PLAN.md).
Source spans are the largest individually counted document owner.
The census will establish whether narrower length storage has enough benefit to justify a representation change.
It will not change rendering or reduce the supported source domain.

### P2: immutable document storage remains substantial

The unchanged ordinary executable ran the existing index-accounting command against all five pinned patches.
These counts describe owned Rust extents, not allocator-sized heap or RSS.

| Input patch | Source records | Counted document storage, MiB |
|---|---:|---:|
| Pierre `0800fb` | 18,420 | 0.21 |
| Ghostty `12291` | 93,579 | 1.26 |
| Node `59805` | 419,306 | 12.53 |
| Bun `30412` | 1,016,971 | 9.36 |
| Linux `v6-v7` | 19,299,175 | 361.82 |

The largest document has these disjoint extents:

| Owner | Bytes | MiB |
|---|---:|---:|
| Source byte ranges and newline flags | 156,836,180 | 149.57 |
| Projection tables, including subindexes | 140,767,788 | 134.25 |
| Metadata allocations | 53,071,108 | 50.61 |
| File descriptors | 27,673,920 | 26.39 |
| Document file-vector capacity | 1,048,576 | 1.00 |
| Total | 379,397,572 | 361.82 |

The [document](../../src/storage.rs:205) owns file handles through `Arc`.
Each [file](../../src/storage.rs:49) owns immutable metadata, source spans, and projection tables.
Navigation does not release these tables while the document remains open.
The projection total already contains its subindexes. Adding those subindexes again double-counts them.

Compact [source spans](../../src/source_spans.rs:19) retain one `u32` offset and one `u32` length per record, plus newline flags.
Each file retains its own offset base and flag array.
The wider fallback preserves larger relative offsets.
The design preserves exact byte ranges without rereading patch headers or inferring lengths from adjacent records.

Impact: input-scaled storage remains large even though source text lives on disk.
These counts exclude the tree, caches, library allocations, allocation headers, rounding, and stacks.
They cannot be subtracted from a separate RSS or heap observation to establish an exact remainder.

Direction: measure eligible length widths before changing span storage.
The complete source domain, flags, record identities, range checks, and wide fallback must remain intact.
An optimization must work by data properties, never by repository name or benchmark identity.

### P2: styles and their containers explain most observed Bun heap growth

One new instrumented process used the current ordinary executable and the corrected three-tour Bun trace.
All 418 complete-screen expectations passed. The process exited normally and restored terminal state.

| Boundary | Native live allocations, bytes | MiB |
|---|---:|---:|
| Opening | 24,229,760 | 23.11 |
| Tour 1 | 143,752,480 | 137.09 |
| Tour 2 | 168,747,264 | 160.93 |
| Tour 3 | 175,677,792 | 167.54 |

Tour one to tour three adds 31,925,312 bytes, or 30.45 MiB.
The following three disjoint native heap-class changes account for 79.62% of that increase:

| Allocation evidence | Increase, bytes | MiB |
|---|---:|---:|
| Direct color-token output site | 16,671,360 | 15.90 |
| Hash-table class, matched to style-cache growth | 6,881,280 | 6.56 |
| Vector-growth class, matched to style FIFO growth | 1,867,776 | 1.78 |
| Combined | 25,420,416 | 24.24 |

The [visible coloring path](../../src/highlight.rs:168) constructs token vectors through `Side::advance`.
Its direct realloc class is an allocation-site count, not a census of cached token payload alone.
The profile does not separate every cached value, pending reply, and current work item at that site.

The cache insertion call was inlined into [highlight acceptance](../../src/app/source.rs:150).
Its table allocation increased from 6,897,664 to 13,778,944 bytes.
Its FIFO allocation increased from 1,589,248 to 3,457,024 bytes.
Those call-site changes exactly match the corresponding heap-class changes between the same two boundaries.
Other allocations in those classes have an unchanged combined total across those boundaries.

Explicitly named regex classes add another 2,400,912 bytes, or 2.29 MiB.
This is a partial engine count. Generic allocation sites can also contain engine-owned storage.
The evidence does not justify another broad regex-engine rewrite.

The [style cache](../../src/highlight.rs:744) retains completed answers under the existing 32 MiB payload allowance.
Its charge includes style descriptors and vector capacities, but excludes keys, unused hash buckets, FIFO storage, and allocator overhead.
The [stable source identity](../../src/view.rs:15) occupies each map key and FIFO entry, including patch-only entries.
The context-capable enum therefore has a container cost outside the existing payload charge.

This profile did not collect cache entry counts or spare token capacities.
It cannot distinguish useful new cache entries from unnecessary representation cost by itself.
The historical [boxed-slice](STYLE_STORAGE_REPORT.md) and [boxed-value](STYLE_INDIRECTION_REPORT.md) candidates remain rejected.
Smaller payloads can admit more entries, so repeating those changes without new evidence is not justified.

Direction: keep cache representation as a separate candidate, with current entry/capacity accounting before implementation.
Lower cache budgets, aggressive eviction, periodic shrinking, and altered coloring coverage are not selected.

### P2 coverage gap: the existing census is not a whole-application owner map

The [owner census](../../src/owner_census.rs:117) reports styles, engine data, context-view counters, and UI scalars.
It does not provide complete document, tree, geometry, page-cache-container, source-descriptor, continuation, checkpoint, or reply accounting.
Its [context counters](../../src/view.rs:197) state their exclusions explicitly.
No current observation supplies a complete disjoint partition of the process heap.

The sidebar [builds the tree before deciding visibility](../../src/app/sidebar.rs:120).
Hiding it does not release or avoid that storage.
The [tree](../../src/tree.rs:82) retains nodes, raw path components, labels, file mappings, row mappings, and vector capacity.
Its size follows the document inventory, not just visible rows.
The reviewed navigation path does not append duplicate trees on each tour.

Tree storage is a possible future representation target, not an established cause of repeated-tour growth.
Some tree construction is inlined, so selected named stacks are not a complete tree total.
If tree bytes become the next decision point, a narrow counter is preferable to a general instrumentation framework.

## Other reviewed owners

| Owner | Retention and limits |
|---|---|
| Source text | Private disk stores. Shared page payload remains bounded at 32 MiB. Containers and borrowed pages are additional owners. |
| Geometry | At most 1,024 cached line/width entries and 16 MiB of charged payload. Multiple widths can retain separate text and wrap data. |
| Parser checkpoints | At most 64 retained entries, 16 MiB total charged bytes, and 256 KiB per admitted checkpoint. |
| Worker replies | Highlight replies have a bounded channel. Pending requests replace obsolete requests rather than accumulating indefinitely. |
| Complete local sources | Prepared source pairs and provider metadata remain owned by the review. Bodies and disk line indexes share the source-page allowance. |
| Expanded context | Segment storage follows checked gaps and hunks rather than allocating one descriptor per revealed line. |

The review covered [page storage](../../src/storage/bytes.rs:14), [geometry](../../src/geometry.rs:120), and [checkpoint ownership](../../src/highlight/checkpoint.rs:10).
It also covered [prepared sources](../../src/input.rs:303) and [context installation](../../src/view.rs:406).
Source-mode descriptors and expanded context were not exercised by the patch-only profile.
Their sustained-resource qualification remains separate.

No reviewed path establishes unbounded retained growth across identical tours.
That is not a proof of leak freedom or a total-memory bound.
Temporary work, allocation history, operating-system residency, and library storage remain relevant.

## Measurement and integrity limits

The ordinary executable remains `1ab24c0c70c45a5b480b83e111fa909d33ed56f4f4af9510b30dad72558f2e4e`.
No build, runtime edit, dependency change, cache-policy change, or new ordinary benchmark occurred.
The [pre-run declaration](../../evidence/current-memory-owners-v1/PLAN.md) specified five document counts and one native Bun profile.

Only the viewer child received `MallocStackLogging=1`.
The existing diagnostic replayed every declared action and expected screen, but omitted ordinary one-second memory holds.
It paused at four boundaries for sequential native commands.
Instrumentation and those pauses can change execution, allocations, and residency.
These results cannot retrospectively partition the earlier ordinary RSS increase or prove a performance improvement.

All native commands succeeded. Every native heap row reconciles to its phase total.
Allocation-stack totals exceed heap totals by 7,053,072 bytes at every boundary.
That discrepancy remains unassigned. Stack totals are supporting evidence, not extra bytes to add to heap totals.
The broader direct-coloring stack selector also includes 1,696 bytes outside the specific realloc-class increase.
The attribution table uses the exact heap class, not that broader selector.

The first offline supplement selected unrelated hash-table specializations and failed its equality check.
The [correction record](../../evidence/current-memory-owners-v1/DEVELOPMENT.md) preserves the failed reader.
Only offline reduction repeated. No capture repeated or changed.

The audit checked all five fixture hashes after capture, closing the driver-only postflight coverage gap.
Source and helper identities matched. AC power, no sleep, sampler shutdown, normal exit, and absent viewer processes all passed.
The host remained a shared workstation. No physical-terminal result follows from these checks.
An independent read-only review checked current ownership paths and the reported arithmetic.

## Evidence

- [Five-patch document counts](../../evidence/current-memory-owners-v1/capture-001/index.json).
- [Diagnostic execution record](../../evidence/current-memory-owners-v1/capture-001/report.json).
- [Native profile and ordered view checks](../../evidence/current-memory-owners-v1/capture-001/profile/report.json).
- [Reconciled heap and saved RSS context](../../evidence/current-memory-owners-v1/analysis.json).
- [Inlined allocation-site attribution](../../evidence/current-memory-owners-v1/attribution.json).
- [Next bounded task](SOURCE_SPAN_CENSUS_PLAN.md).

The performance-audit method kept measured owners separate from source-level possibilities and preserved the unsuccessful historical candidates.
The documentation separates owned extents, native allocations, and RSS instead of combining incompatible totals.
