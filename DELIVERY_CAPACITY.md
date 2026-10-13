# Reply delivery and observation costs

## Result

A small delivery repair is implemented and passes correctness checks. The
combined compact-index and delivery candidate remains **unaccepted**. Its
ordinary continuous-end coloring median is 70.34 ms versus 63.69 ms for the
previously retained build. The original regression is not resolved.

The current working source is this candidate. Its ordinary executable is
`evidence/delivery-capacity-v1/candidate/native-diff`, SHA-256
`611805a3d1e9561c14715d6de1c9d39388edbbcc064fd6453b95781a2104aba5`.
The retained reference and both earlier compact executables remain unchanged.
No viewer, build, or measurement is still running.

## What the diagnostic established

The existing checker decodes and compares the terminal's 8,000 cells between
PTY reads. Its receipt timestamp excludes subsequent validation of that packet,
but preceding validation can delay later reads and constrain the writer. Raw
packet and internal write chronology were absent from the original ordinary
comparisons, so their complete cause cannot be reconstructed retrospectively.

Four declared sessions used unchanged diagnostic executables and the same
continuous fixture. They compare the retained reference with the compact
candidate before the delivery change. Opening was checked normally. End output
was either checked inline or captured for 500 ms before decoding. Every session
passed complete canonical source, exact end cells, trace accounting, and terminal
cleanup.

| Diagnostic | Exact-end receipt | Internal input to exact-end write | Requested parser | Publication |
| --- | ---: | ---: | ---: | ---: |
| Reference, inline | 75.439 ms | 71.270 ms | 56.649 ms | 12.554 ms |
| Candidate, deferred | 63.703 ms | 63.668 ms | 59.060 ms | 2.535 ms |
| Reference, deferred | 62.401 ms | 62.365 ms | 57.776 ms | 2.528 ms |
| Candidate, inline | 72.113 ms | 67.648 ms | 58.017 ms | 7.541 ms |

Moving validation after capture reduced both receipt and publication time in
these observations, while parser time increased slightly. This supports observer
feedback through backpressure. It does not assign every millisecond of the
earlier binary difference. Four sessions do not establish distributions.
Deferred results are diagnostic evidence, not replacement performance scores.
Raw packets were replayed against the same frozen screen; no validation was
discarded. Each clock interval uses its own input origin, and raw-frame order
matches the complete internal synchronized-write sequence.

## Implementation

The full reply queue previously waited on a two-millisecond timer. Its wait
predicate checked only request replacement and closure, not freed capacity.
Consuming a reply did not notify that waiter.

The receiver is now private. `Worker::try_reply` takes a reply while holding the
existing pending mutex and notifies the existing condition variable. Publication
holds the same mutex across its state check and nonblocking send. A full queue
releases the mutex atomically through `Condvar::wait`. Every wake checks closure
and request generation again before retrying.

This prevents capacity release from falling between a failed send and entry to
the wait. Request replacement and consumer drop still notify under the same
mutex. Only the single worker waits for work or capacity; those waits cannot
compete. Error payloads also release capacity. The guard is dropped before the
UI processes a reply or renders.

The 16-reply channel and 64-reply UI drain limit are unchanged. There is no new
dependency, thread, queue budget, grammar policy, fallback, or source behavior.
Only `src/highlight.rs` and `src/main.rs` differ from the preceding compact
build's production-source manifest.

## Verification

All 33 real-process tests pass, including the two opt-in corpus/artifact tests.
Eight library checks pass. The new real-worker test parses a 64-line TypeScript
patch, fills the actual reply queue without consuming, and verifies resumption,
request replacement, exact IDs/epochs, dark/light keyword colors, and source
release when the consumer is dropped. It uses the actual syntax worker, not a
mock parser. This proves the covered full-queue behavior, not every possible
thread interleaving; the shared-mutex protocol supplies the lost-wakeup argument.

The first worker-test attempt failed because its added-file fixture omitted the
required new-file header. The fixture was corrected and the worker test passed.
No parser or runtime change was made for that failure. The failed log remains.
The earlier process tests passed before that test-only correction and the two
opt-in tests passed afterward. Supported PTY contracts use the preserved ordinary
executable; explicit Cargo-target tests retain their separate build.

Application Clippy and formatting pass. The vendored dependency's existing
lifetime-spelling warning remains. Complete independent source digests pass for
all five large inputs on the exact candidate. All five index-accounting results
match the preceding compact candidate exactly. Projection code is unchanged,
so its preserved complete source/row/inverse comparisons remain applicable.

An independent ordinary rebuild differs only in 48 Mach-O linker-identity and
first-page signature bytes. All other bytes and code-page hashes match. Neither
executable was rewritten or normalized.

## Fixed comparisons and limits

Exactly one five-pair ordinary series compared the delivery repair with the
preceding compact build. A second compared the combined candidate with the
retained source-run/checkpoint build. Each has 40 supported visual runs, all
passing, plus ten source-only entries excluded because the Python decoder does
not preserve joined emoji. The unchanged helper, workloads, and expectations
were used. All observations, including slower values, remain in the reports.

| Continuous-end coloring | Reference median / maximum | Candidate median / maximum |
| --- | ---: | ---: |
| Delivery change versus compact predecessor | 70.31 / 71.71 ms | 69.73 / 71.57 ms |
| Combined candidate versus retained build | 63.69 / 66.75 ms | 70.34 / 71.04 ms |

The second comparison still shows a 6.66 ms median difference. Correct text
appears at about 0.71 ms in both. Other results are mixed. For example, continuous
light-theme coloring median is 19.06 versus 22.94 ms in the retained-build
comparison, but dark-theme maximum is 36.27 versus 27.68 ms. Against the compact
predecessor, start-color median is 9.75 versus 8.25 ms. These differences are not
discarded or relabeled as equivalent responsiveness.

One additional diagnostic sequence per before/after compact executable passes
all supported source/cell checks and cleanup. All eight supported request traces
are complete. In the observed continuous-end request, publication falls from
12.109 to 0.209 ms and worker elapsed time from 70.673 to 58.193 ms, with the same
1911 predecessors and 48 requested records. Requested parsing is 57.460 versus
56.793 ms. This supports the intended capacity-wakeup mechanism in those
instrumented runs. Other phases can still wait for a busy consumer. Instrumented
results do not cancel the ordinary regression or prove physical terminal paint.

The preceding 110.49 MiB index-payload saving remains represented in the current
accounting. The 182.32 MiB opening-heap reduction and 887.61 MiB third-tour RSS
belong to the preceding compact executable; they were not remeasured for this
delivery build. No full five-patch memory repetition was run in this focused batch.

## Decision and next work

Do not promote the combined candidate. Preserve the implementation and evidence
for follow-up, with the earlier retained build untouched. The wakeup repair is
supported as a general mechanism improvement, but the promised ordinary
responsiveness gate remains unestablished. No further repetitions belong to
either fixed series.

The next bounded task is to separate prompt byte capture from cell validation
in the measurement path, while preserving exact source/style checks and raw
evidence. Establish how much the observer affects the result before using that
setup for further millisecond-level application tuning. The existing one-process,
capture-then-validate diagnostic is the starting point; this is not authorization
for a broad benchmark framework or weaker checks. Cold parser work remains a
separate attribution question. Actual Ghostty appearance and input-to-paint
latency remain unverified.

## Evidence

- [Observer runbook and results](evidence/coloring-attribution-v2/RESULTS.md),
  with four raw packet captures, trace records, and a frozen execution order.
- [Delivery runbook](evidence/delivery-capacity-v1/RUNBOOK.md), with the ordinary
  and diagnostic build manifests, reproduction, tests, and independent source
  and index checks.
- `evidence/delivery-capacity-v1/paired-vs-compact.json` and
  `paired-vs-retained.json`, plus their separate summaries.
- `evidence/delivery-capacity-v1/all-attribution.json`, with all eight supported
  before/after diagnostic traces. The original traces and observations remain.
