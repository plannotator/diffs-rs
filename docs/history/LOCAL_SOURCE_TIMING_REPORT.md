# Short local-source timing checkpoint

Date: 2026-09-22. Ordinary executable: `c3b337c3549efab30d7fee3f5089d4f9af733a143e28c091cbbd45a21b462074`.

All ten prepared runs passed. All 110 final views matched their frozen text and cell styles.
All ten processes exited with restored terminal settings and no remaining private spool files.
The 120 power checks recorded AC power. No trial failed or required a retry.

This checkpoint accepts descriptive timing observations for two modest local-source workloads.
It does not complete scrolling, sustained-memory, physical-terminal, or release qualification.
No application source, dependency source, executable, fixture, capture driver, or frozen expectation changed.

## What the measurements show

The two-file comparison supplied its expected opening text after a median 46.99 ms.
The complete opening view, including colors and status, arrived after 581.30 ms.
Theme changes also took about 510 ms to reach the complete expected view.
Text therefore arrives before the final colors. The measurements do not identify the cause of that delay.

The Git comparison supplied its expected opening text after a median 132.09 ms.
Its complete opening view arrived after 172.05 ms.
The jump to the last file supplied its expected text after 39.44 ms and complete cells after 65.42 ms.

Context expansion reached complete cells within 4.55 ms across these ten processes.
The isolated width changes reached complete cells within 4.57 ms.
These actions occurred after the preceding view settled. They do not establish responsiveness during active coloring or rapid resizing.

The Page Up/Down actions changed only the selection marker in these frozen views.
The source rows stayed in the same screen positions. These observations are not measurements of viewport scrolling.
The two-file endpoint was also visible and colored before the endpoint action.
Its endpoint result is not a cold first-visit coloring measurement.

## Workloads and conditions

| Item | Declared or recorded condition |
| --- | --- |
| Two-file case (`deep-files`) | Two TypeScript files, 12,000 lines per side, three separated replacements |
| Git case (`multi-git`) | Two fixed commits, 12 changed files, 800 lines per side per file, TypeScript/Rust/C/Python |
| Source behavior | Checked full sources, divergent old/new multiline syntax, incremental context expansion |
| Display | Split initially, sidebar disabled, 160×50 cells, `xterm-256color`, truecolor |
| Layout actions | Split to unified, 160→100→160 columns, height unchanged |
| Theme actions | Dark to light, then light to dark |
| Expansion | `gkk` then Enter reveals 20 leading lines and selects new-side line 77 |
| Endpoint | One `G` selects line 11963 in the two-file case or line 763 of the final Git file |
| Trials | Five fresh processes per case, serial runs, alternating case order, warm filesystem |
| Action spacing | Eleven two-second windows per process |
| Host | Apple M5 Max, 48 GiB RAM, macOS 26.3 build 25D2125 |
| Analysis environment | Python 3.12.13, pyte 0.8.2 |
| Power | AC at every recorded boundary, normal power mode, no reported thermal warning before or after capture |

The Mac retained its normal desktop activity. No other viewer, Cargo build, or measurement job appeared in the process preflight.
This was not an isolated laboratory host. The capture also used the existing RSS safety sampler.
The sampler stopped cleanly in every process. Its samples are not sustained-memory results in this packet.

The frozen screens passed independent source, color, and selected-destination checks before this capture.
The [source-readiness report](SOURCE_READINESS_REPORT.md) records those checks and the rejected earlier reference views.
This series used only the corrected reference. It did not reuse earlier timing values.

## Measurement definitions

**Text** means the first complete synchronized frame whose body characters match the expected view.
The comparison includes gutters and selection markers, but excludes the title and footer.
It is not the first appearance of any text. For theme changes, the expected body text is unchanged.

**Complete cells** means the first complete synchronized frame that matches the entire expected view.
The comparison includes text, colors, other cell styles, dimensions, title, and footer.
This is a presentation milestone, not an isolated measure of syntax-engine CPU time.

Opening starts before the process fork. Other actions start immediately before public input or the terminal resize operation.
Both milestones end at the receipt of the bytes that complete the matching frame.
The reader can receive multiple frames in one packet, which gives them the same receipt time.
These observations include delivery and reader scheduling. They do not measure terminal pixels, display refresh, or perceived smoothness.

The capture stores raw receipts before expensive screen checks. Those checks run after the viewer and RSS sampler stop.
Every action also needs the correct final grid at the end of its two-second window.
The driver rejects late output between action windows, partial frames at boundaries, failed cleanup, and lost AC power.

## Results

Each cell shows the median and the full observed range in milliseconds: `median [minimum–maximum]`.
Each row has five samples. These are descriptive values, not percentiles, confidence intervals, or comparative speedup claims.

### Two-file comparison

| Action | Text, ms | Complete cells, ms |
| --- | ---: | ---: |
| Open | 46.99 [33.65–51.86] | 581.30 [555.16–586.05] |
| Expand 20 lines | 2.25 [1.49–2.72] | 3.22 [2.23–4.00] |
| Page Down, selection only | 0.39 [0.25–0.58] | 0.39 [0.25–0.58] |
| Page Up, selection only | 0.35 [0.24–0.62] | 0.35 [0.24–0.62] |
| Last source, already visible | 0.26 [0.23–0.67] | 0.26 [0.23–0.67] |
| First revealed source | 0.30 [0.24–0.48] | 0.30 [0.24–0.48] |
| Unified layout | 0.96 [0.84–1.63] | 0.96 [0.84–1.63] |
| Narrow to 100 columns | 1.35 [1.14–2.27] | 1.35 [1.14–2.27] |
| Widen to 160 columns | 1.74 [1.60–2.96] | 1.74 [1.60–2.96] |
| Light theme | 1.79 [1.70–3.22] | 509.90 [497.39–525.44] |
| Dark theme | 2.11 [1.95–3.65] | 514.79 [510.29–526.13] |

Quit to observed process exit: **1.01 [0.98–1.44] ms**.
The quit measurement excludes the preceding sampler shutdown.

### Fixed-commit Git comparison

| Action | Text, ms | Complete cells, ms |
| --- | ---: | ---: |
| Open | 132.09 [119.24–141.58] | 172.05 [157.78–180.66] |
| Expand 20 lines | 2.63 [1.67–3.20] | 3.47 [2.36–4.55] |
| Page Down, selection only | 0.32 [0.25–0.43] | 0.32 [0.25–0.43] |
| Page Up, selection only | 0.33 [0.24–0.56] | 0.33 [0.24–0.56] |
| Last file and last supplied source | 39.44 [27.96–62.96] | 65.42 [58.42–93.29] |
| First revealed source | 2.07 [2.04–3.26] | 2.07 [2.04–3.26] |
| Unified layout | 1.61 [1.45–2.69] | 1.61 [1.45–2.69] |
| Narrow to 100 columns | 3.17 [1.89–4.19] | 3.17 [1.89–4.19] |
| Widen to 160 columns | 3.34 [2.44–4.56] | 3.34 [2.44–4.56] |
| Light theme | 4.71 [2.49–4.82] | 38.92 [27.09–40.24] |
| Dark theme | 5.00 [2.73–5.63] | 39.39 [27.05–41.52] |

Quit to observed process exit: **1.47 [1.30–2.11] ms**.

## Evidence and limits

The raw series is under `evidence/source-readiness-v1/qualification/timing-captures/`.
Each trial retains `capture.json`, `receipts.json`, and `validated.json`.
The declaration records the executable, reference, driver, operating system, and power settings.

The summary and preflight records are under `evidence/local-source-timing-v1/`.
`summary.json` retains every sample and its input hashes. `summarize.py` checks the complete serial series before aggregation.
The offline check also establishes all 120 AC records, all cleanup outcomes, and unchanged executable and reference identities.
All 278 preceding readiness files and all 24 hardening files passed checksum checks before and after capture.
The prior evidence remains unchanged. The new manifest also covers the raw capture files.

The run did not exercise the sidebar, wrapping, sustained scrolling, rapid input, cold filesystem pages, or cache pressure.
It did not measure the five large patches or compare against another executable.
All actions waited for the preceding two-second window. No claim about input responsiveness during coloring follows from this schedule.
The Git fixture contains four languages, but this short trace does not visit every file.

## Next work

1. Inspect the opening and theme-change coloring path with a narrowly scoped diagnostic.
   Separate source acquisition, source replay, matching, and style delivery before choosing any optimization.
   Keep the ordinary measurements separate from diagnostic costs. Retain exact old/new syntax checks.
2. Prepare a source-aware scrolling and resource trace whose expected views prove actual viewport movement.
   Include repeated context expansion, revisits, source-cache pressure, and input during active coloring.
   Freeze its destinations before capture. Keep the completed short series unchanged.
3. Complete physical-terminal inspection and broader resource qualification before remote acquisition or release acceptance.

This checkpoint adds measurements and documentation. It does not introduce an optimization or weaken any correctness requirement.
