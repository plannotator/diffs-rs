# Exact-source coloring and expandable context

Date: 2026-09-22.
Status: feature specification, with [storage/reference groundwork](../history/SOURCE_STORAGE_REPORT.md) and [local source inputs](../history/LOCAL_SOURCE_INPUT_REPORT.md) implemented.
The [complete-context coloring checkpoint](../history/FULL_SOURCE_COLOR_REPORT.md) implements the coloring stage.
The [context expansion checkpoint](../history/CONTEXT_EXPANSION_REPORT.md) implements incremental local expansion. Integrated resource and physical-terminal qualification remain open.

This specification refines the input, syntax, and context proposals in [SPEC.md](SPEC.md).
The [input scenario analysis](INPUT_SCENARIO_ANALYSIS.md) defines the broader source modes, GitHub requirements, and delivery priorities.
The existing themes, sidebar, navigation, and read-only behavior remain the visual authority.
The work adds complete source versions without replacing the current patch viewer.

## 1. Outcome and scope

Readers can reveal unchanged code around a change and see syntax colors based on the preceding source.
Hidden code and missing code are different states. The interface must distinguish them.

The first implementation milestone provides:

- Comparisons between two explicit local Git commits.
- Comparisons between two explicitly supplied regular files.
- Separate old-side and new-side syntax state from complete source versions.
- Incremental context expansion before, between, and after textual changes.
- Stable source positions through expansion, wrapping, resizing, and sidebar changes.
- The existing patch-only mode without implicit filesystem or repository access.
- A source contract that also fits remote reviews and application-supplied snapshots.

GitHub PR acquisition is the next required adapter milestone, not an unplanned extension of the local Git command.
Captured working-tree/index comparisons follow that adapter. Each mode needs separate source and lifecycle checks.
The scenario analysis records commit/compare URLs, other hosts, workspaces, patch enrichment, and application integration as follow-on work.
Live refresh and implicit remote fetching are outside the initial milestones.
Search, copy, editing, staging, annotations, whole-file expansion, and context collapse are separate features.
Changes to syntax grammars or inline-diff algorithms are outside this work.

Complete source removes missing-context uncertainty. It does not prove that every grammar assigns every color correctly.
Unsupported languages, parser failures, and oversized lines still need explicit styling fallbacks.
No fallback changes source bytes, line numbers, additions/deletions, or change alignment.

## 2. Existing implementation and compatibility boundary

This section records the preceding sidebar baseline. The local-source report describes the new input boundaries.

The inspected ordinary executable and preserved sidebar executable have the same SHA-256:

`b06a89565cc4f9e1b466cbf2a2d57d70bbe6cbf0f75051de1039a136e0c7f8f3`

| Existing boundary | Relevant constraint |
| --- | --- |
| `main.rs` | Only patch and inspect commands exist. Completed files arrive from an input worker. |
| `storage.rs` | Exact patch bytes use a private spool. Source and row bases are immutable after file publication. |
| `projection.rs` | Source/change blocks resolve split and unified rows. Missing context has only a structural gap row. |
| `highlight.rs` | Parser continuation belongs to a file, patch hunk, and theme epoch. Each new hunk starts fresh state. |
| `highlight/checkpoint.rs` | Bounded checkpoints retain separate old/new parser states. |
| `geometry.rs` | Cached geometry uses a patch source ID and available width. |
| `app.rs` | Reading anchors use source ID, active side, displayed-text byte position, and screen row. |
| `tree.rs` | File identity and totals come from the original patch entries, not visible rows. |

The patch parser, line pairing, inline emphasis, and canonical patch export remain authoritative for supplied changes.
New context must not become fake additions, deletions, or extra original patch records.
The implementation must not increase every stored patch record to hold optional full-file metadata.

The existing [sidebar report](../history/SIDEBAR_REPORT.md) owns current correctness evidence.
Existing measurements do not describe this proposed feature or a future executable.

## 3. Inputs and exact version identity

These local commands are implemented:

```text
diffs patch change.patch
diffs files OLD NEW
diffs git --repo PATH --base REV --head REV
```

The GitHub milestone adds a supported PR-URL entry point. Its final command spelling remains part of adapter design.
A downloaded `.diff` passed to the patch command remains patch-only. The filename does not establish source provenance.

### Shared input contract

Each review supplies three independent facts:

- Comparison identity: origin, scope, repository identities, pinned endpoint identities, and review snapshot ID.
- Comparison completeness: complete, known partial, or unknown, with the evidence and reason.
- Per-side source availability: available, legitimately absent, pending, or unavailable, with the concrete reason.

Successful patch parsing does not prove that a provider supplied the entire comparison.
Complete source for one file does not prove that the provider supplied every changed file or change.
A legitimately absent side differs from an empty source file, failed request, or unknown source identity.
Headers and tree totals must retain partial/unknown status rather than imply a complete review.
Source readiness and complete-context coloring can become available for a checked file within a partial review.

Each original file occurrence owns its source pair. Repository identity and side provenance are part of that binding.
Fork reviews, workspaces, and patch series cannot assume one repository or one global commit pair.
Application hosts can supply immutable contents through this contract without invoking our CLI or adopting our authentication flow.
The renderer, projection, and highlighter consume checked descriptors. They never infer comparison semantics or acquire credentials.

The [scenario analysis](INPUT_SCENARIO_ANALYSIS.md#5-the-common-contract-needs-three-separate-answers) defines completeness evidence and provider boundaries.
Unresolved remote guarantees remain explicit adapter qualification gates, not assumptions hidden in the storage API.

### Direct Git comparison

Both revisions are explicit. The adapter resolves them once to complete commit IDs before generating any patch.
All subsequent operations use those IDs or their blob IDs, never the original moving branch names.
This command compares the two trees directly. It does not select an implicit merge base.
That policy belongs to this command, not to every source adapter. A PR adapter retains the platform comparison scope.
The viewer does not change the repository, index, working tree, or refs.

Use a supervised Git subprocess for generation and object delivery, not a new diff algorithm.
Keep libgit2 for the existing patch parser.
The installed `git2` API does not guarantee streaming reads for packed objects. A nominal streaming wrapper is not a memory guarantee.
Git subprocess memory also counts in performance evidence. Moving an allocation into a child is not a memory improvement by itself.

Generation uses one explicit policy: three context lines, Myers comparison, the indent heuristic, and no whitespace-ignore options.
Use rename detection at 50% with an explicit exhaustive-search limit of 1,000 candidates. Copy detection remains off.
These are initial product settings, not corpus-specific tuning. Missed rename pairs remain complete additions and deletions.
Preserve the generated file order, binary notices, and mode changes.

Raw, NUL-delimited change metadata supplies both original path byte strings, modes, and complete object IDs.
The preferred transport combines raw metadata and patch output in one diff invocation.
The adapter consumes the metadata envelope, then passes patch bytes to the existing parser.
It must check one-to-one entry association, including pure renames, binary entries, and metadata-only changes.
No association uses display labels or unchecked positional assumptions.
Do not run a second full comparison just to discover source identities.

Only full, checked blob IDs reach the object reader. Paths are not shell commands or object-expression fragments.
Source retrieval uses a supervised `cat-file` session with explicit object lengths and types.
There is no process per line. Completed snapshots survive object-reader restarts.

The first adapter supports SHA-1 repositories, matching the current parser dependency configuration.
Other object formats receive an explicit unsupported-format error before generation. No ID truncation or guessed compatibility is allowed.
The first qualification target is the current macOS arm64 environment and installed Git 2.50.1.
Git becomes an explicit prerequisite for the two new commands, not for existing patch-only use.

Git supports the required raw metadata, generation controls, and object transport.
The implementation must qualify the exact framing against the installed Git version before connecting it to the UI.
See the official [diff](https://git-scm.com/docs/git-diff) and [cat-file](https://git-scm.com/docs/git-cat-file) contracts.

### Two-file comparison

Open each supplied file once and capture its bytes into private storage before generating the comparison.
Only regular-file targets are accepted. Directories, devices, and pipes are not two-file inputs.
An explicitly supplied symlink can resolve to a regular file at open time. Its target path is never followed again during the review.
Before/after descriptor metadata detects ordinary concurrent changes. A detected change rejects that capture.
This check is not a claim of an atomic filesystem snapshot against every possible concurrent writer.
The guarantee is internal consistency: comparison, coloring, and expansion all use the same captured bytes.

Generate the diff from the private copies, never from the original paths after capture.
Use Git's no-index comparison with the same textual policy. Its difference exit status is not an input failure.
Bind the single comparison to its two captures explicitly.
Original paths supply display labels and language selection. Private filenames never determine either.
Both captures use the same regular-file mode. This command compares content, not original permission bits.
Identical contents show `No changes`. Missing input files are errors, not implicit empty files.

### GitHub PR comparison

The adapter opens a supported PR URL without requiring a working checkout.
It pins the base/head repositories, revisions, actual comparison ancestor, and selected PR scope.
It never substitutes the current base tip, local HEAD, or working files for those sources.
Forks can require different repositories for the old and new sides.

Patch acquisition, inventory completeness, and raw source delivery have separate acceptance checks.
API success, a missing per-file patch, or matching line counts do not establish complete source correspondence.
Private access, changing PRs, large responses, symlinks, API limits, and missing objects need explicit states and evidence.
Known partial content remains readable with a persistent status. Missing data cannot become an empty or binary file by inference.

The [GitHub acquisition requirements](INPUT_SCENARIO_ANALYSIS.md#4-github-specific-correctness) own the detailed transport and recovery policy.
The adapter can use supervised read-only `gh` commands without a token store in the viewer.
Exact local objects can satisfy source requests. An isolated remote fetch requires explicit authorization and separate qualification.
Locally recomputed changes form a new, labeled comparison. They do not silently replace a supplied platform patch.

### Process and repository safety

Use `std::process::Command` arguments, not a shell string.
Resolve the Git executable once. Strip inherited Git routing, injected configuration, and diff-helper environment settings.
Terminate option parsing for user-supplied revisions and paths. Require resolved revisions to identify commits.
Set repository location explicitly. Disable pagers, external diff programs, text conversion, replacement objects, and lazy fetching.
Do not disable Git's repository ownership checks or add a broad safe-directory exception.
Disable optional locks. Drain stderr separately with a bounded retained diagnostic tail.
Read raw objects without checkout filters or symlink following. Never execute hooks, submodule commands, or source code.

Repository attributes can affect classification. Full-source checks still govern whether textual expansion is safe.
Tests must cover hostile helper configuration, stale branches, missing objects, and paths containing control bytes.
The selected Git version must support the required no-fetch control. Do not silently ignore an unavailable safety control.
Git documents these controls in its [top-level command reference](https://git-scm.com/docs/git/2.50.0).

Git symlink blobs contain link text, not permission to read the target.
Gitlinks remain metadata, not recursive repository reads.
Binary entries remain metadata. No binary context expansion is included.
These no-network Git controls govern local inputs and local object reads.
The GitHub adapter permits only its explicit read-only transport. It must not enable hidden lazy fetches in local helpers.
URLs found inside patch content cannot trigger requests. Credentials cannot enter diagnostics, saved responses, or cross-host redirects.

## 4. Source snapshots and proof of correspondence

A snapshot is immutable source content owned by this review session.
It has an internal snapshot ID, source provenance, byte length, line count, final-newline state, and disk ranges.
A source pair belongs to a stable original file ID and review snapshot ID. Display names are never its identity.
Each side retains its original repository, version or capture identity, and raw path.
An absent side of an added/deleted file is distinct from a present empty file and from a failed read.

For Git and remote reviews, prepare only source pairs requested for viewing, not every changed file at startup.
For two-file input, both captures already exist before comparison.
Textual patch rows can appear before source preparation completes.
The input-only checkpoint retains the existing patch-context coloring, with that limitation stated in the footer.
It does not present source readiness as full-context coloring.
The coloring stage now uses plain syntax until complete-context styles are available, while retaining change backgrounds and valid inline emphasis.

Before publishing an expandable source pair, perform a streaming correspondence check:

1. Match every old/new patch source record to the captured side at its declared line number.
2. Check source bytes and final-newline state, not only line counts or file sizes.
3. Check omitted ranges on both sides for byte equality and the expected line displacement.
4. Check the prefix, gaps between changes, and the suffix through the true end of both sources.
5. Reject an incomplete, overlapping, out-of-range, or inconsistent mapping.

This check must establish that the file patch describes the complete difference between these captures.
It does not establish completeness of the entire review. The input adapter owns that evidence separately.
An unrelated same-length change inside an omitted range must fail.
Raw CRLF bytes, missing final newlines, empty files, and zero-length hunk ranges retain their exact semantics.
The correspondence checker must handle the existing parser's separate newline-marker representation explicitly.

On a failed check, retain the supplied patch view but disable expansion and complete-context coloring for that pair.
Show `Source does not match this diff`. Do not silently substitute another revision or recalculate different changes.
Operational source failures produce a nonzero final exit status. Intentional patch-only and unsupported-language modes do not.

## 5. Storage, ownership, and resource limits

Keep source bodies on disk. Do not retain `String` or `Vec<u8>` copies of complete source files after capture.
Use shared session stores for captured bytes and line offsets, not one open descriptor per visited file.
Store fixed-width `u64` line offsets on disk, plus a final sentinel. Do not retain one heap entry per hidden source line.
Buffer index writes. A newline must not cause a separate filesystem write.
An empty snapshot has no source lines. A trailing newline does not create a phantom final line.
Capture and correspondence checks use bounded byte chunks, including for files with enormous individual lines.
Index arithmetic uses checked conversions. An unrepresentable line count receives an error, not a truncated source number.
The two named captures required by no-index comparison can serve directly as the two source stores in two-file mode.
This fixed pair does not introduce a descriptor per visited repository file or require another full copy.

The index costs approximately eight bytes per source line on disk, plus its sentinel and small metadata.
Snapshot bodies add disk space for the visited source versions. The patch spool remains separate and unchanged in meaning.
The first implementation does not add compression, a persistent cache, or cross-file content deduplication.
These can be reconsidered only after measured evidence of a specific problem.

Reuse the existing paged source access through a small storage refactor.
Patch bytes, snapshot bytes, and index pages share one source-page allowance, keyed by store and page identity.
Sequential capture, checking, and syntax replay use bounded sequential buffers instead of polluting the visible-page cache.
Random line access reads adjacent offsets and only the requested source bytes.
Reuse sequential scans where the operations consume the same bytes. Never repeat correspondence checks for each expansion or redraw.
Oversized hidden lines must not allocate complete display strings merely to discover that syntax requires a fallback.
Existing visible long-line allocation costs remain separately measured, not concealed by these storage limits.

| Existing limit to retain | Meaning |
| --- | --- |
| 32 MiB source-page payload | Aggregate across patch, snapshot, and index stores, not per file or side |
| 16 MiB geometry payload, 1,024 entries | Shared allowance for original and expanded rows |
| 32 MiB style payload | Shared allowance for original and expanded rows |
| 16 MiB parser checkpoints, 64 entries | Shared across all files, sides, and source modes |
| 256 KiB maximum checkpoint payload | Reject an oversized checkpoint rather than retaining it |

These limits sum to 96 MiB of allowances. They are not a total-process memory cap or a claim of fully allocated memory.
Cache tables, pinned pages, parser/grammar state, active lines, input parsing, worker queues, and Git processes require separate accounting.
The older blanket cache figure in `SPEC.md` does not replace the inspected per-cache limits above.
No allowance increases automatically because another source mode or worker is present.

Only one source pair can be under preparation at a time, with one replaceable pending destination.
Source preparation serves visible files first, starting with the file containing the reading cursor.
Other visible files remain eligible after that job completes. Redraws must not restart an identical active request.
Moving away cancels obsolete preparation without discarding already completed source pairs.
One additional source worker is allowed alongside the existing input and highlighting workers.
No thread, queue, or highlighter instance is created per file or per side.
The source worker owns its helper process and publishes only immutable completed descriptors.
Remote adapters use the same resource boundary, with bounded transfer buffers, request delivery, and cancellation.
Neither a provider nor an additional repository gets another source-page allowance or highlighter instance.
No cache lock can remain held across subprocess delivery, source checking, syntax parsing, or a blocking channel wait.

Unpublished store tails belong to the active source job. Cancellation or failure reclaims those tails before another append.
Published source ranges never move or change. Failed jobs cannot accumulate unreachable disk storage on repeated navigation.
Private temporary storage uses restrictive permissions and RAII cleanup.
Every helper is stopped and reaped on cancellation or exit, including error paths and pipe backpressure.
No repository files are changed to recover from missing source, disk exhaustion, or cancellation.

## 6. Complete-context syntax

Keep Syntect, the bundled grammars, the current themes, and the accepted native matching optimizations.
This feature changes the available source context, not the language engine.

For each side, start at the beginning of its source or a compatible saved parser state.
Advance through every preceding line, including all hidden unchanged lines.
If complete source is available, do not reset state at an original patch boundary.
Do not infer equal parser states from equal line text.
Different old/new comments, strings, heredocs, and language selections require separate states.
On renames, select the grammar from each side's own original path.
Unified shared context retains the existing new-side color policy.

Offscreen replay advances parser scopes without constructing final color spans.
Only requested visible lines receive retained styles. Revealing context does not require coloring the whole gap.
Requests use the existing latest-destination mailbox and bounded result delivery.
Compatible checkpoints and continuation remain reusable after navigation or expansion.

Complete-source syntax identity includes the snapshot, side/language, and current theme epoch.
Patch-only syntax retains its separate file/hunk identity.
Source origin changes must invalidate incompatible styles and replies, even if a numeric line ID or theme is unchanged.
Expansion does not invalidate correct syntax merely because display rows moved.
Keep existing conservative theme invalidation. Theme-independent checkpoint reuse is not a second optimization in this feature.

Cancellation checks occur between source lines and bounded I/O chunks.
An individual regex operation is still not preemptible. Do not advertise a hard cancellation deadline for that operation.
Rendering and keyboard input must not wait for the syntax worker to reach a distant line.

### Fallback and status

Use explicit states rather than combinations of readiness booleans:

| State | Visible meaning |
| --- | --- |
| Patch-only source | Supplied lines remain readable. Missing context has no expansion action. |
| Preparing source | Diff text is available. Source preparation remains pending. |
| Complete source, syntax pending | Expansion is available. Visible syntax is still pending. |
| Complete source, syntax ready | Visible syntax used the preceding source. No missing-context warning applies. |
| Source unavailable or mismatched | Patch remains readable. Show the concrete reason and disable expansion. |
| Styling fallback | Source and expansion remain available. Show why syntax uses plain styling. |

The current long-line protection remains in force without silently skipping parser input.
In complete-source mode, an unparsed oversized line invalidates subsequent syntax state on that side through the end of the file.
A later patch boundary is not a safe restart point. The other side remains independent.
This limitation must be visible and tested. It must not become speculative coloring to improve apparent completion time.
Known checkpoints before the failure remain usable for earlier lines.

Source errors outrank pending messages. The footer summarizes the current viewport, not unfinished offscreen jobs elsewhere.
Comparison completeness remains a separate persistent review status. Successful coloring cannot clear an incomplete-review warning.
The settled generic patch warning is replaced by source-aware status as part of this feature.
Complete source is not labeled as proof of universal grammar correctness.
Safe display escaping and the existing newline normalization remain unchanged.
Whole-file syntax reference checks use that same declared display transformation.

## 7. Context presentation and navigation

The current visual language remains unchanged: gutters, themes, change backgrounds, inline emphasis, and split alignment.
Only real unchanged source lines can occupy an expanded context range.
An alignment spacer is not a context gap and never receives an expansion action.

A ready gap displays `N unchanged lines · Show 20`, with the actual reveal count when fewer than 20 remain.
The full row is a mouse target inside the diff rectangle. Enter activates it when the reading cursor selects that row.
Use existing Up/Down movement to reach it. Do not add an unexplained shortcut or overload tree controls.
Unavailable gaps keep a non-actionable explanation. Pending gaps do not queue repeated expansion requests.

| Gap location | Reveal direction |
| --- | --- |
| Before the first change | Reveal the last 20 hidden lines, nearest the first change |
| Between changes | Reveal the first 20 hidden lines, after the preceding visible section |
| After the last change | Reveal the first 20 hidden lines, after the last visible section |

This leading-gap rule refines the earlier blanket start-of-gap rule in `SPEC.md`.
Each action reveals at most 20 original source lines, not 20 wrapped screen rows.
Repeated actions continue in the same direction. The gap row disappears at zero remaining lines.
No visible source line repeats at the join. There is no automatic expansion during scrolling or endpoint navigation.
Expansion lasts for the process and remains independent of layout, theme, wrap, and sidebar visibility.
Whole-file expansion and collapse are intentionally absent from the first version.

In split view, expanded unchanged lines have their corresponding old/new numbers and independent syntax colors.
In unified view, they appear once with both source identities retained.
Expansion never changes file status, addition/deletion totals, or the sidebar hierarchy.
Original `@@` markers retain their original patch ranges. Added context does not masquerade as part of those recorded ranges.
Original change boundaries remain the targets of `[` and `]` even after the gap between changes disappears.
Metadata-only and binary cards do not become whole-file viewers in this version.

### Position contract

Source identity, side, displayed-text byte position, and approximate screen row remain the reading anchor.
Existing source IDs cannot be reassigned when extra rows appear.
A click on a gap is an explicit movement action: select the newly revealed line nearest the activated gap edge.
This is the first newly revealed line for a leading gap and the last newly revealed line for a forward gap.
If the available viewport permits it, keep that line near the former gap row.
Background source readiness does not activate a gap or move the reader.

`g` and `G` select the first/last exposed source line in the current document, including previously expanded context.
They do not expand hidden code or land on a gap, marker, or header.
File jumps retain patch order. Tree activation retains original file identity.
Wrap, split/unified, line-number visibility, and resize changes preserve anchors inside newly revealed lines.
Temporary tiny windows and coalesced resize repaint retain their existing contracts.

## 8. Rust structure and allowed refactoring

Keep one Cargo package, one terminal owner, one input loop, and one highlighting worker.
Use ordinary owned structs, enums, newtypes, and the existing channels.
No async runtime, plugin framework, service container, custom allocator, or generic source-provider framework is needed.
The initial dependency budget is zero new crates. A justified exception requires a recorded design decision before adoption.

| Responsibility | Intended module boundary |
| --- | --- |
| CLI input selection and terminal lifetime | Existing `main.rs`, with a focused `input` module for source-mode setup |
| Git process policy, framed transport, process cleanup | `input/git.rs` |
| Regular-file capture and two-file binding | `input/files.rs` |
| PR identity, read-only acquisition, completeness evidence | A later focused `input/github.rs`, outside rendering and highlighting |
| Immutable snapshot descriptors, disk line indexes, correspondence checks | A small `source` module |
| Shared paged access and private source storage | Existing `storage` with focused child modules |
| Gap ranges, expansion state, effective visible-row lookup | A view/projection layer above the immutable patch projection |
| Reading anchors, focus, Enter/click routing | Existing `app` and its control module |
| Full-prefix replay, source-aware cache identities | Existing `highlight` and checkpoint module |
| Gap row presentation and source-aware footer | Existing `render` |

Introduce typed source references at access boundaries, not a wider allocation for every original patch line.
A reference distinguishes an existing patch record from a line in an immutable snapshot.
Display rows resolve these references through one source-access boundary used by rendering, geometry, and reading anchors.
Original patch records retain their compact representation and canonical export.
References to the same original line must resolve to the same bytes regardless of its current presentation.

Gap records hold a stable file ID, old/new start lines, remaining length, and revealed extent.
Use ranges, not one object per hidden or revealed context line.
Lookup computes a requested row from those ranges. It does not materialize all context rows.
The immutable patch projection continues to resolve changed rows and their pairing.
Do not mutate published `FileIndex` row bases to accommodate expansion.

Effective display counts and file offsets belong to the view layer.
Start with compact prefix arrays updated only after structural changes, with publication updates batched.
Expansion rebuilds the affected file's range metadata and numeric file-offset suffix, not every source line or unaffected file projection.
Drawing and coloring replies do not rebuild those arrays or the tree.
Large file-count checks must qualify that metadata update cost before release.
A more complex dynamic index needs evidence that the simple representation causes a real problem.

Use `Arc` only for immutable stores and descriptors shared by workers. Mutable application state stays on the UI thread.
Use private or `pub(crate)` visibility unless the existing binary/library boundary requires a public item.
Keep raw paths as bytes and offsets as checked integers. Never recover identity by parsing a display string.
Keep provider-specific metadata outside the per-line records. Use explicit capabilities rather than provider checks throughout the renderer.
Handle partial reads, overflow, failed channel delivery, and malformed child output with explicit errors.
Do not add production `unwrap`, unsafe code, recursive deep-path ownership, or unbounded channels to these paths.
Keep refactoring and behavior changes in separate source checkpoints, each with a reviewable compatibility result.

## 9. Performance acceptance

Performance is a joint result: accurate content, visible responsiveness, coloring delay, memory, disk use, and cleanup.
There is no percentage improvement target and no fixture-name-dependent execution path.

The important observations are:

| Observation | Start and end |
| --- | --- |
| First readable diff | Process start to a complete visible viewport of real diff text |
| Source preparation | Source request to a checked source pair with working expansion |
| Correct visible coloring | Destination request to complete known-context colors, or a separately counted declared fallback |
| Navigation and expansion | Public input to the corresponding complete terminal grid |
| Sustained resources | Peak and settled process memory, helper memory, live cache/index ownership, and temporary disk use across repeated visits |

For remote inputs, record network acquisition separately from local preparation, readable text, and complete coloring.
Request counts, transferred bytes, helper memory, and cache state belong to that evidence.
An incomplete response is not a successful complete-review performance sample.
Capture patch-only compatibility and exact-source behavior separately.
Compare the same exact-source workload before and after any later optimization, not a complete-source review against a cheaper patch-only review.
Record source snapshot identities, original file paths, Git version, executable hashes, themes, terminal dimensions, and available diff width.
Include sidebar-on and sidebar-off cases without comparing unequal diff widths as equivalent work.

Use representative shapes: small multi-file code, a large hidden prefix, many gaps, many changed files, and long Unicode/tabbed lines.
Include the existing five-patch workload for patch-mode regression checks.
Downloaded patch files alone are not proof of complete-source behavior. Exact-source cases require retained matching source versions.
Exercise deep first visits, repeated tours, cache evictions, repeated expansion, cancellation, and width changes while coloring is pending.

No repeated comparison, repository command, or file-wide scan belongs on the render/input thread.
Some bounded source-page reads remain on the current rendering path. Measure their cold-read effect instead of claiming zero UI-thread I/O.
A deep first visit still requires prefix parsing without a compatible checkpoint. This is real work, not an instant-coloring promise.
Syntax fallback and source failure must not count as successful fully colored samples.

Retained RAM must not acquire a new per-hidden-line index or per-revealed-line object graph.
Resource accounting must include source copies on disk, active buffers, child processes, and memory retained after tours.
No unexplained growth across identical warmed tours is acceptable.
Small timing differences inside run variation are inconclusive, not wins or failures.
Material regressions require an explanation and an explicit acceptance decision before this becomes the normal supported source mode.
Cache payload limits alone cannot satisfy that decision.

Implementation checks do not require a controlled AC measurement campaign.
Formal timing and sustained-memory captures remain separately scheduled work with the established power and exclusive-workload conditions.
Actual terminal inspection remains separate from emitted-cell evidence.

## 10. Correctness evidence

Use real temporary repositories, exact before/after files, the real Git binary, the real parser, and the production highlighting worker.
PTY checks send actual public keys, mouse events, resize events, and termination signals to the ordinary executable.
Pure tests are limited to byte/line correspondence, range projection, and identity invariants that screen output cannot establish alone.
No mock source provider or fake successful highlighter can satisfy an acceptance gate.

| Contract | Required check |
| --- | --- |
| Exact revision | Move a branch and modify the working tree after startup. Existing review bytes and expansion remain unchanged. |
| Input identity | Distinguish direct and merge-base comparisons, repository identities, repeated paths, and a legitimately absent side from unavailable content. |
| Completeness | A checked file can expand while its review remains partial. Unknown totals cannot become zero or complete totals. |
| Complete difference | Check every supplied record and all omitted ranges against independently retained sources. Reject equal-size mismatches. |
| Stateful syntax | Place comment, string, heredoc, and embedded-language boundaries outside supplied context. Assert explicit old/new syntax roles. |
| Sequential equivalence | Compare visited tokens against an uncached complete-file pass with the bundled grammar and theme. This is an integration reference, not an independent grammar proof. |
| Expansion | Reveal leading, interior, and trailing gaps to exhaustion. Assert exact bytes, numbers, order, counts, join boundaries, and EOF markers. |
| Source positions | Exercise expansion, side changes, wrapping, sidebar changes, tiny windows, and rapid resize around tabs and Unicode. |
| Cache identity | Revisit after eviction, source readiness, theme changes, and navigation cancellation. Reject stale-origin replies. |
| Fallback | Put an oversized line in hidden context on one side. Later colors must not pretend that parsing skipped it safely. |
| File kinds and paths | Cover empty/add/delete, rename, mode-only, binary, symlink, gitlink, quoted/control paths, and differing old/new extensions. |
| Lifecycle and safety | Cover invalid revisions, unsupported object formats, missing objects, helper configuration, pipe errors, source-read failure, and cancellation. |
| Resource shape | Many files and repeated actions retain bounded queues, descriptors, pages, and snapshot tails. No source body is retained per tree node. |
| Patch compatibility | Existing canonical outputs and emitted cells match the saved baseline, except reviewed status/help wording. Existing process contracts still pass. |

Challenge the checker with a changed hidden byte, wrong-side line, missing EOF marker, and stale style identity.
These deliberate errors must fail before the new correctness evidence is accepted.
Large source checks must compare all relevant bytes, not only chosen endpoints.
Independent syntax-role examples supplement full-file equivalence so shared implementation mistakes are not the only authority.
The GitHub milestone also needs real read-only transport checks, including fork and moving-review behavior.
Public evidence cannot qualify private authentication or enterprise hosts. Recorded responses cannot replace those acceptance checks.

## 11. Ordered implementation and stop conditions

Each stage preserves its ordinary executable and source identity before the next stage changes runtime behavior.
Historical evidence and prepared measurement binaries stay unchanged.
The compatibility-only store/reference refactor is complete, as recorded in the [storage checkpoint](../history/SOURCE_STORAGE_REPORT.md).
The local-source checkpoint adds snapshots, disk indexes, local identities, exact correspondence, and both local commands.
The expansion checkpoint preserves the typed identity mapping before it adds range expansion and terminal controls.
Its concrete review type represents complete local comparisons only. Partial/unknown remote coverage states remain part of the remote adapter checkpoint.

1. **Input contract, source storage, and references.** Add comparison identity, completeness, per-side availability, immutable snapshots, and typed access boundaries.
   Add disk offsets and shared page accounting. Keep the first implementation small without fixing every input to two local commits.
   Prove byte/newline behavior and unchanged patch output before adding source-aware UI behavior.
2. **Exact-source input adapters.** Add the two commands, supervised Git transport, captured-file comparison, and source correspondence checks.
   Prove stable revisions, process cleanup, and rejection paths. Do not enable expansion yet.
3. **Complete-context coloring.** Extend the existing worker and cache identities to full-source sides.
   Pass explicit syntax-role, sequential-reference, fallback, cancellation, and patch-regression checks.
4. **Expandable context.** Add view ranges, gap actions, source-aware statuses, and stable anchors.
   Pass both-layout PTY contracts before inspecting theme previews and physical terminal behavior.
5. **Integrated quality checkpoint.** Review ownership, duplicated caches, failure handling, module boundaries, and dependency impact.
   Run the ordinary suite, relevant diagnostic checks in a separate target, formatting, Clippy, and the declared Rust 1.88 minimum.
6. **Local milestone qualification.** Run the scoped current-build observations under controlled conditions.
   Record unresolved costs and limits. This gate qualifies the local source modes, not remote or live-workspace coverage.
7. **GitHub PR adapter.** Resolve the acquisition checkpoint in the scenario analysis before adding its transport.
   Qualify the adapter through the same source and UI boundaries, with separate network, completeness, and authentication evidence.
8. **Captured local changes.** Add staged, unstaged, and uncommitted modes with their exact snapshot and inclusion policies.
   Do not infer their correctness from the immutable-commit adapter.

If source identity is uncertain, correspondence fails, or expansion changes original diff meaning, stop that stage.
If canceled work leaks resources or a stale result can overwrite another source version, stop that stage.
Do not compensate by weakening source checks, increasing all cache limits, or silently suppressing difficult cases.

Before stage 2, qualify the combined Git metadata/patch framing on real files.
Before stage 4, demonstrate that the chosen view mapping preserves every existing patch-only destination.
These are narrow engineering checkpoints, not invitations to build replacement frameworks.

## 12. Milestone completion

The local milestone is complete when both exact-source commands produce read-only, internally consistent comparisons.
Supported textual sources provide checked context and complete-prefix syntax state.
Expansion works with the existing sidebar, layouts, themes, navigation, and reading anchors.
Unavailable context and styling fallbacks have honest, specific states.
The ordinary process suite and the new source/context contracts pass.
Evidence distinguishes correctness, physical terminal inspection, timing, and total resource use.
The feature documents its supported Git version, object format, input limits, and remaining grammar limitations.
The GitHub milestone requires the separate adapter evidence in the scenario analysis.
Completion of the local milestone does not imply support for remote URLs, live working trees, or every reference-application workflow.
No timing, memory, universal syntax-correctness, or release claim is inferred from this specification alone.
