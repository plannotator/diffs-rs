# Highlighting ownership and memory

Date: 2026-09-12. Status: implementation and verification complete. Retained as an ownership improvement, not full performance acceptance.

Identical static regex patterns now share their lazy compiled state within each loaded grammar. The representative three-tour allocation profile shows 11.8% fewer bytes in named regex-engine allocation classes and 11.4% less live heap overall. This is a measured reduction, not a total-memory bound or an engine replacement.

## Change and boundaries

The local Syntect 5.3.0 patch changes two runtime files. A temporary per-grammar map joins exact duplicate patterns to an `Arc<OnceCell<Regex>>`, then drops. Clones share the cell. Compilation options, Fancy Regex version, grammar assets, matching errors, scopes, rule order, and caller-owned capture regions remain unchanged. Match rules retain their original identities. Source-derived heredoc expressions do not enter the map. The compiled wrapper's existing thread-safe search-cache pool is shared with it.

There is no persistent registry, global cache, new eviction rule, or reduced styling budget. Cross-grammar duplicates remain. The patch adds initial reference-counted allocations and a temporary hashing pass when a grammar loads. The small-workload measurements below retain those costs.

The lifetime review traced source-derived regexes through the actual parser: each is created for one search and dropped afterward. Parser state and native-diff's retained interpreters keep context IDs, captures, and source strings, not those compiled regexes. Shared clone semantics also preserve static sharing when a loaded syntax set is cloned; native-diff does not currently clone a loaded set.

All 16 native-diff runtime Rust and palette files match the pre-change archive. Source storage, projection, pairing, reading anchors, event delivery, and fallback thresholds are unchanged. The root lockfile changes only Syntect's source to the vendored package; it updates no other dependency.

The [dependency patch note](vendor/syntect/NATIVE_DIFF_PATCH.md) records provenance, ownership, clone/serialization behavior, and test inputs. This adds a small local dependency-maintenance obligation; it is not an upstream release.

## Correctness evidence

- All 33 real-process viewer tests passed, including source bytes, exact cells, signs and line numbers, old/new syntax context, Unicode, wrapping, layout changes, rapid resizing during highlighting, burst navigation, loading errors, and terminal cleanup.
- The new Ruby heredoc contract uses different old/new closing delimiters, travels past 320 shared lines, and checks explicit string/keyword colors through theme changes. Both the preserved original viewer and the candidate pass it.
- Thirteen rendering, geometry, and syntax contracts also passed against the exact measured release binary. The fourteen-test filtered log includes one streamed-input contract that uses Cargo's test-built executable.
- Four selected dependency regex tests and all 41 upstream parser tests passed after exact upstream test inputs were restored. The new tests exercise real capture ranges, concurrent searches, final-owner release, distinct rule actions, cloning, and serialization. These are not the entire upstream test suite.
- Independent complete-source digests match all five corpus patches: 18,420; 93,579; 419,306; 1,016,971; and 19,299,175 supplied records.
- Formatting and all-target/all-feature Clippy passed for native-diff. The vendored upstream code retains an unrelated lifetime-spelling warning.

The failed initial dependency runs are retained: first the published crate lacked testdata, then the restored source lacked its pinned grammar submodule. No expected answer was changed to make those failures pass. The restored test assets do not replace runtime grammars.

## Allocation attribution

Both preserved release binaries ran the same Pierre three-tour trace with exact settled-cell comparisons. Child-only MallocStackLogging was enabled. These are allocation observations, not accepted RSS or latency measurements.

| After tour three | Before | Candidate | Reduction |
| --- | ---: | ---: | ---: |
| Named Fancy Regex / regex-automata allocations | 253,318,896 B | 223,523,136 B | 11.8% |
| All reported live heap | 325,381,680 B | 288,383,408 B | 11.4% |
| Live allocation count | 713,368 | 641,255 | 10.1% |

Named engine bytes were unchanged between tours two and three in both runs. Total live heap grew by about 75 KiB during the last tour in each. This finite observation does not establish an indefinite memory bound. Most remaining live heap is still attributed to regex-engine automata and their search structures.

## Small-workload responsiveness

Five fresh-process pairs alternated order per input, with no discarded warm-up. Every observed completed screen matched the preserved viewer and independent source checks. Each trial opens, visits the end/start, changes theme, narrows/widens the terminal, and quits.

| Input | Median opening styling state, before / candidate | Maximum, before / candidate |
| --- | ---: | ---: |
| Many small Rust files | 40.76 / 41.88 ms | 42.83 / 56.87 ms |
| Continuous TypeScript hunk | 42.02 / 42.44 ms | 47.63 / 46.28 ms |
| Styling-guard boundaries | 62.62 / 63.84 ms | 63.81 / 73.72 ms |
| Ratatui release patch | 65.49 / 63.25 ms | 66.78 / 67.25 ms |

These results are roughly flat, not a general opening speedup. The slower many-small and guard maxima remain visible. Small-input peak RSS rose by about 0.1 to 0.7 MiB; Ratatui's peak fell from 37.6 to 35.3 MiB. Median child CPU was close between builds. A defined plain-styling fallback is not full syntax highlighting.

The shared position fixture has an independent source pass but remains unsupported by this Python visual probe because pyte does not preserve its joined emoji. It supplies no timing or visual pass here; the Rust Unicode/position contracts cover that input separately.

## Full corpus checkpoint

All 25 latency trials and all five three-tour memory runs completed with exact expected screens and clean exits. New captures match the existing workload semantics and the preserved unpatched sequential highlighter before freezing. The existing uniform targets remain unchanged. Vendored runtime source, manifest, and embedded assets are included in the new freeze hashes.

Hardware: Apple M5 Max, 48 GiB RAM, arm64 macOS 26.3. Rust 1.96.0; pyte 0.8.2. The host was reserved for this checkpoint. Builds, profiles, and ordinary measurements did not overlap one another.

| Patch | Median first text | Median first styling state | Maximum complete-index observation | Largest navigation styling p95 / maximum |
| --- | ---: | ---: | ---: | ---: |
| Pierre | 9.30 ms | 42.11 ms | 42.11 ms | 15.94 / 346.84 ms |
| Ghostty | 9.23 ms | 47.38 ms | 75.51 ms | 24.16 / 48.24 ms |
| Node | 8.76 ms | 52.97 ms | 190.62 ms | 15.32 / 50.74 ms |
| Bun | 9.25 ms | 32.61 ms | 214.25 ms | 17.61 / 174.07 ms |
| Linux | 10.14 ms | 21.57 ms | 4,934.98 ms | 7.50 / 59.07 ms |

Each navigation category has 200 observations per patch, 5,000 category observations overall. The last column reports the largest p95 and maximum across forward, reverse, distant seek, layout, and resize categories, not a pooled percentile. All category p95 values meet 50 ms. Pierre fails the 200 ms maximum: its slowest distant seek showed correct text at 6.58 ms and final styling at 346.84 ms. Linux fails the two-second complete-index target, not first-visible-text or first-styling targets. All five patches meet the quit target; the largest observed latency-trial quit was 84.79 ms.

| Patch | Tour one | Tour two | Tour three | OS peak | Memory targets met |
| --- | ---: | ---: | ---: | ---: | --- |
| Pierre | 224.1 MiB | 271.0 MiB | 271.1 MiB | 271.1 MiB | No |
| Ghostty | 109.4 MiB | 110.8 MiB | 111.4 MiB | 111.4 MiB | Yes |
| Node | 244.9 MiB | 254.2 MiB | 255.6 MiB | 255.6 MiB | No |
| Bun | 288.2 MiB | 326.4 MiB | 333.3 MiB | 333.3 MiB | No |
| Linux | 1,114.4 MiB | 1,085.8 MiB | 1,086.5 MiB | 1,115.8 MiB | No |

Tour values are the maximum RSS sampled during the fixed settled holds, not a post-exit or minimum sample. The OS peak covers the whole child process. Bun's tour maximum still grew by about 6.9 MiB during tour three. Linux's second-tour maximum fell below its first, then rose about 0.7 MiB in the third. That sequence does not describe the entire residency timeline: both largest-input runs also have sharp RSS drops, detailed below. Neither cache limits nor these finite tours prove bounded total memory. The shared goals remain 128 MiB retained / 192 MiB peak for every input.

## Fresh ordinary memory controls

The immediately preceding viewer runs the same three-tour workload without allocation instrumentation. Each fresh capture must pass the independent checks, original sequential colors, and candidate trace semantics before replay. These reference controls follow the candidate batch serially. They are one memory process per binary and input, not randomized repeated estimates or confidence intervals.

| Input and build | Tour one | Tour two | Tour three | OS peak |
| --- | ---: | ---: | ---: | ---: |
| Pierre, before | 245.0 MiB | 304.4 MiB | 304.5 MiB | 304.6 MiB |
| Pierre, candidate | 224.1 MiB | 271.0 MiB | 271.1 MiB | 271.1 MiB |
| Linux, before | 1,146.9 MiB | 1,114.6 MiB | 891.1 MiB | 1,146.9 MiB |
| Linux, candidate | 1,114.4 MiB | 1,085.8 MiB | 1,086.5 MiB | 1,115.8 MiB |

Pierre's third-tour settled maximum falls by 33.5 MiB, about 11%. This ordinary RSS observation agrees with the separate live-allocation profile. Older S3 measurements predate the event-delivery change and are not used to isolate this patch's effect.

The Linux comparison does not establish a clean sustained-RSS improvement. Its candidate has a lower process peak but a third-tour maximum 195.5 MiB higher than the reference. The reference loses resident memory sharply late in tour two, while the candidate has sharp drops during tour three. Their last settled observations are 891.1 MiB before and 852.2 MiB after; those are additional observations, not replacements for the declared tour-maximum metric.

For example, the reference drops from 1,034.6 to 945.9 MiB between adjacent samples around 902 seconds. The candidate drops from 956.1 to 675.0 MiB around 1,106 seconds. RSS does not distinguish freed allocations from changes in which pages remain resident. No contemporaneous per-process compressed-memory or live-allocation observation identifies the cause. A host snapshot taken after the run records compression and swap use, but cannot attribute these earlier drops. Both timelines and all unfavorable observations remain retained. Do not infer an application-memory saving, regression, or steady-state bound from that pair alone.

The patch is retained because the ownership profile and focused ordinary comparison demonstrate a real reduction, the covered grammar/source answers remain unchanged, and alternating small-workload responsiveness is roughly flat. The outstanding coloring maximum, index latency, finite-tour growth, and residency ambiguity remain open. The next memory investigation should distinguish live ownership from residency before changing more storage or cache policy.

## Evidence and remaining limits

Evidence is under [evidence/regex-v1](evidence/regex-v1); [the runbook](evidence/regex-v1/RUNBOOK.md) records commands, failures, identities, and profiling boundaries. The unchanged original sequential highlighter remains separate from the candidate, so the new engine ownership cannot redefine its own expected colors.

Reference release SHA-256: `3b67259e205d918f1eae1d94b2fcfea86e842ca8e233b313e811f1e1048d5627`.

Candidate release SHA-256: `6f56bdd7814e19efe8745fd3d836f0a2a032089f906b5de129422fbecc2077ed`.

PTY observations include output delivery and observer overhead. They do not establish Ghostty paint latency, trackpad smoothness, terminal-renderer memory, complete Pierre visual parity, or correctness for every possible input. Existing explicit pairing and long-line fallback limits remain. Further engine ownership work must preserve grammar answers and account for cross-grammar duplicates, registry lifetimes, and cold compilation costs before changing the design.
