# Finalized style storage

Completed 2026-09-14: this representation-only change is not accepted. Correctness
checks passed, but largest-input memory increased after revisits. The three
application files have been restored to the prior source; the candidate and all
results are preserved. [Plan](evidence/style-storage-v1/PLAN.md),
[measurement scope](evidence/style-storage-v1/QUALIFICATION.md).

## Implementation

In the preserved candidate, completed style answers use boxed slices for primary color tokens, optional
old-side color tokens, and changed-word ranges. Existing Vec builders remain
unchanged. Finalization happens on the highlighting worker before publication,
not in the UI's cache-insertion path. An exactly equal old-side answer is still
discarded before optional old-side storage is finalized.

Every token range, foreground color, old-side distinction, changed range, and
fallback answer is preserved. There is no boundary merging, new sharing,
grammar change, smaller budget, allocator replacement, or source/index change.
The renderer did not change. The logical cache weight uses actual inline type
size plus owned slice extents. Its budget remains 32 MiB.

Experimental application input changes were limited to `src/highlight.rs`,
`src/highlight/census.rs`, and `src/highlight/census/tests.rs`. The diagnostic
schema explicitly identifies boxed slices and owned-element extent. The
validator continues to accept both the prior Vec schema and this experimental
schema. It remains available for inspecting both sets of evidence after the Rust
source rollback. Allocator rounding remains outside this logical accounting.

## Completed correctness and ownership gates

- Formatting and all-target/all-feature Clippy passed.
- Both preserved release builds passed 32 real-process tests and the worker
  lifecycle test. The diagnostic build also passed 11 library, two inner-engine,
  and eight regex-census tests; the ordinary build passed its nine library tests.
- Two existing opt-in process tests remained ignored in each suite: the
  pre-storage comparison and full five-body endpoint test. The separate current
  source check matched all 20,847,451 supplied records across the five patches.
- All seven diagnostic PTY controls passed. Two actual-record counter
  corruptions were rejected. The updated checker accepted six prior-schema
  snapshots and all six new boxed-storage snapshots.
- The two declared diagnostic workloads passed 605 ordered views, six snapshots,
  and cleanup. Every new snapshot has zero spare owned-element capacity. The
  inline `LineStyle` value is 56 bytes on this build, compared with 80 previously.

The completed diagnostic endpoints illustrate why vector savings are not total
memory savings. Bun's cache accounted for 17,211,808 bytes with 77,715 entries.
The largest-input cache still accounted for approximately 32 MiB, but retained
178,561 entries. Its FIFO capacity was 524,288 slots, compared with 262,144 in the
preceding Vec census. These are separate processes; asynchronous worker progress
can also change retained contents. Map capacity reports usable slots, not the
backing allocation size. No heap or RSS reduction follows from these counters
alone.

## Focused responsiveness

The unchanged capture-first gate passed 17 checks and retained its existing
source-only position case. Five alternating pairs produced 40 complete terminal
recordings and ten explicit source-only position checks. Every supported saved
recording passed exact replay and cleanup. The joined-emoji decoder limitation
was not converted into a terminal-presentation pass.

First-color medians, milliseconds, five observations per executable and case:

| Opening case | Reference | Candidate |
| --- | ---: | ---: |
| Many small files | 11.58 | 11.55 |
| Continuous syntax | 14.79 | 14.49 |
| Long-line guard | 56.53 | 56.56 |
| Rust library patch | 22.96 | 23.45 |

Changes were small and mixed, not a demonstrated speedup. The continuous-syntax
end jump's color median increased from 18.56 to 19.74 ms; dark-theme recoloring
increased from 6.70 to 7.92 ms. Individual observations overlapped and included
changes in both directions. For the dark action, much of the median difference
was already present at first text (5.10 versus 6.16 ms). These observations do
not establish a causal finalization delay or prove no such delay exists. The
existing guard's roughly 60 ms theme-color delay remains visible work.

The [complete paired results](evidence/style-storage-v1/paired/summary.json)
retain text, color, and exact-screen milestones separately. These are PTY receipt
times with bounded capture windows, not physical paint or scrolling feel. They
must not be pooled with the inline-validation memory workload's observations.

## Ordinary memory comparison

Complete: one fresh ordinary process per executable per fixture, three tours,
alternating which executable goes first by fixture. All five existing checked
memory traces retain their original producer identity. Their source/color
reference sidecars and rejection controls were bound before execution. The
[offline audit](evidence/style-storage-v1/memory-audit.json) passed 4,324 ordered
views, all 2,856 prescribed one-second holds, and 45,921 raw RSS queries. All ten
processes exited cleanly. No failed or extra attempt was omitted or repeated.

Observed process peaks, MiB; one process per table cell:

| Patch | Reference | Candidate | Change |
| --- | ---: | ---: | ---: |
| Showcase | 70.22 | 69.86 | -0.36 |
| Ghostty | 58.64 | 57.73 | -0.91 |
| Node | 137.31 | 126.09 | -11.22 |
| Bun | 165.89 | 155.75 | -10.14 |
| Largest | 770.34 | 805.69 | +35.34 |

The largest input's tour-hold maxima were 768.45 / 770.31 / 770.34 MiB for the
reference and 739.47 / 805.42 / 805.69 MiB for the candidate. Its lower first-tour
reading did not persist. The candidate's final raw RSS sample was also 805.69 MiB;
the adverse result is not explained by a brief isolated peak alone.

Node's first-to-third-tour maximum growth decreased from 10.25 to 8.86 MiB, and
Bun's from 26.38 to 20.86 MiB. The candidate met the existing memory goal on
showcase, Ghostty, and Node; Bun and the largest input still missed it. All quit
goals passed. Smaller-case improvements do not justify accepting the largest
input's adverse result. These are paired observations, not peak distributions or
proof that compaction helps every workload.

The [residency extraction](evidence/style-storage-v1/memory-residency.json)
retains the peak held view, last held view, and final live RSS sample separately.
For the largest input, all three last-held-view readings also equal their
respective tour maxima. The adverse result persists beyond the peak's first view.

This stage does not rerun the dedicated 25-process five-patch latency cohort.
It does not establish physical Ghostty/Kitty presentation, a peak-memory
distribution, or a universal improvement across possible inputs.

## Follow-up and disposition

The [declared follow-up](evidence/style-storage-v1/REGRESSION_CHECK.md) reused the
existing candidate owner census. All 568 ordered views, four owner snapshots,
accounting checks, and cleanup passed. No ordinary workload was repeated.

Cache payload stayed near 32 MiB at all three tour boundaries:

| Tour | Entries | Reported map capacity | FIFO capacity |
| --- | ---: | ---: | ---: |
| 1 | 178,548 | 243,756 | 524,288 |
| 2 | 178,563 | 909,647 | 524,288 |
| 3 | 178,326 | 893,999 | 524,288 |

The lookup container is the next ownership lead: reported capacity changed
substantially while endpoint payload and entry count stayed close. This count
does not identify the underlying allocation size or which RSS pages are retained.
Compiled engine instances increased from 2,362 to 2,374 on tour two, adding
82,367 estimated strategy bytes; neither changed on tour three. These estimates
exclude scratch, outer VM storage, strings, and allocator overhead.

The diagnostic omits ordinary one-second holds and pauses for census output.
It cannot partition the ordinary 35.34 MiB increase or prove its exact cause.
The [audited counters](evidence/style-storage-v1/followup-audit.json) narrow the
investigation; they do not turn an adverse result into an accepted optimization.
The first offline follow-up audit had an integer-versus-JSON-string key mismatch.
Its source and correction are preserved in the
[audit correction record](evidence/style-storage-v1/AUDIT_CORRECTION.md).

The verified rollback restores only this experiment's three application files.
The [restoration record](evidence/style-storage-v1/restoration.json) checks all
application input hashes against the preserved reference and records formatting
and the normal local rebuild. The restored build then passed all 32 existing
real-process tests; two opt-in cases remained skipped. Their
[log](evidence/style-storage-v1/restoration-plain-process-tests.log) and
[test identity](evidence/style-storage-v1/restoration-tests.json) are preserved.
The initial Cargo test invocation also passed but replaced the local executable
with a differently hashed build. Its log is retained. The final plain release
was rebuilt and tested directly through the existing compiled runner and binary
override; its hash matched before and after that run. These are the same 32
contracts against two build identities, not 64 distinct tests.
The local rebuild has a separate executable hash and no performance cohort.
Experimental source archives and executables remain
intact. No cache budget, grammar, token boundary, or rendering rule was changed
to rescue the result.

Next, attribute lookup-container allocation and high-water storage with the
existing heap/VM tools before choosing a cache representation change. Treat
payload, lookup storage, and eviction bookkeeping as separate owners. Reconsider
boxed payloads only as part of a new, separately scoped general change, with the
same source/color/anchor, responsiveness, and sustained-memory gates. Full-index
latency, first-visit coloring, and physical terminal acceptance remain separate
open work.

## Preserved executables

| Role | SHA-256 |
| --- | --- |
| [Retained measured reference](evidence/style-census-v1/ordinary/build.json) | `9b6e45076368f96830e9d64ec58609bfc96ffe6d6e038496094f7609ca4e165d` |
| [Rejected candidate ordinary](evidence/style-storage-v1/ordinary/build.json) | `d65c2205edc15e9dab04ea9438fe370e52ca6c4b20aacb42072a983286f2502b` |
| [Rejected candidate diagnostic](evidence/style-storage-v1/diagnostic/build.json) | `ca10284123aa851fc7433e699ef3afce6219a9dd3da7bd60870e1229b1d50661` |
| [Restored local build, not performance-measured](evidence/style-storage-v1/restoration.json) | `67a6ee1954039ed5fae1cdb437641342e0e512b5566c2b95ceae1f20c5917bd1` |

Prior performance records remain bound to their original executables. No earlier
measurements were relabeled as observations of this candidate. Application work,
builds, and measurements are serial on the shared local host; no newly
acknowledged companion-task host reservation is claimed.
