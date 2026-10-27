# First-visit highlighting: scope and execution plan

Date: 2026-09-14. Status: diagnostic execution complete.

The [execution report](FIRST_VISIT_HIGHLIGHTING_REPORT.md) records the checked
captures, initialization attribution, focused stack sample, and next proposal.
No production optimization was adopted in this diagnostic stage.
The original specification is preserved in `evidence/first-visit-highlighting-v1/PLAN.md`.

The next stage identifies the work that delays correct syntax colors on a first visit.
It starts with a small diagnostic extension, not an engine replacement or a larger cache.
The first production change will depend on that evidence.

## What the user must experience

Correct source text must remain available while syntax work continues.
Navigation, layout changes, resizing, help, and quit must remain responsive during that work.
Completed colors must match the sequential syntax reference for each supplied source side.
An optimization must preserve source bytes, numbering, pairing, emphasis, reading position, and existing fallback behavior.

This stage measures PTY receipt of correct text and colors separately.
Those observations do not establish physical screen paint or smooth scrolling in Ghostty.
The [actual-terminal check](../../TERMINAL_PRESENTATION_CHECK.md) remains separate.

## What we already know

The [previous attribution](LOADING_COLORING_REPORT.md) contains these diagnostic observations:

| Observed action | Correct text | Checked colors | Explanation supported by the trace |
| --- | ---: | ---: | --- |
| First deep end jump | 1.14 ms | 1,721.64 ms | The worker parsed 11,908 preceding records. Predecessor parser calls dominated the request. |
| Backward revisit | 3.48 ms | 47.08 ms | A saved checkpoint reduced replay to 132 predecessors. |
| Theme change at the deep position | 3.80 ms | 1,740.67 ms | The theme generation invalidated saved state and repeated the long replay. |
| Large-patch endpoint after public navigation | 4.23 ms | 111.65 ms | Parsing dominated. Publication also included possible observer backpressure. |

A dense-line request spent 96.60 ms of 96.97 ms inside parser calls.
Its longest parser call took 49.94 ms.
The worker checks cancellation between groups of records, not inside one parser call.

These observations belong to the previous preserved executables, not a new current-build latency series.
The later [alignment-buffer change](ALIGNMENT_SCRATCH_REPORT.md) did not change highlighting.
The existing timings do not separate initialization from the remaining parser work.
They do not justify larger caches, more workers, or reduced grammar coverage.

## Current source and safe boundaries

The source review covers the request path, parser state, checkpoints, lazy initialization, publication, and existing process checks.
It excludes unrelated indexing and renderer optimization.

| Boundary | Current behavior | Consequence for this stage |
| --- | --- | --- |
| `src/highlight.rs`: `Side::advance` | The timer surrounds `ParseState::parse_line`. Old and new sides advance separately. | The timer includes initialization encountered during parsing, but excludes initial `Side::new` work. |
| `src/highlight.rs`: `start` | One worker resumes a compatible continuation or restores a checkpoint before replay. | A first unseen prefix still needs its supplied syntax context. |
| `src/highlight/checkpoint.rs`: `SavedSide`, `set_epoch` | Saved data contains parser state, scope state, and validity. Identity also includes theme and epoch. | Theme-independent state reuse is a separate possible change. |
| `crates/diffs-syntect/src/parsing/syntax_set.rs`: `SyntaxReference::lazy_contexts` | A `OnceCell` initializer deserializes contexts and shares exact patterns. | Measure actual initialization, including work before the first parser call. |
| `crates/diffs-syntect/src/parsing/regex.rs`: `Regex::regex` | A `OnceCell` initializer prepares an outer regex. | Measure initialization inside the closure, not every cache access. |
| `crates/diffs-syntect/src/parsing/syntax_definition.rs`: `MatchPattern::regex_with_refs` | Captured delimiters construct source-dependent regexes. | Initialization can repeat. It is not always a one-time grammar cost. |
| `crates/diffs-fancy-regex/src/engine_cache.rs`: `EngineCache::compile` | An outer regex can reuse a compiled inner engine. | An outer initialization count is not a count of new inner engines. |
| `src/highlight.rs`: `Mailbox::publish` | A bounded reply queue wakes the UI and waits for capacity. | Publication time includes downstream pressure. Observer delays need separate treatment. |

## Step 1: add only the missing attribution

This is the next implementation task. It does not change normal viewer behavior.

1. Preserve the current source, ordinary executable, build recipe, and reference outputs in a new evidence directory.
2. Add a small optional timing module to the pinned Syntect dependency.
3. Enable that module through `delivery-trace`, without enabling allocation census or a different worker shutdown policy.
4. Add timers inside the grammar and outer-regex initializer closures.
5. Add a count of source-dependent pattern constructions at `regex_with_refs`.
6. Read counter deltas around each request and each existing parser-call interval.
7. Extend the existing trace schema and its offline reader without changing historical trace files.

Each phase needs call count, accumulated duration, and maximum duration.
Interval maxima need explicit accumulation. Subtracting cumulative maxima does not produce an interval maximum.
Fixed-size thread-local counters are sufficient. The observer must not retain patterns, source text, parser states, or compiled engines.
The implementation must not hold a mutable observer borrow across measured work.
Initializer guards must retain failure or unwind status rather than count interrupted work as successful initialization.
Normal builds must contain no timing hooks, counter storage, or new dependency feature activation.

Request accounting starts before `Side::new`, which can load the initial grammar through `ParseState::new`.
Per-call deltas distinguish requested lines from off-screen replay.
The existing request IDs, source IDs, side labels, replay reasons, and longest-call record remain available.
The trace keeps bounded storage, explicit overflow status, and explicit incomplete requests.

The report uses these names and rules:

- **Grammar initialization:** actual context initialization, including deserialization and pattern sharing.
- **Outer regex initialization:** regex preparation, including any inner-cache lookup or compilation.
- **Parser work excluding observed initialization:** parser-call time minus initialization within that same call.
- **Request initialization:** all observed initialization during the request, including work outside parser calls.

Per-call initialization is a subset of request initialization. The report must not add those totals together.
The parser remainder includes dispatch, matching, allocations, and bookkeeping. It is not a pure matching measurement.
Timers inside initializers exclude another thread's `OnceCell` wait. The viewer uses one syntax worker.
Counter overflow, missing summaries, or inconsistent nested totals invalidate attribution, not the retained raw observation.

No per-search timer belongs in this first extension.
If the parser remainder dominates, a separate focused stack sample is the next diagnostic.
If sampling cannot resolve the owner, a second diagnostic can time backend search after lazy initialization completes.
That interval includes capture conversion and search-state preparation. It must not be labeled pure matching time.

## Step 2: use a small declared workload

The first pass uses one fresh ordinary process and one fresh diagnostic process per case.
It supplies attribution and observer controls, not a latency distribution.
All processes run serially. No warm-up or unfavorable observation is discarded.

| Case | Reused source | Required observations |
| --- | --- | --- |
| Deep continuous syntax | The 12,000-line fixture and actions in `tools/syntax_diagnostic.py` | Opening, first end jump, backward revisit, theme change, uncached forward movement |
| Dense lines and fallback boundaries | The pinned guard case in `evidence/delivery-v1/cases/manifest.json` | Opening and both themes, with colored lines separate from declared fallback |
| Repeated grammar across files | The pinned many-small-files case in the same manifest | Opening, end, start, both themes, narrow and wide views |
| Ordinary multi-file changes | The pinned Ratatui case in the same manifest | The same public actions, without a selected-language fast path |
| Captured delimiters | The Ruby input from `heredoc_end_delimiters_stay_private_to_each_side` | Both delimiter states, theme changes, and revisits |

The Ruby fixture must preserve the existing generated source and explicit expected syntax roles.
Its diagnostic preparation must record the resulting patch hash before measurement.
The existing real-process contract remains the correctness authority for those roles.

The run packet records fixture and executable hashes, public actions, terminal geometry, reference cells, and capture limits before observations.
“Fresh process” does not mean a cold OS file cache.
“First visit” requires no earlier styling of the destination in that process.
The trace must distinguish a warm grammar, a saved parser checkpoint, and an already-colored viewport.
A theme change is not a fresh-process observation.

The capture path must continuously drain PTY bytes during each declared window.
Expensive cell comparisons occur after that window, through the existing capture/replay machinery.
The deep replay needs an explicit longer window than the current 500 ms helper default.
Use a five-second window for its actions and the captured-delimiter case.
Keep the existing 500 ms windows for the three other cases.
These are diagnostic capture limits, not new performance targets.

The implementation can add explicit schedule and window inputs to the existing capture helper.
Historical defaults and frozen traces remain unchanged.
The helper must retain late, incomplete, overflow, and failed captures without silently extending a window.
It must reject unassigned output between actions rather than attributing that output to the next action.
Reference preparation and offline checking durations remain outside measured capture windows.

Correct text, exact completed colors, declared fallback, index completion, and quit remain separate observations.
The report retains request-generation changes and incomplete work during cancellation.
An incomplete shutdown trace cannot establish completed highlighting or worker cancellation latency.

The large-patch endpoint is a later transfer check, not a required repeat of its long navigation setup during initial attribution.
The five-patch memory workload also waits for a production candidate.

## Step 3: choose one production change

The evidence determines which branch receives an implementation proposal:

| Dominant cost | Next bounded proposal | Required caution |
| --- | --- | --- |
| Lazy grammar or outer-regex initialization | Identify repeated preparation or allocation at that owner. | No eager loading of every grammar or wider cache without a measured memory tradeoff. |
| Source-dependent regex initialization | Examine ownership within the active captured-delimiter state. | Old/new captures and delimiter lifetimes must remain private and exact. No global source-pattern interner. |
| Parser remainder across many predecessor lines | Use one focused sample, then target the demonstrated dispatch, matching, or allocation owner. | Do not skip predecessors, guess syntax state, or parallelize dependent lines. |
| Repeated replay after a theme change | Separate theme-independent parser checkpoints from theme-dependent color state. | This improves compatible repeat work, not a first unseen prefix. |
| Obsolete work after navigation or resize | Examine cancellation between complete source records. | Both sides and the continuation must describe the same completed prefix. One parser call can still run uninterrupted. |
| Publication after computation | Recheck with continuous PTY capture before changing queue or draw behavior. | Observer backpressure does not establish a viewer scheduling defect. |

Construction counts alone cannot assign initialization time to source-dependent patterns.
That branch requires a follow-up that distinguishes their initialization from static-pattern initialization before a production proposal.

Theme-independent reuse is the clearest source-supported secondary candidate.
The saved checkpoint already excludes token colors, but the current identity rejects a changed theme.
A proposal must separate document/syntax identity from appearance and reply generation.
It must reconstruct highlighting state from the saved scope stack under the new theme.
It must reject stale replies and preserve invalid-side state, including fallback.
Checkpoint count, byte limits, eviction, and source ownership remain unchanged in that candidate.
This change is not included in the diagnostic extension.

If the evidence identifies no worthwhile change, the stage ends with that finding.
An optimization is not mandatory merely because an attribution task exists.

## Correctness and acceptance

The diagnostic extension must preserve ordinary and diagnostic source output, token answers, and completed terminal cells.
The existing sequential reference checks colors without the viewer's continuation, checkpoint, or reply machinery.
It shares the grammar dependency, so it cannot alone establish correctness after a grammar-engine change.
A dependency change also needs a preserved pre-change reference and its applicable real grammar/capture tests.

Before production qualification, the candidate must pass these checks:

1. Run the existing Rust source, syntax, geometry, delivery, and cleanup contracts against the exact candidate executable.
2. Check separate old/new comments, strings, captured delimiters, and context colors in both themes and layouts.
3. Check fallback propagation and recovery at a new hunk without changing the line-size guards.
4. Check final exact colors after pending-highlight resize and navigation, not only the text anchor.
5. Check source line, active side, and text position through wrapping and width changes.
6. Check complete five-patch source digests, projections, and the existing independent interior views.
7. Run formatting, application Clippy, and applicable pinned-dependency tests.

The existing rapid-resize contract checks text position before shutdown. It does not establish final coloring or physical frame pacing.
Any scheduling or checkpoint change therefore needs a final-color assertion using a real process and an independent expected destination.
A narrow attribution-only build does not justify a new family of mocked tests.

After correctness passes, use three alternating ordinary reference/candidate pairs for the focused workload.
Report each action and every trial. Three pairs do not establish tail-latency guarantees.
Then run the unchanged five-patch responsiveness and sustained-memory checks once for that production candidate.
Repeated tours, cache revisits, resizing, text/color milestones, and adverse observations remain part of qualification.
Allocation profiling stays separate from ordinary timing and RSS measurements.

The latest memory comparison still has an unresolved Bun increase and unmet memory targets.
Faster colors cannot erase that result or justify unexplained new retained storage.
Any new owner needs a clear lifetime, accounting, release behavior, and measured tradeoff.
No memory reduction or coloring speedup is claimed by this plan.

The [measurement policy](../../MEASUREMENT_POLICY.md) remains unchanged.
Wrong source, wrong colors, lost position, stale theme output, or failed cleanup blocks acceptance.
Unexplained responsiveness or memory regressions require investigation before a keep decision.

## Deliverables and stopping point

The next execution produces a preserved baseline, bounded diagnostic source, raw captures, checked traces, and one attribution report.
That report selects one justified production candidate or records why no candidate is ready.
Each production change receives its own implementation and qualification checkpoint.

The original specification changed documentation only.
Its later execution is recorded in the linked report. Do not restart completed stage drivers.
