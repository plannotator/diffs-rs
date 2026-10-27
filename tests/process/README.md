# Rendering contracts

`render_contract.rs` checks the release executable through the shared PTY harness.
Expected source rows and emphasis spans are explicit test data. They do not come from the viewer or its pairing algorithm.
Git generates patches from retained before/after strings with fixed diff settings and no user configuration.

## Coverage

`engine_host_contract.rs` runs the generic embedding example in an actual PTY. Six opt-in checks cover ordered patch/file-pair/pinned-Git reviews, close/reopen, replacement during input or coloring, checked-context expansion, source coordinates across resizing, idle progress without viewer focus, missing lazy sources, FIFO cancellation and Git-child reaping. The shared harness checks terminal restoration. These checks do not qualify persistent-host memory or physical screen behavior.

Build and select the exact example explicitly:

```sh
cargo build --release --locked --example embedded_viewer
NATIVE_DIFF_HOST_BINARY=/absolute/path/to/diffs-rs/target/release/examples/embedded_viewer \
cargo test --release --locked --test process engine_host_contract -- --ignored --test-threads=1
```

`context_expansion_contract.rs` runs both local source commands through actual PTYs.
It checks clicks, Enter, exact exposed endpoints, original change jumps, final-newline markers, and captured bytes after live files change.
Wrapped Unicode text retains its byte anchor through layout, sidebar, and width changes.
A three-file Git case checks lazy pinned sources after a branch moves, file order, tree activation, and retained expansion.
`tests/context_expansion.rs` reconstructs both sources after expansion and checks every exposed byte and number after each action.
It also checks unavailable sources, stale replies, and reverse-order readiness plus expansion across 256 files with later publication.
The real-worker color checks compare mixed patch/context requests against sequential full-file colors and reject a wrong-side context reference.

`local_source_contract.rs` runs the captured-file and fixed-Git input commands through real PTYs.
It checks stable captured text through navigation and resizing, readable patches after source failure, and nonzero failure exit.
It also stops an actual Git child before cancellation, then checks child reaping, temporary-file cleanup, and terminal restoration.
`tests/local_source.rs` checks real source bytes, disk indexes, omitted-range corruption, pinned revisions, Git file kinds, and input rejection.

`git_paths_contract.rs` checks 56 real Git rename/copy inputs, twelve malformed
headers, and a two-file interactive trace. It preserves exact safe-display names,
source bytes, numbers, newline markers, and retained-source export. The terminal
case checks both layouts, exact file-header destinations, and clean exit.
Names cover leading/interior/trailing spaces, quotes, tabs, and Unicode.
Both `core.quotePath` settings and metadata-only/text changes are included.
The [correction report](../../docs/history/GIT_PATH_CORRECTNESS_REPORT.md) records the dependency
change, reproduced failures, and additional CRLF controls.

| Check | Independent expectation |
| --- | --- |
| Small source rows | Five examples across six layout, width, wrap, and theme states. Every content row, number, sign, gap, and ASCII emphasis cell must match. |
| Unicode boundaries | Explicit cells for CJK, combining accents, and tabs at wrap and horizontal-clip boundaries. |
| Horizontal boundaries | Repeated Right/Left input stops at exact displayed-cell limits. One Left moves immediately after excess Right input. Split sides, unified layout, resize, gutters, wrapping, shorter files, and metadata-only files are covered. |
| Unsafe text | Literal escape notation for control sequences and invalid bytes. Newline markers belong to the correct side. |
| Burst navigation | Every captured complete update preserves numbered source rows and valid emphasis. A bounded capture fails after 512 updates. |
| Side-specific syntax | Added and removed comment delimiters in C, Rust, and TypeScript. Added and removed multiline-string delimiters in Python. Both themes and revisits. |
| JSON syntax | Known string and Boolean colors from the attributed theme assets. |
| Unequal wrap widths | A changed word spans different rows in the two panes. Every emphasized continuation cell must match. |
| Replacement alignment | An import replacement and a reordered expression retain the same shared text on both sides through layout, theme, wrap, and width changes. |
| Hunk gaps and metadata | Explicit missing-context rows, source numbers, rename paths, modes, and binary-change labels. |

The small examples include insertions before replacements, removals before a replacement, repeated lines, blank lines, and empty source sides.
The checker also reconstructs both example sources from the explicit rows and checks their line numbers.
Emphasis uses half-open `(start, end)` byte spans on these ASCII examples.
The word-level contract keeps unchanged punctuation outside emphasis, such as the semicolon in `3;` becoming `5;`.
Both sides use the same old-to-new token comparison.
This contract does not require a minimal character edit or exact Pierre tokenization.

The PTY decoder accepts complete synchronized updates. It does not treat an arbitrary read boundary as a finished screen.
Pending rows can use plain styling. Final small-example checks require all specified emphasis cells.
Existing tests separately observe pending highlighting during rapid width changes.

## Run

From the project root, run the focused contracts:

```sh
cargo test --release --locked --test process render_contract -- --test-threads=1
```

Run the complete suite, including the downloaded large patches:

```sh
cargo test --release --locked -- --include-ignored --test-threads=1
```

Set `NATIVE_DIFF_CONTRACT_BINARY` to an absolute executable path to run the process suite against a preserved build.
Every current-viewer command uses the shared `viewer_binary()` resolver, including CLI, PTY, shell-pipeline, and opt-in checks.
Without this override, the resolver uses Cargo's test-built executable.
Explicit reference paths remain separate, including `NATIVE_DIFF_BASELINE_BINARY`.
An invalid override fails the check instead of silently using the Cargo executable.
To protect a preserved build from replacement by Cargo, use a separate target directory:

```sh
NATIVE_DIFF_CONTRACT_BINARY=/absolute/path/to/preserved-native-diff \
cargo test --release --locked --target-dir /absolute/path/to/test-target --test process -- --test-threads=1
```

Set `NATIVE_DIFF_CAPTURE_DIR` to retain diagnostic ANSI and SVG cell previews in a temporary directory.
These previews are not Ghostty screenshots.

## Confirmed defects, 2026-09-11

The pre-change executable has SHA-256 `f584b5bf2317bd59c4a046cb00aaf541f5c4fb23bb48c16aa0d1eaa92dc32353`.

1. Shared context used new-side syntax colors in both panes, even when removed comment delimiters changed the old-side syntax state.
2. Binary notices appeared as ordinary modifications because the model read individual file flags instead of the diff flag.

The corrected highlighter retains separate old-side tokens only when the token sets differ.
Its cache estimate includes those tokens. Unified context uses the new-side syntax colors.
The binary-label correction uses the parser's existing binary flag. It does not add a second metadata parser.

The executable source, failing runs, corrected runs, and checksums are retained in the temporary verification report.

## Limits

These contracts do not cover every possible patch or syntax grammar.
They do not establish Pierre pixel parity, complex-emoji behavior in Ghostty, display latency, or process-memory limits.
Patch-only input cannot establish syntax state from omitted source context.
The existing large-corpus checks cover all supplied source records and selected terminal destinations, not every interior screen.
Additional independent large-file interior viewports, cache-eviction styling tours, and actual Ghostty checks remain open.

The preserved-viewer grid comparison remains a regression check, not an independent correctness authority.
The historical trace's pairing model is also insufficient as independent evidence for pairing correctness.

## Geometry contracts

`geometry_contract.rs` adds two real-process checks for the bounded geometry cache.

- The sparse-boundary case places wide characters, a combining accent, and tabs around the 256th and 512th graphemes.
  Explicit expected cells cover horizontal clipping through wide characters. The selected text remains visible through wrapping, resize, and unified layout.
- The eviction case visits 1,151 distinct short lines and forty 512 KiB source lines.
  These visits exceed both normal cache limits without test-only settings. Earlier source cells remain correct after each revisit.
  Every visible payload card has an independently expected file name, hunk, sign, line number, and source prefix.

The test waits for the exact file destination before it checks a settled screen.
A short interval without output does not prove that a large redraw finished.
The fixture uses valid new-file headers. Initial fixture and expectation failures remain in the temporary evidence package.

Run these checks with `cargo test --release --locked --test process geometry_contract -- --test-threads=1`.
The same `NATIVE_DIFF_CONTRACT_BINARY` override supports the preserved executable.
These are geometry-cache checks, not proof of style-cache eviction or total-process memory limits.

## Storage compatibility checkpoint

`storage_contract.rs` compares the complete terminal grids of two real executables.
It covers the multi-file review and long-line position fixtures, with the sidebar shown and hidden.
Both layouts, themes, wrapping, mouse input, file/end navigation, horizontal movement, and four widths must match.
The comparison includes the header and footer and checks both canonical source streams.
The shared process harness also checks clean exit and restored terminal settings.

This check requires an explicit compatible reference and runs separately from the default suite:

```sh
NATIVE_DIFF_BASELINE_BINARY=/absolute/path/to/preserved-native-diff \
cargo test --release --locked --test process storage_contract -- --ignored --nocapture --test-threads=1
```

Preserved-build agreement supplements the independent source and cell expectations. It does not prove physical terminal presentation or performance.
The [storage report](../../SOURCE_STORAGE_REPORT.md) records the accepted reference and current executable.

`horizontal_contract.rs` adds two real-process checks for horizontal bounds.
The expected widths count ASCII, tabs, CJK, and combining accents independently.
A help open/close barrier confirms that each input batch was consumed before the
screen is checked. Both checks reproduced the unbounded-offset defect on the
preserved executable. The older Unicode clipping fixtures now include enough
visible content to exercise their original boundaries without overscrolling.

## Syntax continuation contracts

`syntax_contract.rs` contains four real-process checks. The expected sources, numbers, and syntax colors are explicit.

- The deep case adds or removes multiline delimiters around 512 shared lines in C, Rust, TypeScript, and Python.
  It checks keywords, numbers, and comments or strings on both sides after end seeks, reverse movement, and uncached forward reading.
  Theme changes, burst input, and unified layout must preserve the specified syntax roles. Closing delimiters restore ordinary code coloring.
- The fallback case exceeds the existing long-line limit on the old side only.
  Forward reading must preserve old-side plain styling while the new side remains colored.
  A later hunk must start with fresh syntax state on both sides.
- The Ruby heredoc case uses different old/new closing delimiters with 320 shared body lines.
  After the old delimiter closes, the old side must show a keyword while the new side remains a string.
  After both delimiters close, both sides must show code. End/start revisits and theme changes preserve these roles.
- The pending-resize case uses 12,000 shared lines, with an open comment on the old side and code on the new side.
  It changes width and theme while highlighting is pending, acknowledges the final resize, then navigates to the first and last source lines.
  The final view must contain the exact expected rows and side-specific colors. This checks final coloring, not physical resize frame pacing.

Run these checks with `cargo test --release --locked --test process syntax_contract -- --test-threads=1`.
The preserved-executable override applies to all four checks.
The pending-resize check also passed against the preserved pre-scratch executable, as recorded in the [qualification record](../../evidence/line-search-scratch-v1/QUALIFICATION.md).
These are correctness gates. The separate [syntax diagnostic](../../docs/history/SYNTAX_REPLAY.md) measures the cost of the same public reading actions.

## Shared leading-state contracts

`../shared_prefix.rs` checks the actual highlighting worker with Git-generated patches and bundled grammars.
These are worker integration checks, not terminal-process checks.

- A shared Ruby heredoc prefix crosses the 255, 256, 257, and 520-line boundaries.
  Old/new delimiters then close at different positions, in both directions and themes.
  Exact tokens must match independent sequential highlighting after end seeks, backward restoration, and repeated divergence.
- A side-only oversized line follows 260 shared TypeScript lines.
  The affected side must remain plain. The other side must retain its exact keyword color after revisits.

Both contracts passed before the shared-state experiment and remain active in the accepted baseline.
The experiment's private single-owner checkpoint check remains in its archived source, not the accepted implementation.
The accepted checkpoint checks retain real grammar state, restoration, and separate old/new colors.
These checks do not establish a general speedup or total-memory limit.

## Complete-source coloring contracts

`full_source_color_contract.rs` runs the actual local file and Git commands through a PTY.
One check assigns explicit old/new colors around a hidden comment boundary.
It retains those roles through navigation, wrapping, both layouts, both themes, and width changes.
Unified deletion rows retain old-side colors. Unified shared context uses new-side colors.
Section headers are not source rows and do not enter syntax assertions.

A second check prepares a real Git comparison with 120,000 hidden preceding lines.
It changes theme, layout, wrapping, and width while source coloring is pending.
It checks the final text and colors, then checks cancellation, terminal restoration, and private-file cleanup.
The existing local-input contracts also check missing objects and cancellation of an actual stopped Git helper.

`../full_source_color.rs` checks the real source adapter and syntax worker without a terminal.
It compares requested tokens against uncached complete-file passes for comments, strings, heredocs, and embedded JavaScript.
Explicit keyword/literal colors supplement those same-engine comparisons. Patch-only negative controls produce different, incomplete-context colors.
Revisits exceed the shared checkpoint count limit. Themes, differing extensions, empty sides, CRLF, unsafe bytes, and missing final newlines remain covered.
An 8 MiB hidden line invalidates only its side through later patch sections. Earlier source positions remain usable.
Real queued replies cannot cross source readiness, review identity, or theme changes. Cancellation releases request-owned snapshots.

These checks establish covered output and lifecycle behavior. They do not establish physical terminal pixels, frame pacing, or sustained-memory performance.

## Source-readiness position contract

`context_expansion_contract.rs` opens a real 12-file Git comparison and sends one `G` command.
As lazy sources arrive, the selected last supplied line must remain visible on the new side, in split and unified layouts.
The coloring status must settle, and exit must restore the terminal and remove private spool files.
The preserved-executable override applies to this contract.
It failed on the preceding ordinary executable before the position correction.

`../context_expansion.rs` separately applies real checked-source replies in both arrival orders, with a draw after each reply or only after the batch.
Both layouts and wrapping modes retain the exact old-side source, text byte, wrapped part, and visible row.
The shared fixture in `../support/readiness.rs` includes a long line with tabs, CJK characters, and a combining mark.
This integration check also failed against unchanged application code before the correction.
