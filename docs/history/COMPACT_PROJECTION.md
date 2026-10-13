# Compact file-local projection tables

## Status

The candidate is implemented and passes the source, projection, and process
correctness checks. The full five-patch characterization is complete. The
largest patch's third-tour settled RSS maximum is **887.61 MiB**.

**Decision: do not promote this candidate.** One final-coloring step is about
7.3 ms slower in both fixed paired series, and its cause remains unexplained.
The current working source remains an explicitly unaccepted development
candidate for follow-up. The previously accepted source-run/checkpoint executable
and source archive remain preserved and unchanged. No target or expected result
has changed, and no further measurements are running or queued in this batch.

The reference is the completed source-run/checkpoint build. The ordinary
candidate is [candidate-final/native-diff](../../evidence/compact-projection-v1/candidate-final/native-diff),
SHA-256 `f87c8e7b1d96c3d1211d3a98c495f9aa135d2aed908667ec75b5c86b6b32e7c3`.
The [runbook](../../evidence/compact-projection-v1/RUNBOOK.md) preserves the original
order and the [coloring addendum](../../evidence/compact-projection-v1/COLORING_CHECK.md)
records the explicit decision to characterize this candidate before retention.

## Implementation

Four tables now have private compact and wide storage forms: display blocks,
replacement ranges, source runs, and side-run indexes. The existing projection
algorithms still determine rows, side pairing, source information, and navigation.
The storage module decodes one record on the stack and allocates nothing during
lookup.

Compact storage is selected only when the checked upper bound
`source_count + 2 * hunk_count + 2` fits `u32`. This covers logical source and
structural rows, counts, range endpoints, ordinals, and replacement padding.
Overflow selects wide storage rather than rejecting the file. Narrowing uses
`TryFrom`; coordinate arithmetic and public coordinates remain `usize`.

Construction writes directly into the chosen buffers and publishes boxed arrays.
It does not retain complete wide and compact copies. Marker and hunk-boundary
tables remain wide, as do document-global coordinates and spool offsets.
Source-span storage already has compact and wide forms; it and file metadata
remain unchanged. No grammar, cache, source guard, fallback, thread, terminal
setting, or pairing policy changed. Only `src/projection.rs` and its new private
storage module differ in the build-source manifest.

## Memory ownership

The selected compiler reports smaller projection payload on every patch. The
representation tag adds eight bytes per file descriptor; the net column includes
that cost. Other accounted owners are unchanged.

| Patch | Projection saving | Descriptor increase | Net document saving |
| --- | ---: | ---: | ---: |
| Pierre | 20,252 B | 584 B | 19,668 B |
| Ghostty | 456,852 B | 200 B | 456,652 B |
| Node | 4,858,288 B | 27,360 B | 4,830,928 B |
| Bun | 289,164 B | 17,504 B | 271,660 B |
| Linux | 116,472,340 B | 614,976 B | 115,857,364 B |

On the largest patch, net document payload falls by **110.49 MiB**. A separate
reference-then-candidate opening allocation profile shows live heap falling from
729,446,288 to 538,273,696 bytes, a **182.32 MiB** reduction. Both snapshots contain
1,492,320 live allocation nodes. Allocation groups containing projection call
sites explain 191,165,040 of the 191,172,592 saved bytes; the 7,552-byte remainder
is not assigned to an invented owner.

Exact payload, allocator-sized live heap, and ordinary process RSS are different
quantities. The opening profiles are instrumented and are not sustained-memory
or latency results. Allocator rounding can amplify a representation saving;
these observations are not a total-memory guarantee.

## Correctness and code review

- All 33 real-process tests pass: 31 ordinary cases and the two opt-in cases
  requiring preserved artifacts. All eight library checks pass. Existing
  contracts cover resizing, wrapping, pending highlighting, input failures,
  source anchors, and terminal cleanup.
- New boundary coverage checks compact selection and arithmetic overflow,
  checked narrowing, field round-trips at compact and wide limits, and actual
  parsed views through both storage forms. The parsed case includes interleaved
  replacements, nonzero padding on both sides, gaps, newline markers, and a
  document-global source base above `u32::MAX` on this host.
- The unchanged projection evaluator passes all five large patches, all five
  existing structural challenges, and 2,888 accepted exhaustive small cases;
  both parsers reject the same 716 invalid cases. For the largest patch it checks
  19,299,175 source records, 39,524,122 split/unified rows, 38,598,350 inverse
  lookups, and 6,457,902 boundary probes.
- Complete independent source digests pass on the exact ordinary candidate for
  all five patches. This supplements projection equivalence; matching the
  preserved model alone would not prove ideal pairing for all possible inputs.
- Application formatting and all-target/all-feature Clippy pass. The vendored
  dependency's existing lifetime-spelling warning remains. Vendored code did not
  change in this batch.

Self-review replaced private narrowing `From` conversions with `TryFrom` before
measurement. Independent review found no remaining runtime defect and requested
the added nonzero-padding coverage. The initial build is retained as an unmeasured
draft. Shared process contracts use the exact preserved executable; explicit
Cargo commands in other cases still exercise their separately built executable.

## Responsiveness evidence

The first fixed comparison and one declared confirmation each used five
alternating ordinary-process pairs. Each series has 40 supported visual runs,
all passing. Ten additional position entries per series pass complete source
checking but remain excluded from visual/timing claims because the Python
terminal decoder does not preserve joined emoji. Rust position tests are separate.

Continuous-input end coloring shows a repeatable slower result:

| Series | Reference median / maximum | Candidate median / maximum |
| --- | ---: | ---: |
| First | 64.13 / 70.96 ms | 71.55 / 71.70 ms |
| Confirmation | 62.24 / 63.96 ms | 69.57 / 71.07 ms |

Correct text remains about 0.7 ms at that destination. Other actions have mixed
differences, and all individual observations remain in the two reports. For
example, confirmation opening-color maximum on the continuous case is
56.37 ms in the candidate versus 41.53 ms in the reference. A memory saving does
not erase these slower observations or establish equivalent responsiveness.

One separate diagnostic sequence per build finds the same 1911 predecessors and
48 requested records at the end request. Instrumented worker time is 65.57 versus
68.32 ms; requested parser calls account for 56.90 versus 59.56 ms, and publication
is 7.54 ms in both. The unattributed remainder, including projection access, is
0.469 versus 0.514 ms. This does not show additional source work or a
multi-millisecond projection-access cost, but it does not establish the complete
cause of the ordinary slowdown. Instrumented timings are not acceptance results.

## Full characterization and disposition

The full five-patch characterization completed with fresh checked captures, the
unchanged sequential style oracle, unchanged historical workload semantics,
25 latency processes, and five three-tour memory processes. All completed their
checked views and terminal cleanup without a run failure. This does not cancel
the paired small-case regression or establish its cause.

All 25 latency processes have completed with matching checked views and clean
exits. The table reports maxima across five fresh processes per patch, not
physical terminal paint latency. Navigation coloring covers the five frozen
action categories; the raw report also preserves each category's p95.

| Patch | First text | First color | Complete index observed | Navigation color | Resize | Exit |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Pierre | 8.99 ms | 40.74 ms | 40.74 ms | 224.27 ms | 11.25 ms | 3.05 ms |
| Ghostty | 9.67 ms | 49.23 ms | 69.76 ms | 47.38 ms | 30.20 ms | 2.54 ms |
| Node | 9.07 ms | 58.58 ms | 238.13 ms | 54.19 ms | 11.34 ms | 6.12 ms |
| Bun | 8.54 ms | 66.34 ms | 194.64 ms | 153.49 ms | 12.01 ms | 5.77 ms |
| Linux | 9.98 ms | 21.12 ms | 4,374.36 ms | 78.31 ms | 8.71 ms | 73.61 ms |

First-text, first-color, and exit targets pass for all five patches. Navigation
targets pass for four; Pierre's 224.27 ms distant-coloring maximum exceeds
200 ms. The largest patch's complete-index observation exceeds 2,000 ms. These
misses remain failures of their respective development targets. Earlier usable
text does not substitute for complete indexing.

Each memory process performed three tours. Values below are maxima during
one-second settled holds in each tour, plus the OS-reported process peak.

| Patch | Tour 1 | Tour 2 | Tour 3 | Process peak | Memory targets |
| --- | ---: | ---: | ---: | ---: | --- |
| Pierre | 223.48 MiB | 270.98 MiB | 271.19 MiB | 271.22 MiB | Fail |
| Ghostty | 109.97 MiB | 110.92 MiB | 113.11 MiB | 113.14 MiB | Pass |
| Node | 235.41 MiB | 245.50 MiB | 246.97 MiB | 246.97 MiB | Fail |
| Bun | 289.81 MiB | 328.42 MiB | 351.33 MiB | 351.33 MiB | Fail |
| Linux | 866.38 MiB | 886.86 MiB | 887.61 MiB | 887.61 MiB | Fail |

Four inputs still exceed the unchanged 128 MiB settled / 192 MiB peak targets.
The largest input grows by 20.48 MiB between the first two tours and 0.75 MiB
between the last two. Three finite tours do not establish an indefinite plateau,
leak freedom, or a total-memory bound.

These are absolute RSS observations of this candidate. The earlier checkpoint's
1,021.9 MiB largest third-tour value is not a fresh paired control, so their
difference is not an attributed sustained-RSS saving. The controlled evidence
for this storage change remains the 110.49 MiB payload and 182.32 MiB opening
live-heap reductions. Small-patch RSS changes cannot be attributed to their much
smaller index savings.

The memory saving warrants preserving the candidate, not waiving its gate.
Acceptance remains on hold. The next bounded effort is to explain the repeated
continuous-input end-coloring difference before changing another memory owner.
It must preserve the existing paired samples, distinguish parser time from
publication/receipt time, and avoid changing grammar, fallback, workload, or
targets to hide the difference. The two existing paired series are final for
this batch; any further experiment needs a stated hypothesis and finite scope.

An independent repeat build differs in 48 bytes: the Mach-O linker UUID and its
corresponding first-page signature digest. Every other byte matches and all
code-page hashes verify. No executable or signature was normalized. Each trial
uses its actual preserved binary hash, not an assumed byte-identical build.

Actual Ghostty appearance, frame pacing, and trackpad behavior remain unverified.
PTY observations do not establish physical input-to-paint latency or whether a
7 ms difference is perceptible. No full product acceptance milestone is complete.

## Evidence

The batch is stored in [evidence/compact-projection-v1](../../evidence/compact-projection-v1).
The main records are:

- `candidate-final/build.json` and `build-source.tar.gz`: exact ordinary build
  recipe, source, dependency features, and executable identity.
- `change-scope.json`, `reproduction-check.json`, and `reproduction/`: scope and
  independent build checks.
- `tests.log`, `tests-opt-in.log`, `boundary-tests-002.log`, `clippy-002.log`,
  `source-check.json`, and `equivalence.jsonl`: correctness and static checks.
- `payload-comparison.json` and `allocation-comparison.json`: disjoint payload
  and controlled opening-heap comparison. Both allocation profiles are retained.
- `paired-challenges.json` and `paired-confirmation.json`: both complete ordinary
  paired series, including slower observations and decoder exclusions.
- `trace-reference/`, `trace-candidate/`, and `coloring-attribution-002.json`:
  separate instrumented attribution. The failed analyzer command is recorded in
  `COLORING_CHECK.md`; no trace was overwritten or viewer run repeated for it.
- `five-patch-characterization/checkpoint.json`: completed full-workload
  results, with per-patch captures, freezes, corruption controls, and raw trials.
- `final-verification.json`: passing active/archive source, executable, paired
  series, and frozen-workload integrity checks. Performance acceptance is
  explicitly separate and remains unestablished.
- `source.tar.gz` and `SHA256SUMS`: source package and artifact checksums. Corpus
  bodies, build targets, and the measurement environment are not packaged.
