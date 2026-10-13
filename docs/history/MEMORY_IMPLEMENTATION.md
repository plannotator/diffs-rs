# Memory implementation checkpoint

Date: 2026-09-11. The storage refactor is implemented. All five interaction series and three-tour memory runs completed without correctness or cleanup failures. Syntax-allocation changes and full performance acceptance remain open, including a confirmed first-color regression.

## Changes

`source_spans.rs` stores relative offsets and lengths in eight-byte records, with a separate final-newline bitset. Each file selects compact or wide storage using its validated input-unit byte base. A checked wide representation retains the previous supported offset/length domain. Source bytes still come from the private patch spool. A multi-file input unit can select wide storage even when a later file's own source extent is small; this affects compression, not supported content.

`projection.rs` replaces `layout.rs`. One set of context, replacement, and structural blocks resolves both layouts. Source runs supply line metadata and inverse lookup. Replacement side-run indexes preserve accepted interleaved additions/deletions. Explicit marker positions retain paired and unpaired final-newline rows. Header/hunk boundary lookup does not scan source records.

The input path no longer constructs the temporary full split/unified row vectors or source-location vectors. Alignment candidate IDs are allocated only when the existing bounded alignment rule can search for an offset. The rule, threshold, scoring, and padding remain unchanged. Libgit2 validation and temporary per-file source bytes remain; a borrowed parser rewrite has not been justified or implemented.

The application, renderer, sanitizer, geometry, palette, and highlighting source files remain byte-for-byte unchanged from the reference. No dependency, cache limit, fallback threshold, command, terminal setting, or source-content rule changed.

## Retained index payload

These numbers count source spans/flags plus the reported projection components. They exclude metadata allocations, file descriptors, allocator overhead, syntax allocations, and caches. They are not RSS.

| Patch | Before, MiB | After, MiB |
| --- | ---: | ---: |
| Pierre | 0.36 | 0.19 |
| Ghostty | 2.95 | 1.84 |
| Node | 23.93 | 15.47 |
| Bun | 16.59 | 8.60 |
| Linux | 704.76 | 441.11 |

Linux source spans/flags changed from 308,786,800 to 156,836,180 bytes. Its other index components changed from 430,208,368 to 305,695,992 bytes. Every pinned file uses the compact span path. Per-file descriptors increased; those allocations are reported separately rather than hidden in the savings.

The shared source runs grew because they now carry block/side coordinates as well as source metadata. Their Linux payload is 126,184,128 bytes. Shared blocks use 161,566,688 bytes. Checked narrowing of these file-local fields is a possible later step, not part of this checkpoint.

Exact component counts: [reference](../../evidence/memory-v1/reference-index.json), [compact spans](../../evidence/memory-v1/s1-index.json), [shared projection](../../evidence/memory-v1/s2-index.json).

## Correctness evidence

The baseline evaluator in `examples/projection_reference/model.rs` is an exact copy of the pre-refactor model. Its SHA-256 is `6de246d31c083cd260b5ca1cf2be261be7062b62211e8e28b7530b3c12f2fef4`. It does not call the new projection. The diagnostic streams file units through the preserved evaluator and the actual production storage reader, with only bounded reference work queued.

Across all five patches, the diagnostic compares:

- Every displayed row in both layouts, including padding and marker side flags.
- Every source record's bytes, kind, line numbers, and final-newline flag.
- Every source-to-row lookup in each layout and every replacement partner.
- Complete file/hunk metadata, normalized to global source coordinates.
- Every file/hunk navigation boundary and its neighboring-row probes.

Linux alone passed 39,524,122 row comparisons, 38,598,350 inverse lookups, 19,299,175 partner checks, and 6,457,902 boundary probes. This is exhaustive representation equivalence, not independent proof that every baseline pairing is ideal.

The diagnostic also enumerates all nontrivial context/deletion/addition sequences of lengths one through six with valid final-side marker combinations. It compared 2,888 accepted patches. Both parsers rejected the other 716 cases. Existing handwritten terminal contracts remain the independent answers for ambiguous pairing and inline emphasis.

Independent complete-source digests passed for all five patches at S1, S2, and S3. All 31 real-process tests, including both normally ignored evidence-dependent tests, passed at S1, S2, and the final S3 checkpoint. The two numeric span tests cover maximum compact offsets, full `u32` lengths, bitset boundaries, wide fallback, overflow, out-of-spool ranges, and empty spans without allocating multi-gigabyte fixtures. Formatting and warnings-denied Clippy passed for all targets after the final diagnostic edits.

The final equivalence pass also checks append order and cumulative split/unified row bases across every file. It compares values directly and records counts; fixture, source, and executable hashes provide identity rather than a second row-digest implementation.

Evidence: [final S3 equivalence](../../evidence/memory-v1/s3-equivalence-final.jsonl), [S3 canonical checks](../../evidence/memory-v1/s3-canonical.json), [process checks](../../evidence/memory-v1/s3-process-final.log), [migration negative controls](../../evidence/memory-v1/trace-migration-controls-final.json), [all ten final freeze checks](../../evidence/memory-v1/s3-final-identities.json).

Actual Ghostty pixels, shaping, frame pacing, and trackpad behavior remain unverified by these PTY checks.

## Allocation findings

The native macOS tools could inspect our child processes without GUI automation. `profile_memory.py` uses the existing PTY commands and expected cells, enables malloc stack logging only for the child, and records `vmmap`, `heap`, and `malloc_history` snapshots. These instrumented runs supply attribution, never accepted timing values.

Linux was profiled with an optimized debuginfo build in a separate target directory. The live heap after opening was 1,141,132,016 bytes. The two largest resolved allocation stacks were the former split/unified projection builders. Component accounting separately confirms 322,572,624 bytes of logical row-run payload before refactoring. Heap allocation classes and logical payload counters measure different quantities.

The preserved Pierre executable supplied usable Rust allocation symbols. Its live heap was 2,825,616 bytes at opening, 258,858,992 after tour one, 325,515,440 after tour two, and 325,591,840 after tour three. Heap classes whose allocating function names contain `regex_automata` or `fancy_regex` account for 253,318,896 bytes at the third snapshot, about 78% of its live heap. Other regex-related allocations may have generic allocation names, so this is a named-class attribution, not a complete ownership census.

The resolved stacks enter Syntect's lazy regex compilation through the syntax worker. The pinned Syntect source retains each compiled regex in a `OnceCell`; its fancy-regex backend builds regex-automata state. This supports compiled grammar storage as the next small-patch target. It does not establish an unbounded leak or justify shrinking line caches.

An opt-in [static grammar inventory](../../evidence/memory-v1/regex-inventory.json) found 21,739 pattern entries and 9,605 unique pattern strings across 213 syntaxes. This makes immutable compiled-pattern sharing worth investigating. It does not tell us which entries compile during a tour, weight their costs, or predict a memory saving. No syntax dependency was patched and no compiled-pattern sharing was implemented.

The first Pierre attribution run used the preserved release binary, not a new debuginfo binary. Its final snapshot overlapped compilation of the separate Linux profiling build. No timing/RSS acceptance is taken from it; live allocation stacks remain diagnostic evidence. Linux's attribution run was serial. Both processes completed their checked commands and restored their PTYs.

Private footprint, live heap, allocator slack, mapped library pages, and RSS are recorded separately. None replaces the existing RSS acceptance targets.

Raw attribution: [Pierre](../../evidence/memory-v1/profile-pierre-reference/report.json), [Linux](../../evidence/memory-v1/profile-linux-reference/report.json).

## Completed timing series

All five fresh-process repetitions completed for each patch, with 200 observations per category. The table reports maxima across the five opening observations and the worst styled percentile/maximum among the five navigation categories. Percentiles are not pooled across categories.

| Patch | First text, max ms | First colors, max ms | Index ready, max ms | Worst styled p95, ms | Worst styled maximum, ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Pierre | 7.9 | 77.3 | 17.9 | 28.0 | 258.6 |
| Ghostty | 9.0 | 63.7 | 69.1 | 28.0 | 70.0 |
| Node | 8.4 | 45.7 | 168.4 | 29.2 | 72.9 |
| Bun | 8.9 | 67.2 | 197.1 | 30.1 | 167.8 |
| Linux | 9.1 | 19.3 | 4,156.4 | 7.5 | 86.4 |

Opening and quit limits passed for all patches. Navigation limits passed for Ghostty, Node, Bun, and Linux. Pierre's 258.6 ms styled distant-seek maximum fails the 200 ms limit; its preserved maximum was 251.8 ms. The failure remains open.

Linux's peak RSS across these latency repetitions fell from 1,343.2 to 1,010.5 MiB. These are not sustained-memory observations. The three-tour results are reported separately.

Small-patch text-response percentiles remain close to the prior values, with some increases. For Pierre, the forward text p95 changed from 3.57 to 3.79 ms, layout from 8.64 to 9.03 ms, and resize from 9.99 to 10.65 ms. Shared lookup does more local arithmetic and searches than the former retained row maps. These comparisons do not isolate the cause of each timing difference.

### First-color regression

Pierre's five-repeat first-color observations increased from 34.1–67.2 ms to 71.6–77.3 ms. Two additional diagnostics each ran ten alternating reference/candidate pairs against the exact retained opening cells. The second diagnostic reused both executable paths after the first had exercised them. All openings and exits were correct.

| Diagnostic | Reference text median, ms | Candidate text median, ms | Reference colors median, ms | Candidate colors median, ms |
| --- | ---: | ---: | ---: | ---: |
| First paired series | 7.21 | 7.37 | 48.14 | 67.58 |
| Warm-path paired series | 7.29 | 7.36 | 49.23 | 77.26 |

The first candidate-copy launch took 283.77 ms to text and 335.82 ms to colors. It remains in the first series, including its medians and maxima. Later warm-path text maxima were 7.84 ms for the reference and 7.81 ms for the candidate. The cause of the initial launch outlier is unresolved; it is not silently removed as warm-up.

The warm-path colors ranged from 22.68–80.27 ms for the reference and 37.35–84.01 ms for the candidate. This supports a first-color regression, not a universal fixed penalty. These short diagnostics do not replace the full acceptance run or establish pixel paint times.

The unchanged event loop checks worker replies, then waits up to 4 ms on input publication while loading or 16 ms on keyboard input after loading. The unchanged syntax worker has a 16-reply bounded queue. Faster index completion can therefore move a still-pending viewport into the slower polling path and require several drains. This is a source-backed explanation to test with explicit reply/publication observations; the current timings do not isolate every contribution. Reducing the polling interval blindly would trade wake-up cost for latency. A bounded notification that wakes the existing loop when replies become available is the next design to evaluate, with idle behavior and input fairness checked separately.

Evidence: [first paired series](../../evidence/memory-v1/paired-pierre-opening.json), [warm-path series](../../evidence/memory-v1/paired-pierre-opening-warm.json). The storage candidate passes its correctness gates but is not fully performance-accepted. Memory savings do not waive this regression.

## Sustained memory

All three fixed tours completed for each patch. Third-tour values are the largest settled hold observation in that tour, not the final sample after exit. Peak values are the OS high-water observation across the separate memory run.

| Patch | Reference third tour, MiB | Candidate third tour, MiB | Candidate peak, MiB | Memory limits met |
| --- | ---: | ---: | ---: | --- |
| Pierre | 303.3 | 303.1 | 303.1 | No |
| Ghostty | 123.7 | 124.0 | 124.1 | Yes |
| Node | 300.6 | 291.1 | 291.1 | No |
| Bun | 377.0 | 366.8 | 366.8 | No |
| Linux | 1,503.2 | 1,116.0 | 1,142.7 | No |

Linux's third-tour reduction is 387.2 MiB, approximately 25.8%. Its successive tour maxima were 1,092.7, 1,114.4, and 1,116.0 MiB. Three tours do not establish a long-duration memory bound or rule out a leak. The applicable limits remain 512 MiB steady / 768 MiB peak for Linux and 128 MiB / 192 MiB for each smaller patch. No limit changed.

## Measurement integrity

The preserved reference includes source, binaries, and the prior temporary five-patch evidence bundle. [Checksums](../../evidence/memory-v1/reference/SHA256SUMS) identify the original archive and binaries. S1, S2, and S3 have separate recoverable source/binary snapshots under `evidence/memory-v1`.

Candidate freezes require the baseline's fixture, command sequence, input counts, categories, source states/records, complete settled cells/styles, and syntax-check coverage to match. Only executable/reference provenance, observed quit data, and recorded setup duration are excluded from semantic equality. The normal candidate checks still require successful exit and terminal restoration.

Negative controls reject changed source text, colors, input, anchor state, categories, omitted steps, and unknown root fields. The first preparation mistakenly compared `action.setup_ms`; it rejected timing differences before measurements started. Its artifacts remain in [the interrupted preparation](../../evidence/memory-v1/s3-checkpoint/INTERRUPTED.md). Verified captures are reused in the successor run without changing their expected content.

The [S3 measurement directory](../../evidence/memory-v1/s3-measured/checkpoint.json) retains commands, candidate freezes, replay results, and failures. The standard five repetitions and three tours are unchanged. No new global benchmark framework or hidden viewer controls were introduced.

The final package under `evidence/memory-v1/final` preserves the current source and diagnostics, the measured viewer, the unchanged sequential syntax reference, and checksums for the final reports. S3's earlier source snapshot predates the trace-comparison timing fix and the final diagnostic additions; it is preserved separately, not replaced.

The host reservation was acknowledged, and other coordinated workloads remained idle during measurements. The reservation was released by a coordination message at 2026-09-12 05:31 UTC after all measurements and final checks finished. No competing-product benchmark, external issue, maintainer contact, or GUI capture belongs to this checkpoint.

## Remaining work

1. Resolve the first-color wake-up regression without increasing idle polling or sacrificing keyboard responsiveness. Preserve the completed storage measurements as the new diagnostic reference; do not relabel them as fixed results.
2. Reduce attributed syntax allocations without changing grammar answers or introducing cache-replay latency regressions. The current public Syntect API does not expose compiled-regex eviction. An engine switch or dependency patch requires an explicit design and separate evidence; neither is included here. Keep Pierre's existing styled-seek outlier in the acceptance gates.
3. Reassess Linux's remaining index/metadata payload using the new component counts. Avoid applying machine-width narrowing without checked wide handling.
4. Run final acceptance after further changes. Keep actual Ghostty visual signoff open until real captures are available.

The storage checkpoint does not complete the full S0–S5 plan or establish performance leadership over other viewers.
