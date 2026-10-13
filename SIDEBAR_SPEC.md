# Changed-file sidebar: implementation specification

Date: 2026-09-22.
Status: implemented. The [implementation report](SIDEBAR_REPORT.md) records checks and limits.

The user approved the feature boundary and the exact directory hierarchy.
The controls and dimensions below define the first implementation.
This specification supersedes the file-picker proposal in `SPEC.md` for this feature only.

## 1. Purpose and scope

The sidebar helps readers locate changed files and understand the size of each change.
It belongs to the existing application, beside the existing diff.
It uses the current themes and terminal renderer.

The first version includes:

- Changed files under their actual directory paths.
- One folder row per directory level, without folder compaction or root removal.
- Expand/collapse, file status, file counts, and folder totals.
- Keyboard navigation, mouse selection, and independent tree scrolling.
- Show/hide behavior and automatic hiding in narrow terminals.
- Progressive updates from completed file records.

Search, annotations, staging, filesystem browsing, settings persistence, and divider dragging are outside this version.
Integration with another application and new platforms remain separate work.
The long performance campaign remains paused.

## 2. Existing behavior to preserve

The current resize code preserves the source line, side, text position, and approximate screen row.
The sidebar must reuse this behavior for window resizing and visibility changes.
Automatic layout remains split at 120 available diff columns and unified below that width.
An explicit split or unified setting remains explicit.

The patch retains its original file order.
Existing next/previous-file commands retain that order, including metadata-only files.
The tree has its own directory order, but file activation resolves to the original file ID.

Source bytes, source numbers, line pairing, syntax colors, inline emphasis, and storage budgets remain unchanged.
The highlighting worker remains the existing worker.
The application retains one terminal owner and one input loop.

## 3. File identity, hierarchy, and counts

The tree describes the supplied patch, not the current filesystem.
It performs no directory scan and opens no working-tree source files.
Hidden paths and dependency paths remain visible when the patch contains them.

Each leaf references the existing document-local `FileIndex.id`.
Neither a displayed name nor a visible row number identifies a file.
Repeated patch entries with the same path remain separate leaves.
Equal-path leaves retain patch order and show an occurrence suffix to distinguish them.

Directory identity uses the original path bytes and parent directory identity.
Display labels are not reversible: `safe_label` escapes controls and invalid UTF-8.
The implementation must retain the effective raw path alongside its existing display label.
This is a narrow metadata extension, not a change to source parsing or diff computation.
The chosen path is the old path for deletions and the new path for other changes.
No path canonicalization, case folding, filesystem lookup, or parent-component resolution occurs.
Only the patch path separator creates directory levels.
Unusual path components remain literal data, never filesystem instructions.

Folders precede files within each directory.
Names use deterministic, case-sensitive byte ordering, with file ID as the duplicate tie-breaker.
Folder and file entries with the same name remain distinct.
Tree labels escape terminal controls and literal escape-like names without merging their identities.

File counts come from parsed additions and deletions, not another scan of patch prefixes.
Folder totals include every descendant file, regardless of expansion or scrolling.
Totals use checked arithmetic and never wrap silently.
Binary entries show their binary status instead of a misleading textual `+0 -0` claim.
Metadata-only entries remain selectable with their status and zero textual counts.
Renames use the destination hierarchy. The diff header retains the existing old-to-new path display.

## 4. Layout and visibility

The title, footer, and help belong to the whole application.
The tree and diff occupy adjacent rectangles between the title and footer.
The tree starts with a fixed 32-column width and a one-column divider.
The sidebar requires at least 64 remaining diff columns and ten terminal rows.
Thus the initial sidebar threshold is 97 columns.
Below that threshold, the tree hides without erasing its expansion or selection state.
The existing minimum terminal size remains 40 columns by ten rows.

Default visibility is automatic: show the tree for at least two completed files when space permits.
An explicit visibility preference overrides automatic file-count behavior, but not the minimum space requirement.
The public `--sidebar` and `--no-sidebar` flags select those initial preferences and conflict with each other.
Interactive changes last only for the process.

Opening or closing the tree changes available diff width through the existing reflow path.
Reflow occurs before drawing or hit testing the new layout.
Mouse coordinates become diff-local coordinates only after a bounds check.
Clicks on the tree, divider, title, or footer must not select a source line.
Very small windows clear stale hit regions and retain the last valid reading anchor.

Each tree row occupies one terminal row.
Long labels truncate visibly at grapheme boundaries without overwriting status or counts.
Indentation preserves directory depth. Excessive indentation is clipped with a visible overflow cue.
The focused row and current diff file have distinct indicators, not color alone.
The footer changes its control hints with keyboard focus.
Errors and incomplete-input status take precedence over ordinary hints.

## 5. Controls and state transitions

The diff retains its existing controls, including Tab for the active source side.
The new `f` command means Files and appears in the footer and help.
It is an application shortcut, not a claim of Vim compatibility.

| Input and state | Result |
| --- | --- |
| `f`, diff focused | Show the tree and focus the current file. Expand its ancestors. |
| `f`, tree focused | Hide the tree and return focus to the diff. |
| `f`, insufficient width | Keep diff focus and show the required-width explanation. |
| Escape or Tab, tree focused | Return focus to the diff without changing its reading position. |
| Up/Down or `k`/`j`, tree focused | Move one visible tree row. Do not activate files. |
| Page Up/Down, tree focused | Move by the visible tree height, bounded at either end. |
| Home/End, tree focused | Select the first/last visible tree row. |
| Right, folder selected | Expand it, or select its first child when already expanded. |
| Left, tree focused | Collapse the selected expanded folder, otherwise select its parent. |
| Enter or Space, folder selected | Toggle that folder. |
| Enter, file selected | Jump to that file header and return focus to the diff. |
| Left click on a folder | Focus the tree and toggle that folder. |
| Left click on a file | Activate that file and return focus to the diff. |
| Wheel over the tree | Scroll only the tree, with its cursor kept within visible rows. |
| Wheel over the diff | Use existing diff scrolling. |
| `?`, either focus | Open existing help with the relevant tree controls. |
| `q` or Ctrl-C | Use existing quit or cancellation behavior. |

Escape closes help before it changes focus.
Other existing diff-only movement commands do nothing while the tree has focus.
Opening the tree selects the current file, but does not move the diff.
Activation jumps within the existing document. It does not reload source or clear style caches.
The current file follows the diff reading cursor, including metadata-only file headers.

Manual collapse persists until the user expands it or navigates to a different descendant file.
Merely redrawing the current file must not reopen its collapsed parent.
Diff file changes reveal the destination in the tree when the diff has focus.
Tree navigation retains its own cursor without changing the current diff file.

## 6. Loading, empty input, and failures

Only completed file records enter the tree.
New arrivals retain existing file IDs, folder IDs, collapsed state, and selection.
Insertion before the cursor does not select a different entry.
The tree heading identifies totals as partial while input loads.
A later input failure leaves completed files accessible and identifies the partial result.
Tree navigation never retries parsing or conceals an input error.
Empty input keeps the existing empty view. Explicit tree display shows an empty-state label.

## 7. Rust structure and implementation limits

The implementation stays in the existing Cargo package.
It adds no dependencies, global state, asynchronous runtime, plugin framework, or generic widget framework.

| Responsibility | Intended home |
| --- | --- |
| Stable node IDs, hierarchy, totals, visible rows, expansion | `src/tree.rs` |
| Tree drawing, clipping, row hit regions | `src/render/sidebar.rs` |
| Focus, visibility, cursor, file activation, reading-position coordination | `src/app.rs` or a focused child module |
| Screen rectangles and origin-aware diff drawing | Existing render/layout boundary |
| Key and mouse routing, CLI flags, terminal lifecycle | `src/main.rs` |
| Effective raw path retention | Existing parsed/indexed file metadata |
| Terminal-process acceptance checks | `tests/process/sidebar_contract.rs` |

Small `FileId` and `NodeId` types prevent accidental index interchange at new tree boundaries.
Enums represent node kind, focus, and visibility preference.
Ordinary owned structs and standard collections express ownership.
Parent links use IDs, without reference cycles or shared mutable interior state.
Traversal uses an explicit stack so deep paths do not consume the call stack.
New visibility stays private or `pub(crate)` unless an existing public boundary requires otherwise.
No reusable trait is necessary without a concrete second use.

File publication updates metadata and ancestor totals without inspecting source text.
Visible-row projection updates after structural changes, not during every coloring frame.
Publication batches must not rebuild the entire hierarchy for each arriving file.
Tree drawing visits only the visible rows.
The tree retains IDs and path metadata, not copied diff bodies or syntax state.
Memory grows with files, directories, and path bytes, not source-line count.
This is a structural requirement, not a new measured memory guarantee.

New paths propagate errors with context instead of adding panics or unchecked assumptions.
Existing unsafe-code prohibition, formatting, and Clippy gates remain in force.
No unrelated cleanup or dependency change belongs in this feature.

## 8. Implementation sequence and acceptance

1. Preserve the current source and executable identities before runtime changes.
2. Add raw-path retention and the pure tree model with its identity checks.
3. Adapt diff drawing and mouse translation to an explicit rectangle.
4. Add the sidebar, focus handling, and progressive publication updates.
5. Run the existing checks and the focused acceptance cases below.
6. Review the changed Rust code and inspect both theme previews.
7. Record the resulting executable, evidence, limits, and deferred performance work.

Each stage remains small enough for a direct diff review.
The rectangle change must pass existing rendering checks before sidebar behavior depends on it.

| Acceptance case | Required evidence |
| --- | --- |
| Real multi-file Git patch | Expected hierarchy, statuses, counts, and destinations, including renamed, deleted, binary, and mode-only entries |
| Identity edge cases | Duplicate paths, file/directory prefix collisions, quoted names, control bytes, and Unicode remain distinct and selectable |
| Actual keyboard and mouse events | Focus, collapse, activation, independent scrolling, boundaries, and terminal restoration through a real PTY |
| Reflow and translation | Exact source text, side, and text anchor through show/hide, rapid resize, wrap, and split/unified changes |
| Rendering compatibility | Hidden-sidebar diff cells match the prior renderer, except explicitly changed application hints and help |
| Progressive input and failure | Existing selection survives incoming files. Partial counts and errors remain visible. Earlier files remain readable. |
| Many files and deep paths | Bounded visible drawing, correct last-file navigation, no recursive traversal failure, and clean cancellation |

Tests use the real parser, worker, renderer, files, and process where those behaviors matter.
Small pure-model tests cover identity and hierarchy invariants that terminal output alone cannot establish.
No mocked parser or fabricated successful worker replaces the production paths.
Legacy fullscreen checks explicitly use `--no-sidebar` where geometry requires it.
New checks cover default sidebar behavior so the disabled configuration cannot conceal failures.

Both light and dark previews come from actual emitted terminal cells.
Those previews do not establish physical terminal rendering or trackpad behavior.
Physical terminal inspection remains separately identified until it occurs.
The existing large-corpus evidence and measurement executables remain unchanged.
They do not become measurements of the sidebar build.

## 9. Current evidence and scope correction

The current qualified executable SHA-256 is:

`6cf039592f9bc400ee9e90ea510eb4b2daa48ebc24ae1b75371d7d6cc16e2eb3`

Relevant current boundaries are `App::reflow`, `render::draw`, `FileIndex`, `FileMetadata`, and the main input loop.
The existing drawing code assumes a zero-origin, full-width diff.
The existing metadata stores escaped path labels, not original path identities.

The earlier statement that the parser stays unchanged needs a qualification.
Source parsing and interpretation stay unchanged.
Effective raw-path retention requires a metadata change at the parse boundary.
This change needs explicit path regressions and unchanged canonical source output.
This specification defines the feature boundary. The implementation report owns results and remaining qualification work.
