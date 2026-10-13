# Inline highlighting

## Scope

Inline emphasis identifies changed words and punctuation within paired replacement lines.
The full-line backgrounds still identify added and deleted source lines.
This change does not modify source text, line pairing, syntax colors, geometry, navigation, or cache limits.

The tokenizer groups Unicode letters, numbers, and underscores into words.
It keeps each extended grapheme intact, including combining marks and joined emoji.
It groups whitespace into runs and treats other graphemes as separate tokens.
Comparisons use exact text without Unicode normalization or ignored whitespace.
Adjacent changed tokens share an emphasis span. Unchanged spaces and punctuation do not join separate spans.
Both sides use an old-to-new comparison. The old side emphasizes deletions, and the new side emphasizes insertions.
Repeated tokens can permit multiple optimal alignments. Both sides must retain the same shared token sequence.

For `old-14` versus `new-14`, only `old` and `new` receive inline emphasis.
For `old_value` versus `new_value`, the complete identifiers receive emphasis.
This is a word-level policy, not a minimal character diff or a language parser.
The existing long-line fallback remains unchanged.

## Checks

Explicit expected spans cover punctuation, numbers, identifiers, whitespace, repeated words, combining marks, and emoji on both sides.
Real-process checks cover exact terminal cells in both themes, split/unified layouts, wrapping, and resize.
Existing source, navigation, syntax, cancellation, and terminal-restoration checks remain required.
These checks do not establish physical terminal appearance or performance improvements.

## Paired-alignment correction

The 2026-09-22 correction gives both replacement sides the same comparison direction.
The preceding implementation could select inconsistent common tokens in ambiguous replacements.
The real-process regression first failed against the preceding code, then passed after the correction.
All 842 sampled paired sides now pass independent optimality and shared-sequence checks.
The ordinary executable passed all 724 reviewed large-patch views.
The [correction report](evidence/inline-pair-v1/REPORT.md) records the checks and current executable hash.

## Original result

The implementation passed 68 normal checks and 85 diagnostic checks.
Each suite included 42 real-process checks. The two existing exclusions remain unchanged.
The final ordinary executable also passed ten focused rendering checks.
Formatting and application Clippy passed. The existing vendored lifetime warning remains unchanged.

The explicit `old-14` regression failed before the change in both span and real-process checks.
The original position fixture now passes in both layouts and themes.
Expected spans for numeric replacements also exclude unchanged semicolons.
New test development required corrections for fixture paths, terminal readiness, and emulator continuation-cell attributes.

Only `src/highlight.rs` changed among the preceding 2,064 build inputs.
The new `src/highlight/inline.rs` contains tokenization and span construction.
No dependency changed. Borrowed token slices remain temporary, and cached span capacity remains part of the existing cache accounting.
This change does not establish new timing or sustained-memory results.

The ordinary executable is `target/release/native-diff`.
Its SHA-256 is `14eedb677bfee72f22ea73026aae5ad90c9aacbdad806ba46adac17766925a63`.
The earlier visual gallery remains a record of the previous build.

## Deferred presentation observation

The user reported that empty alignment areas look different in the light theme and are difficult to distinguish in the dark theme.
These areas represent absent opposite-side lines, not blank source lines.
Their contrast needs a separate visual review. This change does not alter their colors.
