# Display-block storage

Date: 2026-09-12. Status: implementation and verification complete. Retained as a storage cleanup, not full performance acceptance.

Context display blocks now retain only their source start. Removing the unused end coordinate reduces every display-block record from 40 to 32 bytes on the measured build. The largest input saves exactly 22.8 MiB of retained projection payload. The row algorithms, source storage, supported numeric domain, and checked visible behavior are unchanged.

## Evidence and scope

`Projection` owns boxed arrays for display blocks, replacements, source runs, side indexes, markers, and navigation boundaries. Fresh before/after `inspect --summary` accounting measures live owned payload, independent of whether macOS keeps those pages resident. It does not measure allocator overhead or total process memory.

Previously, `BlockKind::Context` stored a source `Range<usize>`. Source-to-row and row-to-source lookup read its start; neither read its end. Block starts, the next block, and the file's total row count already delimit displayed rows. The unused end enlarged the enum carried by every display block, including headers and replacement blocks.

The runtime change is limited to that private variant, its constructor, and its two readers in `src/projection.rs`. All coordinates remain `usize`. Replacement pairing, line numbers, source runs, newline markers, syntax state, geometry, anchors, cache budgets, and fallback behavior are unchanged. No other index is compacted.

An independent review confirmed that the deleted value had no reader. A temporary compile-time size assertion was removed during review; the product does not require future compilers to choose today's exact representation. The existing metadata-only fixture has four structural blocks and no replacements. Its ordinary inspection reports 160 bytes before and 128 after, directly confirming the measured 40/32-byte record sizes without a production diagnostic hook.

## Verification plan and outcome

1. Preserve the current ordinary binary, original sequential style reference, and source archive before editing. Retain new evidence under `evidence/index-v1` without rewriting earlier runs.
2. Confirm Rust type sizes and disjoint retained-payload counts before and after. Use the existing profiler to distinguish current live allocations and allocator/residency observations at a checked viewport. Instrumented observations are not latency results.
3. Run the preserved projection evaluator over all five patches, the small challenge inputs, and its exhaustive short-pattern cases. Compare every row in both layouts, inverse lookup, partner, source byte, marker, metadata record, and navigation-boundary probe.
4. Run all real-process tests, independent complete-source checks, formatting, and Clippy. Check the exact measured release binary with the existing rendering/geometry/syntax contracts.
5. Run the existing alternating small-workload comparison and the five-patch latency/three-tour memory checkpoint with unchanged workload semantics and source/style expectations. Keep failures and slower observations. Attribute the storage saving from owned payload; do not infer a universal RSS reduction from one residency timeline.

Retain the change only if it reduces owned storage and preserves the checked behavior without an unexplained responsiveness regression. This does not complete actual Ghostty/Pierre visual acceptance or establish a total-memory bound.

The completed checks satisfy the scoped retention decision. The initial small-sample coloring concern did not consistently reproduce in its declared follow-up, as detailed below. That is not proof of performance equivalence.

- All 33 real-process tests passed, plus the two existing numeric storage-boundary tests. Formatting and all-target/all-feature Clippy passed. The unchanged vendored dependency retains its existing lifetime-spelling warning.
- The exact final ordinary binary passed the 13 rendering/geometry/syntax contracts that accept an executable override. The filtered log also contains one streamed-input contract using Cargo's test executable.
- Full projection comparison passed on the five large patches, all five challenge inputs, and 2,888 accepted short-pattern cases. Both parsers rejected the same 716 invalid patterns. The large patches cover 42,657,296 split/unified rows, with source, inverse lookup, partner, metadata, marker, and boundary checks.
- Independent complete-source digests match all five large patches, including all 19,299,175 supplied records in the largest input.
- All 25 latency trials and all five three-tour memory runs completed with matching checked views and clean exits. New traces match the prior workload semantics and unchanged original sequential highlighter; the input and source hashes verify.

## Owned storage and allocation observations

| Patch | Projection payload before | After | Exact bytes saved |
| --- | ---: | ---: | ---: |
| Pierre | 53,472 | 48,824 | 4,648 |
| Ghostty | 1,174,256 | 1,105,128 | 69,128 |
| Node | 12,809,096 | 11,787,256 | 1,021,840 |
| Bun | 756,488 | 684,280 | 72,208 |
| Linux | 305,695,992 | 281,762,360 | 23,933,632 |

All non-block component counts are unchanged. The saving scales with display-block count, not a repository name or a patch-byte threshold. On the largest input it removes eight bytes from each of 2,991,704 blocks.

Two separate instrumented opening snapshots used the same checked viewport and trace. Reported live heap fell from 788,469,216 to 753,615,440 bytes, a 33.2 MiB reduction, with 1,492,319 live allocation nodes in both. Allocation classes labeled `Projection::new` account for 34,298,048 of the 34,853,776-byte difference. This coarse allocation-owner grouping supports the intended change. Allocator-sized allocations are not the same quantity as structural payload, and equal allocation counts do not prove identical allocation identities.

These snapshots are opening-only allocation evidence, not ordinary latency, peak-RSS, or sustained-memory acceptance. They do not establish a universal 33.2 MiB saving.

## Responsiveness

The initial alternating five-pair comparison completed 40 supported real-process runs with matching source and screens. Ten position-fixture entries have source passes but remain unsupported by the Python decoder's joined-emoji handling. They supply no visual pass or timing; the separate Rust position contracts still pass.

Continuous TypeScript end coloring was slower in four initial candidate pairs: median 64.09 ms before and 71.45 ms after. Light-theme coloring was also slower in four pairs, with medians 19.13 and 23.40 ms. Correct end text arrived around 0.7 ms. These observations were retained and triggered one declared five-pair follow-up, with reversed initial order and many-small files as a control.

The follow-up completed all 20 runs. End-coloring medians were 68.77 ms before and 65.35 ms after; light-theme medians were 24.05 and 24.07 ms. The initial slowdown did not consistently reproduce. Other slower observations remain: the follow-up dark-theme median was 27.51 versus 31.32 ms, and the candidate's many-small opening maximum was 56.75 versus 47.09 ms. Neither series replaces the other. These small samples establish neither a general speedup nor performance equivalence.

The full current-build checkpoint uses five fresh processes per patch and 200 observations per navigation category. The navigation column below reports the largest category p95 and maximum, not a pooled percentile.

| Patch | Median first text / styling state | Maximum full-index observation | Navigation styling p95 / maximum |
| --- | ---: | ---: | ---: |
| Pierre | 8.51 / 40.33 ms | 56.28 ms | 16.60 / 298.34 ms |
| Ghostty | 8.74 / 47.30 ms | 71.23 ms | 23.27 / 48.52 ms |
| Node | 8.46 / 53.38 ms | 187.41 ms | 15.37 / 47.40 ms |
| Bun | 9.00 / 31.62 ms | 184.18 ms | 17.73 / 153.44 ms |
| Linux | 8.24 / 19.84 ms | 4,794.56 ms | 6.93 / 64.83 ms |

The distant-coloring maximum on Pierre and full-index target on Linux still fail. All navigation-category p95 values and all quit targets pass; the largest latency-trial quit was 70.23 ms. These are PTY output observations, including observer overhead, not Ghostty paint latency. A declared plain-styling fallback is not full syntax highlighting.

## Sustained memory

Tour values are maximum RSS samples during the fixed settled holds. The OS peak covers the entire process. Each row is one process making three tours, not a repeated estimate or indefinite bound.

| Patch | Tour one | Tour two | Tour three | OS peak |
| --- | ---: | ---: | ---: | ---: |
| Pierre | 224.8 MiB | 271.9 MiB | 272.0 MiB | 272.0 MiB |
| Ghostty | 107.3 MiB | 108.7 MiB | 109.2 MiB | 109.3 MiB |
| Node | 243.1 MiB | 253.0 MiB | 254.5 MiB | 254.5 MiB |
| Bun | 285.3 MiB | 115.3 MiB | 127.0 MiB | 285.3 MiB |
| Linux | 1,036.0 MiB | 1,052.3 MiB | 1,052.5 MiB | 1,086.4 MiB |

Only Ghostty meets both shared memory goals. Bun's large RSS drop is not explained by its 70.5 KiB index saving. The ordinary sampler cannot distinguish freed allocations from resident-page changes. A host snapshot taken during that phase is not per-process attribution. Keep the full timeline; do not report its lower later values as an optimization win or sustained plateau.

The largest-input readings are also not an isolated before/after estimate. The exact index saving and separate allocation snapshots support this cleanup; these ordinary tours describe current behavior and remaining memory costs. They do not resolve the prior residency ambiguity.

## Evidence and remaining work

Reference ordinary binary SHA-256: `6f56bdd7814e19efe8745fd3d836f0a2a032089f906b5de129422fbecc2077ed`.

Final ordinary binary SHA-256: `5d2b7a7816ac21f6e8c60c549efb7656466e6888496a83732b90723b6a09507a`.

The [private runbook](../../evidence/index-v1/RUNBOOK.md), [payload comparison](../../evidence/index-v1/payload-comparison-final.json), [checkpoint](../../evidence/index-v1/five-patch/checkpoint.json), and [fixed follow-up](../../evidence/index-v1/confirmation.json) retain commands, identities, failures, and individual results. Source packages and executable copies remain under `evidence/index-v1`. The first profile attempt used a nonexistent trace path and failed before any viewer launched; that failed attempt remains recorded.

Remaining work includes other retained-memory owners, expensive cold/backward coloring, ordinary residency attribution, and actual Ghostty/Pierre presentation checks. Existing long-line styling and pairing limitations are unchanged. This cleanup is not full product or performance acceptance.
