# Git rename and copy path correction

Complete, 2026-09-14. The viewer accepts the covered Git rename and copy paths
without losing filename whitespace. The normal executable includes this fix.

## Cause and implementation

The previous parser stopped unquoted rename/copy names at whitespace. It rejected
interior spaces and silently removed trailing spaces in some metadata-only inputs.
Both failures reproduced on the preserved metadata-stage executable.

The correction uses a local copy of the existing `libgit2-sys` crate, version
`0.18.8+1.9.7`. It changes filename consumption in libgit2, not application parsing
or rendering. Unquoted rename/copy names extend to the line ending without
filename trimming. Quoted names retain their existing token boundaries and escape
decoder. Decoding errors release the temporary buffer.

Other path-header call sites retain their previous behavior. Original patch
bytes and parser offsets remain untouched. Source validation, pairing,
projections, highlighting, cache limits, and scrolling remain unchanged.

The application enables the vendored dependency. Its build rejects a nonzero
`LIBGIT2_NO_VENDOR` override instead of accepting an unpatched system library.
The intentional rejection returned exit 101 with the expected explanation.
The normal build succeeded without that override.

The [dependency note](../../vendor/libgit2-sys/NATIVE_DIFF_PATCH.md) records the original
crate hash, VCS identity, modifications, and retained licenses. Only two files
in the complete published crate changed: the C parser and its Rust build script.
The application also changes `Cargo.toml` and `Cargo.lock` to select that copy.
No shared Cargo registry files changed.

## Checks

The [three new process contracts](../../tests/process/git_paths_contract.rs) use actual
Git output and actual viewer processes:

- 56 cases cover renames and copies, unchanged and edited contents, and both
  `core.quotePath` settings. Names include ordinary text, leading/interior/trailing
  spaces, quotes, tabs, and Unicode.
- Expected filenames and source bytes come from the generated repository inputs.
  Checks include status, modes, additions/deletions, line numbers, and final-newline
  markers. A separate retained-source export matches the checked parsed records.
- Twelve malformed rename/copy headers remain errors and produce no diagnostic
  model output. Cases include empty names, invalid escapes, unterminated quotes,
  and extra data after a quoted name.
- A two-file PTY case checks both layouts, renamed source text, metadata labels,
  exact next/previous file-header destinations, and terminal restoration.

The [completed checks](../../evidence/git-paths-v1/checks/report.json) also record:

- Formatting and application Clippy passed. Two existing vendored warnings remain.
- The release suite passed 10 library tests, 37 process contracts, and one worker
  test. Two existing opt-in process tests remained excluded.
- Independent expectations matched all 20,847,451 supplied source records across
  the five pinned patches.
- Complete projection checks passed for all five patches and the metadata fixture.
  The pattern controls passed 2,888 accepted cases and 716 matching rejections.
- Additional LF/CRLF metadata controls preserve trailing spaces for renames and
  copies. The old executable rejects the retained interior-space inputs for both.
- The original crate archive and all unchanged vendored files match their hashes.

The source and test hashes remained unchanged during the final checks. Contracts
that honor the binary override used the preserved candidate. Older direct calls
used the separately identified Cargo-built companion from the same source.

## Review corrections and limits

The first focused run passed, but its navigation assertion could match a file
already visible elsewhere on screen. The stronger assertion exposed a test error:
it used hunk-jump keys instead of file-jump keys. The final test uses `n` and `N`
and requires the exact header at the top of the viewport. The failed run remains
preserved and does not count as final navigation evidence.

Self-review also retained the quoted-name token boundary. Passing an entire
quoted header to the escape decoder alone can accept extra quoted data.
The final malformed-input contract covers that case.

These checks establish the declared rename/copy behavior, not complete Git-format
support or physical Ghostty paint. Other filename-header parsing is unchanged.
No opening-time, RSS, or rendering-performance improvement is claimed. Earlier
performance observations remain bound to their earlier executables.

## Identities

| Role | SHA-256 |
| --- | --- |
| Preserved pre-fix executable | `16f71830c815741ce1cdb5037d5a69ecd886778c238042beb66b2567f72beb58` |
| Preserved fixed executable | `a1b6401654299e0c548b37ed24ef1dd08c0e7a43d24c5922cf5daff26d3aa1f7` |
| Current normal executable | `4a477beb65871042b5a3d31a2721e98d357e819beb2dd6ef482b476d0673b7dc` |

The [build record](../../evidence/git-paths-v1/candidate/build.json) and adjacent archive
preserve the fixed executable, source, and fresh-target build recipe.
The [checkpoint record](../../evidence/git-paths-v1/completion.json) identifies checks,
supporting sources, and the earlier failed attempts.
