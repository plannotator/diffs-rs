# Shared replacement boundary

Date: 2026-09-15. Status: scoped. Implementation and qualification have not started.

## Purpose

Each replacement stores the boundary between its old-side and new-side run ranges twice.
The candidate stores that boundary once.
It preserves every range, padding value, pairing decision, and public coordinate.
This is one representation change, independent of the deferred parser and highlighting candidates.

## Representation

The private `Replacement` uses five named fields:

- `old_start: usize`
- `new_start: usize`
- `end: usize`
- `old_pad: usize`
- `new_pad: usize`

`old_range()` returns `old_start..new_start`.
`new_range()` returns `new_start..end`.
Both methods return ordinary stack `Range` values.
The two ranges cannot store inconsistent shared boundaries.

`CompactReplacement` mirrors these five fields with `u32`.
Every narrowing conversion keeps `TryFrom`.
The existing compact-selection bound remains unchanged.
Wide storage continues to use the private `Replacement` and full `usize` values.

The production constructor already supplies `side_start`, `middle`, and the final side count.
The side count never decreases between these observations.
Old-only and new-only changes produce an empty range on the corresponding side.
Interleaved source records retain the current side-run ordering.

Expected compact storage decreases by four bytes per replacement.
Expected wide storage decreases by one `usize` per replacement.
These are record-layout expectations, not measured allocator or RSS savings.
No new allocation or lookup branch is necessary.

## Boundaries

Only `src/projection.rs` and `src/projection/storage.rs` need production changes.
The parser model, alignment score, source spans, markers, hunk boundaries, and source-run representation remain unchanged.
Both padding values remain explicit.
Caches, grammar coverage, worker policy, spool access, and terminal behavior remain unchanged.

No bit packing, unsafe code, custom allocator, source truncation, or repository-specific rule belongs in this candidate.
No smaller cache budget or altered workload belongs in its qualification.
The independent projection reference remains unchanged.

## Implementation and correctness tasks

1. Preserve the current source and default identities.
2. Create an isolated source directory from the preserved kept archive.
3. Check every extracted build input against the kept manifest.
4. Implement the five-field private records and their range accessors.
5. Update the sole production constructor and its existing consumers.
6. Check compact and wide record sizes with the compiler.
7. Extend the existing boundary tests without a new test framework.
8. Run formatting, application Clippy, and the release contracts.
9. Run complete source and projection checks across all five pinned patches.

The boundary tests must cover old-only, new-only, equal, unequal, and interleaved replacements.
They must include empty ranges, nonzero padding on either side, numeric maxima, and failed narrowing.
Both forced storage forms must match actual parsed rows, inverse destinations, partners, and boundary navigation.
Document-global coordinates above `u32::MAX` remain supported.

Existing real-process contracts cover split/unified layouts, wrapping, resizing, source anchors, highlighting, input errors, and terminal cleanup.
The existing alignment contract must remain exact.
Complete corpus checks must preserve all 20,847,451 supplied records and both projection domains.
No mock replaces a parser, terminal process, or independent source oracle.

## Qualification tasks

After correctness passes:

1. Preserve ordinary reference and candidate binaries with their exact build records.
2. Compare existing index summaries across all five inputs.
3. Check that only replacement storage changes within the disjoint owner accounting.
4. Review the exact storage reduction and code complexity before the runtime series.
5. If the candidate remains worthwhile, use the existing protocols for the fixed runtime series.
6. Audit every view, hold, sample, source identity, and cleanup result.
7. Record the tradeoff and adoption decision separately.

The runtime series contains one fixed focused response pair with capture-first observation.
It also contains three parse/index pairs with alternating role order and one three-tour memory pair per patch.
No runtime series starts merely to produce a favorable number for a negligible storage change.

The compact-versus-wide replacement counts must reconcile the exact shared-block byte reduction.
The same ownership checks must include temporary builder storage and retained descriptor sizes.
Allocator rounding can change independently of logical extents.
No native profile is part of this initial qualification.

The existing actions, dimensions, holds, oracles, sample protocol, and watchdogs remain unchanged.
The driver must record power and sleep conditions and use disconnect-safe optional progress.
The measured series requires AC throughout.
Known competing measurement work must pause before the series starts.
Shared-host limits remain explicit.

There is no automatic replacement trial, discarded warm-up, or retry after a failed role.
The qualification must preserve slower or larger observations.
Favorable storage arithmetic does not override a correctness failure or unexplained adverse behavior.

## Completion

The candidate needs exact behavior and a defensible storage/runtime tradeoff before adoption.
The default remains unchanged during qualification.
An inconclusive result leaves the candidate isolated and deferred.
A rejection retains all evidence without changing historical results.
This plan establishes no general speedup, total-memory limit, or physical-terminal acceptance.
