# RSS shutdown boundary and current-build measurements

Date: 2026-09-13. Candidate: `da5754e8…`. Protocol: `process-group-rss-v3`.

## Result

The measurement helper now stops and joins its RSS sampler before intentional
quit. Real-process controls, the capture correctness gate, five ordinary paired
comparisons, and the full five-patch checkpoint pass their correctness and
completion checks. The offline audit verifies their evidence. No application
source, executable, grammar, cache limit, workload, or performance target changed
in this stage.

The candidate remains unpromoted. Four memory goals, a distant-coloring maximum,
and the largest patch's complete-index goal remain unmet. Earlier coloring
differences are not resolved by this batch. Actual Ghostty/Kitty paint and complete
Pierre visual parity remain unverified.

## Measurement-helper change

After the last completed viewing action, the helper requests sampler shutdown
and joins with a two-second bound. An in-flight query may finish; its result and
any error remain. The record includes the monotonic clock basis, request and
join-finish times, final query count/end time, reason, and status. Repeated cleanup
uses the same record. Unknown process state and unavailable memory still fail;
neither becomes a zero sample.

The helper then checks for premature process exit. It checks again after any
input-cadence wait, before writing. Quit latency starts at actual input injection;
negative elapsed times fail both capture replay and corpus acceptance. Cleanup
still terminates/reaps the child and closes the PTY if joining fails.

The query command, 100 ms post-query wait, one-second holds, five-query minimum,
full-query-containment rule, and targets are unchanged. Sampled group RSS now ends
at the viewing-phase boundary, not during intentional teardown. OS-reported peak
RSS still covers the viewer's full lifetime. The longest observed sampler join
in the 30 measured processes was 0.435 ms; this helper time is separate from quit.

Current captures use schema 3. Readers require explicit evidence that the last
view finished before the shutdown request, joining finished before quit input,
and cleanup did not overwrite that boundary.

## Verification

- Real-process controls cover group membership and state, frozen views, all five
  required holds, retained safety errors, normal quit, forced cleanup, premature
  exit, and an actual SIGINT during the input-cadence wait. The failed-join cleanup
  branch was reviewed, not forced at runtime.
- The final capture gate passes 17 checks. Four actual-viewer recordings match
  exact cells and source. Recorded-output controls reject bad text/color,
  truncation, reversed timestamps, skipped packets, late sampler errors, missing
  or invalid join evidence, and negative quit time. A real overflowed capture
  fails and reaps its viewer.
- The five alternating pairs contain exactly 40 visual sessions and 280 checked
  actions. Ten joined-emoji entries remain source-only because of the decoder's
  limitation. All successful recordings replay from disk with identical receipt
  observations and clean exits.
- The full checkpoint completes 30 scheduled commands: ten checked rehearsals,
  ten preparations, and ten measured series. Its 25 latency processes and five
  three-tour memory processes match 7,257 action views plus 30 openings.
- All 1,428 memory holds contain at least eight fully contained valid queries.
  The audit reconciles 32,968 raw queries with membership, RSS totals, timelines,
  hold intervals, and final shutdown records. All 30 measured groups are absent
  in the post-run process check.
- The offline audit binds the exact paired roster and checkpoint schedule to
  executable, fixture, trace, preparation, repetition, and helper identities. It
  reconciles replayed observations and summaries, and requires every hold at its
  frozen trace position. Independent review identified and closed those checks'
  initial gaps. No new Rust test-suite run is claimed; unchanged application
  sources still match the previously tested candidate manifest.

The first helper control failed because its fixed 81-action prefix omitted two
required hold destinations. That failure is retained. The corrected control
preflights the same destinations and runs through the last required one. The
first corrected controls and capture gate passed; subsequent review found the
cadence-exit edge case. Their helper versions and artifacts remain preserved.
Final controls and a new final gate passed before any paired timing run. No
paired or full-corpus performance series was retried.

## Paired responsiveness

The preserved reference is `1feef607…`; the candidate is `da5754e8…`. The fixed
capture-first series uses the same 500 ms capture windows and actions for both.
These are terminal-output receipt times, not display-paint times.

| Observation | Reference median / maximum | Candidate median / maximum |
| --- | ---: | ---: |
| Continuous end, body coloring | 62.84 / 67.36 ms | 63.22 / 74.41 ms |
| Ratatui narrow resize, exact view | 2.80 / 4.38 ms | 4.06 / 4.23 ms |
| Guard opening, exact readiness | 104.12 / 105.37 ms | 106.18 / 107.58 ms |

Other observations are mixed. All 28 case/action rows retain all five values,
medians, and maxima for correct text, body styles, and exact readiness. The
continuous-end paired differences change direction; the larger candidate maximum
remains reported. The smaller resize maximum does not erase its slower median.
Five pairs supported proceeding with characterization, not claiming equivalence,
dismissing differences as noise, or promoting the combined candidate.

## Five-patch responsiveness

Ordinary release build on Apple M5 Max, 48 GiB RAM, macOS 26.3 arm64; Python
3.12.13 and Rust 1.96.0. Instrumentation was disabled; inherited
`MallocNanoZone=0` is recorded. All viewer/build/profile workloads launched by this
task were serial. This was not a controlled cold-cache or dedicated-host trial.

Each latency fixture has five processes and 200 scored observations per category.
The existing full-corpus observer still performs inline validation; its times
must not be equated with capture-first timings, application CPU time, or paint.
Public navigation used to reach interior destinations is recorded separately as
setup. Its substantial elapsed time on the largest input is not hidden in the
scored action figures.

| Fixture | First text max | First body color max | Complete index observed max | Navigation text / color max |
| --- | ---: | ---: | ---: | ---: |
| Pierre showcase | 8.17 ms | 35.89 ms | 35.89 ms | 12.34 / 299.01 ms |
| Ghostty patch | 8.22 ms | 46.42 ms | 69.18 ms | 25.75 / 53.17 ms |
| Node patch | 11.55 ms | 55.25 ms | 192.90 ms | 10.84 / 50.96 ms |
| Bun patch | 10.22 ms | 42.55 ms | 194.20 ms | 13.54 / 198.12 ms |
| Largest Linux patch | 11.86 ms | 22.53 ms | 4,443.58 ms | 9.76 / 74.73 ms |

All category p95 values meet the unchanged 50 ms goal. However, the showcase's
first distant jump, `endpoint-0`, exceeds the 200 ms coloring goal in every
repetition: 227.99, 225.66, 221.91, 224.50, and 299.01 ms. Correct text arrives
around 5.5 ms at that step. The category's low p95 does not erase this repeated
first-visit delay. Its correct-text maximum across all distant seeks is 10.31 ms.
The largest input's index observations are 4.20–4.44 seconds across all five
repetitions, exceeding the shared 2,000 ms goal. All first-text, first-color,
and quit targets pass. The longest
observed quit across all 30 measured processes was 104.44 ms, below 250 ms.

## Sustained memory

Each value below is MiB. Settled values are maxima of the required one-second
holds in each complete tour. They are not a total-memory guarantee or proof of
a steady-state plateau. The last column is the OS-reported viewer-lifetime peak
for that memory process, not the terminal emulator's memory.

| Fixture | Tour 1 settled max | Tour 2 settled max | Tour 3 settled max | OS peak |
| --- | ---: | ---: | ---: | ---: |
| Pierre showcase | 196.77 | 213.63 | 213.81 | 213.86 |
| Ghostty patch | 108.81 | 109.77 | 111.95 | 111.98 |
| Node patch | 224.13 | 233.70 | 235.13 | 235.14 |
| Bun patch | 274.84 | 313.00 | 337.06 | 337.06 |
| Largest Linux patch | 762.31 | 752.08 | 646.61 | 762.31 |

Only Ghostty meets both the unchanged 128 MiB third-tour and 192 MiB peak goals.
Bun continues growing across these tours. The largest input shows substantial
RSS decreases, including an adjacent-query drop of 203.09 MiB. Their cause remains
unattributed. Its smaller third-tour maximum cannot be credited to regex sharing
or used to claim a live-memory reduction.

The previous controlled allocation pair remains the evidence for the ownership
change: 61.87 MiB less live heap after three showcase tours, including registry
overhead. It is separate from this ordinary RSS cohort. The largest input's
381.34 MiB of accounted document structures is unchanged. Historical RSS cohorts
used earlier builds and observer versions; differences between their maxima and
this table are not controlled application deltas.

## Decision and next work

Close the sampler-lifecycle repair and this finite qualification run. Keep the
working candidate unpromoted and all earlier slower or failed evidence intact.
No viewer, build, profile, or measurement from this stage remains running.

Next, use owner-level evidence to distinguish retained allocations from RSS
changes before another memory refactor. The repeated first-distant-jump delay
provides a concrete destination for cold-coloring attribution. Physical terminal
verification remains open. Do not change grammar coverage, source
answers, fallback thresholds, or cache limits to meet the numbers above. This
batch does not justify another optimization or a claim of universal correctness
on its own.

## Evidence

- [Declaration](evidence/rss-shutdown-v1/RUNBOOK.md) and
  [final real-process controls](evidence/rss-shutdown-v1/process-checks-final.json)
- [Final capture gate](evidence/rss-shutdown-v1/capture-check-final/sessions/checks.json)
  and [all paired observations](evidence/rss-shutdown-v1/paired/summary.json)
- [Decision to continue](evidence/rss-shutdown-v1/paired-decision.json) and
  [full checkpoint](evidence/rss-shutdown-v1/five-patch/checkpoint.json)
- [Offline audit](evidence/rss-shutdown-v1/verification.json) and
  [process-exit check](evidence/rss-shutdown-v1/process-exit-check.json)
- [Helper/source overlay](evidence/rss-shutdown-v1/rss-shutdown-source-overlay.tar.gz)
  and [SHA-256 manifest](evidence/rss-shutdown-v1/SHA256SUMS)
- [Prior implementation and allocation report](CROSS_GRAMMAR_REPORT.md)

The earlier seal was verified before helper edits. Its historical source overlay
retains those versions; it is not rewritten to match current helper files.
