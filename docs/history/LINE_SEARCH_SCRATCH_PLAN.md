# Line-local regex search scratch

Date: 2026-09-15. Status: completed and kept; see the [checkpoint report](LINE_SEARCH_SCRATCH_REPORT.md).
The original pre-measurement plan remains unchanged in `evidence/line-search-scratch-v1/PLAN.md`.

The [attribution checkpoint](FIRST_VISIT_HIGHLIGHTING_REPORT.md) identified repeated regex search during syntax replay.
Its stack sample includes temporary VM buffer growth and release.
It does not prove that allocation dominates search or predict a speedup.

## Scope and ownership

Add one caller-owned scratch object for VM branch, save-history, and delegated-slot vectors.
Each `parse_line` call owns one object and releases it before returning.
The VM borrows these vectors for a search and creates fresh state scalars and an owned capture vector.
Each search starts with empty scratch lengths, including after a miss, error, or unwinding caller.
Vector capacity can be reused within the line. Numeric contents from earlier searches are not valid state.

Keep returned capture ownership and the existing public API unchanged.
The existing API creates temporary scratch and calls the same matching implementation.
The explicit scratch API supports successive calls with unrelated patterns.
Wrapped engine searches retain their existing allocation and matching behavior.
The Oniguruma backend remains a direct pass-through with no scratch storage.

Syntect groups its existing line-local region and match cache with the new scratch owner.
Do not put scratch inside cloned regions, `ParseState`, saved checkpoints, or compiled engines.
Do not introduce a global pool, thread-local pool, new dependency, unsafe code, or larger cache budget.
Do not change rule order, capture numbering, matching options, backtrack limits, fallback, or source traversal.

Scratch capacity has a checked byte-accounting method.
The prototype reuses capacities only for one source line, not across an entire document.
One difficult search can increase temporary memory for the rest of that line.
No lower memory peak is assumed from the shorter allocation path.

## Execution and evidence

1. Preserve a fresh ordinary reference build and its source in `evidence/line-search-scratch-v1/reference`.
2. Run explicit real-regex cases against the old API before edits. Preserve those test inputs and outputs.
3. Implement the borrowed buffers and line-local integration. Change only the test adapter for the same expected regex cases.
4. Check actual captures, held results, misses, errors, pattern changes, Unicode, atomic groups, backreferences, and concurrent callers.
5. Run the applicable pinned-dependency suites, including the alloc/no-std and alternate backend build paths.
6. Preserve the ordinary candidate build after review. Run process contracts and the full-source and projection checks.
7. Check final syntax colors after pending-highlight navigation and resize, not only the source anchor.
8. Compare complete token answers with the preserved pre-change sequential executable.
9. Run three alternating ordinary reference/candidate pairs of the frozen five-case actions.
10. If correctness holds and the focused tradeoff remains plausible, run the unchanged five-patch responsiveness and sustained-memory qualification.

Use the existing frozen source and terminal expectations. Do not restart completed stage drivers.
Keep all new logs, failures, raw captures, identities, and execution order in this new stage.
Use fresh output directories for each observation. Do not replace an unfavorable observation.
Keep profiling separate from ordinary timing and memory checks.
The shared measurement policy remains unchanged.

## Acceptance and stopping conditions

Any wrong capture, source line, side, pairing, syntax color, reading position, or cleanup blocks adoption.
Any unexplained responsiveness or memory regression requires investigation before adoption.
The prototype must show a worthwhile ordinary tradeoff, not only fewer allocator calls.
If the added ownership plumbing does not help enough, restore the exact prior production inputs and keep the evidence.
Do not retain a rejected experiment solely because its numbers improve on one fixture.

The prior higher Bun RSS and incomplete actual-terminal comparison remain open.
This stage does not change or reinterpret earlier observations.
Theme-independent checkpoint reuse is a separate future candidate.
