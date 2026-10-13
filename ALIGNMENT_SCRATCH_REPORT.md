# Local alignment-buffer qualification

Date: 2026-09-14. Status: complete. Keep the local allocation change.
Correctness passed and the evidence supports less alignment CPU work.
Elapsed loading time and RSS remain mixed. This is not performance acceptance.

## Change

The only changed production build input is `src/model.rs`.
Unequal-replacement scoring reuses two local byte vectors instead of creating
filtered vectors for every comparison. The existing guards run before allocation.
The byte filter, score, threshold, comparison order, and tie handling are unchanged.
The vectors have at most 8,192 bytes of requested capacity in total, excluding
allocator overhead, and are released when the scorer returns.

There is no persistent cache or change to parser behavior, source storage,
pairing policy, syntax coverage, or rendering policy. Up-front reservation and
two length scans are costs of the change. All-whitespace blocks can now reserve
storage even when their filtered content is empty.

Reference executable: `a1b6401654299e0c548b37ed24ef1dd08c0e7a43d24c5922cf5daff26d3aa1f7`.
Candidate executable: `294b866a1c1f7d838b776ea6823d845f63ecea807da5436dbd1de654be51bcd9`.
Both executables and their build manifests/source archives are preserved in
[`evidence/alignment-scratch-v1`](evidence/alignment-scratch-v1).
The final default `target/release/native-diff` has SHA-256
`20cfafd01537e3539438e23d526d35da73dceeed5335ae55859bb27e24300a5d`.
It was rebuilt from the qualified source. Its hash is distinct from the preserved
candidate and the Cargo test companion; those identities are not interchangeable.
The final default executable then passed all 37 existing real-process contracts
and the new 34-input alignment contract, using the existing test runners without
another build. Its identity and source remained unchanged across those checks.

## Correctness

- Formatting and application Clippy passed.
- The ordinary suite passed 37 process contracts, 10 library checks, and one
  worker check. Two existing opt-in process checks remained excluded.
- The new alignment integration contract passed 34 actual `inspect` invocations,
  explicit pairing offsets, and preserved-reference row checks.
- Independent canonical checks matched all 20,847,451 supplied source records.
- Complete split/unified projections, partner lookups, inverse lookups, and
  boundary checks matched the preserved reference for all five corpus patches.
- The projection pattern check accepted 2,888 cases. Both parsers rejected the
  same 716 invalid cases.
- The finite capture gate passed. Its fault controls rejected wrong final text,
  wrong colors, incomplete updates, packet faults, and invalid cleanup accounting.
- All 40 paired terminal recordings replayed exactly. Ten joined-emoji position
  cases remained explicitly source-only because of the inherited decoder limit.

The first pre-change test attempt had a wrong vertical-tab expectation.
The example was corrected before production changes, and the original failure
remains recorded. See the [source review](evidence/alignment-scratch-v1/REVIEW.md).

## Index observations

Three alternating pairs used the existing `inspect --summary` diagnostic.
Each invocation passed the count checks. The table uses macOS `time -l` elapsed
seconds, not the wrapper's polling duration.

| Patch | Reference elapsed | Candidate elapsed | Reference user CPU | Candidate user CPU |
| --- | ---: | ---: | ---: | ---: |
| Pierre | 0.01 s | 0.00 s | 0.00 s | 0.00 s |
| Ghostty | 0.07 s | 0.03 s | 0.06 s | 0.03 s |
| Node | 0.17 s | 0.12 s | 0.13 s | 0.10 s |
| Bun | 0.18 s | 0.19 s | 0.14 s | 0.15 s |
| Linux | 4.51 s | 4.86 s | 3.90 s | 3.60 s |

The diagnostic's two-decimal resolution cannot establish tiny-input ratios.
Largest-patch elapsed times were `7.76, 4.51, 4.42` seconds for the reference
and `5.70, 4.86, 3.70` seconds for the candidate. These shared-host observations
do not establish a wall-clock loading improvement.

Largest-patch user CPU times were `4.61, 3.90, 3.85` seconds for the reference
and `3.97, 3.60, 3.26` seconds for the candidate. User CPU time was lower in
all three pairs. This supports reduced CPU work, not a general latency guarantee.
No run was replaced to obtain a preferred result.

The four small capture cases had broadly similar response times. Their capture
cadence and workload remain separate from the corpus memory runs and from
physical terminal paint. Full values remain in the paired capture summary.
The largest candidate observations across those captures were 19.97 ms for text,
73.61 ms for styling, and 133.55 ms for readiness. The corresponding reference
maxima were 11.06, 62.28, and 110.14 ms. These maxima include opening and are not
paired measurements of the same individual action. They do not establish
responsiveness on all large inputs.

## Sustained memory

All ten ordinary processes completed three tours using the unchanged checked
bounded traces and one-second holds. The audit checked 4,324 views, 2,856 holds,
51,115 RSS queries, complete tour order, and exact summary reconstruction.
All exits passed the quit target and terminal cleanup checks. All recorded
viewer process groups were absent at the final audit.

| Patch | Reference OS peak | Candidate OS peak | Reference third-tour settled maximum | Candidate third-tour settled maximum |
| --- | ---: | ---: | ---: | ---: |
| Pierre | 66.72 MiB | 66.94 MiB | 43.77 MiB | 66.94 MiB |
| Ghostty | 58.09 MiB | 60.06 MiB | 58.06 MiB | 29.88 MiB |
| Node | 138.44 MiB | 116.16 MiB | 138.44 MiB | 116.16 MiB |
| Bun | 139.27 MiB | 153.75 MiB | 112.39 MiB | 153.66 MiB |
| Linux | 680.03 MiB | 673.22 MiB | 592.38 MiB | 593.89 MiB |

The largest reference's settled tour maxima were 676.36, 605.59, and 592.38 MiB.
The candidate's were 673.17, 485.67, and 593.89 MiB. The second-tour drop is not
a total-memory guarantee or evidence of fewer retained application objects.
RSS does not identify its owner. These are single-process observations per
executable and patch on a shared host, not replicated allocation measurements.

Bun's candidate OS peak was 14.48 MiB higher. Its first-tour maximum was close
to the reference's, but its later-tour resident footprint was higher. That
observation remains unresolved; Node's decrease does not cancel it out.
The older metadata checkpoint's separate Bun increase also remains unresolved.

The candidate missed the existing memory target on Bun and Linux. The reference
missed it on Node and Linux. No target, cache size, grammar coverage, fixture,
or expected screen was changed to improve these outcomes.

## Separate stack sample

The planned three-second, 1 ms sample completed after all ordinary runs.
It recorded 2,516 main-thread observations. The candidate alignment subtree
accounted for 197: 186 in its scoring fold, nine in outer allocation/free
subtrees, and two in other outer work. No allocation subtree was sampled below
the candidate scoring fold. The prior sample had 547 observations in the old
inlined scoring fold out of 2,542 main-thread observations.

The preserved disassembly identifies the byte filters, prefix/suffix comparison,
division, and ordered sum. The outer function reserves both vectors and releases
them after scoring. Defensive growth branches remain in the fold, but refills
cannot exceed the reserved raw-line maxima. No unsafe code or unchecked access
was introduced to remove those branches.

The compiled stack boundaries differ, and these are separate sampled intervals.
The counts support the intended removal of repeated allocation work. They are
not exact durations, allocation-call counts, or a measured whole-program speedup.
Parser extraction, libgit2 allocation/copying, input scanning, and spool work
remain in the profile.

## Decision and next work

Keep the change as a bounded, local allocation cleanup. It removes repeated
allocation from an identified hot path without a persistent owner, new cache,
algorithm change, or wider API. Exact correctness passed, ordinary user CPU
time fell in all three largest-patch pairs, and the profile supports the intended
mechanism. The small-case response observations do not show a broad benefit.

This decision does not classify the elapsed-time or Bun RSS observations as
noise, prove a memory saving, or close their investigation. The current source
remains a prototype with incomplete performance and visual acceptance.

Next, distinguish lazy regex initialization from matching in the already
identified slow first-visit coloring path. Keep the Bun RSS difference as an
explicit ownership question. More broad comparison runs cannot substitute for
that attribution. Physical Ghostty/Pierre comparison and release acceptance
also remain open.

The [qualification plan](evidence/alignment-scratch-v1/PLAN.md) and
[runbook](evidence/alignment-scratch-v1/RUNBOOK.md) define the workload and order.
