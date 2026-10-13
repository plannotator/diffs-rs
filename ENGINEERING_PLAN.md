# General-purpose performance: audit and next actions

Historical plan and dated execution notes. Use the [current project state](CURRENT_STATE.md)
for the active next steps and the [latest checkpoint](INNER_ENGINE_CHECKPOINT_REPORT.md)
for current results. The original requirements and evidence below remain preserved.

Date: 2026-09-12. Owner: the native-diff task.

Latest next-action update, 2026-09-13: the [inner-engine census](OWNER_CENSUS_REPORT.md)
is complete. It identifies 276 repeated pattern/configuration groups with
74.21 MiB of additional-instance strategy estimates. All 415 frozen views and
the diagnostic controls pass. Cache growth includes more actual entries; its
accounting reconciles. Search-pool bytes remain unknown, and these estimates are
not RSS or promised savings. Next is an explicitly scoped inner-engine sharing
design with unchanged matching/capture behavior and separate allocation and
ordinary-performance acceptance. No runtime optimization was made in this stage.

Latest next-action update, 2026-09-13: the [retained-owner review](RETAINED_MEMORY_REPORT.md)
is complete. One diagnostic replay confirms live growth, with engine-named
allocations accounting for 50.72 MiB of an 83.44 MiB increase and additional
style-container growth. Next is the report's bounded, diagnostic-only census of
engine identity/storage and style-cache occupancy/capacity. No runtime optimization
was made; the ordinary checkpoint and unpromoted status below remain unchanged.

Latest update, 2026-09-13: the [RSS lifecycle repair and current-build checkpoint](RSS_SHUTDOWN_REPORT.md)
are complete. The five-pair comparison and full five-patch workload pass their
correctness/completion checks. Four memory goals, a repeated first-distant-jump
coloring delay, and the largest complete-index target remain unmet. Substantial
RSS drops remain unattributed; Bun grows across three tours. Use owner-level evidence before another
memory change. Coloring attribution and physical terminal presentation remain
open. The working candidate remains unpromoted.

Current next-stage specification: [memory ownership, coloring, and terminal presentation](NEXT_PERFORMANCE_STAGE.md). It starts with one reproducible build recipe and current owner/cost attribution, then gates individual runtime changes on that evidence. The implementation history and original requirements below remain intact.

The viewer has no repository-specific fast path in the reviewed source. Two measurement/reporting shortcuts did exist and are corrected below. The storage improvement remains real, but it is not full performance or visual acceptance. The next product work is event delivery, syntax allocation ownership, and general index overhead.

Implementation update, 2026-09-12: the small challenge set is pinned and the syntax-delivery change is implemented and checked. [The delivery report](DELIVERY_REPORT.md) records its evidence and limits. A small pinned Syntect patch now shares identical compiled patterns within each loaded grammar. [The highlighting memory report](REGEX_MEMORY.md) records ownership, source/style checks, the allocation reduction, roughly flat small-workload responsiveness, and the completed five-patch checkpoint. Fresh reference memory controls are complete: Pierre agrees with the allocation saving; sharp RSS drops at different times make the Linux sustained-memory comparison indeterminate. Four memory targets and two latency maxima remain unmet; no grammar, source, fallback threshold, or workload was weakened. The next investigation must distinguish live ownership from residency and use general input dimensions, not patch names. The numbered sections below retain the original plan and acceptance requirements.

## Audit result

Display-block update, 2026-09-12: the first general index cleanup is complete. Removing an unread context endpoint reduced block records without narrowing coordinates or changing lookup algorithms. [The storage report](INDEX_STORAGE.md) records exact payload savings, full source/projection and process checks, the completed five-patch checkpoint, and the fixed follow-up to mixed small-workload coloring observations. The cleanup is retained; full performance acceptance remains open. Further index changes are not implied by this result. Choose the next owner from live allocation evidence, and keep cold/backward coloring and residency attribution open.

This review examined the current Rust input, storage, projection, geometry, rendering, syntax, and event paths; the measurement/replay and aggregate-report code; existing correctness contracts; and retained measurement failures. It compared relevant source against the preserved package. It is not an exhaustive history: this workspace has no Git history, and unpreserved intermediate states cannot be audited. Web, database, and distributed-service checks do not apply to this local CLI.

| Question | Finding |
| --- | --- |
| Does production behavior recognize the five fixtures? | No such branch was found. The production references to Pierre are palette assets. File extensions select grammars generally. Git's empty-blob hash identifies a patch-format case, not a benchmark. |
| Were source records removed to reduce memory? | No such path was found. Checked compact spans fall back to wide storage; shared projection preserves source order and both layouts. Complete-source checks and exhaustive projection equivalence passed. |
| Did reporting hide regressions in averages? | The retained reports disclose first-color regression, the seek maximum, incomplete preparations, the copied-binary opening outlier, and memory failures. Percentiles are withheld for incomplete series. |
| Did acceptance depend on a fixture name? | Yes, in measurement code. A name starting with `linux` selected more lenient index-ready and RSS limits. This was documented, and that patch still failed memory. It was nevertheless inappropriate as a general policy. |
| Could the report overstate verification? | Yes. It emitted source/test passes without consuming their results and claimed complete repetitions/tours even when completion was false. The current run's separate evidence supports those checks, but the generator itself did not verify them. |
| Have we made no quality tradeoffs? | That would be too broad. General styling and pairing limits already exist. They preserve source, but do not establish full syntax or Pierre presentation parity. |

### Corrections completed in this review

- `tools/corpus_measure.py` now uses one shared target object, includes it in new freezes and summaries, and rejects historical or altered policies before replay. Fixture names do not select limits.
- [The new measurement policy](MEASUREMENT_POLICY.md) applies the existing stricter development targets uniformly. It introduces no size crossover fitted to these patches. Targets are not supported-input limits and cannot justify degrading the display. A general scaling policy, if needed, must be justified separately before another candidate is evaluated.
- `tools/aggregate_five_patch_report.py` marks source and engineering checks unverified by that generator. It reports actual completion status, tolerates missing opening evidence, and does not describe failed corruption controls as rejected. It labels supplied historical observations accurately.
- `tools/check_reporting.py` exercises the actual report CLI with copies of retained process evidence. Controls rename fixtures, retain a real seek failure, remove observations, omit an opening, and invalidate an oracle control. Original input hashes must remain unchanged. These are reporting tests, not fabricated performance runs.

The [final reporting checks](evidence/engineering-review-2026-09-12/verified/checks.json) pass. Preparing a new policy-bound freeze also passed the independent capture checks without launching the viewer. No Rust source, viewer binary, grammar, rendering expectation, or existing measurement result changed in this review. The former runner is retained in [the source package](evidence/memory-v1/final/source.tar.gz). Do not use the new runner to reinterpret an old freeze.

### Existing limitations, not hidden wins

`src/highlight.rs` uses a general 10,000-display-scalar guard, with a corresponding byte guard, for expensive lines. Such lines retain their complete source and change backgrounds but omit syntax and word-level emphasis. Skipping a line invalidates the affected side's syntax state through the rest of that hunk; the viewer avoids guessing later colors and reports plain styling. The other side can remain colored. Existing process tests verify that behavior and recovery at a new hunk.

`src/model.rs` bounds the content-based offset search for uneven replacements, including a 4,096-byte line guard. Outside that search, pairing is positional. Source records remain present and ordered, but this is not proof of optimal pairing or exact Pierre pairing. Memory refactoring retained these rules; it did not lower their thresholds.

These limits were disclosed in the earlier specification. Their presence means that an unconditional promise of no visual compromise was inaccurate. Keep them explicit until a separately verified implementation replaces them. Removing guards without a correct, responsive replacement would not improve quality.

The five-patch timings also do not cover every cold interior request, evicted revisit, or rapid-input sequence. PTY output is not Ghostty paint time. These remain missing evidence, not implicit passes.

## Implementation sequence

Each product change gets one recoverable candidate, focused checks, and a measured reason to retain it. Use the existing Rust package and PTY tools. Introduce no new benchmark service, GUI automation, or competing-product dependency.

### 1. Establish a small independent challenge set

Before tuning the next candidate, add a few inputs beyond the five known patches using the existing fixture/process helpers. Choose them by structure:

- Many short files and small hunks, to expose per-file/per-hunk overhead.
- Few files with long continuous hunks and multiline syntax, to expose state continuation and cold replay.
- Unequal replacements, repeated lines, empty/metadata-only files, tabs and Unicode, reusing the existing explicit rendering contracts.
- Long lines at several widths, including cases on both sides of the current styling guard. The expected fallback remains explicit.

Use known before/after source texts and normal Git-generated patches where applicable. Include one additional real-world patch outside the DiffsHub set, chosen and pinned before the first candidate trial. Record bytes, records, file/hunk counts, and long-line dimensions. Keep a bounded development-size case for frequent tests; larger stress cases run separately. Avoid a full cross-product of every input and setting.

Done when the unchanged viewer has a recorded outcome for every chosen case, independent source expectations are established, and failures or fallbacks are documented. A holdout failure becomes a reproducible regression case; it is not replaced with an easier input.

### 2. Fix delivery of completed syntax work

Hypothesis: faster indexing exposes the existing polling delay. After loading, `src/main.rs` waits on keyboard input for up to 16 ms between reply drains. The syntax worker has a 16-reply queue. This can delay a completed viewport even when computation is ready. The first-color regression is measured; the exact share caused by this mechanism is not yet isolated.

First add a bounded, opt-in event trace for request creation, reply publication/drain, input receipt, and completed frame emission. Keep it outside terminal output, identify its instrumentation cost, and use it only for attribution. Trace timestamps do not become accepted latency measurements.

Then evaluate a capacity-one wake notification for the existing loop. Data stays in its current bounded queues. Publish data before notification; the loop drains ready work before sleeping. Coalesced notifications must not lose work or accumulate per source line. Preserve bounded per-turn processing, keyboard priority, generation checks, cancellation, and failure propagation. No new async runtime and no permanently faster polling loop are needed.

Primary files: `src/main.rs`, `src/highlight.rs`, and the narrow construction boundary in `src/app.rs`. Extract a small concrete helper only if it makes ownership clearer.

Verify through real processes: a short already-loaded patch, an incrementally supplied patch, a long syntax hunk, resize/theme changes with pending replies, quit with a full reply queue, and idle behavior after styling completes. Reuse existing cell/anchor contracts. Measure separate first-text and first-style times, CPU/wake-up cost, and input responsiveness in alternating candidate/reference order. Do not accept a delayed-input or idle-CPU regression in exchange for faster colors.

Done when observations explain the mechanism and the candidate removes the demonstrated delivery delay without source/style/anchor changes, lost wake-ups, unbounded queues, or worse cancellation.

### 3. Reduce syntax allocations at their owner

The preserved Pierre allocation snapshot attributes about 78% of live heap to named compiled-regex allocation classes. The static grammar inventory contains many repeated patterns. Neither fact predicts the saving from sharing.

Use the same profiler to attribute the current candidate on a multilingual workload and the challenge set. Measure which patterns actually compile and remain live, including embedded syntaxes and revisits. Distinguish grammar storage, parse state, cached styles, and allocator-retained pages.

First evaluate sharing immutable compiled representations for identical regex semantics. Identity must include the engine and compilation options, not just similar-looking text. Capture ranges, search scratch state, old/new syntax state, and scope stacks remain independent. Preserve the current grammar, regex engine, flags, error handling, and ordering. A small pinned dependency patch may be necessary; document its ownership and serialize/clone behavior before applying it. No engine substitution is assumed.

Do not introduce eviction until live sharing results justify it. Any eviction design must account for strong and weak owners, retained keys, active parsers, and the cost of recompilation. Do not lower style-cache or long-line thresholds to produce a lower RSS number.

Verify exact settled cells against the preserved executable, sequential scheduling against the unchanged reference, and explicit grammar examples with independent expected roles. Exercise old/new multiline differences, embedded languages, theme changes, cold visits and revisits. Profile live allocation change separately from uninstrumented CPU, latency, and RSS.

Done when a general ownership change demonstrably reduces live memory without changed grammar answers, false colors, or an unexplained responsiveness regression. If repeated patterns are not the costly ones, record that result and target the actual owner.

### 4. Reduce general index and temporary-storage overhead

Use component counts across contrasting input shapes, not the name of the largest fixture. Candidate work includes checked narrowing of file-local block/run coordinates, eliminating redundant fields, and reducing retained per-file/hunk capacity. Wide handling must preserve the existing supported domain. Prefer removing unnecessary structure before introducing compressed formats or unsafe decoding.

Consider temporary source borrowing only if the current profile still identifies per-file parse copies as material. Keep libgit2 validation and verified byte offsets. Borrowed data cannot outlive its input unit; published files continue to read immutable source from the private spool. A disk-backed index is a separate design if ordinary representation changes cannot meet the product's memory needs.

Primary files: `src/projection.rs`, `src/storage.rs`, `src/source_spans.rs`, and narrowly `src/model.rs`/`src/ingest.rs`. Rendering and reading anchors should not need to know which representation was selected.

Verify every row, inverse destination, partner, source byte, marker, and metadata record against the preserved evaluator on all existing patches and the challenge set. Keep independent source/cell contracts as the correctness authority. Exercise numeric boundaries with small explicit values and publication/cleanup through real processes. Report both payload and total process memory, plus any lookup cost.

Done when the representation saves measured memory on the relevant structural workloads without regressions elsewhere. No guarantee is inferred from cache limits or one successful large patch.

### 5. Complete the user-experience checks and reassess limitations

Run the existing five-patch series and the bounded challenge workloads after the product changes stabilize. Keep the host serial and coordinate its reservation with other active work. Full corpus runs belong at a meaningful implementation checkpoint, not after every edit.

Add explicit cold-interior and evicted-revisit observations. Public setup that already colors the destination cannot count as a cold request. Include repeated widths and themes, sustained tours, and rapid public inputs. Keep text, defined style/fallback state, full-index completion, cancellation, CPU, and memory separate.

Inspect user-provided Ghostty captures for split/unified display, both themes, width changes during coloring, clipping, and movement. Do not infer frame pacing or visual parity from a PTY grid.

The long-line styling and bounded-pairing limitations receive their own quality decision. Improving them is product work, with source/order and grammar-state invariants intact. Do not weaken existing expectations to accommodate an optimization or claim fallback frames as fully syntax-colored views.

## Acceptance and scope

- Correct source and adopted presentation behavior are required at every stage. Baseline equivalence alone cannot establish that an old pairing or grammar answer is ideal.
- Investigate a repeatable slowdown even if it remains under a development target. Preserve the slower result and state any proposed tradeoff before accepting it.
- Keep general algorithms and a small number of concrete modules. Additional caches, dependencies, or concurrency abstractions need measured justification and clear ownership.
- No repository-name branches, fixture-hash recognition, target-driven fallback changes, or selected-result reporting.
- The current plan does not implement search, editing, watch mode, a file picker, or new key bindings. No maintainer contact, AWS work, or global terminal configuration is involved.

This review corrected reporting code and created the plan. It did not launch the next viewer optimization or a new benchmark run. The next executable change is the syntax-work delivery investigation and fix, after the small challenge inputs are pinned.
