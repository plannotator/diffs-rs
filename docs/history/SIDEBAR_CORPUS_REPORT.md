# Sidebar-aware corpus qualification

Date: 2026-09-23. Status: preparation complete. Performance captures have not started.

## Outcome

The current ordinary executable passed 1,765 selected views across the five pinned patches.
These views cover the opening/navigation workload and one preparation tour of each memory workload.
The checks include source text, line numbers, source order, syntax colors, navigation, wrapping, and the default sidebar.
All ten workload packets have current hash bindings.

The normal executable, Rust source, dependencies, cache limits, and diff algorithms are unchanged.
The executable SHA-256 remains `1ab24c0c70c45a5b480b83e111fa909d33ed56f4f4af9510b30dad72558f2e4e`.
This checkpoint establishes correctness for the declared views. It establishes no speedup or memory reduction.

Scope: [the frozen plan](../../SIDEBAR_CORPUS_PLAN.md).
Evidence index: [the machine-readable manifest](../../evidence/sidebar-corpus-v1/manifest.json).

## Checker changes

The existing checker now uses the available diff width after the sidebar occupies its columns.
One geometry helper supplies the origins and widths for source text, gutters, colors, wrapping, and frozen comparisons.
The checker derives the file tree from raw patch paths and independent hunk counts.
It checks the visible hierarchy, names, counts, current-file marker, divider, and color roles.

The horizontal limit comes from the visible source lines and available code width.
Dedicated corruption controls exercise both bounded horizontal scrolling and source text after automatic sidebar hiding.
The old no-sidebar decoder remains available for historical traces.
The current measurement mode rejects historical freezes that lack the new presentation contract.

The timed path uses previously checked screens.
Expected-region preparation now occurs before process launch or input injection.
The runner still parses terminal output and compares frozen cells during capture.
Expensive source and sequential-color checks run during preparation, outside accepted timing captures.

## Results

Each count includes the opening view. Repeated views remain counted as observations, not unique source coverage.

| Patch | Opening/navigation views | Memory-workload views | Rejected corruption cases |
| --- | ---: | ---: | ---: |
| `pierre-0800fb` | 205 | 139 | 54 |
| `ghostty-12291` | 205 | 118 | 54 |
| `node-59805` | 205 | 139 | 56 |
| `bun-30412` | 205 | 139 | 54 |
| `linux-v6-v7` | 205 | 205 | 54 |
| Total | 1,025 | 740 | 272 |

All 272 corruption cases failed the checker as required.
The cases change actual captured cells or their required source destinations, not a simulated viewer.
The checks cover source glyphs, gutters, missing rows, source order, syntax colors, backgrounds, and sidebar errors.
Thirty views exercise independently bounded horizontal offsets.

The separate geometry trace passed 15 views at widths of 160, 100, 96, and 97 columns.
It covers split/unified layouts, wrapped long text, and automatic sidebar hiding and restoration.
Its final checker rejected all 27 corruption cases.
A second real-process run reproduced those 15 complete screens through the frozen measurement matcher.
That run also passed source, color, terminal-mode restoration, cursor restoration, and mouse cleanup checks.

The source archive check matched all 2,083 runtime, build, vendor, and asset files before and after the frozen-matcher run.
The final manifest repeats that identity check and checks all ten freezes.
No Rust test suite rerun belongs to this checkpoint because the Rust source and executable did not change.

## Coverage limits

The tree trace uses the default expanded sidebar without tree focus, folder collapse, or manual tree scrolling.
Earlier sidebar feature checks retain responsibility for those controls.
This checkpoint does not replace physical-terminal inspection.

Of the 1,765 workload views, 1,516 have one independently constrained current-file identity.
The other 249 have multiple visible source files and no exact independent current-file constraint.
For those views, the checker checks consistency with the visible-file set and records the weaker coverage.
Even a constrained identity proves tree-slice consistency, not unique identity from truncated labels alone.

The syntax reference uses sequential parsing with the same grammars.
It checks optimized replay against sequential output, not the correctness of every grammar.
Patch-only input still lacks omitted full-source context.
These workloads do not replace the separate complete-source qualification or canonical checks of every supplied source record.

The memory preparation contains one correctness tour per patch.
The frozen measurement policy requires five fresh processes, with three tours per memory process.
No sustained-memory result follows from preparation alone.

The opening milestone records receipt of correct text in the final layout.
It is an upper bound, not necessarily the first earlier usable text during progressive loading.
Diff-color completion excludes sidebar publication. Complete-screen readiness includes it.
PTY receipt does not measure physical terminal paint or display latency.

## Preserved attempts

The first opening/navigation attempt stopped because the checker assumed exact current-file identity without sufficient visible evidence.
No viewer defect followed from that failure. The corrected checker records the observable identity scope.
That attempt remains under `evidence/sidebar-corpus-v1/pierre-latency-attempt-1/`.

The second attempt passed its then-current checks but lacks the final coverage fields.
It remains preparation history, not a current measurement input.
The first `frozen-latency/` packets became obsolete after helper review corrections.
No performance capture used those packets. Only `frozen-v2/` is current.

All attempts remain available. Incidental preparation timing and resource observations are not accepted performance results.

## Next capture

1. Coordinate a quiet, AC-powered window before the serial capture schedule.
2. Check the current freezes and preserve the power and background-work conditions.
3. Run the current-build workload with the frozen repetition and memory-tour policy.
4. Retain every attempt, including failures and adverse observations.
5. Report text receipt, color completion, responsiveness, peak memory, and settled memory separately.

The first result will be a baseline for the current build.
A paired historical comparison needs separately checked expectations for the older presentation.
The current sidebar screens cannot silently become that older executable's expected output.
Physical captures and complete-source resource qualification remain separate work.
