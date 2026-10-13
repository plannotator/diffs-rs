# Bounded reuse of complete-source parser state

Date: 2026-09-22. Status: implementation, correctness checks, and the declared ordinary comparison are complete.
The [implementation report](LOCAL_SOURCE_REPLAY_REPORT.md) records the code and correctness evidence.
The [comparison report](LOCAL_SOURCE_REPLAY_COMPARISON.md) records the resumed series and mixed performance results.
Basis: [local-source coloring investigation](LOCAL_SOURCE_COLORING_REPORT.md).

## Objective and limits

Avoid parsing the same hidden source again when a compatible saved parser state already exists.
Keep source bytes, diff structure, line identity, colors, fallback behavior, and navigation unchanged.
Reuse applies to revisits and theme changes. An uninterrupted cold parse has no later saved state to reuse.

No source-specific shortcut, new worker, new cache, dependency change, or larger cache allowance belongs in this change.
The work does not reopen earlier parser-sharing or matching experiments.
It does not promise a timing target or a fixed maximum replay distance.

## Storage and identity

The checkpoint compatibility key must contain the immutable source origin, file, and parser scope.
The scope keeps complete-source old/new sides separate. Patch checkpoints retain their section identity.
Checked-pair identity already distinguishes source replacement, even when the patch entry and visible text look the same.

Represent parser compatibility separately from the active color identity.
Active state, style-cache invalidation, and reply acceptance must retain the request epoch and appearance.
Removing those fields from all state comparisons would allow stale theme colors.

Only saved parser state, the complete scope stack, and fallback state can survive a theme change.
Restoration must bind the active state to the current request identity and rebuild theme-derived caches with the current highlighter.
Checkpoint lookup and duplicate suppression must use the same compatibility key.

The existing shared limits remain: 64 entries, 16 MiB retained checkpoint allowance, and 256 KiB maximum entry size.
Keeping entries across themes extends their lifetime without raising those limits.
Both source sides still compete for that allowance. Eviction remains valid and can require longer replay.

## Gap restoration

`State::next` counts the source lines already consumed. Requested source line `N` is one-based.
Before a requested line with a hidden gap, a saved state is usable only when:

```text
active.next < saved.next <= N - 1
```

A checkpoint at `N` already consumed the requested line and cannot generate its tokens.
Adjacent requested lines retain ordinary sequential advancement. Checkpoint lookup occurs once per gap, not once per hidden line.

Restoration must move both the parser and the buffered source reader.
The new reader starts at `saved.next` in the same immutable side snapshot, using the existing bounded sequential reader.
The scratch line buffer must not retain bytes from the previous read position.
The request can borrow its existing snapshot. No persistent snapshot cache is necessary.

The complete cloned state remains authoritative, including captured delimiters, scope restoration history, and fallback flags.
Cancellation checks surround restoration and reader replacement. The existing check before publication remains in force.
Failure must retain the existing source-error and patch-view behavior.

## Ordered implementation

1. Add focused real-worker checks for multiple hidden gaps within one request.
   Include a theme change over the same sparse sources.
   Compare exact old/new tokens against uncached complete-file passes.
   Establish a bounded diagnostic count of replayed lines or restores, so the checks prove reuse as well as correct output.
2. Separate checkpoint compatibility from active theme identity in the existing highlighting modules.
   Keep checkpoint ownership, admission, eviction, and style acceptance in their current components.
3. Add compatible gap restoration to the complete-source reader.
   Do not change patch traversal, source correspondence, the renderer, or public controls.
4. Run the focused checks, ordinary suite, feature suite, formatting, and Clippy.
   Compare complete terminal grids against the preserved ordinary executable.
5. Run a newly declared comparison only after correctness passes.
   Keep ordinary timing, diagnostic replay counts, and resource observations separate.

## Required evidence

- Divergent multiline old/new syntax, embedded syntax, and different side extensions retain exact tokens.
- Multiple gaps, mixed patch/context requests, and target positions before, at, and after checkpoint boundaries remain correct.
- Theme restoration rebuilds colors and never accepts an old-epoch reply.
- Source replacement, unrelated reviews, unsupported syntax, oversized hidden lines, and parser fallback cannot reuse incompatible state.
- Cancellation and a full reply queue release obsolete work without losing valid published state.
- Eviction still obeys the same count and byte limits. No test assumes all checkpoints survive.
- Actual PTY views retain source positions through expansion, split/unified changes, wrapping, resizing, and themes.
- A repeated sparse request performs less hidden parsing when suitable checkpoints survive.
- Cold opening retains correctness and receives no unsupported speedup claim.

The ordinary comparison must cover both prepared local cases and a separate real-source case.
The remaining scrolling trace must prove actual viewport movement, not selection-only changes.
Broader memory qualification remains separate. Cache limits are not total-process memory guarantees.

Stop on a source/color mismatch, stale style, incompatible restoration, cleanup failure, or unexplained resource regression.
Retain the current ordinary executable until the replacement passes its checks.
