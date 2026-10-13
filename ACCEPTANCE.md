# Acceptance checklist

Status: 31 real-process checks pass as of 2026-09-11. The five-patch interior/latency/memory batch is complete; no full milestone is complete.
The [rendering contracts](tests/process/README.md) define the new independent terminal checks and their limits.
The [large-diff progress report](docs/history/LARGE_DIFF_PROGRESS.md) records the current source checks and remaining acceptance work.
The [implementation report](docs/history/IMPLEMENTATION_STATUS.md) records the tested subset, limitations, and open visual acceptance.
The [large-diff specification](LARGE_DIFFS.md) replaces provisional scale-fixture selection with the five exact cached DiffsHub inputs and defines the next batch.
The [five-patch results](docs/history/FIVE_PATCH_RESULTS.md) records its independent interior checks and current-build measurements.
The [spec](SPEC.md) owns behavior and defaults. This file owns evidence and milestone completion.

## The evidence rules

1. Record the viewer commit, dependency lockfile hash, fixture hash, command, and settings for each pass.
2. Record the OS, hardware, terminal version, font, cell geometry, and color profile.
3. Run the actual executable with actual files and terminal input.
4. Compare source content against independent expected results, not only against our own parsed model.
5. Inspect actual Ghostty screenshots for appearance and short recordings for movement.
6. Record `pass`, `fail`, or `blocked`, with evidence paths and a short explanation.
7. Keep failures visible until corrected. Do not weaken the expected result to accommodate a bug.

An automated terminal check can establish emitted cells and interaction results. It cannot establish actual display timing or visual quality by itself.
A screenshot can establish appearance at one moment. It cannot establish complete content, off-screen correctness, or scrolling behavior.
No mocks replace the diff engine, highlighter, terminal input, or child-process lifecycle in acceptance checks.
Small focused algorithm tests remain useful for ambiguous alignment and Unicode cases. Coverage percentage is not a release gate.

## Fixtures

| Fixture | Contents | Independent expectation |
| --- | --- | --- |
| Small review | Several files across C, Rust, TypeScript/TSX, Python, and JSON | Hand-reviewed old/new files, patch, line counts, and expected changed spans |
| Uneven replacements | Insertions before edited lines, repeated lines, blanks, unequal replacement counts | Both source sequences preserved, documented intended pairing |
| Text boundaries | Tabs, CRLF, no final newline, combining marks, emoji, CJK, empty files | Original bytes and expected terminal-column boundaries |
| Unsafe or unsupported | Terminal escapes, invalid text bytes, malformed hunks, binary notices, combined diffs | Safe literal display or an explicit supported/unsupported diagnostic |
| DiffsHub showcases | Five exact cached bodies, including Linux v6.0…v7.0 at 710,870,899 bytes | Pinned body hashes, independent source-segment digests, metadata, and line counts |
| Controlled large cases | Single-hunk and long-line inputs that isolate specific costs | Fixed generation, hashes, independently specified content, and complete reconstruction |

The first pass supplies `fixtures/review.patch` and deterministic fixtures inside `tests/process.rs`.
The full fixture/reference matrix is not complete. DiffsHub body identities and current inventory are in `fixtures/diffshub/corpus.json`.
The first-slice hashes record the earlier implementation state. Later documentation changes do not retroactively change that evidence.
Expected results cannot come only from the candidate renderer. Generated fixtures retain original before/after data for reconstruction checks.
Patch-only fixtures compare every supplied old/new segment and hunk range. Missing content has no invented expected bytes.

## Milestone 1 checks

Default visual geometry: 160×50 terminal cells. Also inspect 100×30 and 80×24.
Use the same file versions, theme, context, and inline mode in Pierre and our viewer.
Compare layout relationships and color roles, not browser pixels against terminal pixels.

| ID | Requirement | Setup and actions | Expected result | Result |
| --- | --- | --- | --- | --- |
| V-01 | [Split presentation](SPEC.md#3-what-appears-on-screen) | Small review. Open split mode in both themes. Capture the full window. | Correct old/new sides, alignment, gutters, empty cells, and full-width backgrounds | — |
| V-02 | Unified presentation | Same input. Press `u`. Capture both themes. | Correct dual numbers, source ordering, and the same changed spans | — |
| V-03 | Visual fidelity | Open the same examples in pinned Pierre. Compare each initial language and every color role. | No unexplained highlight differences, clipped markers, or unreadable emphasis | — |
| V-04 | Readable whitespace | Open text-boundary fixtures. Toggle wrapping. | Tabs, wide characters, and continuation rows preserve source identity and readable alignment | — |
| V-05 | Content fidelity | Parse the small and uneven fixtures through the real executable. Compare every available side against independent source data. | No missing, duplicate, reordered, or substituted source content | — |
| V-06 | Position retention | Select a changed line. Toggle `u`, `w`, and `t`. Resize wide to narrow and back. | Same source anchor, correct active side, no stale rows | — |
| V-07 | Navigation | Drive line/page movement and both ends through a real PTY. Inspect expected visible cells after each action. | Correct destinations and no invalid line numbers | — |
| V-08 | Clean exit | Quit normally, then repeat with Ctrl-C. Compare terminal modes before and after. | Correct exit codes, restored modes and cursor, no owned live workers | — |
| V-09 | Small-window recovery | Resize below 40×10, then back to 160×50. | Clear size message, responsive quit, preserved reading position | — |
| V-10 | Initial language quality | Open multiline strings/comments, template strings, macros, and nested constructs in the initial languages. | Usable syntax colors, with no false certainty from missing patch context | — |

Milestone 1 requires visual inspection, not just image generation. Each V-03 difference needs a decision: fix, deliberate terminal adaptation, or unacceptable gap.
The user reviews the actual viewer images before the project expands into milestone 2.
The later large-diff request authorizes that engine batch while visual review remains open. It does not mark milestone 1 complete.

## Milestone 2 checks

| ID | Requirement | Setup and actions | Expected result | Result |
| --- | --- | --- | --- | --- |
| R-01 | [All inputs](SPEC.md#5-inputs-and-content-correctness) | Open equivalent patch-file, stdin, two-file, and pinned-Git inputs. | Same content meaning, with explicit context availability differences | — |
| R-02 | Stream navigation | Open many files. Use file/hunk shortcuts and the file picker at both ends. | Correct file order, hunk targets, sticky headers, and boundary messages | — |
| R-03 | Settings | Change each supported setting. Repeat with command-line overrides and a fresh invocation. | Specified precedence, active settings visible, no unintended persistence | — |
| R-04 | Context | Expand available context, then try a patch-only gap. | Correct original lines, or the exact unavailable-context message | — |
| R-05 | Search | Search visible, off-screen, and known collapsed content. Search during loading. | Correct matches and explicit wrap, incomplete-search, and partial-patch states | — |
| R-06 | Copy | Select old and new lines separately. Copy with access off and with OSC 52 enabled. Inspect emitted payloads. | Exact source bytes, no gutter text, no unsolicited clipboard writes | — |
| R-07 | Safe input | Open every unsafe/unsupported fixture. Inspect actual terminal output. | No input-controlled terminal actions or disguised unsupported diff | — |
| R-08 | Error paths | Use invalid arguments, missing files, malformed input, and non-terminal stdout. | Correct streams and exit codes, no traceback or uncontrolled terminal output | — |
| R-09 | Loading failure | Feed valid files followed by a truncated hunk and EOF. | Earlier content remains inspectable with persistent incomplete state and eventual exit code 1 | — |
| R-10 | Input isolation | Change a file during its read, then after capture. Run two viewer instances. | Unstable-read diagnostic where detected, captured content unchanged, independent processes | — |
| R-11 | Interrupts | Send SIGTERM and SIGHUP during loading and viewing. Repeat with a blocked input producer. | Handled cleanup and specified exit codes without waiting for producer EOF | — |
| R-12 | Selection and overlays | Select wrapped lines. Open/cancel search, picker, settings, and help. | Correct focus, retained selection where specified, no accidental quit from typed `q` | — |
| R-13 | Mouse behavior | Click both split sides and a gap. Scroll with mouse wheel and trackpad. | Correct active source row, stable scroll, no additional application acceleration | — |
| R-14 | File metadata | Open rename-only, mode-only, binary, empty, and equal inputs. | Explicit cards or correct empty state, never a silently omitted file | — |

## Milestone 3 checks

| ID | Requirement | Setup and actions | Expected result | Result |
| --- | --- | --- | --- | --- |
| L-01 | [Large correctness](LARGE_DIFFS.md) | Open all five DiffsHub patches and the diagnostic supplements. Compare all supplied segments and totals. | Complete correct content, not only passing sentinel checks | — |
| L-02 | Large navigation | Scroll continuously, reverse direction, jump to distant hunks, switch layouts, and resize. | Correct destinations without stale content or lost position | — |
| L-03 | Honest loading | Record first content and completed visible styling separately from process start. | Real loading states, no hidden styling delay or skipped expensive content | — |
| L-04 | Cancellation | Quit and interrupt during input reads, diff computation, search, and highlighting. | Bounded response, terminal restoration, no orphaned work | — |
| L-05 | Memory behavior | Repeat the fixed structural tour and layout switches three times. Record peak and retained memory. | Disposable caches respect their budget, no traversal-proportional retained growth | — |
| L-06 | Long-line fallback | Open lines around the 10,000-scalar boundary. Navigate, wrap, search, and copy them. | Declared style fallback, complete content, responsive interaction | — |
| L-07 | Compatibility | Repeat the core reading sequence in Kitty and Ghostty through tmux. | Correct cells, keys, Unicode, resize, and exit. Any color difference is explicit. | — |
| L-08 | Color fallback | Repeat the small review in 256-color and no-color profiles. | Signs, source identity, and text remain legible and correct | — |

## Timing evidence, kept small

The development pass records first correct content, visible styling completion, input-to-visible-change delay, and quit-to-shell delay.
It uses the same named reference machine and pinned inputs. Warm and cold-start conditions remain separate.
An application flush timestamp is not proof that a frame reached the display. Actual display evidence uses a visible terminal and screen recording.
The capture frame rate limits timing precision. Report that uncertainty instead of manufacturing millisecond precision.
The 50 ms navigation and 250 ms cancellation targets come from the spec. They remain unproven until the matching evidence exists.
No official comparison with other viewers or change to DiffGrid follows from these checks.

## Results and open questions

The original implementation passed ten development checks. Later work brought the complete suite to 31 real-process checks.
All five large patches pass the complete supplied-source digest comparison. The new rendering contracts cover explicit small-screen expectations.
The full milestone cells remain open because their requirements include additional evidence.
The existing Pierre browser reference and user-supplied Ghostty screenshot do not establish complete visual acceptance.
Geometry-cache eviction and sparse Unicode boundaries now pass independent process checks. The [responsiveness batch](docs/history/RESPONSIVENESS.md) records the scope.
Deep old/new syntax, continued fallback, and hunk/theme isolation pass independent process checks. The [syntax replay batch](docs/history/SYNTAX_REPLAY.md) records the limits.
The five-patch batch now covers 205 independent interior PTY states per patch, five latency repetitions per patch, and three memory tours per patch. All interior oracles and corruption controls pass. Pierre has one styled distant-seek maximum above the strict 200 ms guard. Ghostty alone meets the current memory envelope; Pierre, Node, Bun, and Linux exceed their applicable RSS limits. Actual Ghostty pixel behavior, display timing, and full milestone acceptance remain open. See [FIVE_PATCH_RESULTS.md](docs/history/FIVE_PATCH_RESULTS.md).
