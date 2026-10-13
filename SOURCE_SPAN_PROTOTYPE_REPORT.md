# Exact source-length storage: isolated prototype

Date: 2026-09-23. Correctness and comparison audits passed. The prototype remains isolated and unadopted.

## Result

The isolated prototype preserves exact source, layout, colors, and navigation in the completed checks.
Its retained index storage matches the census model across all five patches.
The largest patch uses 35.05 MiB less accounted storage after descriptor costs.
This value is not a measured reduction in heap usage, RSS, or peak memory.

The normal viewer remains unchanged.
The [completed comparison](evidence/source-span-prototype-v1/COMPARISON_REPORT.md) passed its offline audit across 60 eligible processes and 14,700 exact terminal views.
All 62 attempts remain intact, including two power-ineligible attempts at slot 49.
The original 48 eligible processes did not repeat. The final continuation completed the remaining 12 processes.

The adoption decision is to hold the prototype.
Linux third-tour settled RSS increased from 679.7 to 728.4 MiB, despite the smaller retained index.
The memory-process peak increased from 704.2 to 748.2 MiB.
All three candidate tours had higher RSS than the corresponding reference tours.
The Bun candidate also had slower opening colors and index receipt in all five pairs.

Favorable observations remain visible: Linux index receipt improved in four pairs, and its largest latency-process peak decreased by 40.7 MiB.
Every measured navigation category and normal exit passed the existing goals.
These observations do not cancel the adverse repeated-viewing result or establish its cause.
No comparison, build, or diagnostic remains active.

## Implementation

Only `src/source_spans.rs` differs in the isolated runtime source.
Each eligible file stores exact byte offsets and lengths in separate boxed arrays.
The arrays use `u32` relative offsets, `u16` byte lengths, and the existing newline flags.
Eligibility requires every offset and length to fit exactly.
All absolute ranges retain the existing overflow and spool boundary checks.

A longer record keeps the existing compact representation for its entire file.
A larger relative offset keeps the existing wide representation.
The change does not truncate text, infer lengths, reorder lines, or change source identities.
It does not change caches, parser rules, highlighting, rendering, or navigation.
There is no new dependency, unsafe access, or input-specific branch.

The implementation uses one additional allocation per eligible nonempty file.
On this target, every file descriptor grows by 24 bytes.
The largest patch adds 75,238 allocations by representation count.
These costs remain part of the adoption decision.

## Completed checks

| Check | Result |
| --- | --- |
| Enabled source suite | 122 passed, four opt-in checks excluded |
| Formatting and application Clippy | Passed, including `-D warnings` |
| Preserved ordinary-release compatibility | 96 exact terminal grids and two canonical source streams matched |
| Complete canonical corpus | All 20,847,451 supplied source records matched the independent reference |
| Complete corpus projection | All 42,657,296 split/unified rows matched the independent projection model |
| Existing corpus workload views | All 1,770 exact grids passed across ten latency and memory packets |
| Storage accounting | All five corpus inputs and four small fixtures matched the census model |
| Final evidence audit | Passed exact field coverage, identities, packet completeness, and unchanged ordinary source |
| Ordinary comparison audit | Passed all 60 eligible processes, 14,700 exact views, 2,952 settled holds, and cleanup |

The projection pattern check also accepted 2,888 valid cases and rejected 716 invalid cases in both parsers.
The new boundary checks cover lengths of zero, 65,535, and 65,536 bytes, large offsets, newline flags, and mixed files.
The mixed-file check reads actual patch storage and checks complete bytes and split/unified destinations.
The existing suite covers wrapping, resizing, local sources, context expansion, cancellation, and cleanup.

The terminal replays use the existing source and sequential-color expectations.
They include sidebar behavior, interior destinations, layout changes, resizing, and repeated-navigation actions.
Each corpus process exited normally and restored the terminal.
The correctness replay used one memory tour without the sustained-memory holds.
It does not substitute for the declared three-tour memory comparison.

The source suite used Cargo test-build executables.
The separate preserved ordinary release passed the compatibility and corpus checks.
A pre-existing lifetime warning remains in vendored Syntect.
These results do not complete physical-terminal inspection or broader platform qualification.

## Accounted storage changes

Positive values mean less retained index and descriptor storage.
The values exclude allocator overhead, temporary construction peaks, highlighting, and other owners.

| Input | Reduction in bytes | Reduction in MiB |
| --- | ---: | ---: |
| Pierre patch | 35,088 | 0.0335 |
| Ghostty patch | 186,558 | 0.1779 |
| Node patch | 756,532 | 0.7215 |
| Bun patch | 1,981,430 | 1.8896 |
| Linux patch | 36,753,406 | 35.0508 |

The empty-file fixture grows by 48 bytes. The metadata fixture grows by 114 bytes.
The position fixture saves 26 bytes. The small review fixture saves 12 bytes.
The implementation has no threshold that hides these small-file costs.

## Preserved failures and review

The first suite attempt failed an existing assertion that expected eight bytes per short span instead of six.
Only that accounting expectation changed. Source, colors, navigation, and cleanup expectations stayed unchanged.

The corrected suite and compatibility checks passed, but the combined driver then rejected an executable identity change.
Cargo rebuilt the application during the test workflow without changing source.
Both failed attempts remain intact in `checks-001` and `checks-002`.
The existing release-preservation tool produced the separate ordinary executable for final qualification.

The [review](evidence/source-span-prototype-v1/REVIEW.md) records ownership, allocation costs, and qualification controls.
The classifier now stops at an early wide offset, as the ordinary implementation does.
A separate completion audit requires the complete five-input field and all ten packets.
The measurement driver requires exact agreement with the declared 60-slot schedule before its AC gate.
The preparation check accepted that schedule and rejected seven altered schedules and three incomplete input fields.
Optimized Python also stopped before preparation, so disabled assertions cannot silently weaken the checks.

## Decision and next investigation

The comparison used the preserved ordinary viewer and the isolated ordinary-release prototype.
Each input received five latency processes per role and one three-tour memory process per role.
Role order alternated in the declared serial schedule. Workloads, expectations, cache budgets, and release features stayed unchanged.
Every valid observation remains part of the comparison, including slower actions and higher memory.

Each trial starts a fresh viewer process, but the identity checks read the patch before launch.
These are warm-file measurements, not cold-filesystem measurements.
Text and color milestones describe receipt of correct final-layout output, not first progressive text or physical terminal paint.
The memory comparison measures process RSS and finite three-tour behavior, not a total-memory guarantee or an indefinite plateau.

The evidence does not support adoption yet. Independent review reached the same conclusion.
The next bounded investigation separates live allocations and their owners from allocator-held pages and other resident regions.
The first Linux tour already contains the adverse RSS difference, so the existing first-tour prefix provides a focused diagnostic route.
Both roles must use matching checkpoints, exact-view checks, and unchanged cache policy.
Diagnostic timings must remain separate from accepted ordinary-build timings.
The Bun opening difference also needs attribution before an adoption decision.
The [comparison report](evidence/source-span-prototype-v1/COMPARISON_REPORT.md) records the complete observations, limits, and next diagnostic boundary.

## Evidence

All new artifacts are under `evidence/source-span-prototype-v1/`.

- [Declared plan](evidence/source-span-prototype-v1/PLAN.md)
- [Preserved ordinary build](evidence/source-span-prototype-v1/candidate-release/build.json)
- [Ordinary-release checks](evidence/source-span-prototype-v1/release-checks/report.json)
- [Corpus qualification](evidence/source-span-prototype-v1/qualification-001/report.json)
- [Complete-field audit](evidence/source-span-prototype-v1/audit.json)
- [Prepared comparison](evidence/source-span-prototype-v1/measurement-plan.json)
- [Preparation checks](evidence/source-span-prototype-v1/preparation-check.json)
- [Power-interruption audit](evidence/source-span-prototype-v1/interruption-audit.json)
- [Continuation record](evidence/source-span-prototype-v1/CONTINUATION.md)
- [Continuation interruption audit](evidence/source-span-prototype-v1/continuation-interruption-audit.json)
- [Complete comparison report](evidence/source-span-prototype-v1/COMPARISON_REPORT.md)
- [Complete comparison audit and results](evidence/source-span-prototype-v1/comparison-analysis.json)
- [Preserved corrections](evidence/source-span-prototype-v1/CORRECTION.md)

The normal executable SHA-256 remains `1ab24c0c70c45a5b480b83e111fa909d33ed56f4f4af9510b30dad72558f2e4e`.
The isolated ordinary executable SHA-256 is `46c69f4974361e4ac4c31ea5ac8516c4c0319fcb0c5a122cb053265477a7a8fa`.
Its build record preserves the compiler, SDK, environment, Cargo configuration, feature tree, fingerprints, and exact runtime source archive.
