# Capture-first terminal delivery checks

## Result

The capture-first tester is implemented and verified. Exactly five alternating
pairs completed with 40 successful visual sessions, 280 checked action windows,
and ten explicitly source-only entries. All recorded sessions replayed from disk
with the same frames and receipt milestones. No application change or additional
comparison series was made.

The combined application candidate remains **unaccepted**. End-coloring median
on the continuous fixture is 63.27 ms versus 61.59 ms for the retained reference,
a 1.68 ms difference. The preceding inline series measured 70.34 versus 63.69 ms.
The smaller gap under capture-first observation supports investigating observer
effects, but the changed cadence prevents attributing the whole reduction to
validation overhead. The new data does not establish equivalent responsiveness.

| Continuous-fixture body styling | Reference median / maximum | Candidate median / maximum |
| --- | ---: | ---: |
| Opening | 34.14 / 36.20 ms | 31.43 / 43.20 ms |
| End | 61.59 / 63.39 ms | 63.27 / 63.99 ms |
| Return to start | 1.69 / 1.77 ms | 1.62 / 1.74 ms |
| Light theme | 6.21 / 6.29 ms | 3.84 / 4.33 ms |
| Narrow resize | 2.31 / 2.52 ms | 2.33 / 2.44 ms |
| Wide resize | 3.75 / 5.03 ms | 3.81 / 4.93 ms |
| Dark theme | 8.84 / 11.13 ms | 6.01 / 6.82 ms |

First correct text at the end has medians of 0.81 and 0.84 ms respectively.
Results elsewhere are mixed. The candidate's many-small light-theme maximum is
6.09 versus 4.52 ms; its guard opening median is 63.05 versus 61.77 ms; and its
Ratatui end-coloring maximum is 17.06 versus 13.80 ms. All 28 case/action summaries
retain all five values, medians, and maxima for both builds. No slower observation
is removed. All 40 ordinary exits restored terminal settings and emitted the
expected cleanup sequences; the largest observed quit time was 5.85 ms.

The ordinary reference is SHA-256 `61a3dbea…`; the ordinary candidate is
`611805a3…`, both preserved with full hashes and build manifests. The application
source still matches the latter manifest. Earlier source/projection and Rust
test evidence remains applicable to that unchanged source; this batch did not
rerun the full Rust suite or measure sustained memory. All processes have exited.

## Method

The development tester now records terminal bytes and their arrival times before
decoding screen cells. Application source and the two preserved executables are
unchanged. The older inline checker and its evidence remain available.

For each action, one PTY reader captures output for 500 ms. It does not decode
cells, compare source, compress recordings, or write artifacts during that window.
Afterward it replays every captured update through the existing terminal decoder,
frozen text/color expectations, and independent source checker. The recording is
bounded to 16 MiB and 100,000 packets per viewer session. Exceeding either limit
fails the run; cleanup continues draining and reaping the owned viewer.

The sequence is unchanged: open, end, start, light theme, resize to 100 columns,
resize back to 160 columns, dark theme, quit. Height remains 50 rows. Both builds
use the same four visually supported cases and the same source-only joined-emoji
exclusion. No decoder limitation is treated as a visual pass.

Each action records its input time, fixed deadline, actual capture end, packet
range, and resize dimensions. A complete update receives the timestamp of the
packet containing its closing marker. Replay consumes every frame in each packet,
including frames after the first correct screen. The first correct body text,
first matching body styles, and first exact settled screen are separate metrics.
The final captured screen must also match the frozen cells and source oracle.

Unbalanced or partial updates, out-of-window receipts, output found waiting after
validation, an incorrect final screen, and cleanup failures invalidate a session.
Resize changes the existing replay screen rather than resetting its decoder.
Quit bytes are retained separately from measured frames, including the terminal
guard's final synchronized-update terminator.

## What the method does not establish

Capture still uses Python, ordinary OS scheduling, byte hashing, and the existing
RSS safety sampler. It removes synchronous cell-validation work from the receipt
window; it does not prove zero observer overhead. A nonblocking boundary read
does not prove the producer can never emit another update.

The 500 ms windows and subsequent validation change the spacing between commands.
This is a distinct protocol, not a correction that can be subtracted from older
timings. The comparison uses the same spacing rule for both builds, but results
cannot establish behavior during rapid input bursts. Filesystem and grammar
initialization are not controlled cold-cache experiments.

Recorded times describe receipt of application output, not when Ghostty, Kitty,
or a physical display paints it. First matching body styles can precede the exact
settled footer. Correct source, declared fallback, and completed syntax coloring
remain different claims.

## Verification

The final correctness gate passes all four real-viewer recordings through disk
replay. All seven actions in each match the frozen screens and independent source
expectations. Fragmenting recorded packets while preserving their timestamps
produces identical frames and receipt milestones. Copied recordings with wrong
final text, wrong final color, truncated updates, reversed timestamps, or skipped
packets are rejected. Truncation is checked both within a packet and with a
one-byte final packet.

A real viewer with an intentionally tiny capture allowance fails as required and
is reaped. Terminal settings are restored in the observed failure. A copied
incomplete correctness gate is rejected before any comparison viewer starts.
These tests use the actual viewer and its recorded output, not a simulated viewer.

Three versioned correctness attempts are preserved. All passed their declared
checks. After the first, review moved replay setup before process launch and
required the complete top-level gate report. After the second, review corrected
the truncation control's assumption that an entire escape sequence arrives in
one packet. The third freezes the final implementation. Earlier helper versions
are preserved with hashes matching their respective freezes.

## Evidence and use

- `tools/capture_delivery.py`: bounded capture and standalone disk replay.
- `tools/check_capture_delivery.py`: real-viewer checks and copied-recording fault
  controls.
- `evidence/capture-delivery-v1/RUNBOOK.md`: scope and finite series declared
  before execution.
- `evidence/capture-delivery-v1/check-003`: final correctness gate and recordings.
- `evidence/capture-delivery-v1/paired`: five alternating ordinary pairs, with
  raw recordings, source checks, individual receipt times, and summaries.

The current reader requires schema-3, pre-quit stop/join evidence. See the
[lifecycle report](RSS_SHUTDOWN_REPORT.md) for the new cohort and current results.
The original schema-1
recordings above retain their frozen reader in
`evidence/capture-delivery-v1/source.tar.gz`; they are not requalified by this
reader. Historical schema-2 helpers remain in the cross-grammar source overlay.
To check a successful current recording without launching a viewer:

```sh
.measurement-venv/bin/python tools/capture_delivery.py \
  evidence/rss-shutdown-v1/capture-check-final/sessions/continuous
```

## Next bounded work

Retain this helper for capture-first small-workload diagnostics. Do not replace
the full-corpus timing protocol or declare the application candidate accepted
from these results. The remaining end-coloring difference needs attribution to
application work or residual observation effects under one common cadence.
A useful next experiment would align internal request/parser/publication traces
with capture-first receipts on the same action sequence, using separate
instrumented binaries. It would be diagnostic evidence, not a replacement score.
No further application or memory-owner change is justified by this batch alone.
