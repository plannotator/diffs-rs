# Loading and first-visit coloring

Date: 2026-09-14. The attribution checkpoint is complete. Production source and the normal executable are unchanged.
The next production candidate reuses two temporary buffers during replacement alignment.
Syntax-engine changes need a separate investigation.

## Findings

### P1: full indexing has material CPU work before terminal publication

The current ordinary executable completed all five parse/index summaries with the expected counts.
The largest command took **4.21 seconds wall time**, including **3.67 seconds user CPU** and **0.40 seconds system CPU**.
This command shares ingestion and index construction with the viewer, but has no terminal or highlighting worker.
It establishes a loading cost independent of terminal rendering. It does not measure interactive opening or UI backpressure.

A separate three-second stack sample captured 2,542 main-thread observations.
Of these, **547 included the replacement-alignment scorer**, including its allocation and deallocation calls.
The preserved disassembly identifies the inlined scorer through its whitespace filters, prefix/suffix comparisons, division, and ordered sum.
These counts describe a sampled interval, not exact phase durations or an expected speedup.

[The scorer](src/model.rs:396) creates two filtered byte vectors for every pair at every candidate offset.
The same source lines therefore trigger repeated allocation, growth, and deallocation.
The parser also copies source bytes into temporary owned lines before index construction.
The profile contains parser extraction, libgit2 allocation/free, input scanning, and spool writes too.
Removing alignment allocations cannot eliminate the whole loading cost.

### P1: deep first visits still require expensive syntax replay

One fresh ordinary process ran the existing 12,000-line syntax sequence.
These observations measure PTY receipt through the existing checker, not physical terminal paint.
They are a diagnostic, not a latency distribution or a before/after comparison.

| Action | Correct text, ms | Checked coloring, ms |
| --- | ---: | ---: |
| Opening | 7.83 | 50.65 |
| First end jump | 1.14 | 1,721.64 |
| Backward page movement | 3.48 | 47.08 |
| Theme change at that position | 3.80 | 1,740.67 |
| Uncached forward movement | 5.03 | 33.19 |

The separate instrumented run explains the deep jump.
It replayed **11,908 preceding records**, with **1,776.65 ms** in predecessor parser calls.
The complete request took **1,797.98 ms**.
The backward movement used an existing checkpoint and replayed only **132 predecessors**.
The theme change discarded epoch-specific state and replayed **11,908 predecessors** again.

The current checkpoints help revisits. They cannot remove the first parse of an unseen prefix.
Reusing parser checkpoints across themes needs a separate design for parser state, scope state, and theme-dependent state.
Skipping predecessors or resetting syntax at an arbitrary source line weakens correctness.

### P1: the showcase endpoint and dense lines have different costs

The existing opening and first 81 actions reached the same checked endpoint with both executables.
The ordinary observation reached correct text in **4.23 ms** and checked coloring in **111.65 ms**.
The instrumented endpoint request took **130.82 ms**:

| Phase | Instrumented time, ms |
| --- | ---: |
| Parsing 288 predecessors | 89.58 |
| Parsing 47 requested records | 9.63 |
| Publication, including backpressure | 30.06 |
| Theme preparation | 0.26 |
| Visible color construction | 0.24 |
| Word comparison | 0.00 |

The longest predecessor parser call took **23.02 ms**.
The existing trace does not separate regex compilation from matching inside that call.
The endpoint checker decodes and compares frames between reads.
Its pauses can contribute to publication backpressure. The 30.06 ms interval is not proof of a viewer scheduling defect.
The current diagnostics do not justify a new worker architecture or larger caches.

The dense-line guard challenge had no predecessor replay in its slowest request.
Requested parser calls consumed **96.60 ms** of that **96.97 ms** request.
One parser call took **49.94 ms**. The current line-size limits bound text size, not execution time.
The worker checks cancellation between groups of source records. It cannot interrupt one parser call.
Any finer cancellation design must preserve consistent old-side, new-side, and continuation state.

## Next production change

The first candidate targets repeated alignment allocation only.
It does not change the pairing algorithm or remove repeated whitespace scans.

The buffers remain local to one `alignment_offset` call, after both existing early-return guards.
Each buffer reserves at most the largest eligible raw line on its side.
Each pair clears and refills those buffers with the existing ASCII-whitespace predicate.
The requested byte capacity is at most **8,192 bytes**, plus vector descriptors and allocator overhead.
This is a representation bound, not an RSS guarantee.

The following behavior remains exact:

- The eligibility and 4,096-byte guards.
- Pair iteration and floating-point summation order.
- Prefix and non-overlapping suffix comparisons.
- The denominator, initial threshold, ascending offsets, and strict winning comparison.
- Source bytes, source numbering, newline markers, and fallback behavior.

Precomputing every normalized line is not part of this candidate.
The current eligibility rule permits 4,097 distinct lines in one replacement.
At the byte limit, their normalized payload can reach **16,781,312 bytes**, before descriptors and allocation overhead.
That temporary ownership needs its own tradeoff decision.

The qualification checkpoint must compare against the preserved pre-change scorer, not two candidate paths that share the same function.
The existing `examples/projection_reference/model.rs` retains a separate scorer and row model.
The candidate must pass complete five-patch projection comparisons and independent supplied-source checks.
Focused contracts must cover threshold ties, both longer sides, whitespace and invalid bytes, prefix/suffix overlap, and eligibility boundaries.
Existing real-process rendering contracts must retain exact pairs, filler rows, numbers, and emphasis in both layouts.

Only after these checks pass, a fixed ordinary comparison will measure indexing, responsiveness, and sustained memory.
One comparable stack sample will check whether allocation work decreases.
Slower results and increased memory remain part of the decision. No timing or memory saving is claimed yet.

After this candidate, the coloring investigation will distinguish regex initialization from matching in the same first-visit workload.
Theme preparation costs about a fraction of a millisecond per request here.
Caching theme preparation alone cannot resolve the observed deep replay delays.

## Checks and limitations

The [offline evidence check](evidence/loading-coloring-v1/checks.json) passed:

- Five summary processes matched all declared counts, totaling 20,847,451 supplied records.
  Counts are not full source digests. This checkpoint did not repeat the earlier complete source-digest suite.
- The ordinary and instrumented deep sequences checked ten states and clean exits.
- The two endpoint sequences checked 164 states, including their openings, and clean exits.
- Four diagnostic challenges checked 28 retained screens and complete source digests.
  The fifth challenge passed its source digest only because the Python decoder does not preserve joined emoji.
- Six finite request traces have no dropped events, dropped summaries, or missing worker summaries.
- The existing rapid-resize process contract passed with the diagnostic executable.
  Its trace remains partial because request 24 had no summary before shutdown.
  The contract checks the text anchor and cleanup, not completed coloring or physical frame pacing.
- Build-input hashes match the preserved Git-path correction source exactly.

The diagnostic executable's first deep-sequence opening took 266.23 ms to reach text and 309.84 ms to finish the checked view.
This isolated instrumented observation is retained. It does not establish an ordinary opening regression or its cause.
No measurement was rerun to replace an unfavorable observation.

The ordinary executable remains `4a477beb65871042b5a3d31a2721e98d357e819beb2dd6ef482b476d0673b7dc`.
The separate diagnostic executable is `45d302c131b32a458c018d4fa14af1276fbdd21069a196c8b4c315693d679a0b`.
Its build recipe, source archive, logs, and request traces remain in [the evidence directory](evidence/loading-coloring-v1/).

The user reported no issue with the latest manual view. Independent Ghostty pixel comparison remains incomplete.
The earlier adverse Bun RSS observation also remains unresolved.
This checkpoint supplies no new sustained-memory result, release acceptance, or benchmark ranking.
