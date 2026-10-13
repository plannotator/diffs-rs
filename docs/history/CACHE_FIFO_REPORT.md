# IDs-only cache queue

Completed 2026-09-14. The cache now stores source IDs without duplicate weights
in its FIFO. This narrow storage cleanup is accepted. It does not establish a
whole-process memory reduction or a responsiveness improvement.

## Change and correctness

The [cache](../../src/highlight.rs) recovers an evicted style's weight from the value
already returned by map removal. One private helper owns the unchanged formula.
There is no additional map lookup. Duplicate handling, insertion before eviction,
FIFO order, and the 32 MiB payload budget remain unchanged.

Source bytes, token data, old-side distinctions, changed spans, grammars, and
rendering remain unchanged. The public cache API cannot mutate a stored style.
An inconsistent queue fails explicitly instead of permitting silent counter drift.

Queue elements use 8 rather than 16 bytes on this target. At the same capacity,
their storage extent is half as large. The diagnostic observed:

| Diagnostic boundary | Queue capacity | IDs-only extent | Two-word extent at that capacity |
| --- | ---: | ---: | ---: |
| Bun, third tour | 131,072 | 1 MiB | 2 MiB |
| Largest patch, first tour | 262,144 | 2 MiB | 4 MiB |

These extents exclude allocator rounding. They are not ordinary RSS savings.
The diagnostic labels FIFO payload accounting as recomputed from styles.
It retains independent reconstruction from vector capacities and queued-ID checks.
The reader also passed checks on both earlier storage schemas.

The release gates passed:

- Formatting and all-feature application Clippy. The existing vendored syntect
  lifetime warning remains. No dependency source changed.
- Both preserved release suites, including 32 real-process contracts each.
  Two existing opt-in process tests remained skipped in each suite.
- The normal local rebuild passed the same 32 real-process contracts, with the
  same two exclusions.
- Real-allocation checks for spare capacities, old-side answers, duplicate IDs,
  multiple evictions, oversized entries, reinsertion, exact survivors, and counters.
- Independent source checks across all five patches: 20,847,451 supplied records.
- Seven diagnostic controls, 605 corpus views, six snapshots, and cleanup.
- Forty complete capture comparisons, plus ten explicitly source-only position
  cases. Joined emoji remains outside that capture parser's cell oracle.

These checks cover the adopted contracts. They do not prove every possible diff,
grammar, line pairing, or physical terminal presentation.

## Ordinary memory observations

MiB means 1,048,576 bytes. Each completed comparison contains one fresh process
per executable per fixture, with three tours and one-second holds.

| Patch | Reference peak RSS, MiB | Candidate peak RSS, MiB | Candidate minus reference |
| --- | ---: | ---: | ---: |
| Pierre | 70.06 | 70.80 | +0.73 |
| Ghostty | 57.70 | 59.03 | +1.33 |
| Node | 137.42 | 136.17 | −1.25 |
| Bun | 165.52 | 166.11 | +0.59 |
| Linux | 790.27 | 759.05 | −31.22 |

The complete records passed 4,324 ordered views, 2,856 holds, and 50,393 raw RSS
queries. The reference's largest-patch third-tour held maximum was 790.27 MiB.
The candidate's was 750.47 MiB. Final live samples were lower: 717.69 and 634.42 MiB.
Do not substitute those final samples for peak or sustained-view observations.

RSS changed substantially within unchanged processes. The Pierre candidate fell
from a second-tour maximum of 70.80 MiB to a final hold of 10.50 MiB that tour.
Its third-tour maximum was 25.72 MiB. Large RSS drops also occurred in both Linux
processes. The records do not partition heap release, allocator retention, or
operating-system residency changes.

These records cannot attribute the 31.22 MiB peak difference solely to the queue.
The four smaller comparisons are mixed. No uniform RSS saving, total-memory
bound, or indefinite plateau follows from this cohort. Node, Bun, and Linux
remain outside the existing memory goals. The targets and workloads did not change.

## Responsiveness follow-up

The initial five-pair capture comparison showed slower guard-theme medians.
A separately declared follow-up ran twenty guard processes, including five
reference/reference pairs and five alternating reference/candidate pairs.
All twenty captures passed exact persisted replay and cleanup.

The table retains both summary methods. A difference of medians and a median of
matched-pair differences are distinct calculations. Positive values mean slower
candidate observations. Neither cohort is pooled with the other.

| Guard action | Initial difference of medians | Initial median pair difference | Follow-up difference of medians | Follow-up median pair difference |
| --- | ---: | ---: | ---: | ---: |
| Light | +5.33 ms | −0.37 ms | +5.06 ms | +4.71 ms |
| Dark | +10.42 ms | +7.98 ms | −2.53 ms | −2.26 ms |

The dark slowdown did not repeat. Light-theme matched-pair differences changed
direction between cohorts, although the difference of medians stayed positive.
Identical reference executions varied from −8.65 to +14.01 ms for light coloring
and from −9.57 to +8.67 ms for dark coloring.

These observations do not demonstrate a repeatable candidate-specific slowdown.
They also do not prove equivalence or rule out a small effect. The acceptance
decision rests on unchanged cache policy, exact correctness checks, smaller queue
storage, and the absence of a consistent paired responsiveness regression.
No source adjustment or further timing run followed these results.

Guard coloring still takes roughly 50–60 ms in these observations. Its latency
remains open work. The source-only case remains source-only. Receipt times do not
establish physical paint, frame pacing, or actual Ghostty acceptance.

## Retained failures and integrity

The first outer runner stopped between phases because it reused a create-only
status path. The diagnostics completed successfully. Their workloads did not
repeat. The original runner and stale status remain preserved with the correction
in [RESUME.md](../../evidence/cache-fifo-v1/RESUME.md).

The first Linux reference attempt later failed the RSS sampling requirement.
One hold contained four complete queries instead of five. Two queries took
241 and 294 ms. The latter crossed the hold deadline. The underlying system
cause remains unknown. No diff mismatch or viewer crash was reported.

Failure cleanup stopped the sampler, terminated the viewer with exit code 143,
and restored terminal settings. The failed record retains 515 fully recorded
views, 371 complete holds, one incomplete hold, and 11,626 queries.
These observations are separate from the completed-process totals.

One new full Linux reference/candidate pair then completed under unchanged rules.
The eight successful processes did not repeat. There were eleven ordinary memory
attempts: ten complete and one sampling-invalid. See the
[retry declaration](../../evidence/cache-fifo-v1/RETRY_LARGEST.md).

The runs used the shared local host, without a dedicated-host reservation.
No application code changed during qualification. No grammar reduction,
allocator replacement, forced shrinking, fixture-specific policy, or GUI control
was introduced. Earlier evidence remains unchanged.

## Evidence and next work

- [Declared scope](../../evidence/cache-fifo-v1/PLAN.md) and [code review](../../evidence/cache-fifo-v1/REVIEW.md).
- [Build and correctness gates](../../evidence/cache-fifo-v1/checks/report.json).
- [Completed memory audit](../../evidence/cache-fifo-v1/memory-audit.json) and
  [complete results, including residency and both timing summaries](../../evidence/cache-fifo-v1/results.json).
- [Guard follow-up](../../evidence/cache-fifo-v1/guard-followup/report.json).
- [Local executable check](../../evidence/cache-fifo-v1/local-build.json) and
  [sealed identities](../../evidence/cache-fifo-v1/seal.json).

The measured candidate is `057a9b4e4635f0b54cf1049917cfbeeda62e2810b310b37a942c4477d6bf3b2a`.
The reference is `9b6e45076368f96830e9d64ec58609bfc96ffe6d6e038496094f7609ca4e165d`.
The local rebuild has its own recorded identity. Measurements belong to the
preserved candidate, not automatically to every rebuild of the same source.

Next, observe peak live entries and map capacity transitions around admission
and eviction before selecting a lookup redesign. Keep that diagnostic scalar and
off by default. General index storage, coloring latency, and physical terminal
presentation remain separate open work. This checkpoint is not a complete release.
