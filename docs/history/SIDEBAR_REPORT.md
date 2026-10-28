# Changed-file sidebar: implementation report

Date: 2026-09-22.

The sidebar is implemented against [the agreed specification](../design/SIDEBAR_SPEC.md).
It belongs to the existing application. It does not introduce an integration layer or another terminal owner.

## Delivered behavior

- Exact patch directory levels, without directory compaction or filesystem traversal.
- Collapsible folders, file statuses, textual additions/deletions, and descendant totals.
- Separate tree selection and current-file indicators.
- Keyboard navigation, file activation, mouse selection, and independent wheel scrolling.
- Automatic display for multiple files, explicit visibility flags, and narrow-window hiding.
- Stable identities and selection during progressive file publication.
- Existing themes and source-position preservation through changes in available diff width.

The tree uses raw path components and original file IDs.
Deleted files use their old path. Other changes use their new path.
Duplicate paths remain separate entries, with visible occurrence suffixes even when labels truncate.
Control bytes and literal escape sequences remain distinct.
Binary entries show their binary status. Metadata-only entries remain selectable.

The tree sorts folders before files. The diff and next/previous-file commands retain patch order.
Folder browsing does not restart highlighting. File activation uses the existing document and style cache.

## Implementation boundaries

| Responsibility | Source |
| --- | --- |
| Stable tree, totals, expansion, visible rows | `src/tree.rs` |
| Focus, visibility, viewport preparation, activation | `src/app/sidebar.rs` |
| Key and mouse routing | `src/app/controls.rs` |
| Sidebar cells and label clipping | `src/render/sidebar.rs` |
| Diff rectangle translation | `src/render.rs` |
| Raw path retention and memory accounting | `src/model.rs`, `src/storage/metadata.rs`, `src/storage.rs` |

The implementation adds no dependencies, unsafe code, runtime, worker, or global state.
The tree retains file and directory metadata. It does not retain source bodies or syntax state.
Visible rows update after structural changes. Drawing visits only the displayed rows.
Hierarchy traversal uses an explicit stack. Totals use checked arithmetic.
These properties are not a measured total-memory guarantee.

## Checks

Evidence directory: `evidence/sidebar-v1/`.

| Check | Result | Evidence |
| --- | --- | --- |
| Rectangle-only refactor, before sidebar connection | 12 rendering contracts passed | `rectangle-tests.log` |
| Normal Cargo suite | 81 passed, three opt-in checks excluded | `qualification.log` |
| Ordinary release, complete enabled process suite | 52 passed | `ordinary-qualified-process.log` |
| Preserved baseline comparison | 21 views and three canonical source outputs matched | `baseline-qualified-compatibility.log` |
| Formatting and application Clippy | Passed with warnings treated as errors | `clippy-qualified.log` |
| Dark and light emitted-cell previews | Inspected | `qualified-previews/sidebar-review-dark.png`, `qualified-previews/sidebar-review-light.png` |

The normal suite excludes two historical large-input comparisons and the explicit pre-sidebar comparison.
The pre-sidebar comparison ran separately against the preserved executable.
It compares every body cell, including foreground and background colors. It excludes the intentionally changed application footer.

The eight new process contracts cover these behaviors:

- Real Git additions, deletions, renames, binary changes, and mode-only changes.
- Exact nested hierarchy, counts, activation, focus, and persistent manual collapse.
- Independent scrolling across 300 files, with reversed patch order and exact destinations.
- Duplicate filenames with clipped labels and distinct destinations.
- Unicode, combining marks, tabs, wrapping, mouse translation, and exact text anchors.
- Sidebar show/hide, rapid width changes, tiny-window recovery, and automatic split thresholds.
- Incremental insertion before the selected entry, followed by a partial input failure.
- Empty input, conflicting flags, keyboard controls, and terminal restoration.
- Complete repaint after coalesced resize signals return to the original dimensions.

Three parser-backed model checks cover identities, aggregation, escaped bytes, and 2,000 directory levels.
Existing source, syntax, inline emphasis, worker, navigation, and failure checks remain in the suite.

Development failure logs remain available.
Corrections addressed resize-message handling, an outdated help-text assertion, and a relative fixture path in the new test driver.
Self-review also removed redundant viewport preparation and unnecessary highlighting restarts during tree navigation.
Compact tree help now fits the minimum supported height.
The pre-existing vendored Syntect lifetime warning remains unchanged.

### Resize correction

A rapid-resize check exposed missing cells after a shrink-and-grow round trip.
The terminal buffer lost clipped columns, but the renderer sometimes observed only the original final dimensions.
Its unchanged-size assumption then suppressed a complete repaint.

A deterministic regression pauses the real viewer, resizes its PTY, then resumes the viewer and compares every cell.
The driver checks the stopped process state before resizing.
This regression fails on the preserved pre-sidebar executable and passes after the correction.
The failure evidence is `coalesced-resize-preserved-baseline.log`.

Resize notifications now force Ratatui's fullscreen resize operation inside the existing synchronized update.
The operation reads current terminal dimensions and invalidates the previous cell buffer, even when dimensions match.
It does not change source layout or syntax state.
An initial `Terminal::clear` attempt required cursor-position replies and failed the PTY checks.
The final resize operation requires no cursor-position query. Both failure records remain available.

## Executables and preservation

The ordinary executable is `target/release/native-diff`.
Its preserved copy is `evidence/sidebar-v1/native-diff`.

SHA-256:

`b06a89565cc4f9e1b466cbf2a2d57d70bbe6cbf0f75051de1039a136e0c7f8f3`

The pre-sidebar executable remains at `evidence/sidebar-v1/baseline-native-diff`.
Its SHA-256 remains:

`6cf039592f9bc400ee9e90ea510eb4b2daa48ebc24ae1b75371d7d6cc16e2eb3`

The pre-change source archive is `evidence/sidebar-v1/baseline-source.tar.gz`.
The final source archive is `evidence/sidebar-v1/sidebar-source.tar.gz`.
The evidence checksum manifest is `evidence/sidebar-v1/SHA256SUMS`.
The earlier measurement executables and sealed evidence remain unchanged.

## Try the sidebar

In an existing terminal window, run:

```sh
/Users/ramos/oss/native-diff/target/release/native-diff patch /Users/ramos/oss/native-diff/fixtures/review.patch
```

Press `f` to focus Files. Use arrows to navigate. Press Enter to open a file.
Press Escape to return to the diff. With Files focused, press `f` to hide it.
The [README](../../README.md#changed-files) lists all controls and width requirements.

## Remaining qualification

The previews represent emitted terminal cells, not physical terminal screenshots.
Physical Ghostty or Kitty presentation, rapid real-window resizing, and trackpad behavior remain unchecked for this build.
No GUI automation or terminal launch occurred.

No accepted timing or sustained-memory campaign ran for this build.
The next measurement must declare sidebar visibility and available diff width.
The previous frozen workloads and executables retain their original meaning.
Their results do not qualify the sidebar build automatically.

The five-patch presentation recapture, diagnostic-feature suite, and clean installation qualification remain separate from these focused checks.
Application integration, search, copy, and release-platform qualification remain deferred.
