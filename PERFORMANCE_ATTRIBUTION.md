# Memory and coloring attribution

Date: 2026-09-12. The first batch from [the stage specification](NEXT_PERFORMANCE_STAGE.md) is implemented and checked. Build reproduction and the two memory profiles are complete. Coloring attribution identifies two different delay patterns; eviction-specific tracing remains a follow-up before a parser-checkpoint experiment.

**This batch does not reduce production memory or change rendering behavior.** It establishes a reproducible ordinary build and explains current costs before changing their owners. The previous 1,052.5 MiB settled-RSS observation remains unresolved. Actual Ghostty appearance and motion acceptance remain open.

## Implemented

- [Release preservation](tools/preserve_release.py) records the ordinary build command, inputs, toolchain, selected environment, dependency features, and executable hash. Tests and diagnostic builds use separate target directories.
- [Memory profiling](tools/profile_memory.py) records each sequential command's observation interval and checks input identities, command completion, expected views, and cleanup.
- The optional `delivery-trace` feature now records bounded per-request work summaries: continuation/reset reason, requested and replayed records, parser/color/word work, publication waits, and the longest parser call. Ordinary builds contain none of these timers or trace buffers.
- [The analyzer](tools/analyze_delivery_trace.py) distinguishes missing observations from zero work. [Its CLI controls](tools/check_delivery_attribution.py) exercise real traces and deliberately corrupted copies. Existing legacy traces remain readable.
- The shared interactive test launcher now honors the existing executable override. Explicitly constructed test commands still use their specified executable; not every process test uses the override.

No dependency, grammar, cache budget, pairing rule, source limit, fallback, worker count, or scheduling policy changed.

## Build identity

The ordinary recipe is `cargo build --release --locked --offline --bin native-diff --target-dir ABSOLUTE_PATH`, using the recorded Rust 1.96.0 toolchain. Exact target path matters on this Mac: changing it changed the Mach-O UUID and code signature. Rebuilding from scratch at the recorded reference path reproduced the complete signed executable byte-for-byte. No binary normalization was used.

| Role | Preserved executable SHA-256 |
| --- | --- |
| Ordinary reference and independent reproduction | `a7a84a949c3ea09e982c874f253a76beb9534be32c52cc870a4ede980d070462` |
| Ordinary build after diagnostic source changes | `8f5400866e1790d795678f0ffab6f0ca3d7f10f2a7637b48a7dd3ef14e185cad` |
| Separate `delivery-trace` build | `90284ed00123ad4820e5ddff1e3bb77bca9f21daec6d1f73679aad57866c804f` |

The reference archives initially omitted embedded palette assets. Their preserved asset supplement is required for reproduction; the later ordinary build archive includes those assets. The first preparation archive also included unnecessary vendored build output. Both mistakes and the unused artifacts remain documented in the [private runbook](evidence/next-stage-v1/RUNBOOK.md). Historical measured executables and results were not replaced.

## Memory: two different large owners

Both profiles replayed the existing checked memory trace through opening and three complete tours. Every checked view and terminal cleanup passed. All 24 profiler commands completed successfully. Workloads ran serially.

These are allocation-instrumented live-heap observations, **not ordinary RSS measurements**. Named regex allocations are a subset of the heap total, not additional memory to add to it.

| Workload | Phase | Live heap, MiB | Named regex allocations, MiB |
| --- | --- | ---: | ---: |
| Small syntax-heavy patch | Opening | 2.5 | 0.01 |
| | Tour 1 | 224.4 | 170.4 |
| | Tour 2 | 274.9 | 213.2 |
| | Tour 3 | 275.0 | 213.2 |
| Largest index-heavy patch | Opening | 718.7 | 0.01 |
| | Tour 1 | 998.2 | 117.6 |
| | Tour 2 | 1,020.3 | 135.4 |
| | Tour 3 | 1,021.1 | 135.8 |

The disjoint structural accounting is 0.24 MiB on the small patch and 511.9 MiB on the largest. The largest total includes 149.6 MiB of source spans and 268.7 MiB of projection storage. Source-run records occupy 120.3 MiB **within** that projection total. These are owned representation bytes, not allocator-sized blocks or process RSS.

The live heap supports two priorities: reduce the large document representation and investigate compiled syntax ownership. It does not show that cross-grammar duplicate patterns account for the regex cost. That specific sharing proposal remains conditional on live-duplication evidence.

There is also a smaller source-page ownership opportunity. On this Mac, 512 live page allocations occupy 40 MiB although their payload limit is 32 MiB. The allocation stacks and source indicate that converting a 64 KiB buffer to `Arc<[u8]>` adds the shared header to the large allocation, which the allocator rounds upward. Keeping the large buffer behind a separately shared owner is worth testing. It must preserve page size and cache behavior; changing page size to fit an allocator bucket is not the proposal. Extra allocation and indirection costs remain unmeasured.

Allocation call sites can combine inlined owners. Do not add their totals to structural accounting, assign every generic vector to syntax, or call the remaining process memory a leak. Active parser state, other caches, temporary work, allocator overhead, and OS residency are not fully separated. Small growth continues between the last two tours. These finite runs do not prove a plateau or unlimited-run boundedness.

## Coloring: repeated prefix work is material

Five fresh ordinary-reference processes ran the unchanged 12,000-line syntax sequence. Each destination checked the actual text and expected keyword colors. All exits restored the terminal. These observations measure PTY receipt with warm files, not Ghostty paint latency or sustained memory.

| Action | Median correct text, ms | Median checked styling, ms | Maximum checked styling, ms |
| --- | ---: | ---: | ---: |
| Open | 7.57 | 106.24 | 111.38 |
| First end seek | 0.96 | 1,634.55 | 1,644.46 |
| Backward page movement | 3.24 | 1,638.16 | 1,654.56 |
| Theme change | 2.96 | 1,633.32 | 1,651.69 |
| Uncached forward movement | 4.30 | 29.75 | 31.72 |

A separate instrumented process explains this sequence. The first end seek, backward movement, and theme change each parse 11,908 predecessors. Parser calls for those predecessors consume about 1.59–1.60 seconds per request. Visible color construction, theme preparation, and word comparison do not explain that delay. The forward request resumes at its next source record with no predecessor replay.

The opening request also contains a 77.6 ms first parser call. Syntax-set construction takes only 0.58 ms in this trace, but that does not include all lazy grammar work. Grammar initialization can occur both in parser calls and during parser-state creation in the unattributed remainder. This instrumentation does not separate compilation from matching inside a parser call.

### The corpus endpoint is not the same case

The previously slow endpoint was diagnosed by replaying its unchanged opening and first 81 actions. One ordinary observation reached correct text in 4.91 ms and checked styling in 222.65 ms. This declared prefix is a diagnostic, not a replacement for the full trace or a new accepted latency distribution.

The separate instrumented request processed 288 predecessors and 47 requested records. Predecessor parser calls took 176.70 ms; requested parser calls took 7.14 ms; publication took 18.05 ms. One predecessor parser call lasted 55.51 ms. No word comparison ran in this request. The trace identifies parsing as the main owner, but does not prove whether that long call is compilation, matching, or both. Nor does it establish the exact cause of every historical outlier.

Parser checkpoints can address repeated compatible prefixes. They cannot avoid a first-ever uncached prefix or interrupt an indivisible parser call. The initial theme-epoch-scoped checkpoint proposal also would not accelerate the theme-change replay above.

## Verification and limits

- Ordinary build: all 33 real-process tests and both numeric storage tests pass, with the executable distinction above. Complete independent source checks pass for all five patches, including all 19,299,175 supplied records in the largest input.
- Diagnostic build: 13 exact-executable rendering/geometry/syntax contracts pass. The filtered test run also passes one streamed-input test using Cargo's test executable. The exact diagnostic binary separately passes pending-highlight rapid resizing and cleanup.
- Four diagnostic challenge traces match unchanged source and cell/style expectations. The Unicode position fixture passes source checking but is not a visual pass in the Python terminal decoder, which does not preserve joined emoji. Rust position contracts remain the covered geometry evidence.
- The four challenges, deep sequence, and corpus prefix have complete request observations with no dropped events or summaries. The rapid-resize trace is correctly marked **partial**: quit ended capture before one worker summary arrived. The resize/cleanup test passed; complete worker attribution is not claimed for that run.
- Six analyzer CLI controls pass: real input, dropped events, dropped summaries, inconsistent counters, missing completion, and unavailable timings. Four historical traces remain readable. Formatting and Clippy pass; Clippy still reports the pre-existing vendored Syntect lifetime-style warning.

Exact source bytes, old/new state, stale-response handling, fallback, and cleanup expectations were not relaxed. No full five-patch latency/RSS comparison was rerun for these diagnostic-only changes. Natural style-cache eviction and subsequent replay still need an explicitly observed sequence before accepting a checkpoint design. Actual Ghostty pixels and frame pacing still need user-supplied captures.

## Ordered follow-through

1. **Next production candidate: source-run line-number ownership.** Evaluate the specified private `Option<NonZeroU32>` representation with checked construction. It expresses the existing positive-line invariant without reducing the supported domain. Change no other index representation. Require complete source/projection equivalence, numeric boundaries, process tests, then the declared ordinary responsiveness and memory comparison. A smaller type is not by itself an RSS win.
2. **Then test whether bounded parser checkpoints justify their memory.** Repeated backward replay is now demonstrated. First account for full old/new parser and scope-state ownership and capture naturally evicted revisits. Preserve the existing C2 admission/cancellation rules and sequential style oracle. Reject the cache if its cost or correctness burden is not justified. Cold parsing remains a separate problem.
3. **Keep broader syntax sharing and page ownership conditional.** Establish live duplicate compiled patterns before widening sharing. Evaluate source-page allocation layout separately, with unchanged cache policy. Do not combine these changes with the first candidate.
4. **Recheck the retained production build once.** After a candidate passes its focused checks, use the existing five-patch latency and three-tour memory checkpoint. Keep missing targets and regressions visible. Visual acceptance remains a separate capture-dependent step.

Raw reports, exact commands, preparation history, preserved executables, and derived table inputs are in [the private runbook](evidence/next-stage-v1/RUNBOOK.md). The diagnostic work narrows the next decisions; it does not close performance or visual acceptance.
