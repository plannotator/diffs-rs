# Embedding the diff engine

`diffs_rs::engine::Engine` owns one review, its view state, source storage, and background workers.
The `diffs` binary and [`examples/embedded_viewer.rs`](../examples/embedded_viewer.rs) use this same implementation.
The API is pre-1.0 and can change between minor versions.

## Dependency setup

The host must use Ratatui 0.30.

```toml
[dependencies]
diffs-rs = { version = "0.2", default-features = false }
ratatui = { version = "0.30.2", default-features = false, features = ["crossterm"] }
```

The default `dev-tty` feature makes Crossterm read keyboard input from `/dev/tty`, which the standalone viewer needs when a patch arrives on stdin.
Crossterm features are shared by the whole build, so a host that reads its own input should use `default-features = false`.
Enable the `clipboard` feature only if the host wants the native macOS clipboard.
Diagnostic crate features are not required for embedding.

The patched syntax engine comes from the forked crates `diffs-syntect`, `diffs-fancy-regex` and `diffs-two-face`. They are ordinary dependencies of `diffs-rs`, so the host needs no `[patch]` section.
Their library names stay `syntect`, `fancy_regex` and `two_face`. A host that also depends on upstream Syntect gets a second, separate copy whose types do not interoperate with the fork's.
Do not substitute upstream Syntect, Fancy Regex or two-face to make dependency resolution succeed.
The Git patch parser is the stock `libgit2-sys` crate with its bundled libgit2. The build fails if `LIBGIT2_NO_VENDOR` asks it to link a system libgit2.

## Ownership

The host owns the terminal, raw mode, alternate screen, mouse capture, signal handling, keyboard reader, outer layout, and application quit behavior.
The engine never reads the global keyboard, changes terminal modes, writes terminal output directly, or exits the process.
Only the host should read Crossterm input in the process.

Open with `Engine::open(input, options, wake_sender)`.
`Input` accepts a patch path, an owned file stream with a label, or the existing `LocalInput` file-pair or pinned-Git comparison.
Opening starts background work. Inspect `status()` as input arrives; successful `open()` does not mean the input has been parsed or verified.
Commands issued before the first file arrives return `InputEffect::Ignored`.

`Input::Stream` reads from the file's current offset and takes exclusive control of its shared read cursor and status flags until close.
Caller-held duplicates must not read, seek, or change those flags concurrently.
The patch reader temporarily enables nonblocking input and restores the original nonblocking bit when it finishes or closes.
It supports cancellation while waiting for a FIFO writer, an idle stream, or an incomplete line.
The safe descriptor-set implementation rejects descriptors at or above `FD_SETSIZE` before changing flags.
This is a Unix implementation. macOS arm64 is the most tested system; Linux runs in CI.

## Host loop

Use a bounded `std::sync::mpsc::sync_channel(1)` for wake notifications.
Each notification is a hint that work may be ready, not one notification per reply.
The example implements this order:

1. Clear the pending wake token before draining work.
2. Call `engine.poll()`. Each call drains a bounded number of replies so the host can handle input.
3. Redraw when `engine.needs_draw()` or the host's layout/focus has changed.
4. Call `engine.draw(frame, viewer_area)` inside the host's Ratatui draw closure. Propagate both backend and engine draw errors. After successful terminal output, call `engine.after_draw()` before processing further input or replies. This starts visible work after the text frame has been written and completes redraw bookkeeping.
5. Handle host commands first, then route viewer events or commands.
6. Wait for host input, a wake notification, or a bounded periodic tick.

The example uses a 100 ms maximum idle tick because progressive file publication also has a redraw deadline.
A host that waits only for wake tokens can miss that deadline after clearing a token early.
Continue polling and drawing when the viewer lacks keyboard focus; otherwise colors and source readiness cannot become visible.

The draw rectangle is intersected with the frame. Engine drawing does not change cells outside that intersection.
The supplied rectangle controls reflow. Terminal resize events request a redraw but do not replace the pane dimensions with the full terminal dimensions.
Supply the current rectangle on every draw, including when a pane moves without changing size.

### Without chrome

By default the engine draws a label row at the top of its rectangle, a footer with status and key hints at the bottom, and a help overlay on `?`.
Set `Options { chrome: false, .. }` when the host owns those rows:

- The diff, its file headers and the Files sidebar fill the whole rectangle. The rectangle needs at least 40 columns × 8 rows instead of 40 × 10.
- `Command::Help` and the `?` key are ignored, so the help overlay never opens.
- `draw_with_controls` ignores its `ReadingControls`; the host shows its own hints and notices.
- The loading, failure and too-small placeholders name no keys.

Read the state the footer would have shown from `status()`: `loading`, `coloring`, `preparing_source`, `source_failed` and `error`, and `files` for the count.
The review label, layout and theme are the host's own inputs and `options()`.
`chrome` is fixed for the engine's lifetime, and `replace()` keeps it.

### Terminal background

By default the engine paints its whole rectangle with the theme's background (#0a0a0a dark, #ffffff light) and its own text color.
A host whose other screens use the terminal's own colors can call `engine.set_terminal_background(true)?`:

- Empty rows, plain context lines, gaps, metadata and "no newline" rows, file headers, the Files sidebar, and with chrome the label row, footer and help draw with `Color::Reset` as background.
- Plain text, including syntax tokens the theme leaves uncolored, uses the terminal's default foreground. Other syntax colors stay.
- Added and deleted lines, changed words, hunk headers, the empty half of a split row, and the Files keyboard cursor row keep explicit colors. They come from the palette of `Options::appearance`, so set the appearance that matches the terminal: `Dark` for a dark terminal, `Light` for a light one.
- The engine cannot know the terminal's exact background, so those colors are mixed for it: hunk headers are stronger than on the theme's own background, and the dark tints start from a slightly lighter base than #0a0a0a, so they read on black, #1e1e1e or a blue-grey alike.
- Gutter markers, the cursor and selection indicators, and the search match keep their own colors and underlines.

It is off by default. It can be set before the first file arrives, stays set across `replace()`, and follows the `t` theme toggle.
It is a method rather than an `Options` field so that adding it does not break hosts that build `Options` without `..Options::default()`.

### Layout and the line between the sides

`Options::layout` picks the layout the review opens with, and `set_layout(layout)` changes it later:
`Split`, `Unified`, or `Auto`, which splits a pane 120 columns wide or wider.
It keeps the source line under the cursor on its screen row, as `Command::ToggleLayout` does, and `options().layout` reports it.

A split row shares its width between the old side, left of a one-cell line, and the new side.
`set_split_ratio(SplitRatio::from_permille(old))` gives the old side `old` thousandths of the width beside the line, from `SplitRatio::MIN` (100) to `SplitRatio::MAX` (900); the new side takes the rest.
The default, `SplitRatio::EVEN`, draws exactly as before, so a host that never calls it sees no change.

- Each side keeps at least `SplitRatio::MIN_SIDE_WIDTH` (20) cells, its gutter included. A pane too narrow for two such sides is split evenly.
- `split_ratio()` returns the ratio as it was set. A narrow pane draws the line nearer the middle without changing it, and draws the ratio again when it widens.
- The source line under the cursor keeps its screen row. Wrapped rows are measured again at the new widths, and horizontal scrolling reaches the end of the widest line on either side.
- Clicks and `row_at` take the side the line divides: a cell left of the line is old, right of it new, and the line itself counts toward the narrower side, or toward the new side when both are as wide.
- It can be set before the first file arrives or while the diff is unified, and stays set across `replace()`.

To let the person drag the line, find it with `split_line()`: its screen column in the last frame, or `None` before the first draw, while unified, and while the pane is too small.
It is drawn on rows of code; headings, hunk headings and gaps run across it.
While the pointer moves, set `split_ratio_at(column)`, the ratio that draws the line at that column, or as near as the limits allow.
It answers from the last frame's pane, so a burst of drag events before the next frame ends where the last event points.

```rust,ignore
// A press on the line starts a drag; each drag event moves it.
if engine.split_line() == Some(mouse.column) {
    dragging = true;
} else if dragging {
    if let Some(ratio) = engine.split_ratio_at(mouse.column) {
        engine.set_split_ratio(ratio)?;
    }
}
```

## Commands and selection

`handle_event(event, focused)` applies the viewer's default bindings.
Unfocused key events are ignored. Mouse coordinates are absolute terminal cells; events outside the last drawn pane are ignored.
The host decides whether a click changes outer focus and can withhold pointer events from the viewer.
`execute(Command)` bypasses key mapping for host-owned controls.
The default help/footer describe the standalone bindings; hosts that remap them must provide corresponding user guidance.

`selected_source()` returns an optional `SourceLocation`:

- `review` identifies this open occurrence within the process. It changes on replacement and is not a persistent revision identifier.
- `file` is the stable file occurrence within that review. `file(index)` exposes its paths, status and change counts.
- `side` identifies the selected original side.
- `old_line` and `new_line` are one-based original source coordinates of the selected record. A replacement's old and new records remain separate. A shared context record can carry both numbers.

Headers, gap controls, metadata-only rows, absent replacement halves and wrapped alignment padding return no source location.
Displayed escapes, tab cells and wrapped row offsets are not raw byte offsets. This API does not expose byte-column selections.
Use the review identity with stored host selections so an old selection cannot attach to a new review with the same path or file ordinal.

`execute(Command::SelectLines)` begins or fixes a source-line range. `ClearSelection` clears it.
Both commands are ignored while help or the file tree owns focus, or the diff pane is empty.
`selection()` returns its `SourceRange`; `selection_extending()` reports whether navigation extends it.
The range stores coordinates, not source ownership. Reflow preserves it. Explicit side changes, another file, and review replacement clear it.
Withdrawal of an affected checked source also clears it.

The default mouse route supports Shift-click within the same file and side. The host still controls event routing and keyboard bindings.
The standalone maps `V` and Escape to these commands. `handle_event` alone does not add those keyboard bindings.
Headers and padding never become endpoints. Ranges can span unavailable patch-only gaps; `source_read` rejects those gaps during streaming.

### Host-driven position and selection

`reveal(file, side, line)` scrolls an original source line into view and puts the cursor on it, on that side.
`select_range(range)` makes a fixed selection over `range.first..=range.last`, as if the user had selected it and finished.
The cursor ends on `range.last`; `selection()`, `selected_source()` and `source_read()` then report it as they would a user selection, and `selection_extending()` is false.
Both give the diff keyboard focus and close the help overlay.

Both return an error, and change nothing, when the line is not displayed: the file is not in the review, the line is outside the file or on the other side only, or it sits in a gap that the patch does not supply and nobody has revealed.
Checked context counts once it is revealed. For `select_range` only the two endpoints must be displayed. As with a user selection, the lines between them can cross a gap, and `source_read` then rejects the range.
`select_range` also rejects a range from another review or with `first` of 0 or greater than `last`.
Moving the cursor to another file or side clears the selection, as it does for a user selection.

### Gutter markers

`set_markers(&[Marker])` replaces every marker with a new set; an empty slice clears them.
A `Marker` is a `SourceRange` plus a Ratatui `Style`:

```rust
use diffs_rs::{engine::{Marker, Side}, reading::SourceRange};
use ratatui::style::{Color, Style};

engine.set_markers(&[Marker {
    range: SourceRange { review: engine.review_id(), file: 0, side: Side::New, first: 12, last: 14 },
    style: Style::new().fg(Color::Yellow),
}])?;
```

Each visible row whose source line on `range.side` falls in the range shows `▌` in the first gutter cell, patched with `style`. Continuation rows of a wrapped line show it too.
In the split layout a marker shows on its own side's half. A unified row shows both line numbers, so a marker on either side applies.
The cursor `▎` and selection `┃` indicators take precedence in that cell. Where markers overlap, the later one in the slice wins.
The set is indexed per file and side when it is replaced, so each drawn row costs one lookup however many markers exist.

Markers for lines that are not displayed, or for files that have not arrived yet, draw nothing until they are. Markers can be set before the first file arrives.
`set_markers` rejects the whole set, keeping the previous one, if any range belongs to another review or has `first` of 0 or greater than `last`. It also fails after `close()`.
`replace()` and `close()` drop the markers. Set them again for the new review with its `review_id()`.

### File marks

`set_file_marks(&[FileMark])` replaces every file mark with a new set. A `FileMark` decorates the Files sidebar row of a file:

```rust
use diffs_rs::engine::{Badge, BadgeColor, FileMark};

engine.set_file_marks(&[FileMark {
    path: "src/lib.rs",
    badge: Some(Badge { text: "✓", color: BadgeColor::Added }),
    dimmed: true,
}])?;
```

- `path` is the file's `file(index).path`: the new path, or the old path of a deleted file. Every file with that path gets the mark, so a path that appears twice in a `git log -p` patch is marked twice.
- `badge` is one or two terminal cells of printable text. `BadgeColor` names a theme color (`Text`, `Muted`, `Accent`, `Added`, `Deleted`, `Modified`, `Renamed`), so the badge follows the dark and light themes.
- `dimmed` draws the row's name, status and counts in the muted color. The badge keeps its color.

From the first call, every row keeps a badge column of three cells (a gap and two badge cells) between its counts and the divider, so rows with and without a badge stay aligned. Long names are shortened to make room; the badge is never cut.
The column stays, even for an empty set, until `replace()` or `close()`. Call `set_file_marks(&[])` when the review opens if the first badge should not shift the rows.
A folder row shows a badge when every file under it has the same badge, text and color, and is dimmed when every file under it is dimmed. The engine computes this, collapsed folders included, because the host does not see the tree's folders. While a patch is loading, folders count the files that have arrived.

Marks for paths that are not in the review draw nothing, and marks can be set before the first file arrives. Later marks for the same path win.
`set_file_marks` redraws the sidebar only; the diff does not reflow. It costs one hash of each path in the set plus one pass over the files already in the tree, so it can run on every toggle.
It rejects the whole set, keeping the previous one, if a badge is empty, wider than two cells or contains control characters. It also fails after `close()`.
`replace()` and `close()` drop the marks and the badge column. The standalone viewer never sets marks.

### The cursor's row and the row at a cell

`cursor_row()` returns the row under the cursor as a `DiffRow`. It answers on rows where
`selected_source()` has no line: a file heading, a hunk heading, a gap, a binary or mode-only
file's note, an empty side of a split row, a "No newline" mark, and each wrapped part of a row.
Use it to tell which file the cursor is in.

- `review` is the review the row is from, as `review_id()` reports it.
- `file` and `path` name the file the row belongs to, as `file(index)` does.
- `kind` is a `DiffRowKind`: `Header`, `StickyHeader`, `Hunk(index)`, `Gap`, `Metadata`, `Code` or
  `NoNewline`. The enum is `#[non_exhaustive]`, and so is `DiffRow`.
- `hunk` is the index of the file's hunk the row is part of: on a hunk's heading, the hunk's own
  lines and its own "No newline" marks. Headings, gaps and notes have none, and neither do
  context lines that a gap revealed or their "No newline" mark: they are in no hunk.
- `part` is the wrapped part of the row, 0 for its first screen row.
- `old_line` and `new_line` are the original lines the row shows. A split row takes the old line
  from its left cell and the new line from its right cell, and either can be empty. Rows that
  are not code have neither.
- `side`:
  - Split: the pane the cursor is in. For `row_at`, the pane of the cell, by the rule a click
    uses, so the column between the panes counts as the side a click there would pick.
  - Unified, on a line: the line's side, as `selected_source()` reports it.
  - Unified, on a row with no line: the side the cursor is on, and `New` on the sticky heading.

`row_at(column, row)` answers the same for a screen cell in the last frame, as a click there
would find it. The top row of a file whose rows scroll under its heading is `StickyHeader`; a
click there does not move the cursor.

Neither call moves the cursor, the view or the selection, so a host can ask on every event.
`cursor_row()` reads the cursor itself, so it is right at once, before the next frame. `row_at`
reads the last frame, as a click does.

Both return `None` when there is no true answer:

- for a diff with no files;
- after `replace()`, until the new files arrive (and, for `row_at`, until they are drawn);
- on a revealed context line whose source was withdrawn.

`cursor_row()` also returns `None` before the first file arrives. `row_at` also returns `None`
before the first draw, while the help is shown, outside the diff's area and below its last row.

```rust
use diffs_rs::engine::{DiffRowKind, Engine};

/// The file a "mark viewed" key acts on: the one the cursor is in, on any row.
fn file_under_cursor(engine: &Engine) -> Option<usize> {
    engine.cursor_row().map(|row| row.file)
}

/// Whether a press at a cell lands on code, so a host can start a line selection there.
fn on_code(engine: &Engine, column: u16, row: u16) -> bool {
    engine
        .row_at(column, row)
        .is_some_and(|row| matches!(row.kind, DiffRowKind::Code))
}
```

### Source reading and search

`source_read(SourceRange)` prepares an original-side range with inclusive source-line coordinates.
Its `write_to` operation streams original bytes to a caller-owned writer with cancellation checks.
`SourceRead::check` checks review and checked-source validity without reading bytes. The caller checks it again before publishing prepared output.
Checked full sources can supply hidden context. Patch-only ranges reject missing context instead of joining separated source fragments.
The range preserves tabs, invalid bytes, CRLF/LF, and final-newline state. Display sanitization does not alter the exported bytes.

`search_read(query)` captures diff text and the current source origin after indexing completes.
The scope contains supplied patch records and explicitly expanded context, not hidden complete files.
The caller runs `SearchRead::find` outside its input thread and owns cancellation, reply acceptance, and worker shutdown.
`SearchRead::check` checks scope validity without scanning source text.
`show_search_match` checks review/scope identity, reveals the source position, and marks the current match.
`clear_search_match` removes that emphasis without changing the reader's position.
The caller must reject superseded replies before `show_search_match`, even if those replies have a valid review identity.

The engine supplies no search prompt, extra keyboard reader, clipboard action, or search-worker loop.
`draw_with_controls` optionally accepts borrowed `ReadingControls` for caller hints, help, and status.
Input failures and source progress retain their status alongside these notices.
The existing `draw` method does not add standalone search bindings.
The host must cancel and join its reading jobs before replacement or close, then release retained source readers.
The standalone owns its save prompt, private output, no-clobber publication, and cleanup. The engine does not create export destinations.
The standalone also owns clipboard preparation, its helper process, and delivery acceptance.
Its optional `clipboard` feature enables macOS plain-text publication. Engine-only consumers can disable default features to omit that dependency.
The engine does not launch clipboard helpers or handle the standalone `y` key.

## Failure and shutdown

`status().loading` refers to patch indexing. `coloring` and `preparing_source` refer to the current visible rows, not the whole review.
An empty or too-small pane can have neither visible flag set without implying that all source has been colored.
`source_failed` records an unavailable checked source during this review; existing patch content remains readable with its declared limited context.

An input error appears in `status().error`. Successfully published files remain readable, and the host can replace the failed review.
An unexpected syntax-worker failure, source read failure, or rendering failure returns an error from the corresponding engine call.
The host should stop using that review, report the error, and call `close()` before opening another.

`replace(input)` closes and joins the old review before starting new workers and channels.
It preserves current layout, theme, wrapping, line-number, and sidebar preferences, but not selection or expanded context from the previous review.
No old-review replies can enter the new channels.

`close()` cancels and joins both workers, releases review-owned sources and view state, and is idempotent.
Previously delivered input-operation errors do not make a successful cleanup fail.
Cleanup failures and worker panics are reported, including simultaneous cleanup errors.
Call it explicitly so the host can report those errors. `Drop` is best-effort cleanup, not an error-reporting channel.
The host must also preserve cleanup errors when its own event loop has already failed; the example demonstrates this.

Cancellation cannot interrupt one in-progress syntax-parser call or an operating-system filesystem call.
No universal shutdown-latency bound is promised.
The engine does not detach unfinished workers or leave old reviews running after a successful close.

Closing a review does not promise that process RSS returns to its opening value: the macOS allocator can keep freed regions resident.
After a large patch, closed RSS reached about 590 MiB even though live heap had returned to its earlier size.
On a three-cycle schedule of file-pair, Git, and small-patch reviews, the example host stayed under 34 MiB held RSS, and closed RSS rose 0.41 MiB across the cycles.

## Try the host example

From a clone of this repository:

```sh
cargo run --release --locked --example embedded_viewer -- fixtures/review.patch fixtures/position/review.patch
```

Mix patch paths with repeatable `--files OLD NEW` and `--git REPO BASE HEAD` groups. Reviews open in argument order:

```sh
cargo run --release --locked --example embedded_viewer -- fixtures/review.patch \
  --files fixtures/position/old/review.ts fixtures/position/new/review.ts \
  --git /absolute/path/to/repository BASE_COMMIT_ID HEAD_COMMIT_ID
```

Use full commit IDs to keep Git inputs fixed across repeated opens. A symbolic revision resolves again on each open. File pairs capture fresh bytes on each open; editing the live files does not change an already captured review. Arguments are validated before acquiring the terminal.

The left pane belongs to the host. Tab switches outer focus; `r` opens the next review, `c` closes the review without exiting, `o` reopens it, and `q` exits the host.
`m` marks the selection (Shift-click to make one) or the cursor line with a gutter marker, and `'` selects the last mark again with `select_range`.
`v` marks the cursor's file viewed, or unmarks it, with `set_file_marks`: a `✓` and a dimmed row in the Files sidebar, which `f` shows.
`--no-chrome` turns off the viewer's label row, footer and help; the host then draws its own status row from `status()` below the viewer.
These are example-host controls, not additional viewer bindings.
The example uses the current terminal and does not open windows or change preferences.
`State: source unavailable` distinguishes degraded full-source preparation from a ready review. Published patch content remains readable, and the host can replace the review. Unexpected worker/storage errors and failed cleanup terminate the example with an error; they are not treated as recoverable input errors.

The integration checks cover bounded cells and styles, source coordinates, focus, checked context, replacement, FIFO cancellation, and terminal restoration.
A separate Cargo-root build checks the dependency recipe; same-package examples alone do not prove dependency integration.
These checks establish emitted cells and lifecycle behavior. They do not establish physical screen-paint timing or terminal font behavior.
