# Boxed cache-value experiment

Date: 2026-09-14. Decision: **do not adopt**.

The candidate passed the declared correctness checks. It did not establish a
favorable performance tradeoff for the additional allocations and pointer access.
The four changed application inputs are restored to the preserved reference.
The accepted IDs-only cache queue remains in place.

This is an adoption decision, not a finding of incorrect diff output or proof
that the candidate caused every observed timing or memory difference. All results,
including favorable observations, remain in
[the experiment directory](../../evidence/style-indirection-v1/).
The [decision record](../../evidence/style-indirection-v1/decision.json) contains the reasons.

## Change and cost

The experiment changed `HashMap<usize, LineStyle>` to
`HashMap<usize, Box<LineStyle>>`. On this target, a map value became an 8-byte
pointer instead of an 80-byte style descriptor. The descriptor still existed in
a separate allocation. Token vectors, their capacities, changed spans, side
distinctions, duplicate handling, charged weight, and FIFO eviction stayed unchanged.

Every admitted style gained an allocation, and lookup gained pointer access.
The existing 32 MiB charge still included the entire style descriptor and vector
capacities, not just its pointer. It did not include all table or allocator costs.

The diagnostic's third-tour observations were:

| Patch | Live entries | Public map capacity | Charged style bytes | Separate style-object bytes | Live pointer bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| Bun | 77,395 | 114,688 | 29,966,544 | 6,191,600 | 619,160 |
| Linux | 108,145 | 402,849 | 33,554,352 | 8,651,600 | 865,160 |

Style-object bytes are already included in charged style bytes; do not add them
again. These are Rust value extents. Public table capacity is not allocated bytes,
and this experiment did not profile table allocations plus allocator overhead.
The [complete reduction](../../evidence/style-indirection-v1/results.json) retains every snapshot.

## Completed checks

- Formatting and application Clippy passed. The existing vendored Syntect lifetime
  warning remains; it is not a new application warning.
- The ordinary release suite passed 10 library checks, 32 real-process contracts,
  and one worker check. Two existing opt-in process tests remained excluded.
- The diagnostic suite passed 14 library checks, 32 process contracts, two
  inner-engine checks, eight regex checks, and one worker check, with the same
  two opt-in exclusions.
- Independent source expectations matched all **20,847,451 supplied records**
  across the five patches.
- Seven diagnostic controls, 983 corpus views, eight corpus snapshots, and nine
  processes completed. Their offline audit passed.
- Forty focused captures completed, covering 280 action screens. Their offline
  audit passed. Ten additional position cases are explicitly source-only because
  the terminal decoder cannot represent joined emoji. They are not visual passes.
- All ten ordinary memory processes completed three tours each. The audit checked
  4,324 views, 2,856 holds, and 53,521 raw RSS queries. Every hold met the unchanged
  minimum of five valid samples. No invalid attempt or replacement run occurred.
- The separately declared guard follow-up completed all 20 captures and replay
  checks. No failed capture was replaced or pooled with the first cohort.

The reference test phase has an executable-identity limit: tests honoring
`NATIVE_DIFF_CONTRACT_BINARY` used the preserved reference. Older direct `BIN`
calls used a Cargo-built companion from the candidate source. That phase is not
a complete 32-test run against one old reference executable. The final local
build record distinguishes the restored executable, companion, and test runner.

These checks cover the declared cases. They do not prove all possible patches,
grammar equivalence, or physical terminal presentation.

## Ordinary memory observations

One complete reference process and one candidate process ran for each patch,
in the declared serial order. MiB means 1,048,576 bytes.

| Patch | Reference peak RSS, MiB | Candidate peak RSS, MiB | Candidate minus reference, MiB |
| --- | ---: | ---: | ---: |
| Pierre | 57.98 | 54.84 | −3.14 |
| Ghostty | 38.02 | 41.41 | +3.39 |
| Node | 121.28 | 99.20 | −22.08 |
| Bun | 94.94 | 121.52 | +26.58 |
| Linux | 698.95 | 648.77 | −50.19 |

The results are mixed. Bun's higher candidate reading remains unexplained.
The favorable largest-input reading is not an established 50.19 MiB saving.

Large RSS drops occurred inside unchanged processes. The largest adjacent drop
in the Linux reference was about 441.16 MiB. A contemporaneous, read-only
[host observation](../../evidence/style-indirection-v1/HOST_OBSERVATION.md) recorded
substantial compression and swap use on the shared 48 GiB machine. Compressor
pages occupied about 19.73 GiB; reported swap use was 1,935.69 M.
Those host-wide counters do not attribute memory to either viewer or measure a
compression rate. The snapshot was contextual, not part of the frozen per-process
sampling protocol. No host settings were changed.

The raw sampling checks passed, but they cannot separate application retention
from operating-system residency effects. A final low RSS reading does not prove
that the application freed its indexes. Neither a stable indefinite plateau nor
a total-memory guarantee is established. Do not compare these readings with old
cohorts as though they were a controlled before/after pair.

## Focused responsiveness

The first guard comparison's median paired styled-receipt differences were
**+8.19 ms in light mode** and **+11.73 ms in dark mode** for the candidate.
Other focused differences were small or mixed, with some outliers. All individual
milestone and action results remain in the complete reduction.

One fixed follow-up was declared before it ran. It used five identical-reference
pairs and five alternating reference/candidate pairs, for 20 complete captures.
Its results remain separate from the initial comparison.

| Theme | Follow-up median paired candidate minus reference | Identical-reference paired range |
| --- | ---: | ---: |
| Light | +0.30 ms | −14.13 to +3.64 ms |
| Dark | +2.88 ms | −2.18 to +7.95 ms |

The original slowdown magnitude did not repeat. The small positive paired
differences remain unresolved: this is neither an equivalence result nor proof
of a definite regression. No further timing cohort will be run merely to seek
a favorable result for this candidate.

These are PTY receipt measurements, not physical paint or display-frame pacing.
This stage did not repeat the earlier full five-patch latency cohort.

## Restoration and retained evidence

The application restoration is limited to `src/highlight.rs`, its cache checks,
and the two highlight-census files. Complete application build-input hashes must
match the preserved reference. The normal executable was rebuilt, and the process
suite passed 32 contracts with two existing opt-in exclusions. Its SHA-256 is
`b1b752e8936b29fa6f0314a4a50652e0bb402eadb954002c305afcd5788efcd1`.
The exact outcome and both executable identities are in
[local-build.json](../../evidence/style-indirection-v1/local-build.json).
It is not a newly measured performance executable.

The backward-compatible schema-3 census reader remains in the diagnostic tooling
so the rejected experiment's snapshots stay readable. Plans, reports, and
experimental binaries also remain. This is not a rollback of all project files.

| Preserved build | SHA-256 |
| --- | --- |
| Ordinary reference | `f3a98a5dea92f49bdc6257193badc74020710b715519edd19e859b8606160834` |
| Rejected ordinary candidate | `41a29846358fb08b31608e633f314dcd81a57bf1708bcffc43a74a6fe5b72a4c` |
| Diagnostic candidate | `b59d6235ac42f66e0edc1e55192d6578eb31ab3f93aec5d63bf7e396063d69dc` |

Each build retains its source archive, command, and toolchain record. Qualification,
audits, raw process captures, sample records, and the guard follow-up stay separate.
The final seal checks the selected source, local build, prior source archives, and
all earlier checksum manifests before sealing this stage. The first sealing attempt
found an incorrect path assumption in the new checker; its
[correction note](../../evidence/style-indirection-v1/SEALING_NOTE.md) preserves the failure.
All listed retained evidence matches. There are 106 historical references to
changed working files, such as current source and documentation; these are
explicitly recorded, not claimed as matching old file copies. Completed measurement
drivers must not restart after a reconnect.

## Next work

1. Follow the [small actual-terminal procedure](../../TERMINAL_PRESENTATION_CHECK.md)
   using one user-opened Ghostty window and the retained same-content Pierre
   reference. No actual Ghostty capture occurred in this experiment.
2. Follow the [metadata plan](DOCUMENT_METADATA_PLAN.md): count retained lengths,
   capacities, and descriptor sizes before selecting another storage change.
   The proposed publication-only conversion is conditional, not implemented.

The next investigation remains evidence-first. A smaller Rust descriptor alone
does not justify extra complexity or demonstrate a smaller, more responsive viewer.
