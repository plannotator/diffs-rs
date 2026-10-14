# Native diff viewer

A read-only Rust terminal diff viewer. `native-diff` is a working name.

Status, 2026-10-03: local macOS arm64 candidate with one engine for standalone and embedded use.
The standalone includes literal search, source-line selection, exact-byte save, and explicit clipboard copy.
The [sidebar report](SIDEBAR_REPORT.md) records its checks and remaining qualification work.
The completed local-engine milestone passed correctness, isolated installation and final handoff review. Its sealed source and evidence are preserved. Production hardening is now active; physical-terminal acceptance remains pending. Deferred experiments remain archived outside the active build.
The accepted storage changes, required-byte highlighting optimization, and cached-view cancellation are included.
No special source snapshot or manual executable replacement is necessary.

The viewer opens large patches progressively and supports split/unified layouts, wrapping, syntax colors, and changed-word emphasis.
Source and rendering checks pass for the covered cases. All four final resource cohorts passed evidence inspection on preserved ordinary binaries.
Large-input memory, indexing, and complete-search costs remain support limitations. Native clipboard, physical-terminal acceptance, and the final package remain pending.
The final notice inventory covers all 194 locked packages. Its upstream licensing caveat remains explicit.
This is not the complete v0.1 viewer in the [spec](SPEC.md).

The [current project state](CURRENT_STATE.md) records capabilities, evidence, deferred work, and next steps.
The [embedding guide](EMBEDDING.md) documents the engine, host-owned terminal lifecycle, and dependency patches.
The [pre-cleanup README](evidence/clean-baseline-v1/before/documents/README.md) preserves the earlier checkpoint narrative.

## Run the viewer

Build from the repository root:

```sh
cargo build --release --locked
```

The output is `target/release/native-diff`.
If all dependencies are cached locally, add `--offline`.
The current resource record (external record `native-diff-production-hardening/RESOURCE_RESULTS.md`, not in this repository) identifies the measured viewer and host, target misses, and interpretation limits.
The installation record (external record `native-diff-production-hardening/INSTALLATION_RESULTS.md`, not in this repository) identifies the separate normal and minimum-compiler builds. Different executable hashes do not inherit timings.

The qualified build environment is macOS arm64 with Rust 1.96.0, the separately checked Rust 1.88.0 minimum, and Xcode command-line build tools. Other platforms remain unqualified. For an isolated source installation, use `cargo install --path . --locked --root /absolute/path/to/an/isolated/prefix`; add `--offline` only after locked dependencies have been acquired. This does not add the prefix to PATH.

The build uses the [locally corrected parser](vendor/libgit2-sys/NATIVE_DIFF_PATCH.md).
If `LIBGIT2_NO_VENDOR` is set to a nonzero value, unset it before building.
The build rejects an override that would select an unpatched system library.

In an existing Ghostty window, open the sample patch:

```sh
target/release/native-diff patch fixtures/review.patch
```

The viewer uses the current terminal. It does not open windows or tabs, or change terminal preferences.
At 120 available diff columns or more, the default layout is split. Smaller diff panes use unified layout.
The sidebar uses 33 columns, including its divider.
The minimum usable size is 40 columns by 10 rows.

If `NO_COLOR` is set, remove it for this invocation to inspect the colors:

```sh
env -u NO_COLOR target/release/native-diff patch fixtures/review.patch
```

Other supported invocations:

```sh
target/release/native-diff --layout unified --theme light patch fixtures/review.patch
git --no-pager diff --no-ext-diff --no-textconv --no-color | target/release/native-diff patch -
git --no-pager log -p -3 --no-color | target/release/native-diff patch -
git format-patch --stdout -3 | target/release/native-diff patch -
target/release/native-diff inspect fixtures/review.patch --summary
```

`patch` reads Git patch text, including `git show`, `git log -p`, `git format-patch`, and `--stat -p` output.
Commit headers, messages, diffstats, and mail signatures between file patches are skipped. Source offsets still address the original input.
Text without a `diff --git` file header, traditional `diff -u` output, and combined merge diffs (`diff --cc`) remain errors.

`inspect` emits diagnostic JSON. It is not a formatted pager mode.
`inspect --canonical` emits a binary source stream for independent checks and refuses terminal stdout.
A nonzero exit invalidates that entire stream, including any earlier output.
The interactive viewer requires terminal stdout and a controlling terminal for keyboard input.

## Local source inputs

These commands require Git 2.50 or newer. Qualification currently covers Git 2.50.1 on macOS arm64.

```sh
target/release/native-diff files OLD NEW
target/release/native-diff git --repo PATH --base REV --head REV
```

The file command captures two regular files before comparison. It compares content, not original permission bits.
The Git command pins two commits and compares their trees directly. It does not select a merge base or use working-tree changes.
Only SHA-1 repositories are supported. Neither command runs external diff helpers, text conversion, hooks, or network fetches.
The viewer does not change the repository, index, working files, or refs.

Git sources load for visible files, with the reading cursor first. Completed captures remain stable for the review.
The source check compares every supplied line and every omitted range through EOF, including exact final-newline behavior.
If source preparation fails, the patch remains readable and the viewer exits nonzero when closed.
Binary entries and submodules remain metadata. A missing source is not an empty or deleted file.

Both modes now color checked sources from their complete preceding contents, with separate old/new parser state and filename-based language selection.
Hidden comments, strings, and embedded-language boundaries contribute to the parser state. Patch boundaries do not reset it.
Pending or unavailable sources use plain syntax. Change backgrounds and valid inline emphasis remain available.
The footer distinguishes pending coloring, complete-context coloring, unsupported languages, oversized source lines, and parser failure.
Complete context does not prove universal grammar accuracy.
Checked unchanged gaps show their remaining line count and a `Show 20` action.
Click the gap row, or select it with Up/Down and press Enter.
Leading gaps reveal code nearest the first supplied section. Other gaps reveal the next source lines forward.
Each action reveals at most 20 source lines, regardless of wrapping. Exhausted gaps disappear.
Expansion persists through theme, layout, wrapping, sidebar, and width changes.
Original patch markers and addition/deletion totals remain unchanged. Alignment padding never expands.
Binary and metadata-only entries do not become full-file viewers. Collapse and whole-file expansion remain outside this version.
The original `patch` command never guesses a repository or loads full source.

For a noninteractive source report, add `--check`:

```sh
target/release/native-diff files OLD NEW --check
target/release/native-diff git --repo PATH --base REV --head REV --check
```

This mode checks all textual file entries, not only visible files. It emits JSON with pinned identities, source states, and byte/line counts.
Its disk figure covers retained source bodies and line indexes, not patch storage, temporary helper output, or total process memory.
The [local-source report](LOCAL_SOURCE_INPUT_REPORT.md) records the evidence and remaining limits.
The [coloring checkpoint](FULL_SOURCE_COLOR_REPORT.md) records the complete-context implementation and its checks.
The [expansion checkpoint](CONTEXT_EXPANSION_REPORT.md) records context mapping, controls, and qualification limits.

## Reading controls

| Input | Action |
| --- | --- |
| `j` / `k`, arrows up/down | Move one display row |
| Page Up / Page Down | Move one page |
| Mouse wheel | Scroll three display rows per reported wheel event |
| `g` / `G` | Select the first / last available source line |
| `[` / `]` | Move to the previous / next hunk header |
| `n` / `N` | Move to the next / previous file header |
| Enter or click on a ready context gap | Reveal up to 20 unchanged source lines |
| Arrows left/right | Scroll horizontally, with wrapping off |
| Tab | Change the active side |
| `u` | Change split / unified layout |
| `w` | Toggle wrapping |
| `l` | Toggle line numbers |
| `t` | Change dark / light theme |
| `f` | Show and focus Files |
| `/` | Open the literal diff-text search prompt |
| F3 / Shift-F3 | Find the next / previous match |
| `V` | Begin a line selection / fix its range |
| Shift-click | Extend and fix a range within the same file and side |
| `y` | Copy the selected range or current source line as plain text |
| Ctrl-S | Save the selected range or current source line to a new file |
| Escape | Cancel a reading action / clear the selection / return from help or Files |
| `?` | Toggle help |
| `q`, Ctrl-C | Quit / cancel |

Horizontal scrolling stops at the right edge of the widest visible source line,
accounting for both panes and their gutters. Extra Right presses do not accumulate
an invisible offset. Moving to shorter content or widening the window reduces the
offset when needed. Wrapped views ignore Left and Right.

The active line is a reading cursor. `V` starts a separate source-line selection.
Start/end movement uses supplied patch order and focuses the selected source side. It skips headers, metadata-only files, and newline markers.
Both commands select the first wrapped segment of the endpoint line. Empty and metadata-only patches leave the view unchanged.
While input loads, the endpoints use completed files only.
Resizing, wrapping, and layout changes retain the source, side, and position within the displayed text.
Mouse selection retains the clicked text position. Keyboard movement establishes a new position at the start of its displayed segment.
Unwrapping retains the horizontal offset when possible, or adjusts it to reveal the anchored text.
File jumps select the adjacent file, even from inside the current file. They include metadata-only files and stop at either boundary without wrapping.
During loading, file jumps use completed files only. A later file arrival does not repeat an earlier command.

### Literal search

The standalone searches supplied diff text and context expanded before submission. It includes off-screen files and source lines.
It excludes file paths, headers, gutters, hidden context, and unchanged files outside the review.
Search is case-sensitive. It matches original UTF-8 bytes without normalization, regular expressions, or multiline queries.
Tabs match source tabs, not their displayed spaces. Display escapes do not replace original source bytes for search.

Enter submits the prompt. Escape dismisses it or cancels an active search. An empty submission retains the previous query.
Printable keys, including `q`, edit the prompt. Ctrl-C still exits the viewer with cleanup.
The prompt accepts at most 4,096 UTF-8 bytes. It rejects an oversized or multiline paste as one event.
This limit does not bound terminal-input memory: the terminal event library collects the complete paste before the prompt receives it.
Arrow keys, Home/End, Backspace/Delete, and Ctrl-U edit the query. Cursor movement preserves complete Unicode graphemes.

F3 and Shift-F3 move between matches in logical unified order. Shared context appears once. Old and new replacement records remain distinct.
Each request wraps at most once. Repeated F3 input during an active request does not queue more scans.
Existing `n` and `N` always keep their file-navigation behavior.
The current match uses bold and underline without changing its syntax colors or addition/deletion backgrounds.

Search requires complete patch indexing. Expansion or a source change invalidates its captured scope and displays a restart notice.
F3 restarts the retained query against the new scope. Layout, theme, wrapping, and width changes retain that scope.
Physical F3/Shift-F3 support still requires terminal qualification. The tested modifier sequence is `CSI 13;2~`.
The legacy `CSI 1;2R` sequence conflicts with cursor-position reports in the current terminal event parser.
The final reading-tool cohort passed response, cancellation-feedback, and quit targets. Complete largest-patch scans took 10.97–11.38 seconds.
These are terminal-output observations, not physical paint or instantaneous worker-stop measurements. Filesystem calls can block, and the largest case misses memory targets.

### Source-line selection

`V` starts a range at the current source line. Navigation extends that range until another `V` fixes it.
Shift-click extends and fixes a range from the reading cursor or existing anchor.
Escape clears the range after closing a prompt, canceling reading work, or returning from help or Files.

Each range belongs to one file occurrence and one original side.
The footer shows its original line numbers. Underlining marks selected source without replacing syntax colors or change backgrounds.
Wrapping, layout, theme, width, and context expansion preserve the range.
Unified navigation across opposite-side rows retains the range until another source line on its side becomes the endpoint.
An explicit side change, another file, review replacement, or withdrawal of its checked source clears the range.

Headers, context-gap controls, newline markers, absent replacement halves, and wrapped padding cannot be endpoints.
A range can span omitted patch-only context, but source export rejects unavailable lines before writing them.
Column selection and cross-file ranges are not supported.
Physical terminal selection behavior remains subject to final qualification.

### Save original source

Ctrl-S opens a path prompt for the selected range, or the current source line without a selection.
The request retains that original side and range. Later navigation does not change its contents.
Enter starts preparation. Escape dismisses the prompt or cancels preparation before publication.
The prompt retains the last submitted path. Ctrl-U clears it for a different destination.
Paths are literal UTF-8, relative to the launch directory or absolute. The viewer does not expand `~`, variables, or shell commands.
The prompt rejects multiline paths, NUL, and input longer than 4,096 UTF-8 bytes. It preserves spaces and tabs.

Save preserves original bytes, including tabs, invalid UTF-8, line endings, and the final-newline state.
It does not export gutters, visual wrapping, alignment padding, or display escapes.
Checked full sources can supply omitted context. A patch-only range with unavailable lines fails without creating the destination.
The viewer never overwrites an existing file, directory, or symlink. An error retains the selection for another attempt.

The worker prepares output in an owner-only directory beside the destination, then flushes and syncs the file.
The input loop checks the current request and source before publication. Canceled or superseded preparation removes its private output.
After publication, cancellation cannot undo the saved file. A cleanup error reports that the file was saved and identifies remaining staging.
New output files have owner-only permissions. The viewer does not copy source permissions or create missing parent directories.
Cancellation checks run between bounded copies. Filesystem calls can still block, and interrupted-machine durability is not guaranteed.
Finite save-resource and cleanup checks pass in the final resource record (external record `native-diff-production-hardening/RESOURCE_RESULTS.md`, not in this repository). Physical-terminal acceptance remains pending.

### Copy original source

`y` copies the selected range, or the current source line without a selection.
Copy retains the original side, tabs, Unicode, line endings, and final-newline state.
It excludes gutters, visual wrapping, alignment padding, and display escapes.
Unavailable source gaps fail instead of joining separate fragments.

The macOS build accepts valid UTF-8 text up to 16 MiB. Larger ranges and invalid UTF-8 require Ctrl-S to save their original bytes.
The size limit bounds the text passed to the native API, which can allocate additional copies. It is not a total-memory guarantee.
The viewer publishes plain text without RTF or EPS content detection. It does not send terminal clipboard sequences.

One reading worker prepares an anonymous private file before the input loop accepts delivery.
An owned helper receives that file directly, without terminal input or output.
The worker retains at most 4 KiB of diagnostic text and terminates a helper that exceeds its five-second deadline.
The viewer reaps an interrupted helper before another copy can start.

Escape cancels preparation without changing the clipboard. Once delivery starts, interruption or failure cannot guarantee that the previous clipboard survives.
Opening search or a save prompt also cancels pending copy work and reports this uncertainty after delivery starts.
Errors retain the selection. Ctrl-S remains available when clipboard delivery fails.

Clipboard support is optional and macOS-only. `--no-default-features` disables it without removing source selection or save.
Controlled-process checks cover source fidelity and helper lifecycle. Native clipboard round-trip, native helper memory, and physical-terminal acceptance remain pending.

## Changed files

The sidebar shows changed files under their exact patch directory hierarchy.
It does not scan the filesystem, combine directory levels, or reorder the patch.
Folders appear before files, with byte-sorted names. File activation uses the original patch entry.
File rows show status and textual additions/deletions. Folder rows show descendant file counts and totals.
Binary files show `B binary`, not textual zero counts. Repeated paths show occurrence suffixes.
The `*` marker identifies the current diff file. The `>` marker identifies the focused tree row.

The sidebar appears automatically for two or more completed files.
It requires at least 97 terminal columns and ten rows.
Narrow windows hide the sidebar without discarding its expansion state.
The `--sidebar` and `--no-sidebar` flags override the initial file-count preference, but not the minimum width.
These flags conflict. Interactive visibility changes last only for the process.

With Files focused:

| Input | Action |
| --- | --- |
| Up/Down, `k`/`j` | Move the tree cursor without moving the diff |
| Page Up/Down, Home/End | Move by a page or to a visible-list boundary |
| Left/Right | Collapse or expand folders, or move to a parent or child |
| Enter | Open the selected file or toggle a folder |
| Space | Toggle the selected folder |
| Escape, Tab | Return focus to the diff |
| `f` | Hide Files and return to the diff |

A click opens a file or toggles a folder. The wheel scrolls the pane under the pointer.
Newly completed files retain existing selection and expansion state.
The heading marks counts as partial during loading. Input failures leave completed files accessible and show an incomplete-result message.
The `g` / `G` bindings select source endpoints. The `n` / `N` bindings move between changed files.
The earlier `{` / `}` file bindings are removed. Numeric prefixes are outside the current scope.
A file picker is not implemented. Source-line selection, copy, and save use the diff cursor rather than the Files cursor.

## Implementation

One Cargo package contains one library and one executable. Ratatui draws the terminal cells.

| Module | Responsibility |
| --- | --- |
| `ingest.rs` | Read file-patch units and check their hunk boundaries |
| `model.rs` | Parse file-patch units through libgit2 and apply the existing line-pairing rule |
| `storage.rs` | Publish compact patch indexes and stable source identities |
| `source.rs` | Capture immutable source bodies, disk line indexes, and exact correspondence |
| `input` | Bind local comparisons, supervise Git, and prepare visible source pairs |
| `storage/bytes.rs` | Own private append-only byte stores, checked references, and shared cached pages |
| `source_spans.rs` | Store checked compact/wide source ranges and final-newline flags |
| `projection.rs` | Resolve both layouts and inverse lookups from shared source/change blocks |
| `view.rs` | Map immutable patch rows and incrementally exposed source ranges into each layout |
| `text.rs` | Display safe text and calculate grapheme columns |
| `geometry.rs` | Cache safe display text and sparse/wrapped boundaries within fixed payload and entry limits |
| `theme.rs` | Load the attributed palettes and compose terminal colors |
| `highlight.rs` | Produce syntax colors and inline spans on a worker thread, cache disposable styles |
| `app.rs` | Hold the reading position and apply navigation/settings changes |
| `render.rs` | Draw visible terminal cells through Ratatui |
| `main.rs` | Read inputs, drive Crossterm, handle signals, and restore the terminal |

The input worker publishes complete files while later input loads. The event loop remains available for navigation and cancellation.
Exact source bytes and encoded patch source offsets use private disk-backed storage.
Source identities, newline flags, projection metadata, and other indexes retain resident costs.
The byte store can share one 32 MiB page-payload allowance with additional source stores.
Local source inputs, complete-context coloring, and context expansion use this storage foundation. Remote inputs remain separate work.
The [storage checkpoint](SOURCE_STORAGE_REPORT.md) records compatibility and cleanup checks against the preserved viewer.
One current file still uses temporary resident parser data during conversion.
After a later parse failure, earlier complete files remain readable with an incomplete status. Normal quit then returns 1.
The UI draws visible rows. A separate worker supplies syntax colors and inline emphasis after source content appears.
The status row distinguishes pending highlighting from completed styling or a plain-text fallback.
Visible styles stay protected while new colors arrive, so cache eviction cannot leave a completed request with missing visible colors.
The retained-style target is 32 MiB. If the visible styles alone exceed it, they remain available until the viewport changes; offscreen styles are evicted first. This is not a total-memory bound.

Syntax uses Syntect and the Two Face grammars. Word differences use `similar`.
The local syntax backend skips a regex search only when the complete line lacks a provably required ASCII byte.
All other searches use the existing matcher. This preserves rule priority, captures, and side-specific parser state.
The [integration report](evidence/required-bytes-adoption-v1/REPORT.md) records correctness checks and the accepted cache-occupancy tradeoff.
Shared context keeps separate old/new syntax colors when the surrounding syntax states differ. Unified context uses the new-side colors.
Patch-only continuation resumes within the same hunk and theme generation.
Checked-source continuation follows each complete file side across patch boundaries. Hidden replay uses bounded sequential buffers and retains no offscreen style vectors.
Verified identical hidden ranges can share parsing when both sides use the same known grammar and have equal parser and scope state.
Each side keeps its own source position. Visible lines remain independently colored, and divergent states continue independently.
Bounded parser checkpoints also let backward requests and uncached revisits resume from compatible prior state.
During a complete-source file job, compatible checkpoints needed by that request stay protected from eviction.
If unprotected space is unavailable, parsing continues without another checkpoint. Protection ends with the job, including cancellation.
The limits remain 64 entries, 16 MiB in aggregate, and 256 KiB per entry.
The [retention decision](LOCAL_SOURCE_RETENTION_DECISION.md) records the saved parser work and accepted smaller-workload costs.
When a requested viewport needs no new styles, the application cancels obsolete coloring through the existing mailbox.
Queued styles must match both the theme epoch and source origin. Cancellation does not interrupt an individual regex instruction.
Cold requests replay the supplied hunk prefix in patch-only mode, or the complete source prefix in checked-source mode.
Offscreen replay advances syntax scopes without generating color spans. Both modes share the existing checkpoint budget.
Syntax coverage follows the bundled grammars. Universal grammar coverage and exact agreement with other viewers' replacement pairing are not established.
Very long lines use declared plain styling instead of syntax and word emphasis.
In patch-only mode, the affected side stays plain through that hunk. In checked-source mode, it stays plain through EOF, including later hunks.
A skipped source line cannot safely preserve parser state. The other side remains independent, and source text stays complete.
Uneven-replacement alignment uses a bounded search with positional pairing outside it. This fallback remains a presentation limitation.
Theme assets retain their Apache-2.0 license and attribution in [NOTICE.md](NOTICE.md).

## Run the process checks

The tests require Rust, Git, Python 3.11 or later, `shasum`, and a Unix PTY.
They run the actual executable with real files, pipes, keys, and signals.
They do not launch a GUI terminal or capture the screen.

```sh
cargo test --release --locked -- --test-threads=1
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --check
```

For optional diagnostic-feature checks, use a separate target directory:

```sh
cargo test --release --locked --all-features --target-dir target/diagnostic-tests -- --test-threads=1
```

This keeps the normal executable separate from instrumented builds.
The standard process suite passes 77 checks and excludes ten opt-in checks. Six host checks run separately with an explicit host executable.
The other four checks require pinned corpus or preserved-reference inputs. The corpus endpoint check requires the five downloaded patches:

```sh
cargo test --release --locked --test process diffshub_corpus_opens_and_keeps_exact_end_anchors_in_both_layouts -- --ignored --exact --test-threads=1
```

The old pre-storage visual comparison remains a historical check, not the current acceptance reference.

To check the shared projection against the preserved pre-refactor model:

```sh
cargo run --release --locked --example projection_equivalence -- --patterns fixtures/review.patch fixtures/position/review.patch
```

Pass the pinned corpus paths to the same command for complete large-patch row, source, inverse-lookup, partner, metadata, and navigation-boundary comparisons. It holds only a bounded number of file-patch units. This regression reference supplements the independent source and terminal-cell checks; it does not replace them.

To retain diagnostic terminal-cell previews, set a capture directory:

```sh
native_diff_capture=$(mktemp -d)
NATIVE_DIFF_CAPTURE_DIR="$native_diff_capture" cargo test --release --locked -- --test-threads=1
```

These SVG/ANSI previews come from actual PTY output. They are not Ghostty screenshots or proof of display timing.

The [rendering contracts](tests/process/README.md) add independent expected rows, cell boundaries, changed spans, syntax roles, and metadata checks.
They also record the reproduced syntax-side and binary-label defects and the remaining correctness limits.

## Run the five-patch checks

The [five-patch plan](FIVE_PATCH_ACCEPTANCE.md) defines the independent interior checks and the frozen measurement boundary.
The corpus bodies and independent source references must already exist at their manifest paths.
The commands use the existing local Python environment with [measurement requirements](tools/measurement-requirements.txt).

```sh
cargo build --release --locked --bin native-diff --example sequential_style_reference
corpus_run=$(mktemp -d)
.measurement-venv/bin/python tools/corpus_acceptance.py target/release/native-diff target/release/examples/sequential_style_reference "$corpus_run/rehearsal" --fixture pierre-0800fb
.measurement-venv/bin/python tools/corpus_measure.py "$corpus_run/rehearsal/trace.json.gz" "$corpus_run/freeze" --prepare-only
.measurement-venv/bin/python tools/corpus_measure.py "$corpus_run/rehearsal/trace.json.gz" "$corpus_run/results" --freeze "$corpus_run/freeze/freeze.json"
```

The first Python command checks real PTY captures against independent source data and sequential syntax colors.
The freeze step challenges the checker with corrupted captures and binds the inputs, code, and executables by hash.
The final command runs five fresh-process timing repetitions. It rejects changed identities and existing output directories.
For a three-tour memory run, add `--memory` to the rehearsal command and use separate output directories.
Fixture IDs are `pierre-0800fb`, `ghostty-12291`, `node-59805`, `bun-30412`, and `linux-v6-v7`.
Run one measured workload at a time. Linux public-key setup can take several minutes per repetition.
These checks observe PTY cells, not Ghostty pixels. The [project overview](CURRENT_STATE.md) records results and remaining limits.

## Remaining work

The final five-patch, complete-source coloring, repeated-host, and reading-tool cohorts passed evidence inspection.
The memory, indexing, and complete-search limitations remain explicit in the resource record.
Native clipboard and physical-terminal acceptance, and local packaging remain open.
The [current project state](CURRENT_STATE.md) owns the ordered next steps and the preserved evidence.
The [shared position fixture](fixtures/position/README.md) and [terminal procedure](TERMINAL_PRESENTATION_CHECK.md) support the postponed visual check.

PTY checks do not establish physical paint, frame pacing, or trackpad feel.
Ghostty is the intended physical-terminal target, but its acceptance remains incomplete. Other terminals and multiplexers remain unqualified, outside this local-candidate requirement.
The full [acceptance checklist](ACCEPTANCE.md) is not complete.

## Boundaries

The local candidate targets macOS arm64. Historical source and comparison identities remain in their evidence records.
The viewer performs no editing, staging, telemetry, network requests, or unsolicited clipboard writes.
No maintainer contact, publishing, global installation, or multi-candidate benchmark runs form part of this work.
