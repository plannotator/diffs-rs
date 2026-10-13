# Original-viewer content baseline

Date: 2026-09-11. All 25 trials completed without a harness error.
This report measures one executable against five patches. The patch names are not competing applications.
The preserved original Rust viewer was measured. The newer disk-backed viewer was not measured or changed during this work.

## Results

Opening means the first complete, correct content grid. Values use five fresh processes per patch and a warm filesystem.
The maximum is the slowest observed repeat, not a predicted bound.

| Patch | Correct opens | Opening median / max | Peak viewer RSS | Quit max |
| --- | ---: | ---: | ---: | ---: |
| Pierre `0800fb` | 5/5 | 29.66 / 30.43 ms | 128.55 MiB | 2.59 ms |
| Ghostty `12291` | 5/5 | 66.26 / 73.29 ms | 67.11 MiB | 1.56 ms |
| Node `59805` | 0/5 | Parse error | 77.67 MiB | 1.36 ms |
| Bun `30412` | 0/5 | Parse error | 49.03 MiB | 1.15 ms |
| Linux `v6.0…v7.0` | 0/5 | Parse error | 2,590.36 MiB | 29.02 ms |

RSS is the maximum OS high-water value across five trials, from process creation through exit.
Failed opens retain their memory costs. Linux used about 2.53 GiB before the original parser rejected the patch.
These are interactive-process peaks, not the earlier parse-only diagnostic or the specified three-tour steady-memory measurement.

Each completed scroll category contains 200 correct, whole-grid observations.

| Patch and direction | Median | p95 | Maximum |
| --- | ---: | ---: | ---: |
| Pierre forward | 3.18 ms | 4.31 ms | 4.83 ms |
| Pierre reverse | 3.96 ms | 6.03 ms | 6.76 ms |
| Ghostty forward | 6.03 ms | 8.84 ms | 9.93 ms |
| Ghostty reverse | 7.06 ms | 9.25 ms | 10.48 ms |

The p95 uses sorted rank 190, separately for each patch and direction.
The four complete categories meet the proposed content-navigation limits. They do not establish full viewer acceptance.

## Failures and missing coverage

The `G`/`g` category failed correctness for both patches that opened.
Each patch had 100 correct end seeks and 100 incorrect start seeks.
The original `g` command selected a file header instead of the first source row. No percentile appears for this incomplete category.

The layout and resize setup requires counted navigation to the largest-hunk midpoint.
The original executable lacks that operation. Its layout and resize categories remain missing, with no substitute anchor.

All five Node trials reported an invalid patch header at line 425835.
All five Bun trials reported the same error at line 4. All five Linux trials reported it at line 19933189.
These are the previously identified original-parser limitations. Earlier correctness work already corrected them in the newer viewer.
An error-screen appearance time is not an opening time.

All 5,000 planned navigation slots remain accounted for: 1,000 correct, 200 known-wrong, and 3,800 not run with explicit reasons.
Every trial received its quit input, exited, restored terminal modes, and emitted cleanup sequences.
The completed viewers returned 0. The parse-error viewers returned 1.

## Measurement integrity

Host: Apple M5 Max, 48 GiB memory, macOS 26.3, arm64.
The run started at `2026-09-11T15:01:58Z`.
The terminal surface was a real 160×50 PTY with pyte reconstruction, not Ghostty.
The profile used split layout, dark theme, truecolor, no wrapping, and visible line numbers.

The runner recorded timestamps immediately after PTY reads, before grid interpretation.
The smallest recorded navigation interval was 50.0035 ms. Only one measured command was pending.
The largest navigation validation duration was 10.83 ms. No overhead was subtracted from observations.
The background RSS sampler remained bounded and reported no errors.

The [frozen contract](../../MEASUREMENT.md) describes the boundaries and reducers.
The read-only method review found five harness defects before retained trials. All five were corrected before the final freeze.
The earlier, unrun freeze remains in `evidence/measurement-preflight/unrun-freeze-v1` for inspection. It is not the measurement freeze.

The real-process checks covered a Git-generated patch, Unicode, tabs, CRLF, newline markers, scrolling, layout changes, and resize.
They also rejected a deliberately wrong content grid and checked byte-split replay of actual terminal output.
A real blocked FIFO covered cancellation, forced cleanup, terminal restoration, and process reaping.
A modified trace failed its hash check before any viewer process started.

Post-run checks found all 25 trials and all 5,000 category slots.
An independent reduction checked rank 190 and the absence of percentiles for incomplete categories.
All frozen hashes still matched after the run, including the unchanged viewer source files.

## Artifacts

- [Frozen inputs and tool hashes](../../fixtures/diffshub/measurement-v1/freeze.json)
- [Resolved commands, source anchors, and expected grids](../../fixtures/diffshub/measurement-v1/trace.json.gz)
- [Machine-readable results and individual samples](../../evidence/content-baseline-v1/results.json)
- [Host and runtime record](../../evidence/content-baseline-v1/environment.json)
- [Evidence checksums](../../evidence/content-baseline-v1/SHA256SUMS)

Each trial also has a JSON record, a bounded ANSI tail, and a final text grid beside the results.
Those tails are diagnostic evidence, not complete recordings or screenshots.

| Artifact | SHA-256 |
| --- | --- |
| Original executable | `52646a55a3635d2df506ab6e3e458b4fca2386f861ead2c0e62c57739308ed78` |
| Final freeze | `b1ed4121a5456ebda6ec59bd01c767a8161636b7bc0ff55a968b7f58d90da909` |
| Resolved trace | `ce8ad25165e18c96c6a204b28f16fbcab1ce8d8e3b3bc4cd70f86ad1390e5a95` |
| Results | `14ad8472edf96d4f4e1321d6fa472082988b141b11f68954a62f279fb98baa22` |

## Next work

The next engine work is source-correct start/end movement, counted navigation, and stable layout/resize anchors.
That work can use the frozen content grids. No missing command receives an invented baseline value.
The newer progressive viewer also needs a prefix-publication opening oracle before its first-content latency can be measured fairly.

Sequential style correctness, style latency, full normalized metadata, long-line tours, steady RSS, and actual Ghostty visuals remain pending.
This baseline does not complete Step A or the full large-diff acceptance specification.
The performance-audit skill supplied the independent method review. The documentation skill kept measured results separate from unmet requirements.
