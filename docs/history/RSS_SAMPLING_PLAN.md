# Process-group RSS collection

## Scope

Repair the observer, not the application. Keep the ordinary source-page binary
`b749889e828ca95132f19394bf17c87e8806578f1ff8f0a202a1549d5d3b9e89`,
the original independent style oracle, source expectations, public input trace,
three tours, one-second holds, five-sample minimum, and six-GiB safety stop.
Do not change Rust source, targets, cache policy, or the old failed checkpoint.

Replace whole-system task-info acquisition with a single process-group selector:
`/bin/ps -g PGID -x -o pid=,pgid=,rss=,state=`. Set `COMMAND_MODE=unix2003`
and `LC_ALL=C` only for this command. Keep the one-second query timeout and
100-ms wait after each query. This is not a guaranteed 10-Hz observer.

Apple's [selection and error paths](https://github.com/apple-oss-distributions/adv_cmds/blob/6bed8737a34dbb54782a18f47dccf933a9967a12/ps/ps.c#L525)
select the group before per-task acquisition. Kernel process-list traversal can
remain. Check stderr even when the exit status is zero. A clean empty result
with status one means no members, not a zero-memory sample.

Apple's [task-info collection](https://github.com/apple-oss-distributions/adv_cmds/blob/6bed8737a34dbb54782a18f47dccf933a9967a12/ps/tasks.c#L83)
can leave a zero RSS value when task access fails. Reject unknown process state,
non-zombie zero RSS, malformed rows, duplicate PIDs, wrong groups, and query
errors. Retain zombie records, but do not qualify empty or zombie-only results
as live-view samples. This conservative policy can reject a run; it must not
convert unavailable memory into a small number.

## Evidence and compatibility

Use one shared query function and one sampler loop. Retain every query's
monotonic start and completion, status, members, RSS, stdout, stderr, and error.
Only successful live queries entirely inside a hold count toward its minimum.
Record hold boundaries and qualifying query indices, including failed holds.
Retain the completion-timestamp timeline for existing consumers.

Declare protocol `process-group-rss-v2`. Bind the helper, `/bin/ps`, runtime,
query environment, and hold policy in new freezes. Historical freezes require
their archived runner. The old source-page source archive preserves those tools.
Neither protocol measures unique physical memory, descendants that leave the
group, or an atomic process-group snapshot. RSS can count shared pages more than
once. Query wall time includes scheduling; it is not CPU time.

## Verification and execution order

1. Check real no-terminal parent/child membership, an excluded outside group,
   committed-memory growth, stopped processes, leader exit, and final absence.
   Check the viewer safety stop and cleanup with the actual executable.
2. Replay a bounded subset of existing frozen views through the real PTY,
   including navigation and resize, with unchanged one-second settled holds.
   Audit query containment and membership. No mock clocks or process outputs.
3. Freeze a separate memory-only cohort before starting any measured run.
   Use the existing five source-page memory captures and their original baseline
   relationships. Order: Pierre showcase, Ghostty, Node, Bun, largest patch.
   Run one process with three tours per input, serially. Stop at the first failure
   and retain it. Do not retry until a result passes or run a new latency series.
   Memory replays retain incidental timings, not accepted responsiveness results.
4. Audit all query intervals, hold reductions, source/style destinations, binary
   and input hashes, and terminal cleanup. Publish current absolute RSS separately
   from the old failed checkpoint. Do not infer an application memory improvement
   from an observer change or promote the combined application candidate.

The protocol has no guarantee against host contention. If a valid hold still
has too few observations, the cohort remains incomplete. Actual terminal-paint
verification and further application memory ownership work stay separate.
