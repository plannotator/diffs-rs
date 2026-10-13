# Current-build five-patch checkpoint

Completed on 2026-09-13 with the preserved ordinary candidate
`9f16c2c92a9efd2d3cf77e5c0a5a41d314124099cafa577ad36116ed83a1b657`.

All scheduled correctness and measurement checks completed. Navigation, resize,
first-text, first-color, and quit targets pass in the latency series. Memory
goals remain unmet for Node, Bun, and Linux; Linux also misses the full-index
goal. The candidate is not promoted. No application source, executable, grammar,
cache limit, highlighting guard, workload, or target changed in this checkpoint.

## What ran and what was verified

The [declared v2 runbook](evidence/inner-engine-checkpoint-v2/RUNBOOK.md) uses the
existing serial protocol: ten checked rehearsals, ten preparations, 25 fresh
latency processes, and five memory processes with three tours each. Each fixture
has 200 latency observations per category: forward, reverse, distant seek,
layout, and resize. No v2 performance process was retried or discarded.

The [offline audit](evidence/inner-engine-checkpoint-v2/verification.json) verifies:

- The fixed 30-command schedule, candidate and source identities, preparation
  freezes, original style-reference identity, and historical trace semantics.
- 7,257 action views plus 30 openings, matched during execution against the
  checked source/styling expectations. Completed action order and hold positions
  reconcile with the frozen traces. All ten summaries recompute from raw records.
- All 1,428 required one-second holds completed. Across the 30 measured processes,
  31,270 process-group RSS queries reconcile with raw command output, members, totals, time intervals,
  peaks, and sampler shutdown. Every hold has at least five fully contained
  valid queries. One hold, Linux tour three's `long-page-19`, has exactly five;
  it is retained without changing the minimum or rerunning it.
- All 30 measured processes exited successfully, restored terminal state, and
  stopped/joined sampling before intentional quit. Their process groups are absent
  in the post-run check. The longest recorded sampler join was 0.116 ms.

The audit checks retained observation records; it does not independently replay
a complete raw terminal-byte recording. The separate capture-first checks and
full source/projection checks belong to the preceding
[implementation stage](INNER_ENGINE_REPORT.md). This checkpoint did not rerun
the Rust suites or prove correctness for every possible patch.

## Responsiveness

These are ordinary release-build, warm-file observations on an Apple M5 Max,
48 GiB RAM, macOS 26.3 arm64, with Python 3.12.13 and Rust 1.96.0. Allocation
instrumentation was disabled; the inherited `MallocNanoZone=0` was enforced and
recorded. Owned workloads ran serially. The Mac was not a dedicated benchmark
host, and system caches and background applications were not changed.

The existing corpus observer validates inline. Its terminal-output receipt
times include observer effects and are not physical paint, application CPU
time, frame-rate measurements, or interchangeable with capture-first timings.
The input cadence and terminal geometry remain those of the frozen protocol.

All values below are milliseconds and maxima across the five latency processes.
Navigation combines the five measured categories, excluding setup travel.

| Patch | First text | First body color | Full index observed | Navigation text / color |
| --- | ---: | ---: | ---: | ---: |
| Pierre showcase | 10.03 | 36.38 | 36.38 | 12.84 / 145.97 |
| Ghostty | 11.77 | 53.51 | 86.85 | 37.45 / 51.56 |
| Node | 13.22 | 84.49 | 214.75 | 18.19 / 106.04 |
| Bun | 8.73 | 71.64 | 213.57 | 11.16 / 80.39 |
| Linux, largest input | 8.54 | 19.27 | 4,235.94 | 9.69 / 55.49 |

All category p95 values meet 50 ms; all category maxima meet 200 ms. Resize
body-color p95 / maximum values are 10.34 / 12.84 ms, 23.17 / 37.45 ms,
10.08 / 12.15 ms, 10.32 / 11.16 ms, and 7.45 / 9.19 ms in table order.
The longest quit observation across all 30 measured processes was 59.80 ms.

Linux's full index was observed at 4.17 to 4.24 seconds in the latency series,
above the shared two-second goal. Its separate memory process observed index
completion at 4.34 seconds. First text and body color arrive earlier; neither
milestone implies the whole index is ready.

The first distant jump in the showcase takes 111.18 to 145.97 ms to color in
these five runs. Node retains a 106.04 ms first-jump coloring observation, with
the other four at 48.65 to 49.73 ms. Meeting a development target does not mean
these delays are imperceptible or that the candidate is uniformly faster.
The earlier focused guard/light slowdown remains unresolved.

Public-key setup travel is retained separately. Linux's latency runs spend
94.95 to 98.91 seconds reaching their interior test positions, including the
runner's batching, drain waits, and redraw requests. This is not single-key
latency, but it is not hidden or counted as instantaneous navigation. The
[machine-readable summary](evidence/inner-engine-checkpoint-v2/summary.json)
retains all opening values, category distributions, setup totals, and worst
observations.

## Sustained memory

Values are MiB of process memory. Settled columns are maxima of the required
holds within each tour, not end-of-tour readings or a guaranteed steady state.
The final column is the OS-reported lifetime peak of that memory process, not
terminal-emulator memory, live allocated heap, or a sum across processes.

| Patch | Tour 1 settled max | Tour 2 settled max | Tour 3 settled max | OS lifetime peak |
| --- | ---: | ---: | ---: | ---: |
| Pierre showcase | 66.06 | 70.75 | 70.94 | 70.97 |
| Ghostty | 59.02 | 57.45 | 58.27 | 59.05 |
| Node | 128.16 | 137.00 | 138.39 | 138.39 |
| Bun | 139.92 | 159.84 | 165.77 | 165.77 |
| Linux, largest input | 770.00 | 771.23 | 753.16 | 771.36 |

Only the showcase and Ghostty meet both shared goals: 128 MiB third-tour settled
maximum and 192 MiB lifetime peak. Node and Bun exceed the settled goal. Linux
exceeds both. The largest lifetime peak among Linux's five latency processes was
735.92 MiB; the memory-process peak of 771.36 MiB is also the highest observed
across this complete cohort.

Bun grows by 25.84 MiB between the first and third tour maxima. Linux has a
31.48 MiB adjacent-query decrease; its lower third-tour maximum is not evidence
of an application optimization or a particular memory-release mechanism. Three
tours do not establish a plateau, leak, or total-memory bound.

These absolute RSS observations are not a controlled comparison with an older
build. The prior 162.25 MiB controlled live-heap reduction under allocation
logging remains separate evidence for inner-engine sharing. Do not subtract
these RSS readings from logged heap totals or claim a causal saving from
historical RSS differences.

## Interruptions and retained evidence

The user-requested app restart stopped v1 during its first memory process. Its
five completed latency processes, 373 completed memory actions, 238 completed
holds, and one incomplete hold remain in the separately audited and sealed
[v1 evidence](evidence/inner-engine-checkpoint-v1/verification.json). Its raw memory
record still says `running`; the audit correctly accounts for it as incomplete
without a summary. The interrupted record was not rewritten or pooled with v2.

The user authorized a new full cohort after restarting. A later connection loss
interrupted observation of the task, but the v2 checkpoint continued and wrote
its complete final report with passing integrity checks. Recovery found no
remaining viewer or checkpoint process. No replacement performance run followed.

The v2 directory retains the schedule, freezes, traces, logs, repetitions,
verification, summary, supporting-source archive, and SHA-256 manifest. The
preserved executable and its build-source archive remain in the preceding stage.

## Next work

1. Attribute the remaining large-input memory before another runtime change.
   Start with existing owner evidence, then a bounded current-build profile if
   needed. The unchanged document structures already account for 381.34 MiB on
   the largest input; that is a structural estimate, not a new process-heap
   snapshot or a quantity to subtract from RSS. Separate immutable document
   cost, live cache/parser growth, and allocator retention. Any resulting change
   must apply generally and preserve source, pairing, positions, and styling.
2. Investigate full-index latency and remaining first-visit coloring delays as
   separate paths. Keep the short focused regression visible. Do not shrink
   cache limits, suppress grammar work, or weaken correctness to meet a number.
3. Complete actual Ghostty/Kitty presentation and same-content Pierre comparison.
   This checkpoint did not measure physical paint or resolve visual parity.

All live measurement work has finished. No new runtime implementation is included
in this report.
