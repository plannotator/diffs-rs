# Whole style-cache storage

Status: qualified and integrated on 2026-09-27.
The [checkpoint report](STYLE_CACHE_STORAGE_REPORT.md) records the results.
The preserved candidate completed ordinary comparisons and Rust 1.88 checks before integration.
The integrated executable passed correctness checks. Its diagnostic owner observations are a separate scope, not new ordinary performance measurements.

The original plan follows. It defines the accounting boundary, not a new cache policy.

## Problem

The current 32 MiB charge includes occupied style descriptors and their vector capacities.
It excludes unused table buckets, table control storage, keys, and the FIFO allocation.
The latest diagnostic recorded table growth during eviction without a new entry-count peak.
The [direct-token report](DIRECT_TOKEN_COLLECTION_REPORT.md) preserves that result and the decision against adoption.

The next step is accurate component accounting before any new retention policy.
It does not promise a 32 MiB total cache, lower RSS, or a faster lookup.

## Ownership contract

Each retained style has one immutable source identity and one FIFO entry.
The cache owns the style descriptor and its primary, optional old-side, and changed-span vectors.
The table owns its buckets, keys, inline values, control bytes, and layout padding.
The FIFO owns its full allocation capacity, including unused slots.

The diagnostic reports these disjoint extents:

1. The table allocation size reported by the pinned public container API.
2. All retained style-vector capacities, multiplied by their element sizes.
3. FIFO capacity multiplied by the source-ID size.
4. The inline cache size for the observed build.

Reported heap storage is the sum of the first three extents.
Reported value-plus-heap storage also includes the fourth.
Occupied key/value bytes are a subset of the table, not an additional allocation.
The existing style charge remains separate because it overlaps table values and vector storage.
Diagnostic-only inline fields are identified separately.

Allocator rounding, allocator metadata, free pages, transient resize allocations, stacks, worker replies, and process residency remain outside this sum.
The sum describes retained requested storage at the observation boundary.
It is informational and must not become an admission limit in this stage.

## Selected first implementation

Use `hashbrown::HashMap<LineId, LineStyle, std::collections::hash_map::RandomState>` for the isolated style cache only.
Pin the direct dependency to the existing lockfile version, 0.16.1, with default features disabled in that dependency declaration.
Keep `RandomState` explicit. Do not select a deterministic seed or the library's default hasher.
Keep the current duplicate check followed by `insert`. Do not introduce an `Entry` conversion.

The public [allocation_size API](https://docs.rs/hashbrown/0.16.1/src/hashbrown/map.rs.html#2052-2059) supplies the table extent for profiling.
Its pinned implementation returns the table's allocation layout size, not allocator usable size or RSS.
The diagnostic can therefore report table storage without copying private table layouts or adding unsafe application code.

The existing ratatui dependency already enables hashbrown defaults through another dependency.
The prototype must record effective feature unification. A direct declaration cannot disable features that another dependency requires.
The dependency declares Rust 1.65 and MIT OR Apache-2.0 licensing.
The application retains its Rust 1.88 requirement. Manifest compatibility is not a completed minimum-compiler build.

This is a container/API change that requires qualification, even though the package already exists in the lockfile.
It is not proof that standard and direct containers have identical allocation events.

## Alternatives not selected

| Option | Reason |
| --- | --- |
| Keep the standard map and multiply public capacity by entry size | Public capacity is not allocated storage. The result omits controls and can change during removals without allocation release. |
| Standard BTreeMap | A reasonable later behavioral control, but it changes lookup costs and does not expose node allocation sizes. |
| Dense slots, a custom table, or a custom allocator | More ownership and maintenance work than the current accounting question requires. |
| Periodic shrinking, smaller budgets, or hash-seed tuning | Changes policy before a complete storage account exists. |
| Boxed styles, boxed vectors, or direct token collection | Earlier experiments remain closed without adoption. |

## Unchanged behavior

- The 32 MiB style-charge formula and insertion-before-eviction order.
- FIFO victims, final survivors, and duplicate-ID handling.
- Token boundaries, foreground colors, changed spans, old-side distinctions, and fallback state.
- Patch and complete-source IDs, source replacement, stale-reply rejection, and unrelated-file survival.
- Wrapping, resizing, navigation, cancellation, and terminal cleanup.

No file name, language, corpus identity, or benchmark goal enters the implementation.

## Qualification sequence

1. Preserve normal source and executable identities. Build the prototype in a separate directory.
2. Add the disjoint storage report to the existing optional census. Keep historical records readable.
3. Check empty, populated, evicted, invalidated, and reset caches using real owned vectors.
4. Extend existing real source-replacement checks to reconcile the new storage fields.
5. Run existing ordinary and diagnostic correctness suites, including real PTY processes.
6. Review ownership, arithmetic, dependency features, formatting, and application Clippy.
7. Before adoption, compare preserved ordinary builds for responsiveness and sustained memory.

This first checkpoint ends after the accounting and correctness gates.
It does not rerun completed historical cohorts or adopt a new whole-cache budget.
The later performance comparison requires its own preserved candidate and declaration.

## Next isolated candidate: public entry insertion

Status: the experiment completed on 2026-09-27 without adoption.
It passed correctness checks but retained the same table allocations in both observed workloads.
The [result](STYLE_CACHE_STORAGE_REPORT.md#public-entry-insertion-experiment) closes this candidate. The original scope follows.

The retained table allocation is now measurable independently of the style charge.
The next candidate changes only the duplicate lookup and insertion to the public `Entry` API.
It does not change the container, hasher, source identities, budget, style weights, or eviction order.

The pinned dependency uses different insertion paths for `insert` and `VacantEntry::insert`.
The first path reserves space before its slot search.
The second path can reuse a deleted slot before it reserves additional space.
This difference motivates an experiment. It does not guarantee a smaller table, lower process memory, or faster navigation.
The product will use public APIs only. No private layout or growth threshold becomes application code.

Candidate requirements:

- Preserve duplicate styles without replacing their vectors or FIFO position.
- Preserve insertion before budget eviction, including oversized styles that immediately leave the cache.
- Preserve exact FIFO victims, final survivors, source invalidation, stale-reply handling, and theme reset.
- Keep activity snapshots at the same before-insert, after-insert, and settled boundaries.
- Keep all source bytes, tokens, colors, and terminal expectations unchanged.
- Keep randomized hashing. No seed selection or input-specific branch is permitted.

Qualification sequence:

1. Preserve the integrated reference and its source before any candidate edit.
2. Build the candidate in an isolated directory.
3. Check real-vector churn, duplicates, oversized styles, source transitions, and reset against an independent FIFO model.
4. Keep representation-specific capacity checks separate from that semantic model.
5. Run the existing correctness suites and explicit-executable grid comparison.
6. Compare retained component counts through the existing bounded owner traces.
7. If those checks support adoption, declare an ordinary responsiveness and memory comparison before integration.

All observed outcomes remain evidence, including larger allocations and slower cases.
This scope does not reopen rejected token-storage experiments or introduce periodic shrinking.
