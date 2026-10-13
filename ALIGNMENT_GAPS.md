# Alignment-gap contrast

## Scope

An alignment gap means that the opposite side has no corresponding source row.
The same neutral fill marks unused rows when one side wraps onto more rows.
A real blank source line retains its line number and source background.

The preceding dark theme mixed foreground into the gap at 8% and into context rows at 7.5%.
Those fills are too similar in the inspected terminal screenshots.
This change uses an 18% mix for dark gaps and a 10% mix for light gaps.
The existing Lab color composition and the `empty` color role remain in use.

Only the gap fill changes. Source, line numbers, alignment, wrapping, syntax, change colors, and navigation remain unchanged.
No setting, pattern, dependency, or rendering branch is added.
Other contrast issues and path clipping remain separate work.

## Acceptance

1. Check exact gap colors in both themes through real terminal processes.
2. Check unpaired additions, unpaired deletions, and unequal wrapping.
3. Check that real blank lines retain their source backgrounds and line numbers.
4. Check split/unified changes and width changes for stale gap cells.
5. Inspect actual Ghostty screenshots of multi-file and wrapped views in both themes.
6. Run the existing normal regression suite and rebuild the ordinary executable.

The screenshot pass is limited to two visual rounds.
It does not establish scrolling smoothness, display calibration, or accessibility conformance.
After this change, current-build measurements and release qualification remain the next work.

## Result, 2026-09-21

The implementation changes one color expression in `src/theme.rs`.
Dark gaps now use `#303030`. Light gaps now use `#e3e3e3`.
Context rows retain `#1b1b1b` and `#fbfbfb`, respectively.

The normal suite passed 69 checks, including 43 terminal-process contracts.
Two historical checks remain explicitly excluded. This change did not repeat the diagnostic-feature suite.
All 11 rendering contracts also passed against the final ordinary executable.
Formatting and application Clippy passed. A pre-existing lifetime warning remains in vendored Syntect.

The new reflow contract checks real blank lines and unused wrapped rows at widths 80, 160, and 81.
It checks both themes and the return to an unwrapped unified view.
The existing source-row contract now checks every cell of absent-side gaps.
The new contract failed against the preserved preceding executable, as expected.

The before/after multi-file captures each contain 6,720 emitted cells.
In each theme, only the 632 gap backgrounds changed. Every other cell property matched.
All four completed viewer processes returned exit code zero and restored terminal modes.
These checks establish emitted cells, not physical display behavior or timing.

The light-theme Ghostty captures show the complete view before and after the change.
The dark capture shows the new gap fill, but its physical frame omits the first three application rows.
The emitted grid retains those rows. The cause of this framing discrepancy remains unresolved.
The recorder then reported no visible owned window before the wrapped captures.
The capture stopped. Wrapped physical screenshots and full dark-frame acceptance remain incomplete.
The gallery replaces only its light multi-file image. Other images retain their earlier executable labels.

The final ordinary executable SHA-256 is `6111c3340f1c42185b7091891205af9072509a806d0d22321a57c066f9aab5f5`.
The preceding executable SHA-256 is `14eedb677bfee72f22ea73026aae5ad90c9aacbdad806ba46adac17766925a63`.
The local capture and check records are outside the product source:

`/Users/ramos/Documents/Codex/2026-09-21/native-diff-gap-review/`

No timing or memory measurements ran for this change.
The measurement preflight reported battery power, so comparable timing captures remain pending.
