# Direct color-token collection

Status: closed without adoption. The isolated source and all evidence remain preserved.
The [qualification report](DIRECT_TOKEN_COLLECTION_REPORT.md) records the completed correctness, owner, responsiveness, and sustained-memory comparisons.
The bounded ownership/residency investigation is complete.
The prototype reduced vector slack, but retained more live heap with a larger cache table in the matched diagnostic pair.
The normal viewer remains unchanged. A separate cache-storage design task follows, not another performance cohort for this candidate.

## Purpose

Remove the intermediate fragment vector from visible-line color production.
Keep the existing `Vec<Token>` representation and every rendered result.
The [current allocation profile](CURRENT_ALLOCATION_ATTRIBUTION.md) identifies color output as a material retained owner.

## Proposed implementation

1. Preserve the current source, executable, and diagnostic as references.
2. Use an isolated source copy for the candidate.
3. Let `tokens` accept an iterator of the same `(Style, &str)` items.
4. Pass `HighlightIterator` directly, without the first `.collect()`.
5. Keep existing callers that provide owned vectors through `IntoIterator`.

The function must consume the iterator completely, in its original order.
Each fragment must produce the same byte range and RGB color, including empty ranges.
The parser, highlighter state, grammar, old/new independence, and fallback paths must remain unchanged.
The resulting token vector must own its ranges and colors without borrowing source text.

There is no token merging, new sharing, boxed storage, explicit shrinking, custom allocator, pooling, or capacity threshold.
Changed-word ranges, source identities, cache budgets, weights, duplicate handling, invalidation, and FIFO order stay unchanged.
The existing weight calculation will still use actual vector capacity.
That can change admission behavior, even with identical token contents.

## Why this is a separate candidate

The rejected boxed-slice candidate retained the existing builders and changed finalized storage.
This candidate retains finalized storage and changes the production path.
The old decision remains intact and does not become a successful result for this candidate.

An intermediate vector does not imply two retained allocations.
The compiler can reuse a consumed vector allocation during conversion.
The first checks must compare actual output capacities and token equality, without assuming an allocation reduction.
A different iterator collection path can also change growth costs.

## Qualification order

1. Compare exact token sequences and resulting highlighter states with the preserved conversion.
   Use real grammars, both themes, multiline states, tabs, Unicode, empty lines, and varied fragment counts.
   Include old/new sides with different syntax state and declared fallback cases.
2. Record actual token lengths and capacities.
   Report both larger and smaller cases. Do not select a special input size or grammar.
3. Run the existing ordinary suite and meaningful worker/full-source contracts.
   Check canonical source output, side-specific colors, inline emphasis, navigation, wrapping, resizing, cancellation, and cleanup.
4. Run the existing compatible-baseline terminal grids against preserved binaries.
   Keep all source and color expectations unchanged.
5. Use the current owner diagnostics to check cache entries, payload, map capacity, FIFO storage, and evictions.
   Do not equate reduced per-line capacity with reduced whole-cache memory.
6. Compare ordinary builds on the existing focused-response and five-patch sustained workloads.
   Preserve all adverse observations, power conditions, failures, and binary identities.

Keep the candidate isolated until correctness and the overall resource tradeoff justify adoption.
Do not revise cache policy to rescue an unfavorable result.
If capacities, coloring responsiveness, or sustained memory regress without a defensible tradeoff, reject the candidate and retain its evidence.

## Exclusions

This task does not change inline emphasis, grammar coverage, source context, search, copy, or terminal input behavior.
It does not add regex instrumentation or claim that remaining allocator residency is understood.
The current normal executable stays usable throughout the experiment.
