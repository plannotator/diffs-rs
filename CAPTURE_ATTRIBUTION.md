# Capture-first highlighting attribution

Date: 2026-09-12. The declared two-session diagnostic pass is complete.
**No application change is justified by this pass.** Close the investigation of
the small capture-first difference without another repetition. This does not
prove that earlier slower results were noise, or promote the development build.

## What ran

One preserved diagnostic reference, then one preserved diagnostic candidate,
opened the same continuous TypeScript fixture. Both used the unchanged
capture-first tester: seven 500 ms action windows, frozen source/cell answers,
bounded raw recording, disk replay, and terminal cleanup. No build, ordinary
timing comparison, memory profile, or GUI operation ran in this batch.

Both sessions pass all seven action checks. All ten worker requests completed;
none were canceled, failed, missing a summary, or submitted without starting.
The reference emitted 25 drawn frames; the candidate emitted 23. Every captured
draw maps to an internal frame event in its expected action interval. Different
intermediate draw counts are not missing frames or proof of smoother motion.

The [offline audit](evidence/capture-attribution-v1/final-verification.json)
also verifies frozen identities, unchanged production source, clean exits, and
the end request's generation, epoch, complete replies, and absence of fallback.
The trace implementation, parser, and regex wrapper match between build archives.

## End navigation: the predeclared question

These are single instrumented observations, not medians or acceptance timings.

| Observation, milliseconds | Reference | Candidate |
| --- | ---: | ---: |
| Input to first correct text receipt | 0.868 | 0.814 |
| Input to exact colored-frame receipt | 63.471 | 62.032 |
| Internal input handled to exact frame event | 63.430 | 61.991 |
| Worker elapsed | 62.341 | 60.973 |
| Requested-source parser calls | 60.836 | 59.264 |
| Predecessor parser calls | 0.475 | 0.482 |
| Theme preparation | 0.264 | 0.260 |
| Visible colors, replay scopes, word comparison combined | 0.123 | 0.117 |
| Publication | 0.038 | 0.259 |
| Unattributed worker remainder | 0.605 | 0.590 |
| Last reply ready to exact frame event | 0.302 | 0.281 |

Both end requests process 48 requested records and 1,911 predecessors, using the
same continuation path. Each makes 94 requested-source parser calls and 3,820
predecessor calls, completes 48 replies, and uses no fallback. The longest parser
call is the same old-side requested record, source ordinal 2002: 34.671 ms in the
reference and 33.470 ms in the candidate.

Parser calls account for about 98% of worker elapsed time in each observation.
Predecessor parsing costs under 0.5 ms here. Extra checkpoints or skipped prefix
work therefore have no demonstrated justification for this case. Source review
found no redundant record traversal or extra parser-call count to remove.
Equal counts do not establish identical regex operations or CPU work. The parser
interval can include lazy compilation and scheduling; this trace does not
separate those costs from matching.

## Other actions remain visible

| Action: exact-frame receipt, milliseconds | Reference | Candidate |
| --- | ---: | ---: |
| Opening | 30.362 | 31.613 |
| Start | 1.667 | 1.561 |
| Light theme | 6.073 | 3.214 |
| Narrow to 100 columns | 1.992 | 2.287 |
| Widen to 160 columns | 3.654 | 3.745 |
| Dark theme | 10.005 | 5.971 |

Neither resize starts new highlighting work in these sessions. Theme requests
perform the same source work; light/dark publication intervals are 2.527/4.798 ms
in the reference and 0.398/0.465 ms in the candidate. This is consistent with the
already-implemented capacity notification removing polling waits. It is not a
new optimization or a general latency guarantee.

## Measurement limits and decision

The Python clock measures input injection to received bytes. The Rust clock
measures internal events. The internal frame event follows the terminal write
and trace-lock acquisition, not physical paint. These intervals are reported
separately: subtracting them would mix input handling and output observation,
not isolate a downstream delay. Opening also has different clock origins.

Frame matching uses ordered draws and checked action partitions. Internal input
events do not identify keys; the known seven-input sequence is an explicit
mapping assumption. Worker phase assignment uses the recorded worker-start
event, not a start reconstructed from elapsed and finished timestamps. Worker
and frame intervals overlap and must not be added. In raw attribution JSON,
`full_queue_events` counts failed send attempts, not comparable waiting episodes
under timer polling versus consumption notification.

The earlier ordinary capture-first medians remain 61.59 ms reference and 63.27 ms
candidate; the older inline-validation medians remain 63.69 and 70.34 ms. This
single diagnostic pair reverses direction, but cannot explain those distributions
or prove equivalence. **The compact-projection plus delivery candidate remains
unaccepted.** Do not tune code to make this difference disappear. No application
source, grammar, cache budget, fixture, expected answer, or trace schema changed.

Next is the bounded [source-page ownership proposal](SOURCE_PAGE_PLAN.md), chosen
from existing allocation evidence, not from these timing values. It cannot settle
the current candidate's promotion decision. Larger regex ownership remains a
separate conditional investigation. There is no new sustained-RSS result, actual
Ghostty paint measurement, or completed visual-parity acceptance.

Raw recordings, per-action phases, identities, and the fixed protocol are in
[the evidence directory](evidence/capture-attribution-v1/RUNBOOK.md).
