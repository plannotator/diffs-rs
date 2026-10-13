# Incremental context expansion checkpoint

Date: 2026-09-22.

Checked local comparisons now expose unchanged source through `Show 20` gap rows.
The implementation follows `CONTEXT_EXPANSION_PLAN.md` and section 7 of `EXACT_SOURCE_CONTEXT_SPEC.md`.

## Behavior

Click a ready gap row, or select it with Up/Down and press Enter.
Each action reveals at most 20 source lines. Wrapped screen rows do not change this count.
Leading gaps reveal backward toward the file beginning. Interior and trailing gaps reveal forward.
The remaining count decreases. Exhausted gaps disappear without duplicating source at the join.

Activation selects the newly revealed line next to the remaining gap.
This is the first new line for a leading gap and the last new line for a forward gap.
The reading anchor retains source identity, side, text position, and approximate screen row.
Layout, wrapping, theme, line-number, sidebar, and width changes use that same anchor.
Source readiness does not activate gaps. The `g` and `G` keys include already exposed context but do not reveal more.

Original patch records, section markers, file order, change totals, pairing, and canonical export remain unchanged.
Section navigation still targets original changes after a gap disappears.
Split context retains separate old/new syntax colors. Unified context retains both numbers and uses new-side colors.
Exposed final lines retain their actual final-newline marker.

Pending and unavailable sources cannot expand. Alignment padding never expands.
Binary and metadata-only entries do not become full-file viewers.
Collapse, automatic expansion, whole-file expansion, remote acquisition, and live-workspace inputs remain outside this stage.

## Structure and self-review

`view.rs` combines ranges of original patch rows with hidden gaps and exposed source ranges.
The correspondence worker records checked omitted ranges while it compares source bytes.
Each view gap retains a revealed count. There is no new object or RAM index for each hidden or revealed line.
Structural updates rebuild the affected file's segments and the numeric file-offset suffix.
Rendering and style replies do not rebuild those indexes. Original `FileIndex` offsets remain immutable.

Typed line references distinguish patch records from checked context, including both source numbers.
Geometry resolves immutable byte ranges through the existing shared store.
The existing worker and style cache accept both line types. No additional worker, runtime, crate, or cache allowance was added.
Queued styles retain their checked-pair origin and theme epoch checks.
Source preparation and worker-read failures remove affected expansion, invalidate that file's styles, and retain the patch view.
Other unrecoverable storage/render errors still use the existing terminal-restoration error path.

Self-review removed unnecessary reference-count changes from line information lookup.
Context-anchor lookup uses binary searches over range metadata. Endpoint lookup skips unrelated metadata-only files.
The review also checked source replacement, partial reveals, exhausted gaps, missing final newlines, overflow checks, and source ownership.
Diagnostic records distinguish context destinations from original patch record numbers.
The existing delivery analyzer now rejects full-source traces explicitly. Source-aware timing analysis remains part of later qualification.

Typed cache keys and range metadata add bookkeeping. Existing payload limits are not total-memory limits.
The optional owner census reports view-vector capacity and occupied key sizes, with explicit exclusions.
No current-build memory or speed improvement is claimed.

## Correctness evidence

The initial ordinary suite passed 102 checks, including 57 real-process checks.
The typed-reference checkpoint passed the same suite before expansion was enabled.
It matched 96 complete terminal grids and two canonical source streams against the preserved coloring executable.
That intermediate executable and source archive remain in the evidence directory.

The expanded implementation passed 109 ordinary checks and 123 owner-census checks, including 60 real-process checks.
Both default suites excluded four opt-in tests.
The three new PTY checks also passed with all diagnostic features enabled.
That run checked structured context identities in actual delivery traces.
The patch-only analyzer rejected the resulting full-source trace with the declared scope error.
Final qualification filenames and the ordinary executable hash are recorded in `CURRENT_STATE.md` and the evidence manifest.

Three focused mapping/source checks cover:

- Both layouts, leading/interior/trailing gaps, partial reveals, exhaustion, unequal replacements, and missing final newlines.
- Every exposed source byte and number after each action, followed by exact reconstruction of both full files.
- Unchanged canonical patch records, original section destinations, endpoints, pending/unavailable sources, and stale context styles.
- Reverse-order readiness across 256 files, expansion in multiple files, and later file publication without destination changes.

A real-worker check compares mixed patch/context requests against sequential full-file colors in both themes.
It checks independent side state, backward revisits, and a deliberately incorrect old/new context reference.
The earlier worker suite retains hidden-language boundaries, checkpoint eviction, oversized-line fallback, and cancellation checks.

Three new PTY checks exercise actual application input and output:

- Click and Enter expansion, exact destinations, layout/theme changes, resizing, final-newline markers, captured-file stability, and cleanup.
- A long Unicode/tabbed context line with exact text-anchor expectations through wrapping, sidebar changes, and terminal widths.
- A three-file Git review with a later branch move, lazy pinned sources, file jumps, tree activation, and retained expansion.

ANSI and SVG cell evidence is under `evidence/context-expansion-v1/terminal-cells/`.
These are terminal-cell captures, not physical Ghostty screenshots.
The large-patch check covers all 20,847,451 supplied records across five existing fixtures.
It establishes patch-source preservation, not full-source availability for those downloaded patches.

Early failures came from checker assumptions: reveal-action arithmetic, offscreen gap selection, gutter widths, repeated-line correspondence, and resize/input sequencing.
The final assertions use explicit source numbers, real generated-patch correspondence, and settled dimensions before width-dependent sidebar actions.
The optional diagnostic check also exposed a Python-interpreter assumption in the test driver.
Rust tests now check the actual trace identities without requiring a Python runtime. The analyzer rejection check uses the existing measurement environment separately.
No production shortcut or looser source check was added to satisfy those expectations.

## Preserved artifacts

The ordinary executable is `target/release/native-diff`, preserved as `evidence/context-expansion-v1/native-diff`.
Its SHA-256 is `8fee6ce72866c3f23dfbe7ad3d926d35712f12fde8ae43df00eeebaa5aa4625e`.
The comparison reference is `evidence/context-expansion-v1/baseline-native-diff`.
Its SHA-256 is `1a26004939aa87f3eba258813f60aadbb05cab36b5df3b2e381c73a9ee91c1f0`.
The identity-mapping checkpoint is `mapping-native-diff`, with SHA-256 `019d6d4af6ba80abb4246957f72ec57cc20a9dfb77406e8e9a168c474f008617`.

The evidence directory preserves both source checkpoints, final source, intermediate failures, final results, and `SHA256SUMS`.
Final results use `final-suite.log`, `diagnostic-suite.log`, `final-compatibility.log`, and `final-corpus.json`.
All-feature context results use `all-feature-context-final.log`.
The source archive includes build inputs, assets, fixtures, tests, tools, notices, and current documentation.
Earlier coloring, input, storage, and sidebar packages remain unchanged.

## Qualification boundaries

The checks use macOS arm64, Rust 1.96.0, and Apple Git 2.50.1.
The declared Rust 1.88 minimum remains unqualified in this stage because that toolchain is not installed.
Formatting and application Clippy checks include the normal and all-feature configurations.
The pre-existing vendored Syntect lifetime warning remains unchanged.

No GUI terminal, accessibility automation, recording, network acquisition, global installation, or maintainer contact was used.
No controlled timing or sustained-memory campaign ran.
The next qualification must cover representative complete-source workloads, repeated expansion, many-file update cost, cold page reads, cancellation, and retained memory.
Physical terminal presentation remains a separate acceptance step.
GitHub acquisition follows local qualification. This checkpoint does not declare the viewer production-ready.
