# Incremental context expansion

## Scope

This stage implements section 7 of `EXACT_SOURCE_CONTEXT_SPEC.md` for checked local sources.
Patch-only views retain their current text, colors, row destinations, and canonical output.
Physical-terminal inspection and controlled resource measurements remain separate qualification steps.

## Mapping

The immutable patch projection remains the authority for original rows and replacement pairing.
A view layer combines patch ranges, hidden gaps, and revealed source ranges.
Each file has separate split and unified offsets. Published patch offsets never change.

Checked source preparation records omitted ranges while it checks correspondence.
Each range retains old/new starting lines, length, and its next original section, or the end of the file.
The UI retains one revealed count per gap, not one allocation or index entry per source line.
Structural updates rebuild only the affected file's segments and the numeric file-offset suffix.
Drawing, input movement, and style replies do not rebuild the mapping.

Display references distinguish original patch records from checked context lines.
Context references retain the file identity and both source numbers.
Geometry resolves references to immutable byte ranges through the shared source store.
The existing style cache and worker also accept context references. Their budgets do not increase.
Style replies retain the checked-pair origin and theme epoch checks.

Original section navigation maps through the view without changing the recorded patch ranges.
File navigation retains patch order. Endpoint navigation uses only exposed source lines.
Readiness preserves the cursor and viewport. Explicit expansion selects the nearest newly revealed line.
Source, active side, text offset, and approximate screen row survive later layout changes.

## Actions and boundaries

Click or Enter reveals at most 20 source lines. Leading gaps reveal backward. Other gaps reveal forward.
The action does not expand alignment spacers, pending sources, unavailable sources, or metadata-only entries.
Exhausted gaps disappear without deleting original section markers.
An exposed final line retains its exact final-newline status.
Gutters accommodate exposed source numbers. Changes to gutter width use the existing reading anchor.
Source errors preserve the original patch and reject obsolete styles.

## Checkpoints

1. Preserve the executable and run the existing suite.
2. Add typed view references and the identity mapping with expansion disabled.
3. Compare canonical streams and complete terminal grids against the preserved executable.
4. Add checked ranges, incremental expansion, source access, and coloring.
5. Check exact bytes, numbers, order, gaps, markers, styles, and source destinations.
6. Exercise real input, clicks, layout changes, wrapping, resize, cancellation, and cleanup through a PTY.
7. Review ownership, allocations, failures, and Rust conventions. Run formatting, Clippy, and the ordinary and diagnostic suites.

The reference is `evidence/context-expansion-v1/baseline-native-diff`.
Do not infer current-build timing or total-memory claims from these correctness checks.
