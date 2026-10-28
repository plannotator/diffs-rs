# Local source input checkpoint

Date: 2026-09-22.

The local input implementation is complete. The ordinary and diagnostic source suites passed.
This checkpoint implements the first requested item, not full-context coloring or context expansion.

## What changed

The viewer now accepts `files OLD NEW` and `git --repo PATH --base REV --head REV`.
Both commands also accept `--check` for a noninteractive JSON source report.
The existing `patch` and `inspect` commands retain their input semantics.

Two-file input captures each regular file through one descriptor before comparison.
Git compares the private captures, not a later version of either original path.
Nonblocking open permits a descriptor type check without waiting for a pipe producer.
Before/after descriptor metadata detects ordinary concurrent changes. It does not establish an atomic filesystem snapshot against every writer.
An explicit symlink input can resolve once to a regular file. Missing files are errors, not empty sides.

Git input pins both requested commits and compares their trees directly.
The generated raw inventory binds each patch entry by raw paths, modes, status, and available full object IDs.
Pure renames can omit modes from the patch. The checked raw inventory supplies those modes.
A Git type-change record binds to the emitted deletion and addition patches, preserving their order.
Git sources come from blob IDs, never live working files or later branch resolutions.
Symlink blobs supply link text. Binary entries and submodules remain metadata.

The concrete review type represents complete local comparisons only.
It has a process-local identity, fixed origin, and separate per-file source states.
A checked pair owns its original patch entry and immutable old/new snapshots. An absent side differs from a present empty snapshot.
Future remote partial/unknown coverage needs an explicit adapter contract. This implementation does not claim remote support.

## Exact source checks

The correspondence check matches all supplied records at their declared line numbers.
It also compares omitted prefixes, interior gaps, and suffixes through both true ends of file.
Raw bytes, CRLF, invalid UTF-8, and final-newline markers retain their original meaning.
Equal line counts cannot conceal an unrelated hidden change.

A failed source check leaves the patch readable. It does not replace the patch or try another revision.
The footer reports source failure, and normal quit returns a nonzero status after an operational source failure.
Completed source preparation does not move the reader.

Both new modes still use patch-context coloring. Their footer states this limitation even after source checks pass.
This staged policy is recorded in [the implementation plan](LOCAL_SOURCE_INPUT_PLAN.md).
Full-prefix syntax, new syntax cache identities, and expandable context remain the next feature stages.

## Ownership and resources

Source bodies and fixed-width line offsets reside in private files.
An empty source has zero lines. A final newline does not create another line.
Index writes use a buffer. Long lines do not require complete in-memory strings during capture or correspondence checks.
Sequential checks reuse byte buffers and read source bodies and indexes without filling the visible-page cache.

Git snapshots share one body store and one index store for the session.
Two-file input uses its fixed pair of named capture stores directly for comparison, with private index stores.
Patch, capture, and index pages share the existing 32 MiB payload allowance.
This allowance is not a total-memory guarantee.

The local input worker first generates and publishes patch entries. It then prepares source pairs requested by the viewport.
The reading cursor has priority. At most one source job runs, with one replaceable pending destination and two reply slots.
Completed pairs survive navigation. Canceled or failed jobs reclaim their unpublished body and index tails.
No worker, source descriptor, or highlighter is created per source line.

Each Git command has a supervised process and a temporary stderr-drain thread.
The diagnostic tail retains at most 8 KiB. Child stdout goes to a private file, avoiding a blocked stdout pipe during cancellation.
The process is stopped and reaped before its supervisor returns after cancellation.
The worker joins before the viewer exits. Private files use RAII cleanup.
Cancellation occurs between I/O chunks. No hard deadline is claimed for stalled filesystem operations or existing parser calls.

Helper output temporarily adds disk storage. A source pair uses one batched object-reader process, not a process per line.
The generated patch becomes available after the diff helper finishes. These costs need future opening, memory, and disk measurements.
Moving work into a helper does not establish a performance improvement.

## Git safety and scope

Qualification targets Git 2.50.1 on macOS arm64. The commands require Git 2.50 or newer and a SHA-1 repository.
Arguments use `Command`, without a shell. Revisions must resolve to commits.
The adapter removes inherited Git routing and injected configuration variables.
It disables pagers, external diff helpers, text conversion, hooks, replacement objects, lazy fetches, and optional locks.
It does not bypass repository ownership checks or mutate the index, working tree, or refs.
No checkout filters, submodule traversal, LFS downloads, or network acquisition belong to these inputs.

The generation policy remains three context lines, Myers, the indent heuristic, 50% rename detection, and a 1,000-candidate exhaustive-search limit.
Copy detection remains off. Missed renames remain complete additions and deletions.
The transport follows Git's [diff](https://git-scm.com/docs/git-diff), [object delivery](https://git-scm.com/docs/git-cat-file), and [process controls](https://git-scm.com/docs/git/2.50.0) documentation.

The sole dependency declaration adds the already-resolved `libc` package for the platform's `O_NONBLOCK` constant.
The lockfile adds no resolved package or version change. There is no new unsafe code, runtime, or allocator.

## Evidence

Evidence directory: `evidence/local-source-v1/`.

The starting source passed 84 enabled checks. Four opt-in checks were excluded.
The focused additions use real files, repositories, Git commands, source stores, and terminal processes.
They cover captured-byte stability, exact line boundaries, empty/absent sides, renames, mode changes, binary entries, symlinks, gitlinks, and type changes.
They also cover moving revisions, divergent direct comparisons, hostile helper settings, missing objects, nonregular files, and unsupported object formats.
Deliberate changes in omitted prefixes, gaps, suffixes, supplied lines, and EOF behavior fail the correspondence check.
The cancellation check stops an actual child Git process, cancels the viewer, and checks process reaping, temporary-file cleanup, and terminal restoration.

The final source passed 95 ordinary checks and 109 diagnostic checks. Four opt-in checks were excluded from each default suite.
Application Clippy passed for ordinary and diagnostic features, with warnings treated as errors. Rust formatting passed.
The pre-existing vendored Syntect lifetime warning remains unchanged.

The ordinary executable has SHA-256:

`a8d4595017911c2a149ab05b1de0ab7cfde216b2c7d85c845d862fcea83d9414`

The preserved comparison executable has SHA-256:

`272cf4bbe3b5bfe34705befc6a8738ec2361862b9f194a84bfc89cbd39fb5dd3`

These checks use Rust 1.96.0. Diagnostic features use a separate Cargo target directory and do not replace the ordinary executable.
The exact ordinary executable passed all 55 enabled process checks.
It matched the preserved viewer across 96 complete terminal grids and two canonical streams.
The five large patches matched independent expectations for all 20,847,451 supplied source records.
These corpus checks establish patch-source compatibility, not full-source coverage for downloaded patches or universal syntax correctness.

All 562 entries in the preceding storage and sidebar evidence manifests still match.
The current source archive, ordinary executable, and evidence hashes remain in the checkpoint directory.

| Check | Evidence |
| --- | --- |
| Starting source | `baseline-suite.log` |
| Final ordinary source | `qualified-suite.log` |
| Final diagnostic source | `diagnostic-suite.log` |
| Exact ordinary executable | `ordinary-process.log` |
| Preserved-viewer comparison | `ordinary-compatibility.log` |
| Five-patch independent source check | `current-corpus.json` |
| Clippy and formatting | `clippy.log`, `diagnostic-clippy.log`, `format.log` |
| Earlier evidence preservation | `preservation.log` |

## Remaining work

Use these checked sources for complete-prefix syntax, then add context expansion and its position contracts.
Keep the existing patch-only mode independent.
Qualify current-build opening, coloring, responsiveness, sustained resources, and physical terminal presentation before extending to GitHub.
No controlled performance campaign, GUI automation, physical terminal capture, remote repository access, project repository initialization, or global installation occurred here.
The declared Rust 1.88 minimum is not newly qualified in this checkpoint.
