# Embed the local diff engine

`native_diff::engine::Engine` owns one review, its view state, source storage, and background workers.
The standalone viewer and `examples/embedded_viewer.rs` use this same implementation.
This API is a local integration candidate, not a stable published crate API.
[CURRENT_STATE.md](CURRENT_STATE.md) separates the completed local-engine milestone from the current candidate.
The final four resource cohorts passed evidence inspection, with resource-target misses retained. Both isolated installations passed.
Native clipboard, physical-terminal acceptance, and the final package remain pending. The final notice inventory is complete, with its upstream licensing caveat retained.

## Dependency setup

Use a path dependency on the source package. The host must use Ratatui 0.30.
Cargo does not inherit dependency patches from a dependency's manifest.
The consuming workspace root therefore needs all three patches:

```toml
[dependencies]
native-diff = { path = "../native-diff" }
ratatui = { version = "0.30.2", default-features = false, features = ["crossterm"] }

[patch.crates-io]
syntect = { path = "../native-diff/vendor/syntect" }
fancy-regex = { path = "../native-diff/vendor/fancy-regex" }
libgit2-sys = { path = "../native-diff/vendor/libgit2-sys" }
```

Adjust these paths for the host's workspace. Retain the vendor directories and their notices.
The patches provide required syntax-engine behavior and the corrected local Git parser.
Do not substitute registry or system versions to make dependency resolution succeed.
Keep the host's generated lockfile and use `--locked` for subsequent builds.
Diagnostic crate features are not required for embedding.

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
This is a Unix implementation. The currently tested system is macOS arm64; other platforms remain unqualified.

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
These APIs have functional evidence and finite final-build resource checks.
The [resource record](/Users/ramos/Documents/Codex/2026-09-28/native-diff-production-hardening/RESOURCE_RESULTS.md) retains response results, complete-scan costs, memory misses, and cancellation limits.
Native clipboard and physical-terminal acceptance remain separate pending requirements.

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

Closing a review does not promise that process RSS returns to its opening value. In the prior three-cycle patch-host schedule, closed live heap was identical across cycles, but macOS allocator-held empty regions retained substantial resident memory. Closed RSS reached about 590 MiB. See the [prior resource results](/Users/ramos/Documents/Codex/2026-09-28/native-diff-local-engine-milestone/RESOURCE_RESULTS.md).

A preceding three-cycle host qualification covered a 12,000-line file pair, a pinned 12-file Git review, and small-patch replacement.
Each ordinary and diagnostic process passed 96 stages. Closed live malloc heap was 1,116,528 bytes at all three diagnostic checkpoints, without retained regular file descriptors.
Ordinary first-close RSS medians rose from 33.516 to 34.188 MiB.
The [historical full-source results](/Users/ramos/Documents/Codex/2026-09-28/native-diff-production-hardening/FULL_SOURCE_RESOURCES.md) identify those earlier builds and diagnostic limits.

The final ordinary host passed the same 96-stage schedule across three cycles. Maximum held RSS was 33.98 MiB, with a 42.00 MiB sampled process-group peak.
Closed RSS medians rose 0.41 MiB. Closed states retained no numeric regular-file descriptor, named private source, or owned child.
The [final resource record](/Users/ramos/Documents/Codex/2026-09-28/native-diff-production-hardening/RESOURCE_RESULTS.md) identifies the exact host and workload.
These finite observations do not establish indefinite stability, live-heap size, a largest-input capacity bound, or a total-memory guarantee.

## Try the host example

From the package root:

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
These are example-host controls, not additional viewer bindings.
The example uses the current terminal and does not open windows or change preferences.
`State: source unavailable` distinguishes degraded full-source preparation from a ready review. Published patch content remains readable, and the host can replace the review. Unexpected worker/storage errors and failed cleanup terminate the example with an error; they are not treated as recoverable input errors.

The integration checks cover bounded cells and styles, source coordinates, focus, checked context, replacement, FIFO cancellation, and terminal restoration.
A separate Cargo-root build checks the dependency recipe; same-package examples alone do not prove dependency integration.
These checks establish emitted cells and lifecycle behavior. They do not establish physical screen-paint timing or terminal font behavior.
