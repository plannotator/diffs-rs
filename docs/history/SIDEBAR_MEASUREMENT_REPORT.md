# Current-build patch measurements

Date: 2026-09-23. Status: capture and offline audit complete, with resource goals still unmet.

## Outcome

The current ordinary build completed 30 measured processes across five pinned patches.
All 7,350 selected terminal views matched their frozen expectations.
All measured navigation categories and normal exits met the existing development goals.
These observations establish a current-build baseline on this shared workstation, not a performance comparison against an earlier build.

Memory remains an open concern.
The Linux memory workload reached 663.6 MiB during its final tour and 715.8 MiB at its lifetime peak.
A separate Linux latency trial reached 727.0 MiB, the largest peak across the successful campaign.
The Bun memory workload increased across all three tours and ended at 174.1 MiB.

Linux completed-index receipt took 4.48–4.95 seconds across the five latency trials.
Its initial text and colors arrived much earlier.
The resource results do not justify a fixture-specific shortcut, a correctness compromise, or a larger cache allowance.

No viewer source, dependency, cache policy, executable, or original measurement helper changed during this checkpoint.
Only campaign scripts, a memory-tour boundary correction, and documentation changed.

## Build and host

- Executable: `target/release/native-diff`.
- SHA-256: `1ab24c0c70c45a5b480b83e111fa909d33ed56f4f4af9510b30dad72558f2e4e`.
- Source archive: `evidence/local-source-retention-decision-v1/evidence/ordinary-after-source.tar.gz`.
- Archive SHA-256: `c88cf9d5dab949fe332c94bdb7b08c05a842344258148ceba96bcbc921b25fd8`.
- Host: Apple M5 Max, 48 GiB RAM, macOS 26.3 arm64.
- Compiler: Rust 1.96.0, `ac68faa20`, LLVM 22.1.2.

The preceding preparation matched 2,083 runtime, build, vendor, and asset files against the preserved source archive.
The campaign checked frozen identities before and after captures.
The offline audit also checked those identities.

All capture power guards recorded AC power and no sleep events.
The host snapshots recorded no thermal or performance warnings.
Those observations do not prove that throttling never occurred.

The workstation was not idle or dedicated to measurement.
Other processes remained active, including virtualization, development tools, and desktop applications.
Some boundary snapshots showed substantial CPU use outside the viewer.
The packet retains these snapshots and memory statistics.
The results describe this host session, not isolated-machine timing or a guaranteed memory footprint under other host conditions.

## Workload and measurement meanings

Each patch received five fresh-process latency trials and one fresh-process memory trial with three tours.
The preparation report incorrectly stated five memory processes per patch.
The [pre-capture declaration](../../evidence/sidebar-measurement-v1/DECLARATION.md) corrected that prose before measurement, without changing the runner policy.

All latency trials preceded the memory series.
Both series used the fixed order Pierre, Ghostty, Node, Bun, then Linux.
These names identify input patches, not different applications under comparison.
In particular, the Ghostty row does not represent a physical Ghostty capture.

The terminal started at 160 columns by 50 rows, with split layout, dark theme, and the automatic sidebar.
The traces also exercised layout changes, wrapping, navigation, and resizing.
The visible sidebar left 127 diff columns at width 160 and 67 diff columns at width 100.
Input preparation read each patch before capture, so these are warm-file, fresh-process observations.

The timing values measure terminal-output receipt through a PTY:

- Text: correct text in the final diff layout. This is an upper bound on earlier progressive text.
- Colors: correct colors in that diff layout. This milestone excludes sidebar publication.
- Index: absence of the loading indicator after correct final-layout text. This is an observed completion upper bound.
- Complete screen: the settled screen, including the sidebar.
- Interaction: elapsed time from a declared input to the corresponding expected text or colors.

The runner parsed output and compared frozen cells during capture.
Its live checking overhead remains part of the observations.
Expensive source and sequential-color checks ran during preparation, outside the timing path.
Setup traversal remains separate from individual interaction latency.
Setup can warm a later target, so this trace does not establish arbitrary cold, distant coloring latency.

## Opening and interaction results

All values below are milliseconds.
Opening columns contain medians from five trials.
The p95 column contains the largest styled p95 among the five interaction categories, not a pooled percentile.
Each category contains 200 observations per patch: forward, reverse, distant seek, layout, and resize.

| Input patch | Text median | Color median | Index median | Worst category p95 | Worst interaction |
|---|---:|---:|---:|---:|---:|
| Pierre `0800fb` | 10.3 | 44.3 | 50.1 | 43.3 | 85.5 |
| Ghostty `12291` | 8.2 | 38.2 | 44.1 | 21.7 | 73.1 |
| Node `59805` | 8.5 | 33.1 | 144.1 | 14.5 | 52.9 |
| Bun `30412` | 8.5 | 34.4 | 200.6 | 19.9 | 111.2 |
| Linux `v6-v7` | 11.8 | 35.5 | 4,650.5 | 8.6 | 54.1 |

Every opening met the 500 ms text goal and the 1,000 ms color goal.
Linux did not meet the 2,000 ms observed-index goal.
Its five index observations ranged from 4,481.6 to 4,946.7 ms.
This index result does not mean that the first diff remained blank for that interval.

All categories met the 50 ms p95 and 200 ms maximum navigation goals.
All normal exits met the 250 ms goal.
The slowest observed normal exit across both series took 170.6 ms.
These observations do not measure physical paint, frame pacing, or continuous trackpad scrolling.

## Sustained memory

All values below are MiB, where 1 MiB equals 1,048,576 bytes.
Each tour value is the maximum sampled process-group RSS across its settled holds.
Each hold lasted at least one second and contained at least five complete RSS queries.
Queries started at least 100 ms apart under `process-group-rss-v3`.

The memory peak column uses the operating-system lifetime high-water mark for that memory process.
The latency peak column records the largest high-water mark across the five separate latency processes.
These are process-memory observations, not live-heap sizes or cache-size totals.

| Input patch | Tour 1 | Tour 2 | Tour 3 | Memory-process peak | Latency-series peak | Memory policy |
|---|---:|---:|---:|---:|---:|---|
| Pierre `0800fb` | 65.7 | 69.9 | 70.1 | 70.1 | 45.9 | Pass |
| Ghostty `12291` | 59.7 | 66.5 | 68.8 | 68.8 | 57.3 | Pass |
| Node `59805` | 136.1 | 140.5 | 117.9 | 140.5 | 81.1 | Pass, with earlier high holds |
| Bun `30412` | 145.7 | 169.2 | 174.1 | 174.1 | 83.6 | Settled goal unmet |
| Linux `v6-v7` | 712.5 | 660.7 | 663.6 | 715.8 | 727.0 | Settled and peak goals unmet |

The unchanged memory policy checks the third-tour maximum against 128 MiB and the memory-process lifetime peak against 192 MiB.
Node passed this specific policy, but its first two tours contained settled holds greater than 128 MiB.
It did not remain within 128 MiB throughout viewing.

Bun increased across all three tours.
Three tours do not establish a plateau or prove a memory leak.
Linux decreased after its first tour, but remained greater than both uniform goals.
RSS alone does not identify retained owners, allocator slack, mapped pages, or cache contributions.
No before/after memory improvement follows from this campaign.

## Boundary failure and correction

The original Pierre memory attempt completed one tour, then failed the first layout comparison in tour two.
All 45 visible source records and their sequential-color checks passed.
All 887 different cells belonged to the sidebar.
The viewer retained a valid tree scroll position, which the original trace did not reset.
The diff region and current file remained correct.

The [boundary correction](../../evidence/sidebar-measurement-v1/BOUNDARY_CORRECTION.md) appends one public-input setup step to every memory tour.
The sequence is `f`, Home, Tab, `f`, Tab.
It restores the common tree origin, refocuses the current file, and returns focus to the diff.
It does not activate another file, collapse folders, or change the diff reading position.
The existing setup transport also requests a full redraw at widths 159 and 160.

The reset changes the sidebar preference from automatic to explicitly shown.
All five patches contain multiple files, and both preferences use the same minimum-width hiding behavior in this workload.
Each process still opens with the default automatic preference.

Before replacement measurement, five real processes passed 65 boundary views, including 50 exact complete-screen comparisons.
The other views checked deliberately displaced tree positions against source, tree, and color expectations.
Each replacement trace retains every original action and expectation, then appends the same reset.
The complete opening screen remains the reset expectation.

The first short diagnostic used an incorrect displacement assumption and stopped before the proposed reset.
Its failed evidence remains under `memory-v3/`.
The corrected diagnostic and replacement freezes remain under `memory-v4/`.
Neither diagnostic supplies accepted timing or memory values.

The original failed memory attempt remains preserved and does not supply a sustained-memory result.
Its failure cleanup restored terminal state.
The 25 completed latency trials remain unchanged and did not repeat.
Only the memory series resumed with new packets.
No adverse observation disappeared from the evidence.

## Evidence and limits

The [offline audit](../../evidence/sidebar-measurement-v1/audit.json) passed schedule, identity, power, resource-query, hold, milestone, summary, and cleanup checks.
The 25 latency processes supplied 5,125 checked views.
The five completed memory processes supplied 2,225 checked views across 15 tours.
All successful processes exited normally and restored terminal state.

The preceding [corpus preparation](SIDEBAR_CORPUS_REPORT.md) supplied independent source expectations and sequential-color references.
Its corruption controls rejected 272 deliberately damaged cases.
Some preparation views prove visible-file-set consistency, not uniquely observable current-file identity.
The preparation report retains those coverage limits.

Key artifacts:

- [Original captures](../../evidence/sidebar-measurement-v1/capture-001/report.json): 25 completed latency trials and the preserved failed memory attempt.
- [Corrected boundary preparation](../../evidence/sidebar-measurement-v1/memory-v4/report.json): five qualified replacement memory packets.
- [Completed memory captures](../../evidence/sidebar-measurement-v1/capture-002-memory/report.json): one process per patch, with three tours each.
- [Audit source](../../evidence/sidebar-measurement-v1/audit.py) and [audit result](../../evidence/sidebar-measurement-v1/audit.json).

Each capture directory retains compressed observations, summaries, frozen bindings, logs, power records, and host snapshots.
Captures retain the PTY byte count, hash, and bounded tail, not the complete raw stream.
The offline audit checks retained observations, not independent replay of every original PTY byte.
These artifacts contain local paths and host process details and are not sanitized publication bundles.

This campaign covers patch inputs, not complete-source acquisition or expanded-context workloads.
It does not establish physical terminal appearance, platform support, CPU attribution, leak freedom, or a total-memory guarantee.
The earlier correctness suite remains historical evidence for the unchanged build, not a suite rerun during this campaign.

## Next work

1. Compare the retained resource traces with existing ownership evidence before selecting another optimization.
2. Use a narrow profile only where ownership remains unknown, with the same correctness requirements and no fixture-specific behavior.
3. Qualify complete-source workloads separately, including repeated expansion, actual viewport movement, cancellation, and shared-cache revisits.
4. Complete physical terminal checks and release qualification separately from these PTY observations.

The resource goals remain diagnostic signals, not permission to change source content or weaken rendering expectations.
