# First-visit highlighting attribution

Date: 2026-09-14. Status: diagnostic checkpoint complete. No production optimization adopted.

The long first-visit delay comes from parsing the required preceding source, not from new grammar or regex initialization.
A separate stack sample places most observed syntax work inside regex search execution.
The next candidate is narrowly scoped reuse of temporary search buffers.
It is not a grammar replacement, a larger cache, or permission to skip source context.

## What changed

The optional `delivery-trace` feature now enables fixed-size Syntect timing intervals.
They measure actual lazy grammar and outer-regex initializers, including setup before the first parser call.
They also count source-dependent pattern constructions.
The observer retains no source text, pattern strings, or compiled owners.
Normal builds do not enable these hooks.

Schema 3 preserves the original parser phases and records their initialization subsets separately.
Per-call parser remainders exclude only initialization inside that call.
Interval maxima are accumulated directly, never calculated by subtracting cumulative maxima.
Failed initialization, invalid nesting, saturated counters, and inconsistent totals reject attribution.
The original phase counters lack an overflow flag, so the reader rejects saturation-boundary values.

The capture helper accepts an explicit frozen schedule while preserving its historical defaults.
It drains PTY output during each fixed window and checks complete cells afterward.
The five-case driver checks saved packets again without launching the viewer.
No production grammar, parser, cache, worker, fallback, layout, or source-storage policy changed.

## Observation protocol

The [preserved plan](../../evidence/first-visit-highlighting-v1/PLAN.md) governed this checkpoint.
There was one ordinary process followed by one diagnostic process per case, run as two serial batches.
The cases were deep continuous syntax, dense lines, many small files, ordinary multi-file changes, and Ruby captured delimiters.
Both batches used the same hash-bound fixtures, actions, geometry, and expected cells.
Each contained 39 action windows across five PTY processes.
All 78 windows passed exact settled-screen checks and saved-packet replay.
All ten viewers exited with terminal state restored.

Preparation used two unmeasured reference viewers for the deep and Ruby cases.
The three other cases reused existing frozen screens.
All five sets of screens passed independent source checks and the preserved sequential syntax reference in both themes.
That reference does not use viewer checkpoints or delivery, but it shares the pinned grammar dependency.
Explicit source destinations and old/new Ruby syntax roles provide additional checks.

These are fresh processes, not cold OS file caches.
The deep destination is new in its process, but its grammar is already warm from opening.
The ordinary and diagnostic runs differ substantially on some actions.
One process per role cannot establish instrumentation overhead, a distribution, or a production speedup.
No observation was replaced to obtain a more favorable result.

The [complete action table](../../evidence/first-visit-highlighting-v1/ALL_OBSERVATIONS.md) retains every text and color observation.
The [machine-readable summary](../../evidence/first-visit-highlighting-v1/summary.json) also includes exact-ready and indexing milestones.
PTY receipt does not establish physical paint, smooth scrolling, or Ghostty acceptance.

## Findings

### Deep first visit: warm engines, expensive repeated search

| Action | Ordinary text | Ordinary colors | Diagnostic text | Diagnostic colors |
| --- | ---: | ---: | ---: | ---: |
| First end jump | 3.34 ms | 3,040.85 ms | 1.16 ms | 1,723.88 ms |
| Backward revisit | 3.64 ms | 46.30 ms | 1.99 ms | 50.41 ms |
| Theme change at the deep position | 4.43 ms | 2,103.30 ms | 4.12 ms | 3,106.93 ms |
| Forward into uncached colors | 3.25 ms | 14.89 ms | 2.86 ms | 10.98 ms |

The first end-jump request processed 11,908 preceding records through 23,816 old/new parser calls.
Its total duration was 1,722.59 ms.
Parser work excluding initialization occupied 1,715.19 ms, including 1,708.22 ms of off-screen replay.
There were zero grammar initializers, zero outer-regex initializers, and zero source-pattern constructions in that request.

Opening had already initialized one grammar and 143 outer regexes.
That preparation took 23.05 ms, including 0.67 ms outside parser calls.
It explains some opening cost, not the later multi-second jump.
Outer-regex initialization can reuse a compiled inner engine, so those counts are not new-engine counts.

The backward revisit used a checkpoint and replayed 132 preceding records.
The theme change replayed 11,908 again, with no new grammar or outer-regex initialization.
Theme-independent checkpoint reuse remains a separate candidate for repeated work.
That reuse cannot remove the first unseen prefix.

### Dense lines: one parser call remains a responsiveness boundary

The three guard requests took 95.63, 98.54, and 96.59 ms.
Only the first included initialization, totaling 2.62 ms.
Their parser remainders were 92.66, 98.18, and 96.24 ms.
The longest calls took 46.35, 50.33, and 49.84 ms.

Correct visible colors can precede the end of requested worker work.
For example, guard opening colors arrived at 55.49 ms in the diagnostic process, but the exact settled screen arrived at 101.85 ms.
Worker duration and visible-color latency are not interchangeable.
Cancellation still cannot interrupt one parser call.
This checkpoint does not change that behavior.

### Other cases distinguish the owners

The ordinary multi-file case spent 12.83 ms initializing during its first 15.58 ms worker request.
Later requests used mostly warm state, though the end view initialized additional rules.
The many-small-files trace also distinguishes initial preparation from subsequent warm requests.
These observations do not justify loading every grammar in advance.

Ruby produced 2,996 source-dependent pattern constructions across its observed requests.
Request initialization totaled 71.81 ms out of 131.54 ms of recorded worker work.
This includes interrupted requests, not only completed viewports.
The counts do not identify how much initialization time belongs to source-dependent versus static patterns.
That distinction needs separate evidence before changing delimiter-pattern ownership.

Every started worker request had a summary, and no trace records were dropped.
This does not mean every submitted request completed styling.
Ruby retained two superseded/closed and two publication-stopped summaries.
Many-small retained one superseded/closed summary and six submitted requests that never started.
Final destination colors still passed the independent frozen-screen checks.

## One separate stack sample

After both batches finished, one ordinary viewer opened the same deep fixture and received `G`.
The profiler sampled it for one second at a requested one-millisecond interval.
PTY capture continued through the declared five-second window.
Opening, the final destination, saved-packet replay, and terminal cleanup passed.

Of 858 samples on the syntax thread, 848 included `ParseState::parse_line`, 786 included Syntect regex search, and 761 included `captures_from_pos`.
These are nested, inclusive samples, not additive phases, invocation counts, or precise CPU percentages.
The stack contains VM save/history growth, allocation and release, delegated engine search, and capture construction.
It establishes the search path as the next owner to examine.
It does not establish that allocation alone dominates that path or predict the size of a possible saving.

Source inspection agrees with the sample.
`vm::run` constructs branch, save-history, and delegated-slot vectors for each search.
The successful result transfers a separate owned capture vector to its caller.
Syntect already reuses its output region within one parsed line, but that does not reuse these inner VM buffers.

Evidence: [sample](../../evidence/first-visit-highlighting-v1/deep-stack-sample/sample.txt),
[profile protocol and checks](../../evidence/first-visit-highlighting-v1/deep-stack-sample/report.json),
and [derived counts](../../evidence/first-visit-highlighting-v1/summary.json).

## Next bounded production proposal

Prototype line-local reuse of VM branch, save-history, and delegated-slot buffers across searches within one `parse_line` call.
Keep returned capture-vector ownership unchanged in the first candidate.
This targets observed repeated temporary work without retaining scratch in every compiled rule or checkpoint.

The implementation must satisfy these boundaries:

1. Give scratch one explicit caller owner. Keep it separate from `ParseState`, saved checkpoints, compiled engines, and output `Region` clones.
2. Reset logical lengths, save indices, branch state, options, and error state for each search. Different patterns and old/new sides must not inherit results.
3. Release scratch at the end of the line. Account for allocated capacities and check temporary high-water memory before extending its lifetime.
4. Preserve successful captures, unmatched groups, Unicode byte offsets, search starts, rule priority, backtracking limits, and all error behavior.
5. Keep the existing public search API and its owned results valid. Prefer one shared search implementation with explicit scratch, not two matcher implementations.
6. Do not retain source-derived patterns or add global pools. Keep grammar data and work limits unchanged. Do not skip preceding records or add fixture-specific behavior.

Before ordinary timing, compare the new and preserved search paths using actual regex and grammar cases.
Include alternating patterns with different capture counts, successes, misses, errors, Unicode, lookarounds, backreferences, and separate concurrent callers.
Run applicable pinned-dependency tests and exact pre-change sequential token comparisons.
Then run the existing source, pairing, reading-position, fallback, delivery, and cleanup contracts.
Pending-highlight resize must finish with checked colors, not only the correct text anchor.

If those checks pass, use the three alternating ordinary pairs and unchanged five-patch qualification required by the plan.
Keep every trial and recheck sustained memory, revisits, resizing, and quit.
A line-local buffer can still increase temporary peak memory. Its short lifetime is not proof of a favorable tradeoff.
Do not adopt the candidate unless correctness holds and the ordinary tradeoff is worthwhile.
This proposal is not yet implemented or performance-qualified.

## Checks and boundaries

- Formatting and application Clippy passed.
- The diagnostic build passed 37 process contracts, the 34-input alignment contract, the real worker test, ten library tests, and three real-grammar timing tests.
- Two pre-existing opt-in process tests remained excluded. No new exclusion was added.
- The independent full-source check matched all 20,847,451 supplied records across the five patches.
- All 78 focused action windows, ten saved-capture replays, and the separate sampled viewer passed.
- Fifteen actual-trace controls accepted valid accounting and rejected corrupted, failed, overflowed, or inconsistent copies.
- Six capture-protocol controls preserved legacy replay and rejected changed actions, windows, protocols, or unassigned output.
- Schema 1 and 2 remained readable. The retained schema-2 analysis was unchanged apart from copied input paths and hashes.

Review corrected the parser timer boundary before the diagnostic build and observations.
Closing the observation interval is no longer counted as parser execution.
An earlier development test compile error was also corrected before execution.
There were no failed or replacement measured captures in this checkpoint.

The normal executable was rebuilt without diagnostic features and checked separately.
Its 37 process contracts and 34-input alignment contract passed against the restored default path.
The diagnostic test runners were reused without another Cargo build, so they did not replace that default executable.
Some older qualification tests use the Cargo companion rather than the preserved binary. Both identities are recorded.

This stage did not repeat the full interior-view or sustained-memory cohorts.
It does not resolve the higher Bun RSS, unmet memory targets, physical terminal comparison, or release acceptance.
The prior production checkpoint and its adverse observations remain in effect.

## Reproduction and preserved identities

All stage workloads are complete. Do not restart drivers in `evidence/first-visit-highlighting-v1`.
Use new output directories for any later observation.
The stage contains source archives, build recipes, fixtures, frozen cells, raw packets, request traces, profile output, checks, and hashes.
The copied ordinary build manifest retains its original alignment-stage paths and provenance.

| Executable | SHA-256 |
| --- | --- |
| Preserved ordinary reference used for this stage | `294b866a1c1f7d838b776ea6823d845f63ecea807da5436dbd1de654be51bcd9` |
| Preserved diagnostic | `226677b579442f8d2182277384ca310267ed72ee4eb484ecefe58ee0ea0350ab` |
| Final ordinary executable | `78a17c8fb887684b2a180b04e0bc159a0faf8cb2f8b61f5251b209e46633cebc` |

The final ordinary hash differs from the earlier default build.
Its [build/check record](../../evidence/first-visit-highlighting-v1/normal-final/report.json) preserves the actual recipe, feature tree, source identities, executable, and checks.
No byte-identical rebuild or measured performance equivalence is claimed.

Start with [the stage manifest](../../evidence/first-visit-highlighting-v1/SHA256SUMS),
[qualification](../../evidence/first-visit-highlighting-v1/checks/report.json),
[frozen packet](../../evidence/first-visit-highlighting-v1/prepared/packet.json),
and [request attribution](../../evidence/first-visit-highlighting-v1/analysis.json).
