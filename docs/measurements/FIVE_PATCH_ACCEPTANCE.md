# Five-patch acceptance batch

Date: 2026-09-11. This is the frozen work plan; the separate results report records completion and failures.

## Scope

This batch checks interior presentation and measures the current viewer on all five pinned DiffsHub patches.
It covers the actual executable through a PTY. Actual Ghostty pixels and movement remain a separate visual check.
The existing source bodies, historical traces, controls, palettes, and acceptance targets remain unchanged.

## Tasks and completion

1. Preserve the starting executable and source. Resolve independent source anchors and checkable interior-view contracts.
2. Build a separate sequential syntax reference and a real-PTY runner. Challenge the checker with deliberately corrupted evidence.
3. Run presentation checks and the complete process/source checks. Investigate failures before accepted measurements.
4. Freeze the runner, reference inputs, executable, and public command plan. Run five serial fresh-process repetitions per patch.
5. Run three memory tours per patch, including the fixed file quantiles, structural extremes, both layouts, resizing, and long-line traversal.
6. Investigate demonstrated product failures. Preserve before/after evidence and repeat affected checks without weakening expectations.
7. Record complete results, remaining failures, commands, identities, and evidence locations. Release the measurement host.

Every patch must receive an explicit result. A timeout or wrong view is a failure, not a discarded sample.
An incomplete series cannot receive a passing percentile. A failed target remains failed until a matching rerun passes.
No new reading features, GUI automation, cloud provisioning, maintainer contact, or competing-product benchmark belongs to this batch.

## Independent presentation checks

The independent patch decoder supplies source bytes, side line numbers, hunk boundaries, file metadata, and structural selectors.
The checker must not use the viewer's parser, line-pairing implementation, or emitted diagnostic model as expected content.
For split replacements, both source sequences must remain contiguous and ordered. Shared context must align on both sides.
Explicit small-fixture contracts continue to own exact ambiguous pairing and word-emphasis expectations.
The large-patch checker must state its pairing and emphasis coverage rather than claim complete Pierre equivalence.

The selected views include file-quantile headers, the largest-hunk midpoint and end, the largest-file midpoint, and the longest source line.
Every visible source cell must match its independently identified source segment, with correct side numbers, signs, and continuation markers.
Checks include split/unified layouts, 160×50 and narrower geometry, source-anchor retention, and long-line first/middle/end segments.
Source digests remain a separate complete-content check. Interior views cannot replace them.

Captured rehearsal grids must pass the source checks and sequential color checks before they can become frozen replay expectations.
The replay then requires those exact cells and styles. This gives independent source semantics and exact regression checks.
It does not provide an independently derived answer for every ambiguous line pair or changed-word span.
Background checks enforce added, deleted, and context roles; exact changed-word spans remain the small-fixture contract.

A separate Rust example uses sequential Syntect parsing from each supplied hunk boundary.
It has no viewer cache, continuation, scheduler, projection, or rendering code.
It shares the pinned grammar library and palette translation. It checks asynchronous equivalence, not independent grammar truth.
Both source sides retain their own multiline state. Existing long-line fallback remains explicit and deterministic.

## Measurement boundary

The profile starts at 160×50, split, dark, truecolor, with line numbers and wrapping off.
The input is local and fixed. The host is the current Mac, with other coordinated workloads idle.
No power setting, terminal preference, background application, or operating-system cache is changed automatically.
These are warm-file, fresh-process measurements: identity checks read the complete patch before launch.
Preparation and measurement use separate Python processes. The measured runner does not retain the independent source decoder's full index.

Opening starts before process creation. Interaction starts before actual public input injection.
Correct text and completed visible styling have separate observations.
Visible styling compares the checked body cells without waiting for the rest of the file index.
The runner separately waits for the complete, indexed, styled screen before the next command.
Index completion, cleanup, process peak RSS, and settled third-tour RSS remain separate measurements.
The PTY receipt timestamp is not a Ghostty presentation timestamp.

The new command plan replaces stale numeric-prefix setup with repeated supported keys. Historical command traces remain intact.
Setup travel is recorded separately. It cannot qualify as a cold style request if it already colored the target.
Setup sends bounded bursts of complete public key sequences and drains their intermediate output.
A real one-column resize and restore requests a complete terminal redraw at the destination.
The independent destination check must pass. Setup cost is retained but is not reported as a single-key latency.
Measured categories retain 40 observations per repetition and 50 ms minimum injection spacing.
There is at most one pending measured command. Delayed samples remain in the results.
Five repetitions supply 200 observations per complete category and patch.

The memory tour retains the existing 32 file-quantile selector and structural extremes.
Repeated source selectors remain labeled by their structural role; they are not counted as distinct independent views.
The longest line receives every wrapped page, then first/middle/end unwrapped views reached with public wrap/page controls.
The tour ends by resetting the source, wrap state, horizontal offset, file header, and split layout before it repeats in the same process.
Each checked viewport receives a one-second settled hold. Steady RSS is the maximum during the third tour's holds.
The sampler runs at most 20 times per second. The operating system also supplies the process high-water RSS.
The terminal emulator is not part of this PTY run. Its memory and pixel-presentation latency are unavailable, not zero.
Short-lived temporary allocations, source indexes, and opaque parser state remain outside cache-only guarantees.

## Targets and limits

The targets in [LARGE_DIFFS.md](../design/LARGE_DIFFS.md) remain authoritative.
Navigation targets are p95 at most 50 ms and no response above 200 ms.
First content targets 500 ms. Initial styling targets one second. Cold styling targets two seconds.
Retained styling revisits target 100 ms. Quit and cancellation target 250 ms.
Linux targets 512 MiB steady RSS and 768 MiB peak. Smaller patches target 128 MiB steady and 192 MiB peak.

The 30-second per-state watchdog detects a hang. It is not an acceptable response-time threshold.
Long public setup sequences have a separate bounded deadline and retain their command counts and elapsed cost.
A sampled 6 GiB process-group limit stops runaway local work. It does not redefine the acceptance memory limits.

Detailed run outputs use a temporary evidence directory. The final report separates passed checks, failed targets, and unavailable observations.

## Results, 2026-09-11

The independent interior rehearsal passed for all five patches: 205 unchanged screens per patch, including source, layout, wrapping, resize, navigation, and sequential syntax checks. The deliberately corrupted captures were rejected. Five fresh-process latency repetitions completed for every patch. Pierre has one 251.8 ms styled distant-seek maximum above the 200 ms guard; the other latency guards pass. Three-tour memory runs completed for every patch. Ghostty meets its smaller-patch RSS envelope; Pierre, Node, Bun, and Linux do not.

The authoritative per-patch values and raw evidence paths are in [FIVE_PATCH_RESULTS.md](../history/FIVE_PATCH_RESULTS.md). The batch is therefore complete as measurement work, but it is not a full release acceptance: the recorded memory failures, the Pierre outlier, and actual Ghostty pixel/paint evidence remain open.
