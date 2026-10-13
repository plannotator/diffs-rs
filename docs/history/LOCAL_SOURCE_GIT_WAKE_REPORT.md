# Git completion notification

Date: 2026-09-23. Scope: one child-wait change, independent of checkpoint retention.

The notification is adopted as a Git-readiness improvement. Normal-build qualification is complete.
The checkpoint-retention prototype remains separate and unadopted.

## Change and ownership

The existing diagnostic reader sends one notification after it finishes reading stderr.
The waiting thread checks the real child status after that notification.
If the child remains active, the existing 10 ms cancellation interval continues.
A disconnected channel also acts as one notification, not repeated readiness.

The change adds one capacity-one channel per Git command. Both endpoints expire when that command finishes.
It adds no dependency, worker thread, public setting, source copy, or persistent cache entry.
The private stdout file, 8 KiB diagnostic tail, status rules, and child guard remain unchanged.
Patch parsing, line identities, layout, coloring, and checkpoint policy do not change.

Stream closure can precede child exit. The notification cannot establish success by itself.
An early notification can still require a full fallback wait. It does not guarantee a particular opening time.

## Correctness and code review

The final ordinary suite passed all 118 enabled checks, including 61 terminal-process contracts.
Four opt-in process checks remained excluded.
Each diagnostic version passed 80 selected checks: source inputs, source coloring, input-worker lifecycle, and all 61 enabled terminal contracts.

Three new checks run real shell children through the production process wrapper.
Those children close stderr, signal readiness, and wait at a FIFO without starting descendants.
They verify late stdout, successful exit, allowed difference status, rejected status, bounded diagnostic output, cancellation, and child reaping.
Existing integration checks still use actual Git for source correspondence and terminal behavior.
The tests add no runtime hooks or mock implementation.

Formatting and application Clippy passed. The existing vendored Syntect lifetime warning remains unchanged.
Self-review checked channel ownership, failure paths, cancellation, reader joining, and status authority.
Only one message can enter the capacity-one channel, so reader completion cannot block behind an unconsumed earlier message.
After the first notification, the waiter stops reading that channel. A disconnection cannot create a tight polling loop.

Before capture, 2,166 product source, fixture, asset, test, dependency, and tool files matched the preserved baseline.
The ordinary prototype changes only `src/input/git.rs` and adds `src/input/git/tests.rs`.
The two diagnostic source trees differ only in `src/input/git.rs`.
Dependencies, assets, fixtures, and expected views remain unchanged.

## Diagnostic observations

Eighteen fresh processes opened three existing inputs, with three pairs per input.
The middle pair reversed executable order. Every opening used the same two-second observation window and 160 by 50 terminal.
All eighteen final grids matched the independently checked references.
Offline replay reproduced the recorded text and complete-cell milestones. Diagnostic frame counts matched captured frames.
No diagnostic event or request summary was lost.
Terminal restoration, private-file cleanup, RSS containment, and sampler shutdown passed.

These medians measure stderr closure to observed child completion, summed over the commands in each opening.
They do not measure the exact child-exit instant or physical terminal paint.

| Input | Previous wait | Notification wait |
| --- | ---: | ---: |
| 12,000-line source comparison | 18.626 ms | 0.029 ms |
| Twelve-file Git comparison | 47.247 ms | 0.103 ms |
| Rust source comparison | 6.005 ms | 0.028 ms |

The file comparisons each ran two Git commands. The Git comparison ran eight, including lazy source acquisition.
One candidate Git command required another wait after notification. All other candidate commands completed without a post-notification wait.
This result supports the intended mechanism. It does not replace an ordinary-build comparison.

The deep source parsed 23,926 lines in every diagnostic opening. The Rust source parsed 1,512 lines.
The Git source-readiness schedule changed how much intermediate coloring was canceled; final cells still matched.
No source prefix was skipped and no syntax policy changed.

## Ordinary comparison

Eighteen ordinary processes completed all eleven frozen actions. All 198 final grids passed, then passed offline replay.
No owned build or competing capture overlapped either series. AC checks passed at every capture boundary.
Every attempt remains included, including the slower first pair.

These are medians of three observations per role, in milliseconds from launch to the reference view.
The text milestone checks source text separately from the complete styled grid.

| Input | Text, before / after | Complete cells, before / after |
| --- | ---: | ---: |
| Deep source | 158.80 / 164.85 | 702.27 / 702.81 |
| Git | 284.53 / 239.78 | 325.72 / 281.18 |
| Rust source | 178.35 / 165.89 | 206.10 / 192.10 |

Git opening improved by 42.59, 27.60, and 47.76 ms in the paired observations.
Rust opening improved by 13.42, 8.77, and 14.00 ms.
The first-terminal-output interval did not explain those gains: the remaining interval improved in every Git and Rust pair.
The deep-source complete-cell changes were +30.99, -7.94, and +13.47 ms. Its median was effectively unchanged.
That input still performs the same hidden-source parsing. This change does not solve its coloring delay.

Later actions were mixed. Git theme completion rose from 20.87 to 26.20 ms for light, and 20.32 to 24.56 ms for dark.
Rust page-scroll coloring rose from 32.84 to 37.21 ms. Its expansion action rose from 18.09 to 23.38 ms.
Several expansion, navigation, and resize observations improved. The full per-action record remains in `ordinary-analysis.json`.
These differences are not discarded. The experiment does not establish a general improvement in coloring, layout, or responsiveness.

| Input | Process CPU median, before / after | OS peak RSS median, before / after |
| --- | ---: | ---: |
| Deep source | 1.637 / 1.694 s | 29.00 / 29.00 MiB |
| Git | 0.285 / 0.300 s | 33.56 / 33.28 MiB |
| Rust source | 0.220 / 0.221 s | 16.78 / 17.03 MiB |

CPU directions varied between individual pairs. These CPU medians do not demonstrate a CPU reduction.
Final-window RSS medians were 29.00 / 29.00, 33.56 / 33.28, and 16.78 / 16.67 MiB respectively.
RSS includes operating-system residency effects. It is not a live-heap measurement or memory bound.

## Adoption decision

The diagnostic establishes reduced completion-observation delay. Ordinary Git and Rust opening improve in every pair.
The implementation has a bounded, command-local cost and preserves the tested source and child-lifecycle contracts.
These results justify the small notification change, without requiring every shared-workstation observation to improve.
The result does not justify a universal speed claim or dismiss the adverse observations above.

The production change includes only the process wrapper and its three real-child regression checks.
No coloring, retention, rendering, or acquisition policy is combined with it.
The normal rebuild passed all 118 enabled checks, formatting, and application Clippy.
It then passed three additional correctness tours: 33 complete reference grids, with offline replay and cleanup checks.
Its 2,167 source, dependency, fixture, asset, test, and tool files match the qualified ordinary prototype archive.

The normal build has a different executable hash from the isolated prototype, despite matching source.
The paired timing numbers above belong to the isolated prototype. The normal rebuild's separate correctness tours are not another paired comparison.
Its observations remain in `adopted-analysis.json`, without being pooled into the earlier series.

## Limits and remaining work

This experiment does not adopt the separate checkpoint-retention prototype or explain all its earlier adverse observations.
It does not change hidden-prefix parsing, cache budgets, remote acquisition, or public controls.
Shared-workstation timing differences can include process launch, operating-system scheduling, and recorder delivery.
PTY receipt is not physical terminal paint. Short RSS samples are not a sustained-memory qualification.
The existing file and Git page steps move selection. The second Rust page step also moves the viewport.
These actions do not qualify sustained scrolling across large patches.
The larger resource campaign and physical-terminal checks remain separate work.

## Evidence and preparation

The packet is `evidence/local-source-git-wake-v1/`. Its `evidence/` subdirectory contains the captured data and build artifacts.
It retains the plan, ordinary and diagnostic source archives, exact executables, build logs, declarations, raw captures, and offline analyses.
The scripts preserve their machine-local paths. They are experiment records, not installation commands.
The existing `.measurement-venv` supplies the recorder's Python modules; no package was installed.

A formatting check found the new FIFO retry branch before capture. Formatting and the full suite then passed again.
The initial archive comparison included an existing nested dependency build cache. The source check now excludes `target` directories.
The first driver import used the system Python and lacked `regex`. The established measurement environment resolved that before any capture.
No measured attempt was discarded or replaced.

The preserved ordinary baseline is `3f87c079518ee654053658a7015e56319a5c377a05ce568f45a767b456be5a15`.
The ordinary prototype is `9c23fae73e446f0f17ed6b97d257dff0231192e091d176e412fe8f76ac533221`.
The diagnostic baseline is `5ca9f4ed6961cbf6f257c3a03c8b754cc80843101dfee766281448fde2d7da92`.
The diagnostic prototype is `3a677df2fe22ff991c5d1a21bde3086562b32950ecd8f1d556ce02cb136d3fef`.
The adopted normal executable is `fd13e46f331f41965b4addce77223c6f8a492f0622b4db2a24fe89ed37336b3c`.

The before/after declarations pin the product before adoption. Their recorded old product hash resolves to the preserved baseline executable.
The adopted declaration and integrity result identify the later normal build separately.
Historical evidence, global installations, and external projects remain unchanged.
