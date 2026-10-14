# Native-diff: current project state

Updated 2026-10-13. This page owns the current status and next steps.
Historical plans describe their original checkpoints, not additional active tasks.
Dated plans and reports from finished experiments are in [docs/history](docs/history/README.md).

## Standalone repository cleanup, 2026-10-13

The project is now a standalone Git repository with continuous integration on Linux and macOS
(formatting, Clippy with default and all features, the release test suite, and the embedded-host checks).
Linux had not been exercised before; the CI result is the first evidence for it.
These correctness fixes landed after the 2026-10-03 checkpoint below:

- `patch` accepts `git show`, `git log -p`, `git format-patch` (including `-- ` signatures), and `--stat -p` output.
  Commit headers, messages, diffstats, and mail text between file patches are skipped, and source offsets still address the original input.
  Header-only rename, mode, binary, and empty-file patches followed by a commit header are delimited correctly.
  Plain text without a `diff --git` header, traditional `diff -u` output, combined diffs, and incorrect hunk counts remain errors.
  Fixtures in `fixtures/git-output/` come from a scripted real repository and are checked against `git diff-tree` output for the same commits.
- Syntax coloring selects grammars by whole file name before extension in both patch-only and checked-source modes
  (`Makefile`, `Dockerfile`, `Cargo.lock`, `Gemfile`, `.bashrc`, `CMakeLists.txt`, and similar). Whole-name matches are case-sensitive.
- The footer keeps its status readable at 100–160 columns by dropping optional view hints first. The header label no longer runs into the file count, condensed help keeps its indentation, and the small-terminal notice wraps.
- The footer source status describes the cursor's file instead of any file visible on screen.
- Host calls into the syntax worker recover a poisoned mailbox lock instead of panicking; a dead worker surfaces through the existing `Engine::poll` error.
- Unnamed temporary files (patch spool, Git helper output, clipboard input) are created owner-only. On Linux they previously took the umask-derived mode, usually 0644. Linux CI found this.

Canonical source output is byte-identical to the 2026-10-03 build for the five DiffsHub corpus patches and the repository fixtures.
The performance and resource results below were measured on earlier executables and do not transfer to this build.

## Current checkpoint

The integration-ready local engine milestone is complete; its closeout (external record `native-diff-local-engine-milestone/CLOSEOUT.md`, not in this repository) supersedes pending status in its sealed historical reports.
The active milestone is production hardening of the engine and standalone viewer. Its goal (external record `native-diff-production-hardening/GOAL.md`, not in this repository), acceptance matrix (external record `native-diff-production-hardening/ACCEPTANCE.md`, not in this repository), and checkpoint (external record `native-diff-production-hardening/STATE.md`, not in this repository) cover repeated full-source use, memory/responsiveness, standalone search/selection/copy, actual-terminal acceptance, and a new local release candidate. Plannotator TUI/Herdr integration remains proposal-only; no consumer implementation is active.

### Current integrated candidate

Explicit clipboard copy is implemented in the standalone viewer. `y` prepares original source in the existing reading worker, then starts an owned helper after current-request acceptance. The macOS helper publishes plain UTF-8 text, without terminal clipboard sequences or format detection. The visible limit is 16 MiB; Ctrl-S preserves larger or invalid-byte selections. Source gaps, stale requests and incomplete preparation never become partial clipboard output. Interrupted delivery reports that clipboard contents can have changed.

The full release suite passed **215 enabled checks**, including **74 viewer-process checks**. The ordinary process suite excludes ten opt-in checks.
Six host checks passed separately. The other four require pinned corpus or preserved-reference inputs.

Formatting, strict all-target/all-feature application Clippy, and the ordinary build pass. Clipboard-disabled builds pass all 21 binary checks and the two focused process checks.
The existing vendor lifetime warning remains. Native clipboard fidelity, native helper memory, and physical-terminal behavior remain unqualified. No unattended clipboard API ran.

Independent review corrected diagnostic-file growth, prompt cancellation warnings and missing save alternatives. The final review has no required finding. Controlled-child checks cover exact bytes, Unicode boundaries, input limits, failures, noisy diagnostics, cancellation, timeout, source revocation and reaping before reuse. The first test compile and a clipboard-disabled stale-screen test failure remain preserved with their corrections.

The measured ordinary viewer is SHA-256 `125be0d888b98ae656d84f6ba397b787dd6dae73a37fdce587549e5042d85667`.
The measured host is SHA-256 `5f309a219f75ae9aa955eb6f444421c0ac28f3b2244208536dde399eceaa6bae`.
Functional evidence remains in `2026-10-03-clipboard.f6xKbt` under the verification temp root.
The final resource record (external record `native-diff-production-hardening/RESOURCE_RESULTS.md`, not in this repository) covers all four accepted cohorts and their exact identities.

The five-patch cohort completed 30 processes. Largest-patch settled/peak RSS was 542.84/594.33 MiB, and median index readiness was 5.12 seconds.
Node and Bun missed the 128 MiB settled target. Linux missed that target, the 192 MiB peak target, and the two-second indexing target.
Navigation and quit targets passed.

Nine complete-source processes passed 36 views and applicable first-displayed/revisit targets.
The deep-file case was already visible, not a first-displayed destination.

The final host completed 96 stages across three cycles. Its maximum held RSS was 33.98 MiB, and its sampled process-group peak was 42.00 MiB.
Closed RSS medians rose 0.41 MiB across the three cycles. Closed-state descriptor, private-storage, child, and normal cleanup checks passed.

Three reading-tool processes passed three cycles each, with exact search/selection/source and save checks.
Largest-patch input response reached 11.15 ms, but complete scans took 10.97–11.38 seconds. Response feedback does not mean that a scan is complete.
Largest-case final settled median RSS was 397.11 MiB, and sampled peak RSS was 413.41 MiB. These miss memory targets.
The settled medians rose 4.97 MiB across three cycles that retained the last query and domain.

These are finite warm-filesystem PTY observations, not physical paint, instrumented cache-hit counts, instantaneous worker-stop measurements, or indefinite memory guarantees.
The accepted storage tradeoff and adverse results remain recorded. The separate workloads and historical builds do not establish a paired improvement.

Both isolated installations (external record `native-diff-production-hardening/INSTALLATION_RESULTS.md`, not in this repository), with Rust 1.96.0 and Rust 1.88.0, passed 74 viewer and six host checks.
Their executable identities differ from the measured binaries. Their functional checks do not transfer performance results.

The final lockfile contains 194 packages. The fresh advisory/yanked scan found no known vulnerability or yanked warning, but retained two maintenance warnings.
Those warnings concern `bincode` and `yaml-rust`. The final notice inventory covers all 194 locked packages and retains the upstream SDK licensing caveat.
The final package and consolidated native clipboard/physical-terminal acceptance remain pending.
No result establishes legal clearance, absence of vulnerabilities, or a public release.

### Historical development checkpoints

The following paragraphs preserve earlier source states, evidence, and their original limits. They do not add current tasks or transfer measurements between builds.

The bounded patch source index and corrected sequential canonical-export path are accepted for development. The measured viewer is SHA-256 `c761ac796c55938d9598a15bdc881068471dd094a551ec5e109642c9d2d0023f`; its host is `4135d469a8ab49776d063eac9ad3329ebefc43d3746d59b474d2db1bfc7235a6`. These preserved executables predate the new reading backend and are not the final release candidate. The working source and Cargo output have changed; do not transfer the earlier measurements to them.

The preceding prototype reduced the largest parse/retain command's peak RSS but introduced a canonical-export slowdown. A private body buffer improved that path without resolving the regression. A real-file test then confirmed premature per-record output-buffer publication. The current correction preserves the caller's buffering and the CLI's per-file publication boundary. Its 143 enabled release checks, 96 reference grids, two reference streams and all five independent complete-source digests pass.

The corrected ordinary executable completed 30 serial CLI export samples. Largest paired median time was 4.821 seconds before and 5.016 seconds after; median per-process peak RSS during this export command was 128.797/94.016 MiB. Node retained 13.7% median time overhead, while Ghostty was similar and Bun faster. The first candidate invocation's 227.922 ms delay remains unexplained and retained. These warm-filesystem export diagnostics remain part of the accepted storage tradeoff.

Correctness checks passed: existing release tests, real private-index faults, 96 preserved-reference terminal grids, two matching canonical streams, all five complete corpus source digests, corpus endpoint/reflow checks, and six persistent-host contracts. Source offsets share the existing page-cache budget; identities and newline flags retain their meaning. The largest disjoint resident index/metadata accounting is 216.34 MiB versus 361.82 MiB, with 147.24 MiB of logical offsets on private disk and one extra descriptor per patch document. This accounting is not RSS.

The completed sixty-process interactive comparison measured largest settled RSS of 783.938/605.922 MiB and process peaks of 806.891/605.922 MiB. Node settled RSS worsened from 132.297 to 139.953 MiB. All navigation and quit criteria passed. Node/Bun/Linux memory targets and Linux's indexing target still miss. Three accepted repeated-host processes each passed 96 stages and 6,102 source checks; closed live heap and regular-descriptor release were stable within three finite cycles. Eighteen coloring sequences passed exact source/color views and applicable first-displayed/revisit criteria. These are finite shared-workstation PTY observations, not physical paint or universal memory guarantees. The active milestone's ENGINEERING.md records the decision and adverse results.

New source-range streaming, literal matching, logical source order, domain validity, uncached traversal, match delivery and an owned reading worker pass the 181-enabled-check release suite. Search uses exact original bytes and private bounded readers. Raw hit offsets are converted through the shared display sanitizer off the input thread, then revealed with the existing source anchor. Real engine and worker checks cover selected-hit visibility through reflow, stale matches, queued replies, in-flight cancellation/supersession and owner release. A reproduced full-source rollback defect was corrected in the preceding batch by binding reads to immutable snapshot extents.

Standalone literal search and current-match emphasis are implemented. The prompt consumes text and paste before viewer commands. F3/Shift-F3 navigate matches without changing `n`/`N` file navigation. Escape cancels the prompt or active search. Search uses the existing input reader and a lazy owned worker.

The final release suite passed 188 enabled checks, including 66 viewer-process checks. Ten opt-in checks remained excluded. Formatting and strict all-target/all-feature application Clippy pass. The existing vendored lifetime warning remains. Tests cover preserved syntax/change colors, distant Unicode matches through reflow, bounded paste, joined-grapheme editing, incomplete input, scope invalidation, cancellation and cleanup. Independent follow-up source review found no remaining required correction.

The preceding search checkpoint's ordinary viewer is `0d8cde7b3bf9bf1e1bee7ae9f6b1f1cc8ec5841852a9b1b04fe401bd9dff0031`. No new performance claim applies to it. Physical F3/Shift-F3 support remains unqualified. The legacy `CSI 1;2R` modifier sequence conflicts with cursor-position parsing. The tested sequence is `CSI 13;2~`. The 4,096-byte query limit does not bound the event library's whole-paste allocation.

Source-line selection is implemented. `V` begins/fixes a range, Shift-click extends/fixes it, and Escape clears it after higher-priority reading actions or focus. The range retains original file occurrence, side and source lines through reflow. Explicit side/file changes, review replacement and affected checked-source withdrawal clear it. Selection stores metadata, not another source owner. Underlining preserves syntax and change colors. Headers, padding and unavailable gaps do not become source bytes.

The selection batch passed 194 enabled release checks, including 69 viewer-process checks. Ten opt-in checks remained excluded. Formatting, strict application Clippy and the ordinary build pass. Independent source review found no remaining required correction. Narrow range visibility, checked-source footer priority, search notices, exact bytes, focus, mouse, reflow and cleanup have functional evidence. Failed driver attempts remain preserved.

The selection checkpoint's ordinary viewer is `063e471637fe665c5c5485d133ae8f2713f5b094a442b60284db11b74ba95f6d`. No performance or physical-terminal claim transfers to that build.

Byte-faithful save is implemented. Ctrl-S captures the selected original range or current source line, then opens a literal path prompt. One owned worker prepares output privately. The input loop accepts the current request and publishes without overwriting an existing file or symlink. Cancellation, supersession and source invalidation remove private output before publication. Cleanup errors stay visible, including failure after successful publication. At this save-only checkpoint, clipboard copy was unfinished.

The save batch passed 205 enabled release checks, including 72 viewer-process checks. Ten opt-in checks remained excluded. Exact output, owner-only permissions, publication races, cancellation after partial output, stale acceptance, cleanup failure, prompt input, reflow, error recovery and terminal restoration have functional evidence. The first compile error and footer-clipping regression remain preserved with their corrections. Formatting, strict application Clippy and the ordinary build pass. The existing vendor warning remains.

The preceding save-only viewer is `2b4a958c912dd63690bddd90f41d5bc0e61810fb83655de7e5d17b699a1ba040`. No performance or physical-terminal claim applies to it. The clipboard checkpoint above supersedes its next-step status. Final integrated resources, actual-terminal acceptance and local release packaging remain required. No clipboard operation or consumer change occurred during those checks.

### Completed local-engine baseline

The measured ordinary handoff candidate is SHA-256 `6c22fd50c000d867b7177f193287186156997e11644dc3a14fd5843db871808e`; its host example is `fc3281b26bcebca458e1e30c53dfdff20d1ac3d92cf12b1852b0354a566c81ef`. Both are preserved in the milestone's `engine-checkpoint/`.
At that baseline, the normal checkout built viewer `322dbbec7b9e3422728e828a21357e7403de840f63b13e03f1e2242a2607ec6a` and host `10455cc93851f83bb9dc7be7b38cf98095b1f209be4c2e9ed13c5aa07f78e45d`. Those ordinary rebuilds passed explicitly selected contracts but were not separately resource-measured. Source, compiler, configuration and normal/build feature trees matched; no bit-for-bit reproducibility or timing transfer is claimed.
The former normal executable `9350f499bda0f9442234ce856992ceb3756e0a9324302ee2a6dd62ef04dfa630` and its source remain archived for rollback. No diagnostic executable was copied into the development output.

Current source includes the qualified visible-style protection correction.
A real-worker regression demonstrated that FIFO admissions could evict a visible style omitted from the current request, leaving coloring pending without another input.
The correction protects the current visible IDs, preserves FIFO order among evictable styles, and trims after protection is released.
Its logical style bound is `max(32 MiB, visible style charge)`, not a total-memory limit. Protection metadata is separately accounted.
Its initial ordinary suite passed 126 checks and the diagnostic library suite passed 26 checks. The ordinary reference and correction each completed all 279 expected views of a two-tour large-patch replay and passed cleanup. The final integrated candidate then completed the five-patch qualification, including three Node tours without recovery input.
This does not attribute the earlier diagnostic timeout. Two earlier recorder attempts were ineligible and remain preserved, not counted as product failures. The historical cause remains unknown; the declared ordinary completion scope passed.

The tree-cache proposal is closed without adoption; do not resume its old comparison instructions.
The decision (external record `native-diff-cache-decision/DECISION.md`, not in this repository) and durable evidence archive preserve the disposition.
The isolated tree candidate was subsequently checked with Rust 1.88; the previous statement that the compiler was unavailable is historical. The integrated local engine has now also passed its own minimum-compiler installation and contracts.

The new `Engine` owns review orchestration and is shared by the standalone viewer and a bounded-pane host example. Hosts retain terminal modes, input and outer focus. Patch, local-source, coloring, key and signal workers now have owned shutdown. Input errors remain readable/recoverable; cleanup errors and worker panics are preserved. The patch reader uses safe readiness waits and restores stream flags, including cancellation of no-writer FIFOs and partial lines. Interactive stdin must be piped or redirected, not the keyboard TTY. [EMBEDDING.md](EMBEDDING.md) documents source coordinates, lifecycle, descriptor limits and the required downstream Cargo patches.

The integrated ordinary suite passed 135 checks, including 61 process checks, with six opt-in checks excluded. Four real-worker/renderer engine checks cover bounded cells/styles, focus, both sides, wrapped padding, checked context, sidebar and source replacement. The two opt-in real PTY host checks passed separately, covering live-host close/reopen, failed-input recovery, resize, FIFO cancellation and terminal restoration. A separate Cargo root built and ran the engine with exact source endpoint assertions. Independent review prompted cleanup-error aggregation and poison-safe cancellation. The host calls `after_draw()` only after successful terminal output, preserving text-before-work order and delivery-event meaning. Strict all-target/all-feature application Clippy passed; the existing vendored lifetime warning remains.

### Completed baseline qualification and limits

The exact measured viewer completed 30 large-patch slots, including 5,125 checked latency destinations and five three-tour memory runs. It also passed 108 complete-source short views and 146 sustained views. The host completed 18 lifecycle stages, followed by six paired native heap/VM snapshots. These are finite shared-workstation observations, not an indefinite memory plateau or physical paint measurements.

The largest patch peaked at 809.22 MiB RSS and needed 6.6 to 11.5 seconds for full-index receipt. It misses the unchanged 192 MiB peak and two-second index targets; Node, Bun and Linux also miss the 128 MiB settled target. Early text and visible coloring arrive separately. Timed navigation and quit passed the declared goals. See RESOURCE_RESULTS.md (external record `native-diff-local-engine-milestone/RESOURCE_RESULTS.md`, not in this repository) for the complete measurements, target misses, inherited allocator environment and scope limits.

A persistent host retains high RSS after closing large reviews. Native snapshots show identical closed live heap across three cycles while empty allocator regions retain the bulk residency. This supports owner release in that finite schedule, not an RSS-return guarantee. Complete-source tours and patch-host replacement are separately qualified scopes.

Normal Rust 1.96.0 and minimum Rust 1.88.0 isolated installations each passed all 61 viewer and two host contracts on macOS arm64. Acquisition used an initially empty Cargo cache; subsequent builds were offline. Normal checkout builds passed the same contracts. All build/check groups exited, and source/configuration/toolchain postflight passed. Post-restart checks confirmed the unrelated stuck helper absent; no cleanup issue remains from that milestone. Its original cause is unknown.

The exact lockfile advisory/yanked scan reports no known vulnerability and two unmaintained-dependency warnings, with scoped retention rationale. The independently reviewed notice inventory covers all 169 locked metadata packages, three vendored inventories, 66 grammar entries and theme notices. See DEPENDENCIES.md (external record `native-diff-local-engine-milestone/DEPENDENCIES.md`, not in this repository). No public-release, universal security or legal-clearance claim follows.

Combined packaging, selected reference presentation and whole-milestone review are complete and preserved in the sealed handoff. Physical-terminal acceptance remains pending and is required by the new goal. This baseline is a local integration candidate, not a public release or completion of every feature in SPEC.md.

## Historical checkpoints

The sections below preserve earlier executable identities and results. Their normal-build statements describe those dates, not the current binaries above. Follow the current milestone checkpoint for active work.

### Completed executable-selection fix

The process-suite executable-selection fix is complete.
Every current-viewer launch uses one resolver for `NATIVE_DIFF_CONTRACT_BINARY`, with Cargo's executable as the default.
All 61 enabled process checks passed in each of three runs: default selection, normal-build override, and override with the fallback absent.
The missing-fallback control failed without the override, as expected. The isolated fallback was restored with its original hash.
The explicit-reference check matched 96 complete grids and both canonical source streams while the fallback remained absent.
The reference was a preserved copy of the same normal build. This checks selection and grid agreement, not independent rendering correctness.

Formatting, strict application Clippy, and the evidence audit passed.
All 2,085 production files, fixtures, and the normal executable remain unchanged. Test expectations and assertions remain unchanged.
The four opt-in checks stayed excluded from each default run. The compatible-reference check ran separately.
The selection report (temporary verification record `2026-09-27-executable-selection.WcZiAZ/report.html`, not retained) records commands, identities, controls, and limits.
This checkpoint adds no performance, total-memory, or physical-terminal measurement.

### Closed insertion experiment

The public-entry insertion experiment is complete without adoption.
The isolated candidate passed 125 ordinary checks, 141 diagnostic checks, 96 explicit-executable grids, and two canonical source streams.
Its two diagnostic captures passed 1,034 expected views and eight owner snapshots, with clean exits and no sleep interruption.
Neither retained table became smaller. Linux still retained a 48.50 MiB table, and Bun retained a 12.125 MiB table.
The small vector-capacity differences do not establish a memory improvement.
The decision (temporary verification record `2026-09-27-entry-insertion.kPXXk5/DECISION.md`, not retained) closes this experiment without another comparison campaign.

The executable-selection gap identified by that experiment is now closed.
The subsequent lookup review selected a standard tree as an isolated control.
No replacement is adopted. Smaller budgets and periodic shrinking remain out of scope.

### Integrated accounting baseline

The qualified whole-cache accounting change is integrated.
The normal executable is `target/release/native-diff`, with SHA-256 `9350f499bda0f9442234ce856992ceb3756e0a9324302ee2a6dd62ef04dfa630`.
Its source matches the qualified candidate, including the documented test-only correction.
The ordinary suite passed 124 checks, and the diagnostic suite passed 140 checks.
Formatting and strict application Clippy passed.

The normal executable matched 96 complete candidate grids and two canonical source streams.
The process-suite rerun passed 61 checks with its existing mixed executable selection.
Override-aware cases use the normal executable. Other cases use the Cargo test-built executable.
Ten source-transition records passed the integrated storage reader.
Seven real-process diagnostic controls passed.

Two serial owner observations passed 1,034 expected views and eight snapshots, with clean exits and no sleep interruption.
The final requested cache heap was 39.64 MiB for Bun and 75.85 MiB for Linux.
Linux retained a 48.50 MiB table, 23.35 MiB of style vectors, and a 4.00 MiB FIFO.
Its table-capacity high-water mark increased during eviction without a new live-entry peak.
The table allocation stayed unchanged across the three end-of-tour snapshots. Three tours do not establish an indefinite plateau.

These component counts are not process RSS or a memory-saving result.

The integration plan (temporary verification record `2026-09-27-cache-integration.IhX6h6/PLAN.md`, not retained) defines this checkpoint.
The [storage report](docs/history/STYLE_CACHE_STORAGE_REPORT.md#integration-and-bounded-owner-observations) records the completed checks and limitations.

The accounting reports table allocation, retained vector capacity, FIFO capacity, and inline cache storage separately.
The existing style charge, budget, eviction order, invalidation, and source/color behavior remain unchanged.
This is an accounting change, not a memory reduction or total-memory guarantee.
The previous executable and affected files remain preserved for rollback.

The ordinary qualification (temporary verification record `2026-09-27-cache-resume.6qXEpJ/report.html`, not retained) completed all five pairs.
It matched 6,500 corpus views and 78 focused views, with 2,952 settled holds and 62,169 corpus RSS queries.
The isolated Linux candidate peaked at 798.92 MiB and ended at 781.42 MiB.
The reference peaked at 801.88 MiB and ended at 777.72 MiB.
Mixed timing observations, the higher candidate final RSS, different power conditions, and the 9.27-hour gap remain explicit.
Those results describe preserved comparison binaries, not new measurements of the integrated executable.

## Preserved pre-accounting checkpoints

Compact source identities are adopted in the normal build, alongside guarded hidden-source sharing.
Positive context coordinates use standard Rust nonzero values. Identities occupy 16 rather than 24 bytes on the current compiler and target.
All valid coordinate ranges remain available; checked-source correspondence and numeric diagnostics are unchanged.
Checked old/new sources share parsing only for verified identical hidden ranges with the same known grammar and equal parser and scope state.
Each side keeps its own coordinates and checkpoints. Visible lines remain independently colored.
Patch-only dispatch, source bytes, rendering, cache limits, grammars, and dependencies are unchanged.

The normal executable is `target/release/native-diff`.
Its SHA-256 is `53d28a87bd1669910adfc6f28aedb20063ddb9db270310ad056da5a20a3bd838`.
The integrated source passed 124 enabled ordinary checks, including 61 real-terminal process checks, and 44 targeted diagnostic checks.
Four opt-in checks are excluded from the main suite; the compatible-baseline check also ran separately.
Formatting and strict application Clippy passed; the existing vendored Syntect lifetime warning remains.
The final ordinary executable separately passed all 61 enabled terminal-process checks and matched 20,847,451 supplied source records across all five patches.
It also matched 96 complete reference grids and two canonical source streams against the preceding accepted executable.
Its runtime source matches the measured candidate. The previous normal source and executable are preserved for rollback.

The preceding hidden-source-sharing candidate passed 42,657,296 split/unified projection rows and the generated pattern checks.
That earlier paired five-patch qualification passed 6,500 exact views and 2,952 settled holds across 20 eligible processes.
One sleep-interrupted reference remains preserved but is excluded from sustained-resource pairing.
Only that reference was repeated. All other 19 original processes were retained.
Power is recorded without requiring AC. Sleep interruption is a separate eligibility condition.

Three alternating local-source pairs observed deep-coloring medians of 6.46 to 3.20 seconds.
Text-arrival medians increased from 37 to 49 ms. Those values describe the focused workload, not every input.
The local twelve-file sustained check ended near 35.5 MiB for both roles.
That earlier largest-patch comparison reached 792.17 MiB for the reference and 809.55 MiB for the candidate at the highest third-tour settled sample.
That approximately 17 MiB increase is an accepted observation with no established cause, not evidence of a memory improvement.
Node, Bun, and Linux remain above the unchanged memory goals. Linux complete-index receipt also remains above its goal.
These are terminal-output and shared-host process measurements, not physical terminal paint or a total-memory guarantee.
The paired performance values belong to preserved comparison binaries; the integrated executable was correctness-checked, not remeasured in another paired cohort.

The source-span representation experiment remains unadopted. Remote acquisition, broader release qualification, and physical terminal verification remain open.

The retained highlighting census (temporary verification record `2026-09-25-1000-owners.CdqXDf/report.html`, not retained) is complete.
The diagnostic validator accepts both the preceding 24-byte and current 16-byte source identities and checks their key/FIFO accounting.
Seven final real-process controls passed, seven corrupted records were rejected, and four historical record formats remained readable.
Two serial three-tour captures passed 1,034 expected views and eight snapshots. Runtime sources and the normal executable stayed unchanged.

At the last snapshot, Bun retained 11.68 MiB of spare vector capacity and 4.95 MiB of occupied source-key plus FIFO extent.
The largest patch retained 7.21 MiB and 8.60 MiB respectively. Spare vector capacity is already inside the style charge; key/FIFO extent is outside it.
These counts exclude unused map buckets and allocator overhead and do not partition process RSS.
The largest map recorded capacity growth without a new population peak during tour one; its entry and capacity high-water marks did not increase afterward.
Three tours do not establish an indefinite plateau or explain the earlier approximately 17 MiB RSS difference.

The source-identity candidate's initial qualification (temporary verification record `2026-09-25-1331-identities.JPm1Gk/report.html`, not retained) is complete.
Standard nonzero context coordinates reduce each identity from 24 to 16 bytes on the current compiler without narrowing patch/file IDs.
Checked-source correspondence, source bytes, rendering, cache weights, budgets and FIFO policy are unchanged.
The candidate passes 124 enabled ordinary checks, including 61 real-terminal checks, plus 44 targeted diagnostic checks.
It matches all 20,847,451 supplied source records, 96 compatibility grids, 39 current focused grids and 2,050 paired corpus views.
Formatting, application Clippy, seven real-process census controls and the offline evidence audit pass.

The ten ordinary corpus processes give mixed responsiveness observations, not a demonstrated speedup.
For example, largest-patch complete-index receipt increased from 3.812 to 4.044 seconds; Bun opening coloring increased from 36.0 to 41.9 ms.
All observations are retained. One process per role per patch does not establish causation or a repeated regression.
The earlier focused timing packet predates the sidebar. Its reference failure is preserved; fresh focused grids are correctness-only.
The current sidebar-aware corpus protocol supplies the ordinary observations. No matcher or product behavior was weakened.

The sustained-memory qualification and adoption (temporary verification record `2026-09-25-2141-memory.vsW7IR/report.html`, not retained) are complete.
The unchanged five-patch, three-tour schedule passed all ten processes, 4,450 exact views, 2,952 settled holds and 52,372 audited RSS queries.
There were no interrupted or repeated captures. No sleep event occurred; power was recorded without requiring AC.
Every candidate peak and third-tour settled maximum was lower than its paired reference in this cohort.
The largest patch reached 797.56 MiB for the reference and 792.20 MiB for the candidate, both at peak and at the highest third-tour settled sample.
That approximately 5.36 MiB difference is a single-pair observation, not a demonstrated repeated or causal RSS saving.
The `pierre-0800fb` candidate's unusually large late residency decrease is unexplained and is not credited to the representation change.
Node, Bun, and Linux still miss the unchanged memory goals. Three tours do not establish an indefinite plateau or total-memory guarantee.

The adoption decision (temporary verification record `2026-09-25-2141-memory.vsW7IR/DECISION.md`, not retained) preserves all paired observations and accepted timing tradeoffs.
Only the reviewed representation and test adaptations were integrated. Cache admission, budgets, weights and eviction remain unchanged.
The normal build was restored after Cargo's test build replaced its executable, then checked directly. Both binaries and the failed identity-check log remain preserved.
The memory measurements belong to the preserved candidate, not a new measurement of the integrated executable.
The [diagnostic invalidation accounting](docs/history/CACHE_ACTIVITY_ACCOUNTING.md) is qualified.
Activity schema 2 counts source-driven removals separately from budget eviction and reconciles retained entries, payload, and usable map capacity.
Real source replacement, revocation, recovery, stale replies, unrelated-file survivors, theme reset, and owner release passed.
The ordinary suite, compatible-baseline grids, and seven real-process census controls also passed; the normal executable is unchanged.
Historical activity records retain their original scope. This change does not alter renderer invalidation, cache policy, or the completed patch-only measurements.

The bounded complete-source owner observation (temporary verification record `2026-09-26-full-source-owners.cG8lk4/REPORT.md`, not retained) is complete without a production change.
The 12,000-line file pair and twelve-file Git review passed 58 source/color-checked views, 29 ordinary/diagnostic grid pairs, eight owner snapshots, and four clean exits.
On revisit, styles occupied 20,712 and 119,000 charged bytes; compiled inner-engine estimates were 6.59 and 8.50 MiB. These are partial owner measurements, not process-memory partitions.
Ordinary held RSS was about 29.42 and 33.45 MiB, unchanged between the final deep view and revisit in each short run. Diagnostic overhead and a diagnostic-only RSS increase remain explicit.
The file pair's opening already colored its deep patch sections; later jumps were cached. These bounded complete-source observations do not replace the large patch-only memory evidence or establish a long-term plateau.
That observation did not include shared page occupancy, geometry, or live parser storage. No additional cache policy or representation change was selected.

The [retained-owner extension](docs/history/RETAINED_OWNER_ACCOUNTING.md) now measures those owners and current checkpoints in opt-in builds only.
It passed 124 ordinary checks, 43 targeted diagnostic checks, 96 compatible terminal grids, two canonical source streams, seven real-process census controls, and 11 malformed-record checks.
Repeated observations preserve owner addresses, reference counts, cache contents, and terminal cells in the tested cases. The normal executable remains unchanged.
The new bounded observation (temporary verification record `2026-09-26-retained-owners.6F99J4/REPORT.md`, not retained) is partial.
The paired 12,000-line file case passed nine matching grids and four owner snapshots. It retained 132,456 page-payload bytes, 10,846 geometry-payload bytes, 1,168 live parser/highlighter heap bytes, and a 31,784-byte checkpoint charge at revisit.
These small owner counts do not explain large-patch memory and do not justify a new cache policy.
The ordinary twelve-file Git review passed 20 views, but its paired owner observation remains incomplete after two unavailable startup RSS queries across the two attempts.
All successful processes and both failed captures are preserved. No further capture retry was made.

The settled capture repair and remaining Git observation (temporary verification record `2026-09-26-ready-rss.RXOm0G/REPORT.md`, not retained) now complete that bounded diagnostic checkpoint.
The failing ps rows described exiting process-group members with unavailable RSS. The strict parser remains unchanged.
An explicit harness option collects RSS only during independently validated settled holds; it does not measure or protect startup/transition memory and makes no peak or timing claim.
Real-process controls passed for the default sampler and the new mode, including 102 navigation views, safety rejection, terminal restoration, and cleanup without starting a sampler.
One diagnostic Git capture passed all 20 grids against the previously saved ordinary process, four owner snapshots, 36 contained queries, native inspection, and clean exit.
No completed ordinary or file-pair process was repeated. Runtime source and both executable hashes remain unchanged.
The final Git snapshot retained 94,958 page-payload bytes, 55,177 geometry-payload bytes, 1,636 live parser/highlighter heap bytes, and a 31,340-byte checkpoint charge.
Those owner counts and the existing style/engine counts stayed unchanged on revisit. Diagnostic RSS rose by 655,360 bytes; that difference remains unattributed, not a demonstrated leak or an optimization result.
Power was battery for this capture. The earlier ordinary process is a grid control, not a matched memory or timing comparison.

The [current allocation attribution](docs/history/CURRENT_ALLOCATION_ATTRIBUTION.md) is complete without a runtime change.
One current diagnostic Bun process passed 418 exact views, four owner snapshots, twelve native commands, 1,233 audited queries, and clean exit.
Direct color-output and matched style-table/FIFO classes account for 77.23% of its first-to-third-tour native heap increase.
The final style vectors retained 11.41 MiB of spare capacity, already included in their charge. This is not a process-memory saving.
Pages stayed unchanged across tours. Geometry, checkpoints, and live parser state remained small in this workload.
The grammar/outer-regex review did not justify a storage change or more instrumentation.
The [direct token collection prototype](docs/history/DIRECT_TOKEN_COLLECTION_REPORT.md) is implemented in an isolated copy, without token merging, boxed storage, or cache-policy changes.
It passes 126 ordinary checks, 45 targeted diagnostic checks, 96 compatible terminal grids, and all 20,847,451 supplied source records.
The preserved ordinary candidate separately passes all 61 enabled terminal-process checks.
Exact token and syntax-state comparisons pass across 2,358 source-line advances. Recorded capacities are smaller for 2,142 visible lines, equal for 104, and larger for none.
The Bun owner pair passes 836 exact views and eight snapshots. Final style charge is about 32.00 MiB for the reference and 27.06 MiB for the prototype, with the same map-capacity high-water mark.
Those are diagnostic cache counts, not a process-memory or speed improvement claim.
The incomplete Linux candidate capture was interrupted after discovering an outer 20-minute deadline shorter than the previously completed trace.
Its partial evidence and successful cleanup are preserved; the Linux reference did not start.
The corrected resume wrapper passes seven real-process controls. The resumed Linux pair now passes 1,232 exact views and eight owner snapshots, with clean exits and reconciled accounting.
The prototype retains more Linux style entries within the same charge, with the same map/FIFO capacity high-water behavior in that three-tour pair.

The ordinary qualification (temporary verification record `2026-09-26-token-qualification.HUqf24/report.html`, not retained) is complete.
Thirty ordinary processes passed 6,500 corpus views, 78 focused views, 2,952 settled holds, and 59,004 audited corpus RSS queries. No comparison capture was repeated or excluded, and no sleep event occurred.
The focused capture schema now accepts the existing boolean sidebar state in an isolated helper; negative controls and a separate unmeasured reference control passed. The product's runtime source did not change.
Navigation and focused coloring remained close in the observed pairs. Linux index receipt was 3.749 versus 4.033 seconds, and its latency-process sampled peak increased from 710.12 to 765.12 MiB.
All five prototype sustained-process peaks were lower, including Linux at 800.56 versus 770.30 MiB. That does not establish a causal saving.
Linux's third-tour settled maximum was 800.50 versus 663.56 MiB, but its final held sample was 634.70 versus 663.56 MiB. Bun's final held sample was also higher for the prototype, despite its lower process peak.
Both roles contain substantial residency declines. Host-wide VM statistics do not attribute them to viewer allocation release or compression. Those decreases are not credited to the prototype.
The [qualification report](docs/history/DIRECT_TOKEN_COLLECTION_REPORT.md) preserves every patch, both directions of the memory observations, and the bounded follow-up scope.
The [bounded residency investigation](docs/history/DIRECT_TOKEN_COLLECTION_REPORT.md#completed-bounded-residency-investigation) is complete.
One small control and two Linux diagnostic processes passed 414 expected views, six owner snapshots, twelve native commands, twelve holds, and 2,531 audited RSS queries.
The prototype's final native heap was 646.25 MiB versus 624.46 MiB, despite less vector slack.
Its style-table capacity high-water mark was 458,752 versus 229,376. The larger table arose during insertion and eviction without a new entry-count peak.
Other measured final page, geometry, parser/checkpoint, and engine owners matched. This supports a container-growth mechanism, not complete attribution of earlier ordinary RSS.
Direct token collection is closed without adoption. All evidence remains preserved, and the normal executable remains unchanged.

The [whole style-cache accounting checkpoint](docs/history/STYLE_CACHE_STORAGE_REPORT.md) is complete in an isolated copy.
It reports table allocation, retained vector capacity, FIFO capacity, and observed-build inline storage without overlap.
The existing style charge and retention policy remain unchanged. This is accounting, not a memory reduction or a total-cache limit.
The prototype passed 124 ordinary checks, 140 diagnostic checks, 96 preserved-viewer grids, and two canonical source streams.
Ten real source transitions, seven diagnostic controls, 19 storage-corruption cases, 40 activity-corruption cases, and four historical formats passed.
Independent review corrected a test-only capacity assumption. Both setup failures and their cleanup evidence remain preserved.
Formatting and application Clippy passed. Ordinary performance and Rust 1.88 qualification remain pending before adoption.
The normal source and executable remain unchanged.

No measurement or diagnostic is currently active. See [Next work](#next-work) for the remaining priorities.

## Earlier checkpoints

This section preserves prior results and build identities. Its measurements do not describe the current executable.

The [Git-completion notification](docs/history/LOCAL_SOURCE_GIT_WAKE_REPORT.md) is implemented independently of checkpoint retention.
It uses the existing stderr reader to wake the child-status check, while preserving cancellation and the actual exit-status requirement.
At that checkpoint, the normal suite passed 118 checks, with 18 diagnostic opening grids and 198 ordinary action grids.
Git and Rust-source opening improved in every ordinary pair. Large-source opening and later actions remained mixed.
This is a Git-readiness improvement, not a general coloring or CPU speedup.

The [request-aware retention decision](LOCAL_SOURCE_RETENTION_DECISION.md) is complete and adopted in the normal build.
That suite passed 120 checks. Its diagnostic passed 72 exact grids, the ordinary comparison 198, and the normal rebuild 33.
Both comparison roles use the same Git wait. Cache limits remain unchanged.
Large-source recoloring medians improve from 523 to 147 ms, then 537 to 22 ms, in the isolated ordinary comparison.
Smaller actions remain mixed. Rust-source opening and several navigation actions were slower, and short-run memory did not improve.
Those observations remain accepted tradeoffs, not claims of a universal speedup or memory reduction.
Integrated local-source responsiveness and sustained-resource qualification remain separate from the patch-only corpus campaign.
No further retention-policy experiment is selected.

The [sidebar-aware corpus qualification](docs/history/SIDEBAR_CORPUS_REPORT.md) prepared ten current-build workload packets across the five pinned patches.
All 1,765 selected workload views passed source and sequential-color checks, with 272 rejected corruption cases.
The separate 15-view geometry trace and its 15-view frozen-matcher replay also passed.
The normal executable and runtime source remain unchanged. These preparation results are separate from the measurements below.

The [current-build patch measurements](docs/history/SIDEBAR_MEASUREMENT_REPORT.md) are complete: 30 successful processes and 7,350 checked views.
All measured navigation categories, initial text/color goals, and normal exits passed.
Linux completed-index receipt took a median 4.65 seconds and did not meet the 2-second goal.
The final memory tours reached 174.1 MiB for Bun and 663.6 MiB for Linux.
Linux peaked at 715.8 MiB in the memory workload and 727.0 MiB in a separate latency trial.
The unchanged uniform memory goals remain unmet for Bun and Linux.
The offline audit passed identities, schedules, power, resource queries, summaries, and cleanup.
These are shared-workstation PTY observations, not physical-paint measurements or an isolated-host comparison.
One preserved memory failure exposed a test-only sidebar reset omission. A qualified public-input reset corrected all five memory traces.
No viewer code changed, and none of the 25 completed latency trials repeated.

The [current ownership investigation](docs/history/CURRENT_MEMORY_OWNERS_REPORT.md) is complete without a runtime change.
Current document accounting totals 361.82 MiB for Linux and 9.36 MiB for Bun.
One native Bun profile passed all 418 expected views and clean shutdown.
Live allocations increased by 30.45 MiB between tours one and three.
Direct color-token output and matched style-cache container classes account for 79.62% of that diagnostic increase.
Those instrumented values do not partition the earlier ordinary RSS observations.
The [source-span representation census](docs/history/SOURCE_SPAN_CENSUS_REPORT.md) is complete without a production change.
All 20,847,451 corpus records reconcile with preserved counts and storage extents.
The conditional model saves 35.05 MiB of accounted storage on Linux after descriptor costs, but adds 75,238 allocations.
These are modeled extents, not a heap, RSS, or performance improvement. Some small fixtures become larger.
The [isolated exact-length storage prototype](docs/history/SOURCE_SPAN_PROTOTYPE_REPORT.md) now passes correctness qualification without a production change.
Its source suite passed 122 checks. The preserved ordinary release matched 96 compatibility grids and all 1,770 corpus workload views.
All 20,847,451 source records and 42,657,296 split/unified projection rows matched independent expectations.
Actual retained storage matches the census model, including descriptor costs and small-file increases.
The [ordinary-build comparison](evidence/source-span-prototype-v1/COMPARISON_REPORT.md) is complete: 60 eligible processes and 14,700 exact terminal views passed the offline audit.
All 62 attempts remain intact, including two power-ineligible attempts. The original 48 eligible processes did not repeat.
The prototype remains unadopted. Linux settled RSS increased from 679.7 to 728.4 MiB in the third tour.
Its memory-process peak increased from 704.2 to 748.2 MiB. Every candidate memory tour had higher RSS than the reference.
The Bun candidate also had slower opening colors and index receipt in all five pairs.
Linux index receipt improved in four pairs, and its largest latency-process peak decreased from 717.2 to 676.5 MiB.
All navigation and normal-exit goals passed. Node, Bun, and Linux missed the settled-memory goal in both roles.
Those observations do not establish the cause of the memory difference or a general improvement.
The proposed follow-up separated live allocations and their owners from allocator-held pages and other resident regions.
The existing first Linux tour already contains the adverse difference. The Bun opening difference also remains unresolved.
That experiment left the ordinary source and executable unchanged.

The changed-file sidebar is implemented against [its specification](SIDEBAR_SPEC.md).
It retains the exact patch directory hierarchy, with folder expansion, counts, keyboard controls, and mouse controls.
The [sidebar report](SIDEBAR_REPORT.md) preserves that feature's original build identity, checks, and limits.
The patch-only performance campaign is complete. Historical executables and evidence remain unchanged.

The [exact-source coloring and expandable context](EXACT_SOURCE_CONTEXT_SPEC.md) feature is implemented for local inputs.
Its specification covers stable source versions, source correspondence, memory limits, Rust structure, controls, and acceptance gates.
The [input scenario analysis](INPUT_SCENARIO_ANALYSIS.md) expands the proposed contract beyond local commits and files.
It names GitHub PR acquisition as the next adapter milestone, with explicit comparison identity, completeness, and per-side source availability.
The [storage/reference groundwork](SOURCE_STORAGE_REPORT.md) is implemented and checked against the preserved sidebar executable.
The [local source input checkpoint](LOCAL_SOURCE_INPUT_REPORT.md) implements captured two-file and fixed two-commit comparisons.
The [complete-context coloring checkpoint](FULL_SOURCE_COLOR_REPORT.md) uses those checked sources for independent old/new syntax replay.
The [context expansion checkpoint](CONTEXT_EXPANSION_REPORT.md) adds incremental `Show 20` actions for checked local sources.
The [source-readiness correction](docs/history/SOURCE_READINESS_REPORT.md) keeps the selected source visible as lazy source replies insert gap rows.
The [local-source hardening pass](docs/history/LOCAL_SOURCE_HARDENING_REPORT.md) reviews ownership and failure recovery without changing runtime code.
It adds input-worker shutdown checks and updates dependency maintenance notes. The prepared executable remains unchanged.
The [short local-source timing checkpoint](docs/history/LOCAL_SOURCE_TIMING_REPORT.md) passed ten AC-powered runs and all 110 expected views.
It measures two modest local-source workloads, not sustained scrolling or the five-patch memory campaign.
The [coloring investigation](docs/history/LOCAL_SOURCE_COLORING_REPORT.md) attributes the two-file delay mainly to hidden-source parsing.
Its diagnostic and ordinary sampled processes passed all 33 final views. No runtime source changed.
The [saved-state reuse checkpoint](docs/history/LOCAL_SOURCE_REPLAY_REPORT.md) implements compatible checkpoint reuse across hidden gaps and theme changes.
Exact colors and source identities pass the new work-count and replacement checks.
The [completed ordinary comparison](docs/history/LOCAL_SOURCE_REPLAY_COMPARISON.md) retains all eighteen runs across the power interruption, with 198 exact views.
It finds workload-dependent recoloring benefits and mixed navigation results. The large local-source case still repeats all hidden parsing after checkpoint eviction.
The [retention investigation](docs/history/LOCAL_SOURCE_RETENTION_REPORT.md) now reproduces how replay evicts states that the same request can reuse later.
Its isolated diagnostic passed 27 measured worker requests, 520 old/new token lists, and 33 exact terminal views.
That investigation did not change the normal implementation or executable.
The [original prototype](docs/history/LOCAL_SOURCE_RETENTION_PROTOTYPE.md) preserves the first experiment and its adverse observations.
The [adoption report](LOCAL_SOURCE_RETENTION_DECISION.md) adds matched attribution and the explicit tradeoff decision without increasing cache limits.
Isolated expansion now has separate evidence. Cached layout and resize actions issue no coloring requests in the targeted diagnostic.
That rules out direct checkpoint planning in those actions, not every indirect or host effect.
The [updated retention plan](docs/history/LOCAL_SOURCE_RETENTION_PLAN.md) closes the decision and identifies the next qualification work.
Remote acquisition remains unimplemented. Integrated resource and physical-terminal qualification remain open.

## Preserved retention build

The preceding ordinary-build SHA-256 is `1ab24c0c70c45a5b480b83e111fa909d33ed56f4f4af9510b30dad72558f2e4e`.
It is preserved as the rollback executable, not the current default.
Patch storage now uses an append-only writer, shared read handles, and a cache that can serve multiple backing stores.
Store-scoped source identities keep geometry entries separate without changing original patch record numbers.
That normal suite passed 120 checks, including 61 terminal-process checks, three Git-child checks, and two new retention-worker checks.
Four opt-in checks remain excluded.
The earlier retention prototype passed 136 combined diagnostic-feature checks before the separate Git-wait change.
The combined suite was not repeated for this integration. Matched diagnostic builds passed 80 baseline and 83 candidate selected checks.
Those selected suites include all 61 enabled terminal-process checks and the candidate's parser-work regression.
The current normal rebuild passed 33 complete reference grids across all three local-source inputs, including offline replay and cleanup checks.
Its preserved executable, source-equivalence result, logs, and captures are in `evidence/local-source-retention-decision-v1/evidence/`.
Paired retention timings belong to the source-identical isolated candidate, not a second paired series for the normal rebuild.
One incomplete attempt stopped on an unavailable child RSS reading. The packet retains it and all preceding completed runs.
The resumed schedule used unchanged binaries, capture rules, and expected views.
The preserved source-readiness executable separately passed the endpoint contract and matched 96 complete preserved-build grids and two canonical streams at that checkpoint.
The preceding expansion build checked all five large patches against independent expectations for 20,847,451 supplied source records.
That full corpus check was not repeated for the position-only correction; patch storage and parsing are unchanged.
The new modes capture immutable source bytes, keep line indexes on disk, and check every supplied and omitted range through EOF.
Git source preparation is lazy and cancellable. Source failure preserves the patch view.
Both modes now use complete-source syntax after source checks pass. Pending or unavailable sources use plain syntax.
Replay includes hidden prefixes and keeps each side's parser state across patch boundaries.
Source origins, explicit side scopes, and theme epochs prevent incompatible checkpoint and style reuse.
The [readiness report](docs/history/SOURCE_READINESS_REPORT.md) records the current correction and qualification limits.
The source-readiness executable and archive remain preserved under `evidence/source-readiness-v1/` as historical evidence.
That checkpoint's final checks are `suite.log`, `diagnostic-suite.log`, `ordinary-pty.log`, and `compatibility.log`.
The later test/documentation-only checkpoint is `evidence/local-source-hardening-v1/`, including its new `suite.log` and ownership report.
The preceding source-readiness executable also passed 146 checked local-source views and 22 corrected timing-reference views.
The endpoint and revealed-line expectations are independent of the saved screens.
The short local-source timing series is accepted within its declared scope.
Current patch-only measurements are complete. Sustained-resource qualification for complete-source inputs remains open.
The [expansion report](CONTEXT_EXPANSION_REPORT.md) and `evidence/context-expansion-v1/` preserve the preceding feature checkpoint and corpus results.
The [coloring report](FULL_SOURCE_COLOR_REPORT.md) preserves the preceding coloring-only checkpoint.
The [local-source report](LOCAL_SOURCE_INPUT_REPORT.md) preserves the preceding input-only checkpoint.
The preceding [storage report](SOURCE_STORAGE_REPORT.md) retains its own build identity and checks.

The preserved sidebar reference remains `evidence/sidebar-v1/native-diff`.
Its SHA-256 remains `b06a89565cc4f9e1b466cbf2a2d57d70bbe6cbf0f75051de1039a136e0c7f8f3`.
The tree appears automatically for multiple files when space permits.
The `f` key opens Files. File activation uses the original patch entry, not the sorted tree position.
The existing source-position reflow handles changes in available diff width.
Resize notifications also force a complete repaint after a coalesced shrink-and-grow round trip.

The sidebar adds file and directory metadata, not source copies or another highlighting worker.
The completed corpus campaign measures the full viewer with the sidebar. It does not isolate the sidebar cost.
Physical terminal inspection remains separate from the emitted-cell previews.
Application integrations remain deferred.

## Preserved pre-sidebar baseline

The required-byte highlighting optimization, cached-view cancellation, and punctuation-aware inline emphasis are integrated and qualified.
The last pre-sidebar change corrected inconsistent inline matches between replacement sides.
Both sides now use the same old-to-new comparison, with side-specific insertion and deletion spans.
The preceding change improves alignment-gap contrast through one color expression in `src/theme.rs`.
Source text, alignment, wrapping, and navigation remain unchanged.
The accepted storage changes remain present. Deferred experiments stay outside the active build.

Normal Cargo commands build the accepted viewer from the repository root:

```sh
cargo build --release --locked
cargo test --release --locked -- --test-threads=1
```

The preserved pre-sidebar executable is `evidence/sidebar-v1/baseline-native-diff`.
Its checked SHA-256 is `6cf039592f9bc400ee9e90ea510eb4b2daa48ebc24ae1b75371d7d6cc16e2eb3`.
The [paired-emphasis report](evidence/inline-pair-v1/REPORT.md) records the preceding defect, correction, and qualification.
The [gap report](docs/history/ALIGNMENT_GAPS.md) records that change, checks, and incomplete physical captures.
The [inline report](docs/history/INLINE_HIGHLIGHTING.md) records the preceding inline-emphasis change.
The [cancellation report](evidence/cached-navigation-cancel-v1/REPORT.md) preserves the preceding source archive, executable, and rollback copies.
The [preceding integration report](evidence/required-bytes-adoption-v1/REPORT.md) remains intact.
Normal builds exclude diagnostic features. Optional diagnostic tests use a separate Cargo target directory.
No special source snapshot or manual executable replacement is required.
The pre-sidebar large-patch correctness recapture and measurement-build qualification are complete.
No accepted performance capture is active or scheduled.

## What works

- Patch files and piped patches, with completed files available while later input loads.
- Captured two-file and fixed-commit comparisons, with complete-context coloring after exact source checks.
- Incremental unchanged context through click or Enter, with stable source positions and unchanged original patch records.
- A changed-file tree with exact directory levels, counts, statuses, and stable file activation.
- Split/unified layouts, dark/light themes, wrapping, line numbers, syntax colors, and changed-word emphasis.
- Line/page movement, first/last source movement, file/hunk jumps, vertical wheel scrolling, and bounded keyboard horizontal scrolling.
- Source, active side, and text-position preservation through resize, wrapping, and layout changes in the covered checks.
- Partial-load errors, literal display of covered unsafe text, cancellation, and terminal restoration on covered exit paths.

Exact source bytes use a private disk spool. Compact indexes remain in memory.
The renderer draws visible cells. A separate worker supplies styling.
Geometry, source, and style caches have separate limits, not a total-memory guarantee.
The [README](README.md) owns build commands, controls, and implementation details.

The new rejection check skips a syntax search only when a required ASCII byte is absent from the complete source line.
Otherwise, the existing matcher runs unchanged.
That optimization does not change rule priority, captures, old/new parser state, source display, or cache budgets.

Cached destinations now cancel obsolete coloring through the existing mailbox.
Already queued same-theme replies, cached styles, compatible continuation, and checkpoints remain usable.
Cancellation remains cooperative and does not interrupt an individual regex instruction.

## Preserved pre-sidebar correctness evidence

The pre-sidebar normal suite passed 70 checks, including all 44 enabled terminal-process contracts.
The pre-sidebar diagnostic-feature suite passed 87 checks, including 44 terminal-process contracts.
It used a separate Cargo target directory.
Formatting and application Clippy passed. A pre-existing lifetime warning remains in vendored Syntect.

Alignment-gap checks cover absent source rows, unused wrapped rows, and real blank source lines in both themes.
Width changes and split/unified changes retain the correct source backgrounds without stale gap cells.
The before/after emitted grids differ only in gap backgrounds for both multi-file views.

Inline checks cover exact punctuation, number, identifier, whitespace, combining-mark, and emoji spans on both source sides.
The original position fixture retains unchanged numeric suffixes in both layouts and themes.
Real-process checks also cover unequal wrapping, resize, CJK characters, combining marks, and tabs.
Whole-line colors, source text, line pairing, geometry, syntax behavior, and long-line fallback remain unchanged.

The earlier cancellation regression first failed against unchanged production source, then passed after its fix.
It uses the real renderer, cache, parser, and worker with an in-memory terminal grid.
The real-worker checks cover seven queue scenarios, including cancellation, a superseded replacement, repeated cancellation, and resumed work.
Queued replies remain correct. The canceled request releases its source before worker shutdown.
Post-cancellation revisits match sequential colors for divergent old/new state in both themes.

The final ordinary executable passed all 12 focused rendering contracts through `NATIVE_DIFF_CONTRACT_BINARY`.
The broader suites used their Cargo test-build executables.
The [preceding report](evidence/cached-navigation-cancel-v1/REPORT.md) records the earlier cancellation qualification and its environment-preflight correction.

The earlier integration also passed ten complete sequential-color comparisons and independent checks of 20,847,451 source records.
It matched 42,657,296 projection rows, 2,888 generated accepted cases, and 716 generated rejected cases.
Its standalone dependency checks and five-patch endpoint check passed.
These broader checks were not repeated for the scheduling or inline changes. Their tested parser and renderer sources remain unchanged.
The ordinary suites still exclude the five-patch endpoint check and historical pre-storage comparison.

These checks provide strong evidence for the covered behavior.
They do not prove every possible patch or grammar, ideal replacement pairing, physical terminal pixels, or frame pacing.

## Historical performance evidence and accepted tradeoff

The [ordinary comparison](evidence/required-bytes-comparison-v1/REPORT.md) measured the candidate source before integration.
The preserved source-readiness executable has [short local-source timing observations](docs/history/LOCAL_SOURCE_TIMING_REPORT.md).
Its two-file opening medians are 46.99 ms for expected text and 581.30 ms for complete cells.
Its fixed-commit Git opening medians are 132.09 ms and 172.05 ms, respectively.
Its theme changes in the two-file case take about 510 ms to reach complete cells.
The subsequent diagnostic attributes 96–99% of those opening/theme request durations to hidden-source parser calls.
An ordinary stack sample also locates active work in parsing and regex matching.
Those diagnostic values do not replace the accepted ordinary timings or establish an improvement.
The series passed 110 final-grid checks, all cleanup checks, and 120 AC-power boundary checks.
Its Page Up/Down actions move only the selection, so they do not establish scrolling performance.
The preserved sidebar executable has [five-patch timing and sustained-memory observations](docs/history/SIDEBAR_MEASUREMENT_REPORT.md).
That campaign does not extend the short local-source series into sustained complete-source qualification.
The [saved-state reuse comparison](docs/history/LOCAL_SOURCE_REPLAY_COMPARISON.md) completed all eighteen declared runs and 198 exact views.
The interrupted attempt remains excluded and preserved. Eight resumed runs retain the original schedule, executables, inputs, and references.
Offline replay and RSS checks pass. The series spans two shared-workstation sessions, with mixed opening and navigation observations.
Real-source theme medians improve from approximately 53–55 ms to 21 ms, but no general timing improvement is accepted.
The 12,000-line case still needs about half a second to recolor. A diagnostic records 23,922 parsed lines per theme request, unchanged from before.
Its 64-entry checkpoint limit triggers 212 evictions over the sequence. The Git diagnostic reduces theme parsing from 1,522 to 998 lines without eviction.
All 33 new diagnostic views match. Short-workload peak RSS medians remain approximately 17–34 MiB, not results for the largest patch.
The separate real-worker count check reduced repeated sparse parsing from 11,266 to 2,056 source lines while preserving exact tokens.
The remaining evidence in this section applies to preserved pre-sidebar builds.
The [measurement-readiness checkpoint](docs/history/MEASUREMENT_READINESS.md) checked workload identities and an isolated build/install without accepting performance samples.
The 39 focused views now have reviewed expectations for the accepted gap and inline changes.
The pre-sidebar executable passed those views and saved-output replay.
Two matched ordinary comparison builds also passed all 39 views against their separate presentation answers.
The large-patch inline review is complete after the paired-emphasis correction.
All 842 sampled paired sides pass independent checks, and all 724 expected views retain exact source and syntax colors.
The corrected ordinary executable passed all 724 actual large-patch views against those reviewed expectations.
The fresh ordinary measurement candidate also passed all 12 rendering contracts and 39 focused views.
Its compiler, SDK, environment, Cargo configuration, and feature tree match the prepared reference.
The [historical comparison plan](evidence/inline-pair-v1/NEXT.md) scoped those preserved schedules. No capture of that paired comparison started.
The current unpaired campaign does not complete that historical comparison. The earlier battery checkpoint retains its original evidence.
The table describes preserved executables before the cached-view cancellation change.

| Earlier preserved measurement | Reference | Candidate |
| --- | ---: | ---: |
| Deep first-visit coloring median, three pairs | 1,494.09 ms | 751.69 ms |
| Largest-input peak RSS, one process per role | 736.75 MiB | 670.45 MiB |
| Largest-input third-tour settled maximum | 736.75 MiB | 608.20 MiB |
| Ghostty-patch third-tour settled maximum | 59.39 MiB | 70.97 MiB |

All three focused pairs favored the candidate for deep first-visit coloring.
Small adverse timing observations remain in the report. The five-patch memory campaign passed all 4,324 views and 2,856 holds.
These values do not establish physical paint time, universal improvement, or a repeated process-memory reduction.

The [ordinary allocator checkpoint](evidence/required-bytes-ordinary-owners-v1/REPORT.md) reproduced the moderate-patch memory increase.
The [endpoint census](evidence/required-bytes-endpoint-owners-v1/REPORT.md) then observed a comparable state without stack logging or intermediate inspection.
The candidate retained 24,537 styles versus 7,829, with 8.69 MiB more style payload and 8.22 MiB more total used heap.
Both caches stayed below the unchanged 32 MiB payload budget, with no evictions in the observed cache instances.
Static-pattern sets, compiled-pattern sets, and measured engine inventories matched.

The larger completed-style population explains the direction and scale, not every byte of the earlier ordinary processes.
The integration accepts this cache-occupancy tradeoff for the repeated coloring benefit.
It does not establish an indefinite plateau or a total-memory limit.
The development memory goals remain unchanged. The three largest inputs still exceed those goals in the preserved campaign.

## Historical behavior and presentation work

The [cancellation review](evidence/highlight-cancellation-v1/REPORT.md) identified obsolete coloring at fully cached destinations.
The [completed change](evidence/cached-navigation-cancel-v1/REPORT.md) resolves that scheduling defect in the covered checks.
Its CPU, cache-occupancy, and process-memory effects remain unmeasured.
Less offscreen work can also leave fewer styles available for a later revisit.
The [next checkpoint](evidence/cached-navigation-cancel-v1/NEXT.md) scopes an ordinary-build observation without another scheduling change.

The user reported an inline-emphasis discrepancy in the visual gallery. The [inline change](docs/history/INLINE_HIGHLIGHTING.md) corrects the covered punctuation cases.
The [alignment-gap change](docs/history/ALIGNMENT_GAPS.md) makes empty alignment areas distinct from context rows in both themes.
Actual Ghostty light-theme captures show the complete multi-file view before and after the change.
A dark capture has a framing discrepancy. The recorder lost window visibility before the wrapped captures.
Those physical checks remain incomplete. The light multi-file gallery image uses the preceding alignment-gap executable.
No physical gallery capture uses the latest paired-emphasis executable yet.
At this checkpoint, physical comparison, rapid real-window resizing, and trackpad observations were incomplete.
The [prepared terminal check](TERMINAL_PRESENTATION_CHECK.md) remains postponed.
No GUI automation or capture request is active.

Very long lines retain complete source but use declared plain styling.
Missing syntax context stays unknown. Uneven replacements retain the bounded alignment and positional fallback.
At this historical checkpoint, literal search was implemented. Selection/copy, a file picker, and horizontal trackpad scrolling were unimplemented.
The [product spec](SPEC.md) describes intended behavior. The [release checklist](ACCEPTANCE.md) is not complete.

## Deferred work stays separate

| Candidate | Decision and location |
| --- | --- |
| Shared leading-syntax state | Deferred. The exact removed files are in [the saved experiment](evidence/clean-baseline-v1/before/deferred-source/). The [original report](docs/history/SHARED_SYNTAX_PREFIX_REPORT.md) and all measurements remain intact. |
| Borrowed loading records | Deferred. The [native diagnostic](evidence/borrowed-parser-residency-v1/REPORT.md) did not resolve the earlier ordinary-memory and exit observations. |
| Borrowed capture results | Closed without adoption. The [review](evidence/borrowed-capture-review-v1/REPORT.md) preserves the mixed timing benefit and adverse memory observations. |
| Unchanged regex save slots | Closed without adoption. The [comparison](evidence/redundant-save-comparison-v1/REPORT.md) preserves all three focused pairs and the mixed result. |
| Earlier cache representations and direct input buffer | Not adopted. The [saved-style report](docs/history/STYLE_STORAGE_REPORT.md), [boxed-value report](docs/history/STYLE_INDIRECTION_REPORT.md), and [input-buffer report](evidence/input-buffer-v1/REPORT.md) preserve their decisions. |
| Direct token collection | Closed without adoption. The [qualification and residency report](docs/history/DIRECT_TOKEN_COLLECTION_REPORT.md) records reduced vector slack, mixed ordinary memory, and a larger cache table with higher final live heap. |

The complete pre-cleanup working source, test support, documents, and executable remain in `evidence/clean-baseline-v1/before/`.
The separate experiment copies are evidence, not active build inputs.
No historical trial needs a restart. Archived drivers retain their original assumptions and results.

## Historical next work at the preceding milestone

1. Finish the selected same-content reference presentation checks. Preserve terminal-cell evidence separately from browser pixels and physical-terminal acceptance.
2. Seal the measured ordinary binaries, current source, exact notices, recipes, qualification evidence and rollback into a durable handoff. Keep normal development and installation binary identities separately labeled.
3. Obtain the independent whole-milestone verdict and close owned-process cleanup. Correctness, lifecycle, resource and installation campaigns are complete; do not repeat successful captures for better numbers.
4. Leave one consolidated physical-terminal packet pending for human inspection. PTY cells do not establish paint timing, fonts or trackpad smoothness.

That milestone excluded search and copy. The current production-hardening goal includes them, as specified at the start of this page. Remote acquisition, comments, consumer integration and horizontal trackpad features remain outside the current goal.

Preserve completed comparisons and adverse observations. Do not repeat historical trials to seek better numbers.
New measurements record power and host conditions without requiring AC; interrupted captures require separate eligibility review.
The observation-method repair and direct-token investigation are complete. Direct token collection is closed without adoption.
The whole-cache accounting change is integrated after qualification. The insertion experiment is complete without adoption.

The earlier isolated builds, 152-package inventory, 61-contract installations, and 169-package notice inventory remain historical evidence.
The current installation record covers 74 viewer and six host checks with each compiler. The final notice inventory covers the 194-package lockfile.
The [release-readiness checkpoint](RELEASE_READINESS.md) separates local qualification from public-release gates.

This directory is not a Git checkout.
Source archives and hashes preserve the accepted state and preceding builds.
No repository initialization, public release, or global installation occurred.
