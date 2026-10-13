# Direct token collection qualification

The prototype is closed without adoption after correctness, retained-owner, ordinary-performance, and bounded residency checks.
The normal runtime source and executable are unchanged.
The diagnostic pair retained a larger cache table and more live heap despite smaller token buffers.

## Change

The private token collector accepts `IntoIterator<Item = (Style, &str)>`.
Visible-line highlighting passes `HighlightIterator` directly instead of collecting an intermediate fragment vector.
The mapping still fully consumes the iterator, in order, and produces owned byte ranges and RGB colors.

Token storage remains `Vec<Token>`.
Parser state, side independence, fallback, source identities, changed-word ranges, cache budgets, weight calculations, and eviction policy are unchanged.
There are no new dependencies, unsafe code, custom allocators, or capacity thresholds.

The compiler can reuse the old fragment allocation during conversion.
Removing the intermediate vector is not proof that a second allocation disappeared.
The direct path changes the element size used during vector growth; the recorded capacities are smaller in most tested cases.

## Correctness and capacity checks

- 2,358 source-line advances compare actual visible-line output and parser/highlighter state against the preserved eager conversion.
- Seven bundled language grammars, both themes, divergent sides, hidden prefixes, multiline constructs, tabs, Unicode, empty text, and trailing scope changes are covered.
- Among 2,246 visible lines, 2,142 outputs have smaller capacity, 104 are equal, and none are larger.
- On Rust 1.96.0 arm64, the fragment tuple occupies 32 bytes and a token occupies 24 bytes; both have eight-byte alignment.
- The ordinary suite passes 126 enabled checks, including 61 real-terminal process checks. The two new checks account for the increase from 124.
- All 45 targeted diagnostic checks pass. The preserved ordinary candidate separately passes all 61 enabled terminal-process checks.
- The candidate matches 96 complete compatibility grids, two canonical fixture streams, and all 20,847,451 supplied source records across the five patches.
- Formatting and strict application Clippy pass. The existing vendored Syntect lifetime warning remains.

The capacity input set is deterministic diagnostic coverage, not a representative workload distribution.
These checks establish covered source, color, navigation, wrapping, resize, cancellation, and cleanup behavior.
They do not prove universal grammar correctness, physical terminal paint, allocation counts, CPU savings, or total-memory reduction.

## Bun retained-owner observation

Two current diagnostic processes completed the unchanged three-tour trace.
All 836 expected views and eight owner snapshots passed, with clean exits and reconciled cache accounting.
Power was recorded; AC was not required.

Final snapshot:

| Owner observation | Reference | Prototype |
| --- | ---: | ---: |
| Retained style entries | 85,019 | 87,287 |
| Charged style bytes | 33,553,920 | 28,373,584 |
| Spare vector bytes, already inside the charge | 12,255,656 | 6,451,936 |
| Map-capacity high-water mark | 114,688 | 114,688 |
| FIFO capacity | 131,072 | 131,072 |
| Budget evictions | 2,380 | 0 |

The prototype retained more entries while charging about 4.94 MiB less style storage in this pair.
The map and FIFO high-water behavior did not worsen in this observed trace.
This is partial owner accounting, not ordinary process RSS or a causal speed comparison.
The cache populations are not identical; some diagnostic work overlapped correctness checks.

## Largest-patch interruption

The outer capture wrapper imposed a 20-minute process timeout.
A previous completed Linux owner trace took about 24 minutes between its opening and final snapshots.
That shorter wrapper limit was a preparation error.

The candidate capture was deliberately interrupted after about five minutes, before the wrapper expired.
Its 98 checked views and partial evidence remain preserved, but it is not a completed owner comparison.
Terminal restoration, sampler shutdown, and process-group cleanup succeeded.
The Linux reference had not started. Neither completed Bun process was repeated.

A separate resume wrapper removes the extra whole-process timeout and keeps the established per-state, public-setup, and sampled-memory safety checks.
It passed seven real-process diagnostic controls.
The resumed Linux pair is now complete. Neither completed Bun process was repeated.

## Completed Linux owner comparison

Each role passed 616 expected views, four snapshots, reconciled cache accounting, and clean exit.
The offline audit also rechecked the earlier Bun pair. No sleep event occurred during the resumed pair.

| Final owner observation | Reference | Prototype |
| --- | ---: | ---: |
| Retained style entries | 113,390 | 132,432 |
| Charged style bytes | 33,554,392 | 33,553,920 |
| Spare vector bytes, inside the charge | 7,555,880 | 3,738,176 |
| Map-capacity high-water mark | 458,752 | 458,752 |
| FIFO capacity | 262,144 | 262,144 |
| Budget evictions | 4,797,837 | 4,776,554 |

The prototype retains more colored lines within the same charge. Both final inner-engine estimates are 42,602,885 bytes.
These diagnostic owners do not partition ordinary process memory or explain its residency changes.

## Ordinary qualification

The new comparison uses the preserved executables, unchanged workloads, and unchanged source/color expectations.
All ten corpus packets passed independent oracle and corruption checks after their bindings were refreshed in new files.
A private focused-capture schema repair accepts the already specified boolean sidebar state; six non-boolean values are rejected.
Its five-case unmeasured reference control and raw-packet replay passed. Control timings are excluded.
The failed initial helper-import preparation is preserved; it failed before any comparison capture.

Thirty ordinary processes completed: five focused cases per role, five latency patches per role, and five three-tour memory patches per role.
The audit passed 6,500 corpus views, 78 focused views, 2,952 one-second settled holds, and 59,004 corpus RSS queries.
No comparison capture was repeated or excluded. No sleep event occurred. Power and host conditions are recorded; AC was not required.
Default background RSS sampling remained enabled. All processes restored the terminal and exited cleanly.

The largest within-run navigation-text p95 was 21.53 ms for the reference and 21.35 ms for the prototype; the largest observed navigation-text delay was 21.95 versus 21.70 ms.
Focused cold-end coloring was 710.75 versus 713.08 ms, with text arriving in about one millisecond for both.
Linux complete-index receipt was slower in this pair: 3,748.71 versus 4,032.75 ms.
Its latency-process sampled peak increased from 710.12 to 765.12 MiB. The final sampled value was 700.86 versus 765.12 MiB, without a settled hold at that endpoint.

These are single paired observations on a shared host, not repeated speedup or physical-paint measurements.

### Sustained process memory

Each cell below is reference → prototype, in MiB. Peaks, third-tour maxima, and the final held view are different observations.

| Patch | Sampled process peak | Third-tour settled maximum | Final settled hold, maximum |
| --- | ---: | ---: | ---: |
| Pierre | 69.31 → 69.28 | 69.31 → 69.28 | 69.31 → 69.28 |
| Ghostty | 68.22 → 64.11 | 68.22 → 64.11 | 68.22 → 64.11 |
| Node | 140.56 → 138.00 | 140.56 → 138.00 | 140.56 → 138.00 |
| Bun | 170.05 → 162.84 | 170.05 → 162.84 | 119.84 → 156.25 |
| Linux | 800.56 → 770.30 | 800.50 → 663.56 | 634.70 → 663.56 |

Every prototype sustained-process peak was lower in this cohort, but the Bun and Linux final held values were higher.
The Linux prototype's tour maxima were 770.30, 663.39, and 663.56 MiB; that decrease is not credited to token collection.
Both roles contain substantial RSS declines. Host-wide VM records show compression activity, but do not identify why either viewer's residency changed.
These results do not establish released allocation bytes, a leak, a long-term plateau, or a total-memory guarantee.
Node, Bun, and Linux still exceed the unchanged settled-memory goal; Linux also exceeds the index-readiness and peak-memory goals.

## Completed bounded residency investigation

One small control and two diagnostic Linux processes completed without allocation-stack logging.
Each Linux process used the existing 204-action latency trace, with native and owner observations at opening and the final settled state.
The audit passed 414 expected views, six owner snapshots, twelve native commands, twelve one-second holds, and 2,531 raw RSS queries.
Nine corrupted native records were rejected. All processes restored the terminal and exited cleanly.
No capture repeated, and no sleep event occurred. Power was AC at recorded boundaries, but the driver did not require AC.

| Final diagnostic observation | Reference | Prototype |
| --- | ---: | ---: |
| Native live heap, MiB | 624.46 | 646.25 |
| Retained style entries | 103,480 | 120,204 |
| Charged style bytes | 33,553,624 | 33,554,304 |
| Spare style-vector bytes, inside the charge | 10,714,568 | 6,808,800 |
| Map-capacity high-water mark | 229,376 | 458,752 |
| FIFO capacity, bytes | 4,194,304 | 4,194,304 |
| Held RSS before native inspection, MiB | 708.45 | 774.77 |
| Held RSS after native inspection and census, MiB | 711.14 | 726.92 |

Opening native heaps differed by only 1,344 bytes. The prototype retained 21.79 MiB more live heap at the final state.
Its table reached a larger capacity during an admission that also evicted an entry.
Entry count remained 118,844 across that admission, below its earlier peak.
These observations establish container growth during churn, not a standard-library defect.
Public map capacity is usable slots, not allocated bytes.

The largest anonymous native allocations were 25,444,352 and 50,872,320 bytes.
They support the larger-table interpretation, but their unlogged records do not identify a Rust owner.
The evidence does not assign the complete heap difference to those allocations or retrospectively explain the earlier ordinary RSS difference.

Both final observations had the same page payload, geometry charge, checkpoint charge, live parser state, and compiled-engine estimate.
Both native malloc-zone snapshots reported zero swapped bytes.
The prototype's RSS decrease during inspection remains unattributed.
Native inspection and census serialization can affect residency. These are ordered diagnostic intervals, not ordinary performance measurements.

## Decision and next work

Close direct token collection without adoption.
The candidate preserves covered output and reduces vector slack, but its whole-process tradeoff does not justify adoption.
The earlier ordinary observations remain intact, including the higher latency-process peak and final held values.
The normal source, cache policy, budgets, and executable remain unchanged.

The next separate design task concerns total style-cache storage under repeated insertion, eviction, and source invalidation.
The current charge includes occupied style values and vector capacity, but excludes unused table buckets and FIFO capacity.
This scope applies to the normal cache across patch and complete-source inputs.
It is not a cache-policy change to rescue this prototype.

The design must preserve source identity, exact output, colors, invalidation, navigation, and responsiveness.
No smaller budget, custom allocator, hash-seed tuning, or previously rejected storage representation is selected.
The completed performance cohorts do not need another run to reconsider this candidate.

## Current evidence

Residency verification report (temporary verification record `2026-09-26-token-residency.gOQFhS/report.html`, not retained), reconciled native and owner records (temporary verification record `2026-09-26-token-residency.gOQFhS/analysis.json`, not retained), source review and decision (temporary verification record `2026-09-26-token-residency.gOQFhS/REVIEW.md`, not retained), and capture plan (temporary verification record `2026-09-26-token-residency.gOQFhS/PLAN.md`, not retained).

Verification report (temporary verification record `2026-09-26-token-qualification.HUqf24/report.html`, not retained), combined audit (temporary verification record `2026-09-26-token-qualification.HUqf24/analysis-focused-latency-memory-owners.json`, not retained), residency audit (temporary verification record `2026-09-26-token-qualification.HUqf24/residency.json`, not retained), and review (temporary verification record `2026-09-26-token-qualification.HUqf24/REVIEW.md`, not retained).

## Preserved earlier checkpoint

Verification report (temporary verification record `2026-09-26-direct-tokens.45MLJC/report.html`, not retained)

Audit (temporary verification record `2026-09-26-direct-tokens.45MLJC/analysis.json`, not retained), code review (temporary verification record `2026-09-26-direct-tokens.45MLJC/REVIEW.md`, not retained), and remaining adoption gate (temporary verification record `2026-09-26-direct-tokens.45MLJC/NEXT.md`, not retained).
The exact candidate source, executable, build recipe, raw checks, interrupted capture, and corrected wrapper are in that report directory.

The preserved ordinary candidate is `candidate/native-diff`, SHA-256 `3a78596f433300569c429b48bac7516584211b67b399e020fff672fe5b818a3e`.
The normal executable remains `53d28a87bd1669910adfc6f28aedb20063ddb9db270310ad056da5a20a3bd838`.

The earlier checkpoint retains its original partial status. The completed qualification and current decision above supersede its remaining-work list.
