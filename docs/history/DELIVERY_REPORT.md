# Completed syntax delivery

Date: 2026-09-12. This is a delivery change, not a new grammar, renderer, cache policy, or memory representation.

Completed syntax replies now wake the UI instead of waiting for its keyboard poll. The change is retained: independent source checks and exact terminal-cell contracts pass, two contrasting inputs color sooner, and idle CPU did not increase at the sampler's resolution. The current Pierre comparison is nearly flat. This does not establish Ghostty paint latency or full corpus performance acceptance.

## Implementation

The existing event loop owns a capacity-one notification channel. Keyboard input, completed input-file publication, input completion/errors, signals, and syntax replies notify it **after** publishing their payload. Actual data remains in the existing bounded queues: one keyboard event, two input publications, and sixteen syntax replies. Per-turn input and reply drains remain bounded at 64.

The loop clears an old notification before draining payloads, never after. A notification that arrives during processing remains pending for the next turn. The loop still handles a keyboard event each turn and checks for a queued key immediately after an idle wake, before starting another background drain/redraw. A 100 ms fallback wait supports progress updates and detection of unexpected input-thread disconnection. Normal events wake it immediately. This replaces the former 4 ms input-loading / 16 ms keyboard polling waits; it does not add faster idle polling or an async runtime.

The syntax worker's separate 2 ms full-queue retry remains unchanged. Generation cancellation, epoch rejection, old/new syntax states, pairing, wrapping, reading anchors, source storage, and cache limits are unchanged.

An opt-in `delivery-trace` Cargo feature records requests, completed replies, publication attempts, reply drains, input receipt, and completed frame emission. The trace is limited to 16,384 events, written to a new file at exit. An incomplete trace is rejected by the analyzer. Normal builds contain neither its storage nor its event calls. Traces contain numeric identifiers, not source text.

## Inputs pinned before the change

The [manifest](../../evidence/delivery-v1/cases/manifest.json) binds source bodies, independent decoding, and the fixture generator.

| Input | Patch bytes | Files / hunks | Supplied source records | Purpose |
| --- | ---: | ---: | ---: | --- |
| Shared position fixture | 1,628 | 1 / 1 | 25 | Unequal changes, tabs, Unicode, long reading anchors |
| Many small Rust files | 14,620 | 80 / 80 | 160 | Publication and per-file overhead |
| Continuous TypeScript hunk | 47,133 | 1 / 1 | 2,005 | Multiline state and end-seek replay |
| Styling-guard boundaries | 30,156 | 1 / 1 | 5 | 9,999 / 10,000 / 10,001-character lines |
| Ratatui release patch | 114,800 | 33 / 106 | 2,806 | Additional real-world input, outside DiffsHub |

The Ratatui patch is pinned from `1c3dbd1ff4fad2696543bf332a7e413d1bb69988` to `e665c36cb14752a61cd777fbd06dbef8474f2add`, downloaded from the project's [commit comparison](https://github.com/ratatui/ratatui/compare/1c3dbd1ff4fad2696543bf332a7e413d1bb69988...e665c36cb14752a61cd777fbd06dbef8474f2add). No input was replaced after trials.

The first Python probe failed on a joined emoji: pyte does not preserve that grapheme as the independent oracle expects. [That failure is retained](../../evidence/delivery-v1/baseline-cases/report.json). The shared position fixture therefore has a full-source pass but an explicit unsupported Python visual-oracle disposition, not a visual pass or timing. Existing Rust position/Unicode contracts pass. The other four inputs' observed views pass independent text checks and retained cell comparisons. The Ratatui patch also contains a joined emoji outside those observed views; its entire presentation is not certified by these endpoint probes.

## Attribution

The [bounded traces](../../evidence/delivery-v1/attribution-final.json) confirm completed work waiting in the old event loop. Median successful-offer-to-drain intervals, in milliseconds:

| Input | Before | After |
| --- | ---: | ---: |
| Many small files | 16.65 | 5.89 |
| Continuous hunk | 16.13 | 0.17 |
| Guard boundaries | 9.32 | 0.01 |
| Ratatui | 17.31 | 8.58 |

These are instrumented upper bounds beginning just before the successful channel send. They include scheduling, UI work, and possible PTY backpressure. They are not pure scheduler latency or terminal paint measurements. All eight analyzed traces fit within the event limit. The extra trace mutex, timestamps, and bounded allocation can perturb scheduling; none of these traced runs supplies the uninstrumented results below.

## Uninstrumented comparison

The host was reserved; no other coordinated build or measurement ran concurrently. Hardware: arm64 Mac17,6, 48 GiB RAM, 18 logical CPUs, macOS 26.3 (25D2125). Each candidate/reference pair alternated order. There was no discarded warm-up. All attempts and the slower observations remain in the reports.

The [final challenge comparison](../../evidence/delivery-v1/paired-challenges-final.json) contains five fresh-process runs per binary per supported input. Each performs opening, end, start, light theme, 100-column split, 160-column split, dark theme, and quit. The common independent decoder checks source records; all observed completed cells also match the unchanged binary. The shared fixture remains explicitly unmeasured by this Python probe.

| Input | Median opening style/fallback, before → after | Maximum, before → after |
| --- | ---: | ---: |
| Many small files | 39.82 → 39.44 ms | 40.37 → 59.02 ms |
| Continuous hunk | 83.56 → 41.59 ms | 85.37 → 41.90 ms |
| Guard boundaries | 64.99 → 63.26 ms | 67.71 → 64.14 ms |
| Ratatui | 85.15 → 63.62 ms | 88.69 → 72.55 ms |

The guard case measures the **defined styling/fallback state**, not full syntax coloring of the oversized line or its invalidated continuation. No guard changed.

The many-small-files case shows no meaningful median improvement and a slower candidate maximum. Its candidate time to the expected source viewport also reached 38.73 ms versus 28.91 ms. The first candidate trial was slower in both the initial and final series. A [follow-up with three fresh file copies per binary and two launches per copy](../../evidence/delivery-v1/fresh-copy-check/report.json) did not reproduce the same approximately 59 ms delay. Copying a file does not reset system caches. The cause remains unisolated, and the slower trials are retained. Do not call this a universal opening speedup.

Across the measured post-opening actions, the largest source-view response was 5.17 ms for the candidate and 5.20 ms for the reference. End-seek medians were 0.51 / 0.71 ms; start medians were 1.77 / 1.20 ms. Narrow/wide resize medians were 2.49 / 3.23 ms for the candidate and 2.58 / 3.64 ms for the reference. Start responses remain somewhat slower in this sample; these observations do not establish a universal input improvement or tail-latency guarantee. Rapid pending-highlight resize correctness is checked separately by the process suite.

More intermediate updates are an explicit cost. Median frame count across the seven observations rose from 22 to 38 on Ratatui; median total viewer CPU was approximately unchanged at 67.16 / 67.33 ms. Many-small-files frames rose from 24 to 30 and CPU was 31.84 / 32.02 ms. Continuous-hunk CPU was 99.58 / 96.00 ms. These are child CPU observations, not observer CPU or screen smoothness. The observer validates complete grids on receipt; validation and PTY backpressure can contribute to observed elapsed time.

The [final Pierre comparison](../../evidence/delivery-v1/paired-pierre-final.json) contains ten alternating fresh-process pairs using the previously frozen exact opening cells. Median expected text was 7.22 → 7.08 ms; median colors were 38.93 → 38.58 ms, with maxima 57.80 → 39.78 ms. The former approximately 77 ms reference median was not reproduced this session. The current evidence demonstrates removal of the polling mechanism, not reproduction and elimination of every part of that older regression.

The [final idle comparison](../../evidence/delivery-v1/paired-idle-final.json) contains three ten-second intervals per binary after styling the same many-small-files input. Both emitted zero frames. Reference child CPU increments were 0.01, 0.01, and 0.00 seconds; candidate increments were all 0.00 at `ps`'s 0.01-second resolution. Candidate quit observations were below 1.86 ms with terminal modes restored. Zero at this resolution is not a claim of zero CPU consumption.

Peak RSS in these small challenge sequences was approximately unchanged. This work does not address the previously identified compiled-regex allocations or establish a sustained-memory bound.

## Correctness and reproducibility

- All **32 real-process tests** passed on the final revision: [complete suite](../../evidence/delivery-v1/process-final.log). This includes all five large patches, independent text/color contracts, burst input, wrap/layout/width anchors, fallback behavior, blocked loading, and signal/terminal cleanup. Earlier successful suite logs are also retained.
- The new streamed-input test checks over sixteen visible replies, exact keyword colors, and every interior cell against ordinary input while the producer remains paused. It changes theme/width, checks the final source anchor, and quits before EOF.
- Twelve existing render/geometry/syntax contracts additionally ran against the exact measured executable via `NATIVE_DIFF_CONTRACT_BINARY`. The [13-test filtered log](../../evidence/delivery-v1/measured-binary-contracts-final.log) also includes the new stream test, which uses Cargo's test-built executable. Cargo's test-built binary has a different hash from the ordinary release build; it is not presented as the measured binary.
- The two numeric span-boundary tests, formatting check, and all-target/all-feature warnings-denied Clippy passed. Production renderer, grammar, storage, pairing, and geometry code did not change.

Unchanged measured reference: `7a842a00acfb52d64ea9b7db1dccf980b53cdad4d720683f7d5090b2d1c01d07`.

Measured final ordinary release candidate: `3b67259e205d918f1eae1d94b2fcfea86e842ca8e233b313e811f1e1048d5627`.

The first candidate, `cb5609af95bfad5753c9b038fc898e3009ef63b1d80aed160f1d10c7b5ecdbf3`, and its complete results remain in `candidate/` and the unsuffixed comparison files. Final review identified that an idle-waking key could pass through another background redraw. The final revision restores the immediate key check and repeats the comparisons; it does not replace a slower run of the same binary with a preferred result.

The original and trace-only source packages, binaries, failed probes, successful captures, raw event traces, comparisons, and final source package remain under [evidence/delivery-v1](../../evidence/delivery-v1). Historical corpus freezes/results were not modified. To reproduce comparisons, use the archived source/tools and the commands in [the evidence runbook](../../evidence/delivery-v1/RUNBOOK.md).

## Remaining work

The next product investigation is compiled-regex allocation ownership across multiple grammars, followed by any justified general index overhead reduction. Keep the many-small-files maximum and increased intermediate frame count visible in subsequent measurements. No additional batching or timer tuning is justified solely to improve these particular inputs.

Full current-build corpus latency/memory acceptance, cold and evicted style revisits, long-duration memory behavior, and actual Ghostty presentation remain open. This change makes no claim of complete Pierre visual parity or smooth pixel scrolling.
