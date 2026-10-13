# Local-source opening attribution

Date: 2026-09-23. Status: diagnostic complete. Production source and executable remain unchanged.

## Decision

Checkpoint planning is not a material direct cost in these openings.
The measured planning windows span 0.334 to 1.374 microseconds per candidate opening.
All measured checkpoint admissions total less than 0.063 ms per opening.
These measurements do not explain every difference in the earlier ordinary comparison.
The retention policy remains a preserved candidate, not the default implementation.

The next implementation experiment will examine Git completion waits independently of retention.
The P2 finding is fixed-interval completion polling in `src/input/git.rs:123` through `Git::run`.
The existing waiter checks the child, then sleeps for 10 ms before another check.
The diagnostic shows time between stderr EOF and the later completion observation.
That interval is an opportunity for an earlier wake, not proof that the child already exited.
Cancellation, actual child status, output completeness, and cleanup must remain authoritative.

## Matched experiment

The baseline and candidate came from the preserved ordinary source archives.
Both received equivalent feature-gated phase timers in isolated directories.
Only the three original retention-policy files differ between their runtime sources.
The baseline's checkpoint-planning window surrounds no operation.
The candidate's window surrounds its existing protection loop.

The three frozen inputs retained their independent 160 by 50 opening expectations.
Each input ran through three serial pairs. The middle pair reversed the executable order.
Each process captured two seconds of opening output, then quit.
All eighteen processes stayed on AC power. No owned build or other capture overlapped the series.

All eighteen final grids matched.
Offline replay reproduced every saved opening milestone and matched every diagnostic frame to its terminal receipt.
No trace event or request summary was lost.
The checks also passed for RSS sample containment, sampler shutdown, terminal restoration, and private-file cleanup.

Both instrumented versions passed all 61 enabled real-process contracts.
The selected baseline suites passed 80 checks. The candidate suites passed 83 checks, including its three retention regressions.
Four opt-in process contracts remained excluded in each version.
Formatting, application Clippy, and an ordinary-feature compile check passed.
The existing vendored Syntect lifetime warning remains unchanged.

The self-review covered timer lifetimes, thread synchronization, feature gates, source conservation, and nested-duration accounting.
The product's 2,148 source, test, dependency, tool, and manifest files still match the baseline archive.

## Observed costs

These values are diagnostic medians in milliseconds, not accepted ordinary performance results.
The Git row includes child execution and completion observation. It is not all avoidable waiting.
The parser and Git phases can overlap. Their durations must not be added as a critical-path estimate.

| Phase | Deep source baseline / candidate | Git baseline / candidate | Rust source baseline / candidate |
|---|---:|---:|---:|
| Git command lifecycles | 35.32 / 38.13 | 123.41 / 124.48 | 24.12 / 25.69 |
| Source correspondence | 0.124 / 0.127 | 0.121 / 0.117 | 0.237 / 0.237 |
| Parser calls | 530.59 / 526.53 | 51.59 / 52.30 | 25.80 / 26.05 |
| Parser initialization, included in parser calls | 20.81 / 20.69 | 24.24 / 24.32 | 3.99 / 3.87 |
| All drawing and output spans | 2.36 / 2.63 | 4.46 / 4.66 | 4.11 / 4.23 |

The deep source parsed 23,926 lines on every opening. The Rust source parsed 1,512 lines on every opening.
The Git case parsed 2,432 lines, except one baseline process with 2,332 lines.
Its different source-readiness scheduling canceled a different amount of intermediate work. Final cells still matched.

Each two-file opening ran two Git commands. Each Git opening ran eight commands, including lazy source acquisition.
The Git case's sum of stderr-EOF-to-completion-observation intervals ranged from 23.66 to 43.24 ms across both roles.
EOF does not establish the child-exit instant. This sum is not a promised speedup.
The patch-publication queue also waited for its consumer, with median spans of 6.29 ms and 11.48 ms in the Git case.
Those waits and source-readiness scheduling remain relevant to opening variability.

The new diagnostic receipt medians were 690.31 / 700.74 ms for the deep source.
They were 290.04 / 298.93 ms for Git and 176.27 / 177.88 ms for the Rust source.
The candidate remained slower in all three Git pairs. Directions were mixed for the other two inputs.
These observations do not supersede the earlier ordinary comparison.

## Launch and receipt boundary

The capture clock starts before the process fork.
That boundary is explicit in `tools/terminal_baseline.py:98`.
The viewer's trace starts inside its command handler, after argument parsing.
The first terminal bytes are mode-control sequences, not readable diff content.
They precede local-source worker creation in the application.

The eighteen diagnostic traces matched the captured frame count exactly.
Within a process, receipt timestamps minus corresponding trace timestamps varied by less than 0.367 ms.
Their offsets ranged from approximately 108 to 323 ms across processes.
This identifies a substantial launch-and-receipt interval outside the traced work.
It does not separate OS loading, wrapper execution, pre-trace application work, and recorder scheduling.

Offline replay also checked all eighteen openings from the previous ordinary comparison.
Their original measurements and captures remain unchanged.
For the Git and Rust inputs, the first packet contains only the same 54 terminal-control bytes in every process.

| Earlier input | Candidate delay at first terminal bytes, by pair | Remaining delay to complete cells, by pair |
|---|---:|---:|
| Git | +14.86, +27.90, +22.61 ms | +11.14, +34.95, +1.65 ms |
| Rust source | +6.17, +11.34, +15.16 ms | +1.35, +29.24, +18.86 ms |

Part of the earlier difference therefore precedes the first observed terminal message.
The remaining difference is real in those captures and still needs explanation.
The earlier deep-source outlier also remains unexplained.
No observation was discarded, relabeled as noise, or subtracted from the original end-to-end metric.

Future opening reports must retain the original launch-to-complete-cell value.
They must also separate first terminal output, first readable source, and complete coloring.
None of those PTY milestones measures physical terminal paint.

## Next bounded experiment

The next prototype will use the existing Git stderr reader to signal a possible completion.
The waiter must still obtain the child's exit status and retain its bounded cancellation checks.
An early stderr close must not produce early success, incomplete stdout, or a busy loop.
No new dependency, additional worker, global setting, or polling-frequency target is planned.
The existing private stdout file and bounded stderr tail remain unchanged.

The checks must exercise real child processes, including early stderr closure, delayed exit, successful and failed commands, and cancellation.
The existing exact-source, missing-object, full-queue, navigation, and cleanup contracts remain gates.
A matched diagnostic must show whether the wake signal removes completion-observation delay.
An ordinary comparison follows only after that mechanism and its costs are understood.

This change must remain separate from the retention candidate.
It must not serve as a combined change that hides the retention tradeoff.
Retention adoption still requires a justified decision about its opening and smaller-action observations.
Large-patch performance, sustained memory, and physical-terminal qualification remain separate work.

## Evidence and preparation notes

The packet is `evidence/local-source-opening-attribution-v1/` in the project directory.
`analysis.json` contains all six complete diagnostic groups and the earlier ordinary opening decomposition.
`declaration.json` pins the schedule, binaries, source files, dependencies, inputs, driver, and reference helpers.
The diagnostic scripts retain their original machine-local paths. They are evidence, not release commands.

The initial baseline archive lacked assets. Matching preserved assets and fixtures completed that isolated copy.
A shared-target build then reused the baseline artifact for the candidate. Separate target directories corrected this before captures.
The real-process test build changed executable hashes, so the final capture used the exact test-qualified executables.
An initial summary omitted two grouped tables. The complete offline analysis corrected grouping without new captures or changed raw observations.
All preparation logs and intermediate artifacts remain preserved.

The unchanged normal executable is `3f87c079518ee654053658a7015e56319a5c377a05ce568f45a767b456be5a15`.
The diagnostic baseline is `aa596b5b4bc2b39fd0c84901f2b33263f6b5b103dc9b5cb3fe6db6fa86b0de21`.
The diagnostic candidate is `0120cbeeb9afd1d40e3614361f002ea8bd18c9a9a2d769db9612fbe23417a08b`.

No production runtime file, historical packet, global installation, or external project changed.
