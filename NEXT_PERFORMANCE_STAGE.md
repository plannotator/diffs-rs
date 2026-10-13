# Next stage: memory ownership, coloring, and terminal presentation

Historical stage specification and execution notes. Use the [current project state](CURRENT_STATE.md)
for the active next steps and the [latest checkpoint](INNER_ENGINE_CHECKPOINT_REPORT.md)
for current results. The original scope and acceptance gates below remain preserved.

Next, after the 2026-09-13 [compiled-pattern census](REGEX_CENSUS_REPORT.md):
design sharing within one syntax set and test its actual ownership cost. The
diagnostic found 264 extra compiled roots for identical static patterns across
loaded grammars, with all 412 checked views passing. That establishes an
opportunity, not recoverable bytes. Preserve lazy compilation, matching actions,
capture independence, source-derived delimiters, cloning/serialization, and
release. Address concurrent initialization without a process-global registry.
Run a declared allocation pair before advancing to ordinary performance checks.

The diagnostic is off by default. No production memory optimization was made
in this stage; no new accepted responsiveness or sustained-RSS series exists.
Existing targets, regressions, and the unpromoted status remain. Older next-step
notes below are preserved as history, not additional work currently running.

Next, 2026-09-13: use the now-valid memory observations to identify the next
application owner, not to tune toward a number. The shared RSS collector and its
real-process checks are complete. The separate five-patch memory-only cohort
passes all three tours, frozen views, strict holds, normal cleanup, and offline
audit. All 1,428 holds have at least eight valid samples. No Rust source changed.
The [observer report](RSS_SAMPLING_REPORT.md) records the protocol and full results;
the earlier [sampling failure](evidence/source-page-v1/MEMORY_CAPTURE_FAILURE.md)
remains failed and preserved.

Four memory goals remain unmet. The largest patch reaches 879.41 MiB in settled
viewing, then drops to a lower third-tour maximum of 703.36 MiB. That drop has no
owner attribution and is not evidence of an application improvement. First review
the existing allocation snapshots against this residency pattern. Scope any new
profile around one unanswered ownership question before changing application code.
Do not resize caches or repeat measurements to chase the lower value.

The combined candidate remains unpromoted. The older latency evidence keeps its
original observer protocol; no new latency series was run. Coloring delays and
actual terminal-paint verification remain open. No process from this batch remains
running, and no additional measurement or application change has started.

## Historical execution notes

Current next step: the [source-page ownership proposal](SOURCE_PAGE_PLAN.md).
The bounded [capture-first attribution](CAPTURE_ATTRIBUTION.md) is complete and
identifies no new application fix. Close that small-difference investigation;
do not repeat it to obtain a preferred number. The working candidate remains
unaccepted and earlier timing regressions are not relabeled as noise. The page
task is scoped, not started. It targets an independently observed owner and must
measure its own delta without implying promotion of preceding changes. Broader
syntax sharing remains conditional on actual live duplication. Historical
execution updates below are preserved; this paragraph owns the next action.

Latest follow-up: [capture before validation](CAPTURE_DELIVERY.md) is complete.
The source and executables are unchanged. Forty visual sessions pass all seven
captured actions and disk replay. The capture-first continuous-end comparison is
63.27 versus 61.59 ms; a smaller gap under this distinct cadence is not complete
attribution or acceptance. Next is a bounded alignment of capture-first receipts
with internal request/parser/publication work. Keep the combined candidate
unaccepted; do not begin a new memory-owner change from these results alone.
There is no new sustained-memory checkpoint or running measurement.

Latest follow-up: [delivery capacity and observer attribution](DELIVERY_CAPACITY.md)
is complete. The working source contains the compact candidate plus a verified
consumption-notified reply path, but the combined candidate remains unaccepted:
the ordinary continuous-end regression persists. Next is bounded capture/validation
separation before additional millisecond-level tuning. The prior execution
updates below retain their historical measurements; no new sustained-memory
checkpoint was run for the delivery build.

Date: 2026-09-12. Execution update: B0, M1, and C1 are complete. The first M2 source-run representation change and the C2 bounded-checkpoint experiment are implemented, verified, and retained. The full 25-process latency / five three-tour memory checkpoint completed; four memory targets and two latency guards remain unmet. See [the latest results](SOURCE_RUN_CHECKPOINTS.md) and [the preceding attribution report](PERFORMANCE_ATTRIBUTION.md). Broader M2 candidates remain conditional; V1 visual acceptance is not complete. The specification and historical evidence below preserve their original scope and gates.

Further M2 execution: the compact file-local projection candidate is implemented
and fully characterized, but **not promoted**. It saves 110.49 MiB of largest-input
document payload and 182.32 MiB of opening live heap in a controlled comparison.
Correctness checks pass; a repeated approximately 7.3 ms end-coloring regression
remains unexplained. The working source is this unaccepted candidate, and the
preceding retained artifacts are unchanged. [The candidate report](COMPACT_PROJECTION.md)
records all results and the next bounded attribution effort. Do not begin another
memory-owner change or relabel the regression as resolved from the full-workload
results. The original scope below is preserved.

Correct source and presentation remain the first requirement. This stage seeks lower retained memory and fewer coloring stalls without changing accepted diff behavior. It also separates actual Ghostty inspection from the terminal-cell checks already available.

The original immediate batch was **B0, M1, and C1** below: fix build provenance, attribute current memory, and attribute coloring work. That evidence selected the first M2 change and C2. Other M2 designs are not a commitment to implement every proposed optimization. V1 can proceed when user-supplied captures are available.

## Evidence at specification time

The latest measured executable is the display-block cleanup in [INDEX_STORAGE.md](INDEX_STORAGE.md). Its five-patch checkpoint completed, but performance and visual acceptance remain open.

| Observation | Interpretation |
| --- | --- |
| Largest input: 1,052.5 MiB maximum RSS during third-tour settled holds; 1,086.4 MiB process peak | A substantial current memory cost, not an indefinite plateau or a total-memory bound. |
| That input's source indexes, projection, metadata allocations, file descriptors, and file-list capacity total about 511.9 MiB | Current owned representation costs approximately half a GiB before syntax, other caches, allocator overhead, and other process state. This is not an irreducible lower bound. |
| The same accounting totals about 0.24 MiB for the small Pierre patch, whose third-tour settled maximum is 272.0 MiB | Index reduction alone cannot address both workloads. Earlier instrumented syntax profiles identify compiled-regex ownership as a major separate cost. |
| Current distant-coloring maximum: 298.34 ms; largest complete-index observation: 4,794.56 ms | Both miss their shared targets. First text, complete indexing, and final styling are different milestones. The coloring outlier's cause has not yet been isolated. |
| Source, projection, sequential-style comparisons, and real-process contracts pass at the latest checkpoint | Evidence for their covered behavior, not every possible grammar, ideal replacement pairing, or actual terminal pixels. |

Storage totals come from `evidence/index-v1/candidate-final/index.json`. Sum the disjoint top-level ownership fields; do not add projection subcategories to the projection total again. The largest input's source spans alone occupy about 149.6 MiB. The existing all-resident representation therefore cannot meet the provisional 128 MiB process goal merely by shrinking disposable caches.

Opening allocation snapshots are not third-tour heap snapshots. Allocator call-site labels can combine inlined owners. RSS, live heap, physical footprint, and compressed memory are different, partly overlapping quantities. Do not subtract observations from different phases or add these quantities together as though they were disjoint.

The shared targets in [MEASUREMENT_POLICY.md](MEASUREMENT_POLICY.md) remain unchanged. Missing them must not alter supported input, source content, styling thresholds, or the reported result.

## Invariants and scope limits

- Preserve every supplied source record and its bytes, order, side, number, newline marker, and metadata. Preserve split/unified pairing under the existing contract and all inverse/navigation mappings.
- Preserve independent old/new syntax state, missing-context behavior, declared fallback, word emphasis, safe text rendering, and theme results. A faster plain fallback is not a syntax improvement. Existing limitations remain explicit, not endorsed as full visual parity.
- Preserve source/side/text-position anchors through wrapping and resizing. Stale replies cannot repaint a newer view. Input, cancellation, partial-load errors, and terminal cleanup remain correct.
- Use ordinary ownership and checked conversions. No unsafe layout tricks, custom allocator, regex-engine substitution, extra parsing thread, or input-name-specific behavior.
- Change one memory owner or execution mechanism at a time. Keep the reference, failed trials, and explanations. A no-change conclusion is a valid result.

## B0. Bind tests and measurements to one release build recipe

The integrity review found two optimized, noninstrumented release binaries from the same source and toolchain:

- `5d2b7a7816ac21f6e8c60c549efb7656466e6888496a83732b90723b6a09507a`: the archived measured executable, produced through the release test-target dependency feature closure.
- `6cf1a29655b05d2e58010548d261737cbd2dad32f062602ad5b9596ec2cd5cb4`: the plain release build observed in the review.

Cargo fingerprints show a dependency-feature difference, including `libc/extra_traits` in the test-target build. Running release tests can replace `target/release/native-diff`. This is not evidence of changed diff logic or invalid historical timings. It does mean the phrase “release build” is not a sufficient reproduction recipe.

For the next series:

1. Build reference and candidate with `cargo build --release --locked --offline --bin native-diff`, no optional application features, and the same pinned toolchain. Use separate target directories for each build and a third for tests/examples. Missing offline dependencies are a preparation failure, not permission to change the lockfile.
2. Record source/lockfile hashes, exact command, resolved dependency features, `rustc -Vv`, Cargo version, host/target, SDK/linker identity, and explicit build-affecting environment settings. Do not dump the whole environment. One manifest in the existing evidence directory is sufficient.
3. Copy the executable to a new write-once evidence path, hash it, and use that absolute path for all trials. Check the hash before and after verification. Use `NATIVE_DIFF_CONTRACT_BINARY` for the contracts that support it; identify tests that still exercise Cargo's separately built executable.
4. Confirm the recorded recipe reproduces the executable. Investigate an unexplained mismatch before collecting comparison results. Diagnostic-feature builds and allocation-instrumented processes remain separate.

**Done:** a tested, immutable reference executable and a reproducible build manifest. New plain-build timings get a fresh baseline. Old observations stay bound to their archived executable, not relabeled as measurements of the plain build. No general build service is needed.

## M1. Attribute live memory at the phases that matter

Use [tools/profile_memory.py](tools/profile_memory.py), the existing structural accounting, and the existing checked memory traces. Start with two different ownership profiles: the largest index-heavy input and the small syntax-heavy input. These select diagnostic coverage; they must not select runtime behavior or easier targets.

Capture opening and tour-end snapshots at the same verified views. Record live allocations, allocation counts, owner capacities, and available process residency/footprint fields. Add snapshot start/end timestamps to the existing report so sequential profiler commands are not presented as simultaneous readings. Treat unavailable OS fields as unavailable. Keep instrumented runs separate from ordinary RSS and latency trials.

Produce one reconciliation table with these owners:

| Owner | Required distinction |
| --- | --- |
| Source spans, projection arrays, metadata, descriptors | Disjoint owned payload/capacity versus allocator-sized allocation classes. |
| Grammar and compiled regex storage | Actually loaded/compiled owners versus static pattern inventory; shared allocations counted once. |
| Active parser states and temporary file conversion | Retained state versus a transient overlapping peak. |
| Source pages, geometry, cached styles, queued replies | Payload plus containers, entry counts, and references that keep evicted data alive. |
| Unattributed process memory | Keep a remainder rather than inventing an allocator or OS explanation. |

If the existing sharp RSS drop remains relevant and unexplained, add one same-trace profile of that case. Do not infer reclamation from a lower later RSS sample. These finite tours cannot prove leak freedom or unlimited-run boundedness.

**Done:** identify the largest tractable live owner and its lifecycle, or state why attribution remains insufficient. Choose M2's first candidate from that evidence. Do not run the full measurement suite after every diagnostic edit.

## M2. Reduce a demonstrated owner, with a reversible candidate

### First static-index candidate: encode an existing invariant

`SourceRun` in [src/projection.rs](src/projection.rs) stores two `Option<u32>` source line numbers. The parser already rejects zero source line numbers. Evaluate private `Option<NonZeroU32>` fields with checked construction and unchanged public values. This expresses an existing invariant using Rust's standard niche representation; it does not introduce a sentinel or reduce the supported numeric domain.

Measure actual record size and live payload on the selected compiler. Do not hard-code a whole-struct size assertion or claim an RSS saving from a type-size calculation. Preserve maximum valid line numbers and checked arithmetic. Keep this change separate from broader index compaction.

### Next static-index candidate, only if the remaining cost warrants it

Most projection coordinates are file-local but currently stored as `usize`. A concrete compact/wide representation could reduce blocks, source runs, replacement ranges, and side indexes for any file whose coordinates fit.

- Decide the representation from checked per-file bounds, not a repository or patch-size threshold. A file that does not fit retains the existing wide representation.
- Keep document-global coordinates and spool offsets wide. Test conversion boundaries without constructing a multi-billion-line fixture.
- Build the selected representation directly where practical. Avoid retaining complete wide and compact arrays together during publication.
- Keep storage behind the current projection interface. Rendering and reading anchors must not learn about representation variants. Prefer two concrete internal forms over a new generic indexing framework.

This is a separate change with full projection verification and lookup-cost measurement. Existing compact source spans and their wide fallback are already implemented; do not count them as new work.

### Syntax-memory candidate, only after live duplication is established

Current code shares identical compiled regex cells within each loaded grammar. Sharing across loaded grammars is worth considering only if M1 shows that identical compiled patterns actually contribute material duplicate live allocations. Static counts from loading every grammar do not establish that.

If justified, scope sharing to the syntax-set/worker owner and preserve lazy compilation. Identity includes exact pattern and compilation semantics. Caller-owned captures/regions and mutable old/new parser states remain independent; source-derived patterns stay outside interning. Preserve the compiled wrapper's existing thread-safe internal search-cache sharing. Account for retained keys and strong/weak owners; a persistent registry can cost more than it saves. Preserve clone, serialization, error, and drop behavior in the narrow vendored patch. No eager loading of unused grammars.

### Smaller conditional opportunities, not additional mandatory projects

- **Resize geometry:** different widths currently retain duplicate immutable line text and glyph checkpoints. If resize profiles show material duplication, share that immutable part and retain width-specific wrapping separately. Count shared storage once while any layout owns it; preserve existing cache admission, Unicode/tab boundaries, and anchor mappings.
- **File conversion:** if profiles identify overlapping source copies as a material peak, borrow from the current parse unit during conversion. Published files still use the immutable private spool. Borrowed data must not escape the unit, and libgit2 validation remains intact.

Do not implement these just because they are possible. Disk-backed indexes, broader compressed encodings, and grammar eviction require a separate architectural decision if simpler ownership changes leave unacceptable costs. This stage promises no arbitrary total-memory ceiling for unlimited inputs.

**Retain an M2 change only when** it reduces its measured owner, preserves all applicable correctness checks, and has no unexplained material responsiveness regression. Report owned-byte savings, instrumented live-heap changes, and ordinary RSS separately. A useful storage cleanup is not automatically a user-visible memory win.

## C1. Explain cold and backward coloring before adding state

The current worker retains one continuation. A backward request, a different hunk, or an invalidating epoch can require parsing supplied predecessors again. This is a plausible cost, not attribution of the current 298.34 ms outlier. Older cold-replay timings are historical, not a current-build baseline.

Extend the existing opt-in `delivery-trace` facility with bounded per-request summaries:

- Start record, requested range, number of replayed predecessors, and continuation reuse/reset reason.
- Aggregate parser, visible-color construction, word-comparison, theme preparation, reply-backpressure, and delivery work. Distinguish nested from disjoint durations.
- Cancellation checks and the longest indivisible parser/comparison call. No source text or event for every replayed line.

Use the existing deep continuous-syntax fixture, many-short-file challenge, and the current corpus outlier. Distinguish first-ever cold visits, backward misses, rendered-style hits, and revisits after natural eviction. Preparation must not precolor a supposedly cold destination. Also exercise rapid width/theme changes while coloring is pending.

**Done:** current, hash-bound ordinary timings plus separate diagnostic evidence explaining the dominant delay. If the delay is grammar initialization, word comparison, or one noninterruptible call, fix that owner in a new bounded proposal. Do not add checkpoints to solve a different bottleneck.

## C2. Conditional parser checkpoints for repeated prefix work

Proceed only if C1 establishes repeated valid-prefix replay as material. Syntect supports cloning parser state and rebuilding highlighting state from a full scope stack. Use that capability in the existing worker; do not add a thread or background pre-scan.

### State and lookup contract

- A checkpoint identifies immutable document/file/hunk/grammar state and, initially, the current theme epoch. Its next-record position means both sides have fully consumed the preceding supplied records.
- Store each side's complete `ParseState`, complete `ScopeStack`, and validity state. `ScopeStack` includes a private cleared-scope stack; copying only its public `scopes` field is incorrect.
- Choose the nearest compatible active continuation or retained checkpoint at or before the needed prefix. Keep the existing forward-resume path. When restoring a checkpoint, rebuild theme-dependent highlighting state normally. If neither is available, use the current correct replay path.
- Width, wrapping, and split/unified mode do not change syntax identity. New hunks and invalidating epochs cannot reuse incompatible state. Preserve the existing invalid-state fallback separately on each side.
- Capture only at fully processed record boundaries during already requested work. Never skip required predecessors. This cannot accelerate the first-ever parse of an uncached prefix.

### Memory and cancellation contract

Before implementation, add narrow owned-heap accounting for the necessary vendored state types. Count nested vector capacities, capture strings/regions, the full scope stack, and entry/container storage. Shared immutable grammars and compiled regex allocations belong to their existing owner, not each checkpoint.

Initial experiment policy: consider a checkpoint every 256 supplied records; retain at most 64 entries across the worker, at most 16 MiB including containers, and admit no entry larger than 256 KiB. Use global FIFO eviction, no duplicate identity/next-record entries, and a bounded linear nearest-prefix lookup. Drop entries from invalidated epochs. These are bounded-cache settings, not source limits or claimed optimal values. Freeze them before comparison trials. Oversized state continues parsing normally without being cached.

Preflight the clone cost, evict before admission, and verify actual owned bytes after cloning. Account for the transient clone separately from retained-cache limits. Check cancellation before and after snapshot/restore work. If safe size accounting or acceptable clone responsiveness requires a broad parser rewrite, defer checkpoints.

The proposed 16 MiB fits the unallocated portion of the 64 MiB disposable display/highlight plan in [SPEC.md](SPEC.md): current geometry and style payload budgets are 16 and 32 MiB. The separate source-page payload budget is 32 MiB. This is not free process memory or a total-memory guarantee; allocator overhead, active state, temporary clones, and grammar storage remain additional costs. Do not lower existing cache budgets to conceal the addition.

### Required proof

Compare exact settled styles with the unchanged sequential oracle and independent explicit syntax examples. Cover both sides of checkpoint boundaries, multiline/embedded syntax, capture-dependent delimiters, cleared scopes, invalidated fallback, theme/hunk changes, and real eviction/non-admission under normal budgets. Include resize/navigation/quit while replay or restoration is active.

Retain this cache only if useful backward/revisit work falls without changed styles, misleading fallback, unacceptable cold-path cost, or an unjustified memory increase. If its added memory is not worth the benefit, do not ship it.

## V1. Inspect actual Ghostty presentation without new automation

Terminal-cell traces establish program output. They do not establish font rendering, cropping, trackpad feel, screen tearing, or physical input-to-paint latency. This step stays open until actual captures exist.

Use [fixtures/position/review.patch](fixtures/position/review.patch) unchanged for both viewers. Pin the local Pierre checkout, component options, and rendered patch hash. The inspected checkout is `0a7a733078f093577cd21ca65d2508806a22238a`; `PatchDiff` is in `packages/diffs/src/react/PatchDiff.tsx`. Match split/unified mode, theme, wrapping, visible context, line numbers, and four-cell tab width explicitly. Different fonts do not require literal pixel equality.

The handoff is one short manual checklist and exact launch commands, not another recorder application:

1. The user opens the immutable executable in one existing Ghostty window. Record Ghostty version, font/size, scale, line height, actual grid, terminal profile/TERM availability, fixture hash, and executable hash. Read the grid; do not infer it from screenshot pixels or alter global terminal settings.
2. Capture four baseline pairs: split/unified in dark/light. Add narrow wrapped split and unified views in dark mode for the long-line cases. Compare source/side/numbers, unequal replacement padding, gutters/change backgrounds, word emphasis, and Unicode/tab clipping and wrapping. Classify each discrepancy; a browser/reference difference is not automatically a native correctness failure or automatically acceptable.
3. Capture one short interaction sequence: ordinary wheel/page movement, file/end movement where applicable, wrapping/layout changes, rapid width changes during verified pending highlighting, and clean quit. Use the existing pending-highlight fixture for that last condition if the small patch colors too quickly. Give exact source/side destinations, not instructions to select an unexplained string or character.

The user supplies screenshots and, for motion, a recording through their existing tools. No accessibility/computer-use control, new permission loop, new tabs, or global configuration changes are assumed. Keep diagnostic logs outside the visible diff. Stills prove appearance only; a clip without synchronized input timestamps and known capture behavior is qualitative motion evidence, not a millisecond latency result.

**Done:** a compact case/result table linked to actual paired images and the interaction clip, with defects and unverified cases explicit. Missing captures leave visual acceptance open even if process tests pass. Use [ACCEPTANCE.md](ACCEPTANCE.md) rather than inventing a parallel visual acceptance system.

## Verification and stopping rules

Use the existing harness, independent source expectations, preserved projection evaluator, and unchanged sequential styling reference. Do not alter expected output to make an optimization pass.

1. At each retained runtime candidate, run the existing 33 real-process tests, two numeric storage tests, formatting, and Clippy. A vendored parser change also runs its relevant real parser/regex checks. Add only focused tests for a new boundary, ownership lifecycle, or state transition that the existing suite cannot verify.
2. Representation changes require complete source and projection equivalence on all five large inputs, the existing structural challenge set, and exhaustive small-pattern cases. Check source bytes, rows, inverse lookups, partners, metadata, markers, and navigation boundaries. Baseline equivalence supplements rather than replaces independent expectations.
3. Freeze a small reference/candidate comparison before collecting results: five alternating fresh-process pairs, identical actions and preparation, unchanged targets, explicit cache states. Report every run, maxima, fallbacks, incomplete observations, and known decoder limitations. Do not rerun until a favorable sample appears.
4. After selecting the combined candidate, perform the existing full checkpoint once: 25 latency processes and five three-tour memory processes. Include current-build cold/revisit checks separately; the standard navigation trace does not prove those cache states. Capture ordinary RSS and CPU behavior without allocation or delivery instrumentation. A correctness failure stops performance acceptance.
5. A mixed or noisy result warrants a stated follow-up design or an inconclusive conclusion. Preserve both series. More repetitions cannot repair mismatched binaries, changed workload semantics, or incorrect views.

Completion of this stage means a reproducible build, attributed costs, an explicit retain/reject decision for each attempted change, and fresh correctness/responsiveness/memory evidence for the retained executable. Visual acceptance has its own capture-dependent status. No single RSS reading, cache cap, successful large input, or type-size reduction constitutes full product acceptance.
