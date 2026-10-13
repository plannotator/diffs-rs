# Source-span representation census

Date: 2026-09-23. Scoped, not implemented. The ordinary executable remains unchanged.

## Decision to establish

The [current ownership investigation](CURRENT_MEMORY_OWNERS_REPORT.md) counts 149.57 MiB of source spans and flags on the largest patch.
This is the largest individually counted document owner.
The same representation applies to every patch, independent of its repository.

The next task will measure whether shorter per-record length storage is worthwhile.
It will not yet change storage, parsing, source references, or rendering.
The result can be no change.

## Current invariants

Compact records retain file-relative `u32` offsets and full `u32` lengths.
A separate flag array retains exact no-final-newline state.
The wide fallback retains `u64` offsets.
Record ordinal, file base, byte offset, byte length, and newline state jointly identify the original source bytes.

The parser and range constructor reject invalid or overflowing ranges before publication.
Hunk headers and newline markers can occur between adjacent source records in the spool.
Therefore adjacent record offsets do not determine a source length.
No inferred-length shortcut belongs in this work.

## Bounded diagnostic

1. Preserve the current source and ordinary executable identities.
2. Add borrowed aggregate counts to an isolated diagnostic build, using the existing source-span owner.
3. Count records, storage form, length widths, file-relative offset widths, and newline-flag extents.
4. Report file eligibility as well as record eligibility for an exact `u16` length representation.
5. Include empty files, rejected inputs, boundary lengths, and wide-offset controls.
6. Run the same diagnostic across all five pinned inputs and the correctness fixtures.
7. Reconcile totals with the existing `source_index_bytes` summary before modeling any saving.

The observer must not clone records, read source bytes for each count, load syntax, or change the observed storage.
The ordinary build must remain unchanged.
It can visit borrowed records after publication and write one bounded aggregate result.
Per-file eligibility must account for every record, not a sampled prefix.
Actual input hashes and diagnostic source identities accompany the results.
No timing or ordinary RSS claim belongs to this census.

## Conditional representation model

If the census supports it, model separate boxed offset and length arrays for eligible files.
The simple candidate retains `u32` offsets and uses `u16` lengths only where every length fits exactly.
Files that do not qualify retain the complete current length domain and wide-offset fallback.
No source line is truncated, skipped, recolored differently, or assigned another identity.

The model must include extra slice descriptors, allocation count, alignment, and flags.
A six-byte logical record does not imply a six-byte Rust struct because padding can increase its size.
Separate arrays avoid packed unaligned reads, but add an allocation and an extra array access.
Those costs must remain visible in the design decision.

No unsafe code, custom allocator, general compression framework, global interner, smaller cache, or fixture-specific branch belongs in the candidate.
The census does not authorize an index rewrite or a disk-backed projection redesign.

## Later implementation gates

Any later storage change must preserve complete source and projection equivalence against the independent reference.
Relevant controls include empty records, lengths 65,535 and 65,536, large offsets, flags across byte boundaries, and mixed eligible/ineligible files.
Existing wide-range tests retain their full numeric domain without constructing multi-gigabyte test files.

Real-process checks must retain exact split/unified cells, inline emphasis, wrapping, resizing, anchors, navigation, cancellation, and cleanup.
The independent oracle, grammar coverage, workload, and cache limits remain unchanged.
Opening, coloring, responsiveness, and sustained-memory qualification follow correctness on preserved ordinary builds.
Accounted-byte reduction alone cannot establish adoption or whole-process improvement.
