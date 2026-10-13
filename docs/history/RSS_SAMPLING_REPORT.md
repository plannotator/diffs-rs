# Process-group RSS observer

## Current status

The observer repair is implemented and verified. The separately frozen five-patch
memory cohort completed: five processes, three tours each, 2,157 checked action
views plus five openings, and clean normal exits. The
[offline audit](../../evidence/rss-sampling-v2/verification.json) passes. All 1,428 settled
holds contain at least eight valid queries against the unchanged minimum of five.
The audit checks 25,128 query records, including their raw member sums and full
containment within holds. No missing-publication discrepancy occurred in this run.

No application source or executable changed. The measured binary remains
`b749889e828ca95132f19394bf17c87e8806578f1ff8f0a202a1549d5d3b9e89`.
The earlier source-page checkpoint remains failed and preserved. This task does
not promote the combined application candidate or claim lower application memory.
No viewer or measurement from this batch remains running.

## Memory observations

These are absolute RSS observations in MiB, not controlled application deltas.
Each column shows the maximum across settled holds in that tour. OS peak is the
viewer process's `wait4` high-water mark; every live group query in this cohort
contained only that viewer process.

| Patch | Tour 1 | Tour 2 | Tour 3 | OS peak |
| --- | ---: | ---: | ---: | ---: |
| Pierre showcase | 223.91 | 270.73 | 270.94 | 270.97 |
| Ghostty | 109.64 | 110.61 | 112.88 | 112.91 |
| Node | 231.08 | 241.09 | 242.48 | 242.48 |
| Bun | 281.14 | 319.19 | 342.97 | 342.97 |
| Largest patch | 856.34 | 879.41 | 703.36 | 879.41 |

Four inputs still exceed the existing memory goals. The largest patch's lower
third-tour RSS does not establish reduced live ownership. Its second tour includes
a 239.14 MiB drop between successive long-line holds, after an earlier 86.20 MiB
drop. The query membership remains unchanged and the raw values verify. This
cohort has no matching allocator or host-residency timeline that identifies the
cause. Keep all three tours and the 879.41 MiB high-water mark visible; do not
present 703.36 MiB as an application optimization.

Query wall-time maxima range from 99.88 to 305.91 ms across the five processes.
Thus the targeted query does not guarantee a fixed cadence or immunity to host
contention. It supplied enough fully contained observations in every actual hold.

Next, review existing allocation evidence alongside this residency pattern before
choosing another application memory owner to change. Additional profiling needs
its own bounded question. Do not change cache sizes or repeat the cohort to obtain
a preferred result. Coloring delays and actual terminal-paint verification remain
open; this memory-only cohort does not resolve them.

## Implementation

The [plan](RSS_SAMPLING_PLAN.md) limits this change to observation reliability.
One small shared helper selects the process group before macOS `ps` acquires
per-task information. One sampler loop replaces two copies. It retains every
query's start, completion, group, members, output, status, and RSS. No native
helper, library dependency, or additional viewer instrumentation is needed.

The observer rejects unavailable readings instead of treating them as zero.
Its one-second timeout, 100-ms post-query wait, six-GiB safety stop, one-second
settled holds, and five-sample minimum remain. A query counts only when its full
interval lies inside the hold. New freezes bind the protocol, helper, `/bin/ps`,
and existing inputs. Historical freezes require their preserved runner.

Review caught two failure-evidence gaps. Interrupted holds now retain their actual
end and qualifying query indices. Cleanup errors no longer prevent the replay
from saving its query and hold evidence. Primary and cleanup errors remain
separate. The PTY closes even if sampler joining fails.

## Real-process verification

The [final gate](../../evidence/rss-sampling-v2/check-final.json) passes:

- No-terminal parent/child membership, an excluded outside group, committed-memory
  growth, a stopped child, a surviving child after leader exit, and final absence.
- An actual rejected `ps` query, which retains its error without an RSS number.
- 81 existing frozen terminal views plus opening, including source navigation,
  split/unified changes, and narrow/wide resize.
- Five unchanged one-second holds with 9, 10, 9, 9, and 9 qualifying queries.
- A real viewer safety stop, rejected further input, early failed-hold evidence,
  owned-viewer termination, sampler termination, and terminal restoration.

The initial gate also passed and remains available. The final gate adds failure
evidence checks after review. The tests use actual processes and the preserved
viewer, not mocked process output or clocks. Formatting/parser error branches
and OS task-access denial are reviewed paths, not all induced end-to-end cases.
The intentionally failed hold observes a pending style-frame change while the
safety error is already active; it proves failure evidence retention, not a new
source-correctness failure.

## Interpretation limits

The cohort runs one memory process and three tours per input in declared order.
All five freezes are prepared before measurement starts. Source/style expectations,
public commands, workloads, and targets are unchanged. There is no new latency
series; incidental timings inside memory replays are not responsiveness results.

This observer still samples rather than continuously measures memory. It is not
an atomic group snapshot and does not capture descendants that leave the group.
RSS can count shared pages more than once. Query wall time includes scheduling,
not just CPU work. Group accounting is tested with children; the production
viewer's cleanup checks do not establish a general daemon-tree cleanup contract.

The older 25-process latency series retains its own observer protocol. Any new
absolute memory observations belong to this cohort, not a controlled before/after
application comparison. Actual Ghostty/Kitty paint, broader memory work, and the
earlier unresolved coloring regression remain separate.

## Artifacts

- [Frozen schedule](../../evidence/rss-sampling-v2/five-patch/schedule.json) and
  [five prepared freezes](../../evidence/rss-sampling-v2/five-patch/prepared-freezes.json).
- [Complete cohort](../../evidence/rss-sampling-v2/five-patch/checkpoint.json) and
  [offline audit](../../evidence/rss-sampling-v2/verification.json).
- [Observer source overlay](../../evidence/rss-sampling-v2/observer-source-overlay.tar.gz)
  and [checksums](../../evidence/rss-sampling-v2/SHA256SUMS). The overlay requires the
  preserved source-page source archive and local hash-bound corpus inputs; it is
  not a standalone application distribution.
