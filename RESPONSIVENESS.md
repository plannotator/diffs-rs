# Reading responsiveness: bounded implementation batch

Date: 2026-09-11. Status: bounded geometry batch complete. Full performance and visual acceptance remain open.

## Contract

This batch improves an existing reading path. It does not add a new navigation system.
Source bytes, source order, old/new syntax state, emphasis, and line numbers must remain correct.
Resizing, wrapping, and layout changes must preserve the active source, side, and text position.
Existing controls, themes, long-line styling limits, and patch-only context warnings remain unchanged.

## Sequence

1. Preserve the current executable and source package.
2. Measure a deep single-hunk seek and a 360,000-character line through a real PTY.
3. Select the largest demonstrated blocking cost before changing performance-sensitive code.
4. Implement one bounded optimization with explicit cache ownership and eviction behavior.
5. Repeat the same measurements with the preserved and changed executables.
6. Run independent rendering contracts and the complete real-process suite.

The local Mac is reserved for these serial measurements.
No competing benchmark, package installation, or network download runs during measurements.
The diagnostic trace is separate from the historical frozen benchmark.

## Diagnostic workloads

The deep-hunk fixture contains 12,000 numbered TypeScript context lines and one final addition.
The trace opens the patch, reaches its end, returns to its beginning, and revisits its end.
Content and completed syntax have separate milestones. Expected lines come from the fixture definition, not the viewer.

The long-line fixture replaces one ASCII line with another line of the same length.
Each line contains 36,000 numbered ten-character blocks. The trace enables wrapping and changes the terminal width repeatedly.
Every visible source cell must match the expected slice. A frame with the wrong destination cannot supply a successful timing.
Tabs, Unicode, unequal pane widths, and reading-position preservation remain separate correctness cases.

Measurements use 160×50 cells initially, split layout, dark theme, and truecolor.
The clock starts before process creation or command injection. It stops when the PTY receives the required complete synchronized update.
Each measured command follows the previous completed state. No sleep duration becomes application latency.
Results include individual samples, executable and fixture hashes, peak RSS, and terminal cleanup.
These local diagnostic samples do not establish population tail latency or actual Ghostty presentation latency.

## Implementation boundaries

The selected optimization must reuse immutable source identities and preserve exact display-text byte positions.
A new cache must have a fixed payload budget and a deterministic eviction policy.
Eviction must change cost only. A cache miss must produce the same source cells and styling.
Cache payload accounting is not a total-memory guarantee. Peak and retained RSS remain separate observations.
No speculative background work, general cache framework, new runtime, or new dependency belongs in this batch.

The other measured bottleneck remains explicit if it needs a separate implementation batch.
Broad performance acceptance, the full memory tour, and actual Ghostty/Pierre pixel comparison remain open.

## Selected change

The initial diagnostic found approximately 710–725 ms of blocked display time per long-line resize.
The deep seek displayed correct text in approximately 1 ms, then completed syntax in approximately 1.9 seconds.
This batch therefore changes display geometry first. Syntax replay remains a separate worker optimization.

The application owns a FIFO cache keyed by immutable source ID and wrap width (or unwrapped mode).
Each entry retains safe display text, sparse grapheme boundaries, and wrapped-row boundaries.
Rendering starts near the requested column instead of scanning from the beginning for each visible row.
Each width requires one layout calculation per retained entry. Theme and side changes do not invalidate immutable text geometry.
The cache retains at most 16 MiB of payload and 1,024 entries. Oversized entries remain usable but are not retained.
The payload estimate includes owned text bytes, boundary capacities, and entry structure. Allocator and map overhead remain outside that estimate.
Independent tests must cross these limits and revisit earlier content without test-only cache settings.

## Completion evidence

The temporary evidence package contains the before/after executables, source packages, diagnostics, and test logs.
Meaningful new tests use the release executable and actual PTY input, output, resizing, and terminal cleanup.
Independent expected cells cover difficult text boundaries and cache revisits.
The complete source-corpus checks must still match all five pinned patches.
The report must distinguish completed checks from deferred work and unsupported observations.

## Results

Host: Apple M5 Max, 18 logical CPUs, 48 GiB RAM, macOS 26.3 (25D2125), Rust 1.96.0, Python 3.12.13.
Each executable had one excluded warm-up and three measured fresh-process runs with warm files.
The measured build order was before, after, after, before, before, after. Both diagnostic workloads ran in each process pair.

| Long-line measure | Before | After |
| --- | ---: | ---: |
| Open to correct content, median of 3 | 7.4 ms | 14.8 ms |
| Enable wrapping, median of 3 | 733.9 ms | 8.9 ms |
| First visit to a width, median of 9 | 733.9 ms | 10.6 ms |
| Retained width, median of 15 | 724.5 ms | 3.1 ms |
| Largest process peak across 3 runs | 16.3 MiB | 20.3 MiB |

Every measured source grid matched the independent expected cells. Both workloads also produced identical complete PTY-byte hashes across all six measured runs.
All measured quits restored the terminal modes, mouse mode, alternate screen, and cursor. Changed-build exit observations ranged from 1.0 to 2.1 ms.

On this long-line fixture, median opening time increases by approximately 7 ms and observed peak memory also increases.
That tradeoff removes repeated 700–800 ms display stalls without changes to source content or controls.
The copied changed executable's first warm-up took 229 ms to first content. That sample remains in the evidence and outside the repeat-run medians.
These results describe warm-file development diagnostics, not cold-machine startup or Ghostty display latency.

The deep-hunk seek still needs work. Correct text arrived in approximately 1 ms, but completed syntax took a median of 1.83 seconds.
The corresponding previous-build syntax median was 1.80 seconds. This batch makes no syntax-performance improvement claim.
The changed-build peak for that fixture reached 66.7 MiB. These short runs do not establish sustained-memory behavior on the Linux corpus.

The release suite passes all 29 real-process checks. All five complete source digests match their pinned independent expectations.
Formatting and Clippy with warnings denied also pass. No full acceptance milestone follows from this batch.

Evidence directory: `/var/folders/xg/f67qq94s1qsbxl_nlh9hd_xc0000gn/T/pe-verify/native-diff/2026-09-11-2131/`.
It contains the source packages, executables, raw diagnostic samples, exact run order, cell previews, test logs, and report.

Before executable: `b31c24c983e0f9cbc5fb61b8dbecc61bd0918d0488b0304adf7727cfc82e3012`.
After executable: `01836a549b8cd6e1b4798d8ea0a49faab5b0122226b11c4627ff9fa9cb2f87e4`.
Diagnostic script: `3685dd93a1c79c95cffaf354025fa583de36fbb591b6543e29067052b6beeb4c`.
The dependency lockfile and historical measurement freeze remain unchanged.
