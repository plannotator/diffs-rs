# Full-source coloring checkpoint

Date: 2026-09-22. Scope: complete-context coloring for the two existing local input commands.

The preserved reference is `evidence/full-source-color-v1/baseline-native-diff`.
Its SHA-256 is `a8d4595017911c2a149ab05b1de0ab7cfde216b2c7d85c845d862fcea83d9414`.
Context expansion, remote input, grammar changes, and controlled performance captures are outside this checkpoint.

## Implementation

1. Keep the existing patch replay path and its visual output unchanged.
2. Give each checked source pair a stable identity. Bind requests and replies to their actual coloring origin.
3. Extend the existing worker with complete-source replay. Old and new sides retain independent state and language selection.
4. Read hidden prefixes in bounded sequential buffers. Keep one shared checkpoint budget for both replay paths.
5. Retain only requested styles. Oversized or failed parser input invalidates later coloring on that side, including later patch sections.
6. Show plain syntax while sources are pending or unavailable. Preserve change backgrounds and valid inline emphasis.
7. Reject obsolete replies when a file changes from pending to checked. Update viewport status without moving the reading position.

No dependency or cache-budget increase is planned. Existing display escaping and newline normalization remain unchanged.
Source correspondence is checked once before syntax replay. Neither redraws nor syntax requests repeat that check.
Cancellation remains cooperative. It cannot interrupt one active regex call.

## Checks

- Real files and Git comparisons with hidden comment, string, heredoc, and embedded-language boundaries.
- Exact old/new token comparisons against an uncached complete-file pass, plus explicit syntax-role assertions.
- Backward visits, checkpoints, theme changes, source-origin replacement, cancellation, and bounded delivery.
- Oversized hidden input, independent side fallback, different filename extensions, and safe display conversion.
- Actual terminal-process colors, layout/wrap/resize controls, source failures, cancellation, and cleanup.
- Existing correctness suites, complete preserved terminal grids, canonical exports, formatting, and Clippy.

The final report must separate correctness checks from physical display and performance evidence.
