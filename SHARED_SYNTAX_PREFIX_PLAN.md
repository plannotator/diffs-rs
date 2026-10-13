# Shared leading syntax state

Date: 2026-09-15. Status: implementation and qualification complete. Promotion deferred.

The [checkpoint report](SHARED_SYNTAX_PREFIX_REPORT.md) records the results and unresolved ordinary-memory observation.
The original pre-implementation scope remains unchanged in `evidence/shared-syntax-prefix-v1/PLAN.md`.

## Reason

Both sides currently start with identical syntax state and parse leading common source independently.
The current-build sample still places most observed syntax work in parser and regex search execution.
Sharing this exact prefix removes duplicate work without a new matcher or capture API.

The benefit depends on input structure, not repository identity.
Leading prefixes account for about 50% of two-side source visits in the deep fixture.
They account for 0.14–9.82% across the five real patches.
Those counts describe complete input traversal, not timed workload coverage or predicted speedup.
The [investigation evidence](evidence/cold-search-review-v1/source-shape.json) retains every input and the limitations.

## Ownership and behavior

Each new hunk owns one complete side state until its first side-only source record.
The absent second state explicitly means that both histories share the first state.
Before a side-only record, clone the complete state into a separately owned new side.
Perform this separation before parsing or long-line invalidation.
Keep both states separate afterward, even if their colors or parser stacks become equal.

The state includes the parser, complete highlighting state, pending-color flag, and validity flag.
For shared context, parse once and publish one token vector with no old-side override.
For separated state, retain the current independent old/new processing and token comparison.
A shared parser or scope failure invalidates both represented histories.
A side-only failure after separation affects only that side.

Checkpoints preserve whether state is shared or separate.
Shared checkpoints own one saved parser/scope state. Separate checkpoints own two.
Restoration retains that distinction and the existing lazy color reconstruction.
Heap accounting counts actual owners and preserves overflow rejection and all existing limits.
Source progress, cancellation cadence, checkpoint intervals, and publication guards remain unchanged.

Diagnostics report actual parser calls and explicitly identify a call that serves both sides.
One shared call must never appear as two measured calls.
The updated reader must retain support for earlier trace schemas.

## Exclusions

No reconvergence, speculative work, background pre-parsing, changed grammar, new regex API, or extended scratch lifetime.
No changes to source bytes, line pairing, cache limits, fallback thresholds, or navigation.
No fixture-specific branch or altered measurement target.
This change does not remove required prefix work after an early edit.

## Ordered checks

1. Bind the reference to the kept ordinary executable and its archived source.
2. Add an actual-worker contract for a common captured-delimiter prefix followed by divergent closes.
3. Check that contract against the unchanged implementation before production edits.
4. Implement explicit shared state and checkpoint ownership.
5. Compare exact old/new tokens with independent sequential highlighting through checkpoint restore and divergence.
6. Cover side-only oversized input, invalidation, cancellation, theme changes, and pending-highlight resize with real processes and grammars.
7. Run formatting, Clippy, process contracts, source/projection checks, and preserved sequential-token comparisons.
8. Run three alternating ordinary pairs of the unchanged five-case actions, with all observations retained.
9. If the tradeoff remains worthwhile, run the unchanged five-patch, three-tour memory and reading checks.
10. Rebuild and check the normal executable only after the keep decision. Preserve rejected source and restore the reference otherwise.

Any wrong source, side, color, reading position, or cleanup blocks adoption.
Unexplained adverse responsiveness or memory observations require investigation.
No general speedup, RSS saving, physical terminal acceptance, or release completion follows from fewer parser calls alone.
