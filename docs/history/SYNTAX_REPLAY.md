# Bounded syntax replay batch

Date: 2026-09-11. Status: continuation batch complete. Cold-seek, full performance, and visual acceptance remain open.

## Contract

The worker must preserve separate old/new syntax state for the exact supplied patch.
Missing context remains unknown. A distant seek must parse its supplied predecessors before it reports completed syntax.
Line pairing, source bytes, inline emphasis, themes, navigation, and the existing long-line fallback remain unchanged.

## Selected implementation

The worker retains one movable continuation for one hunk and theme generation.
It owns two parser states and two highlighting states, as the existing active replay does.
A forward request in the same hunk resumes at the next unprocessed source record.
A backward request, different hunk, or changed theme starts from that hunk's beginning.
Cancellation preserves only the fully processed prefix. It never advances the continuation past an unprocessed record.

Off-screen replay applies syntax-scope operations without generating colors or token vectors.
The first requested line rebuilds color state from its accumulated scope stack. Consecutive requested lines reuse that color state.
The worker continues to use Syntect's parser, scope stack, and highlighting implementation.
Requested source IDs select their hunk directly through its ordered source ranges.

This batch adds no cloned parser snapshots, checkpoint collection, dependency, thread, or public control.
Syntect 5.3 exposes resumable states but does not expose their owned heap sizes.
Its parser stacks remain variable-size active state. The existing 64 MiB disposable-cache target does not become a total-memory guarantee.
Sparse multi-checkpoint retention remains deferred until its byte accounting is supportable.

## Evidence

The preserved executable defines the before-build, not the correctness oracle.
A new diagnostic uses the existing 12,000-line TypeScript fixture and independently expected terminal cells and colors.
It measures opening, a cold end seek, a reverse viewport, a theme change, and an uncached forward viewport.
The theme change clears the rendered-style cache before the forward step exposes new lines.
The reverse step remains visible even if it still requires a full replay.

Both builds receive one excluded warm-up and three measured runs in balanced serial order.
Each command waits for its exact destination and completed styling. Content and styling have separate timestamps.
The report includes all samples, peak RSS, executable/fixture/script hashes, and terminal cleanup.
These are local PTY diagnostics, not Ghostty display timing or corpus-wide performance acceptance.

New real-process tests cover deep old/new comment and string states, hunk/theme isolation, and long-line fallback across a continuation.
Existing tests cover burst input, cancellation, source fidelity, wrapping, geometry eviction, and terminal restoration.
The complete release-process suite and all five independent source digests remain correctness gates.

The measurement host was reserved for this batch. No benchmark overlap, provisioning, maintainer contact, or legacy DiffGrid change is authorized.

## Results

Host: Apple M5 Max, 48 GiB RAM, macOS 26.3 (25D2125), connected to AC power.
Both builds received one excluded warm-up and three measured fresh-process runs with warm files.
The measured order was before, after, after, before, before, after.

| Measure | Before, median of 3 | After, median of 3 |
| --- | ---: | ---: |
| Opening to correct text | 7.5 ms | 6.9 ms |
| Opening to completed styling | 147.9 ms | 146.2 ms |
| Cold end seek to completed styling | 1,734.0 ms | 1,646.1 ms |
| Reverse movement to completed styling | 1,741.8 ms | 1,653.1 ms |
| Theme change to completed styling | 1,739.4 ms | 1,663.6 ms |
| Uncached forward movement to completed styling | 1,749.3 ms | 65.8 ms |
| Largest process peak across 3 runs | 65.3 MiB | 64.9 MiB |

The forward step exposes 46 new source records after a theme change cleared earlier rendered styles.
It therefore checks resumed parsing, not a previously colored viewport.
Changed-build forward styling samples range from 63.5 to 68.6 ms. Previous-build samples range from 1,730.3 to 1,752.9 ms.
Cold and reverse styling improve only modestly. They still take approximately 1.65 seconds on this fixture.
Correct text arrives before styling: all changed-build navigation samples range from 0.9 to 5.8 ms.
Opening and process peaks show no material regression in these samples. Three runs cannot establish tail latency or a memory guarantee.

Every measured destination matched independent source cells and the required completed keyword colors.
All six measured processes restored terminal modes, mouse reporting, the alternate screen, and the cursor.
Changed-build exit observations range from 1.8 to 2.0 ms.
The complete release suite passes 31 real-process checks. All five supplied-source digests still match their independent references.
The new syntax contracts also pass on the preserved executable. They establish expected behavior, not a performance threshold.
Formatting and Clippy with warnings denied pass.

The first diagnostic preflight used an incorrect reverse-viewport expectation. It failed and remains in the evidence directory.
The corrected trace follows the existing backward-navigation rule: the cursor becomes the first visible source row.
That diagnostic correction preceded the accepted before/after runs. Preflights and warm-ups are outside the reported medians.

Evidence directory: `/var/folders/xg/f67qq94s1qsbxl_nlh9hd_xc0000gn/T/pe-verify/native-diff/2026-09-11-2200/`.
It contains source packages, both executables, individual samples, the run script, checksums, test logs, and the verification report.
The unchanged historical trace and original benchmark results remain separate.

Before executable: `01836a549b8cd6e1b4798d8ea0a49faab5b0122226b11c4627ff9fa9cb2f87e4`.
After executable: `d78884cb328909322caad55b7ed7b12063eb24b25ded60ecf32e9422665477f9`.
Diagnostic script: `b2496923ff524dc3a0076a667cf8d07f2b2074b8755b6fded361f67fb590714e`.

## Remaining work

Cold and reverse seeks still parse supplied predecessors. A checkpoint collection needs an explicit accounting decision before implementation.
Independent large-corpus interior styling and the three-tour retained-memory check remain acceptance work.
This continuation does not prove a total-memory bound for opaque parser stacks.
Actual Ghostty presentation, movement, and same-content Pierre comparison remain open. No GUI control or screen capture occurred in this batch.
