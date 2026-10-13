# Retained index ownership: next change

Date: 2026-09-15. Status: review complete. No production change or new measurement.

The next candidate removes a duplicated boundary from replacement records.
This is a small index-storage change, not a solution to all retained memory.
The [implementation plan](REPLACEMENT_BOUNDARY_PLAN.md) preserves the current pairing and lookup algorithms.

## Current owner evidence

The preserved reference for the borrowed-parser experiment has exactly the same build-source hashes as the last kept build.
Its three saved index summaries agree for every pinned patch.
Their document extents also match the earlier metadata census.
The [offline reconciliation](../../evidence/index-owner-review-v1/reconciliation.json) records these checks without another viewer run.

The largest patch accounts for these disjoint document extents:

| Owner | Bytes | MiB |
| --- | ---: | ---: |
| Source spans and newline flags | 156,836,180 | 149.571 |
| Projection tables, including subindexes | 144,259,332 | 137.576 |
| Metadata allocations | 50,059,613 | 47.741 |
| File descriptors | 26,443,968 | 25.219 |
| Document file-vector capacity | 1,048,576 | 1.000 |
| Total | 378,647,669 | 361.107 |

These counts exclude allocator rounding, allocation headers, Arc headers, library allocations, caches, and stacks.
They are not a partition of the latest approximately 639 MiB native allocation total.
The native diagnostic and these counts come from different processes.

The same accounting covers all five inputs:

| Patch | Source records | Accounted document storage, MiB |
| --- | ---: | ---: |
| Pierre package cleanup | 18,420 | 0.207 |
| Ghostty library change | 93,579 | 1.275 |
| Node dependency update | 419,306 | 12.449 |
| Bun rewrite | 1,016,971 | 9.266 |
| Linux version comparison | 19,299,175 | 361.107 |

The costs follow record counts, change structure, and metadata, not repository identity.
The largest input exposes the cost most clearly, but does not define production policy.

## P2: replacement records duplicate a boundary

The [builder](../../src/projection.rs:435) appends old-side runs, records their end, then appends new-side runs.
It constructs `old: side_start..middle` and `new: middle..side_count`.
Both ranges therefore contain the same `middle` value.
Empty-side replacements also obey this relation.

The private [logical record](../../src/projection.rs:27) stores four endpoints and two padding values.
The [compact record](../../src/projection/storage.rs:77) stores the same six values as `u32`.
One start, one shared boundary, and one end can describe both ranges.
Both padding values remain independent and unchanged.

This representation removes one integer per replacement without an extra allocation, table, or lookup branch.
Expected record sizes are 24 → 20 bytes for compact storage and 48 → 40 bytes for wide storage on this target.
The compiler and storage checks must establish those sizes before performance claims.
Existing summaries combine replacement and display-block bytes, so they do not establish the exact recoverable total.

The largest input has 1,398,609 side-run entries.
Each replacement owns at least one such entry.
Its compact boundary saving is therefore at most 5,594,436 bytes, or 5.34 MiB.
The actual saving is smaller or equal and needs an exact count.
This ceiling is a reason to keep the implementation small, not a target to maximize.

The proposal makes the shared-boundary relation part of the private data structure.
It avoids a new semantic assertion in a conversion that currently checks numeric bounds.
Public source records, source coordinates, and displayed rows keep their current meanings.
Lookup speed and process-memory effects remain unmeasured.

## Why the larger alternatives come later

Source spans are the largest counted owner.
Their current compact form preserves full `u32` lengths and file-relative offsets.
Shorter length fields require a separate storage form, fallback, extra buffers, and evidence about eligible files.
Inferring lengths from adjacent offsets also requires correct treatment of headers and newline markers.
Neither change belongs in the replacement-boundary candidate.

Source runs retain a kind as well as old/new line-number presence.
The dependency parser enforces their relationship, but the application parser does not check that relationship explicitly.
The public parser model also permits direct construction.
Removing the kind requires a separate invariant review and adds decoding work.
The replacement boundary instead follows directly from its sole production constructor.

The [cache growth investigation](CACHE_GROWTH_REPORT.md) already establishes lookup-table growth under deletion and insertion.
A different table/value arrangement requires slot ownership, reuse, and lookup decisions.
The rejected [boxed-slice](STYLE_STORAGE_REPORT.md) and [boxed-value](STYLE_INDIRECTION_REPORT.md) experiments remain rejected.
The accepted IDs-only FIFO remains in place.

The [style census](STYLE_CENSUS_REPORT.md) also records equal-color token boundaries.
Their hypothetical reduction is smaller, and shorter vectors do not necessarily release capacity.
Lower cache weights can admit more entries, so token compaction is not automatically a whole-cache saving.

## Identity and limits

The default executable remains `1166825b79c6e8f4a2318c772820988d6a250b0663de15b4f84296bd581c9de7`.
The preserved same-source reference is `12fd72649d24c9bdd401317a5f1d1dab6d7396a839d137fa1dd0e0693be3250c`.
The six reviewed storage/input modules match the kept build and earlier metadata census.
The newer saved summaries also account for subsequent parser corrections.

Working highlighting source contains the deferred shared-prefix experiment.
A build from that working tree is not the kept baseline.
The next candidate must use an isolated copy of the kept source.
The borrowed-parser candidate and shared-prefix candidate remain separate and deferred.

Two independent read-only reviews covered cache ownership and index invariants.
Manual review covered storage, publication, projection construction, and their consumers.
Web, database, network, and browser performance are outside this terminal-storage review.
No viewer, profiler, benchmark, GUI, or build ran during this review.
The prior RSS and quit anomalies remain unexplained.
