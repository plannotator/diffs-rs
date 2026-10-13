# Local-source lifecycle hardening

Date: 2026-09-22.
Scope: `LOCAL_SOURCE_HARDENING_PLAN.md`.

The review found no confirmed runtime defect that requires a production change.
It found one lifecycle coverage gap and incomplete dependency maintenance notes.
The changes add two real-worker checks and correct the notes. Application and dependency runtime sources remain unchanged.

## Findings and completed work

### P3: Input-worker shutdown lacked full-queue coverage

`src/input/worker.rs:30` creates a two-message reply queue.
Its publisher checks cancellation while the queue is full. Its destructor cancels the worker, wakes it, and joins it.
The existing full-queue checks covered the separate coloring worker, not this input worker.
This was a coverage gap, not a reproduced runtime failure.

`tests/input_worker.rs` now uses actual Git revisions and the production input worker:

- The first check fills the patch-publication queue, closes the worker, and reads the retained published patch source.
- The second check fills the source-reply queue, requests another file, and closes the worker without consuming those replies.
- A retained checked pair still supplies exact old/new bytes after worker shutdown, including changed and unchanged lines.
- Strong-owner counts show that the producer released its references. Weak references expire after the final consumer releases the retained values.

The checks use the existing shared Git fixture. They add no test hooks or mock implementation.
A bounded wait detects a shutdown failure. It is not a performance threshold or timing result.

### P3: Vendored patch notes omitted active maintenance contracts

The Syntect and Fancy Regex notes now describe the ordinary-build required-byte filter and its conservative fallback.
They distinguish a complete-line requirement from a search-suffix requirement and preserve the public matcher error contract.
The Syntect note also describes the retained-heap APIs used for checkpoint admission, including their exclusions.
Both notes identify the affected files and the relevant application checks.

The local published archives match all three documented SHA-256 values.
Comparison with local release sources confirms the listed changed files.
The libgit2-sys note remains accurate and needs no edit.
No dependency version, runtime source, license, or notice file changed.

## Ownership review

| Resource | Owner and lifetime | Bound or scaling dimension |
| --- | --- | --- |
| Patch records and spool | `Review::doc` and `App::doc` share `Arc<FileIndex>` values. Final store ownership closes the private file. | Patch indexes scale with supplied records and files. |
| Checked source snapshots | The review retains completed pairs. The application and active coloring requests share their snapshot descriptors. | Descriptors scale with prepared files. Git snapshots share one body store and one index store per review. |
| Source bytes and line offsets | `SourceStore` appends bodies and fixed-width offsets to private files. Snapshot readers retain those files through `Arc<ByteStore>`. | Disk use scales with captured source. Capture buffers use 64 KiB chunks. |
| Source-page cache | Related stores share one cache. Store identity separates keys. Store destruction removes that store's cached pages. | 32 MiB cached payload. Live readers, map storage, and allocator overhead are separate. |
| Context expansion | `View` owns gap counts, layout segments, and file-offset metadata. It does not own source snapshots or copy each revealed line. | Metadata scales with files and patch sections, not the number of hidden source lines. |
| Line geometry | The application owns cached display text and layout boundaries. Keys include the store-scoped source reference and width. | 16 MiB retained entries and at most 1,024 entries. Oversized transient lines are outside this cache limit. |
| Coloring work | One replaceable pending request and one active request share source owners. Replies contain styles and scalar source identities. | The reply queue holds at most 16 messages. Cancellation discards obsolete pending work. |
| Styles and parser checkpoints | The application owns styles. The coloring worker owns parser continuation and checkpoints. | 32 MiB style weight. Checkpoints have count, entry-size, and retained-byte limits. These are not total-memory limits. |

Relevant paths are `src/input.rs:303`, `src/source.rs:164`, `src/storage/bytes.rs:118`, `src/storage/bytes.rs:307`, and `src/view.rs:433`.
Coloring ownership and publication are in `src/highlight.rs:275`, `src/highlight.rs:301`, and `src/highlight.rs:348`.
Cache accounting is in `src/geometry.rs:140`, `src/highlight.rs:748`, and `src/highlight/checkpoint.rs:172`.

The review intentionally retains prepared source for revisits and already exposed context.
That state is not an abandoned-job leak. The current implementation does not evict prepared review snapshots.
The snapshot bodies and indexes remain disk-backed. Their descriptors and correspondence metadata still consume memory.

## Failure recovery

Failed or canceled capture reclaims the unpublished body and index tails.
Pair preparation also rolls back a completed first side if the second side or correspondence check fails.
Rollback removes cache pages that overlap the reclaimed tail. Earlier published ranges remain valid.

Git output uses private files. The process guard kills and reaps a canceled helper.
A separate reader retains at most an 8 KiB diagnostic tail from stderr.
The existing terminal contract stops a real Git helper, cancels the viewer, and checks helper exit and private-file cleanup.

Source failure retains the original patch and removes the affected expansion.
Style acceptance checks the file's source origin and theme epoch before applying a reply.
The existing contracts cover missing source objects, mismatched source, stale styles, removed context, and terminal restoration.
These checks passed again in this turn.

## Remaining limits

This review does not establish lower memory use, faster coloring, or a sustained-memory plateau.
The following boundaries still need qualification:

- Expansion rebuilds one file's segment metadata and updates the later file offsets. Its cost depends on section and file counts.
- Geometry constructs the complete displayed line before deciding whether to cache it. A very large visible line can exceed the cache allowance temporarily.
- Cold page reads can block on filesystem I/O. Cache payload accounting does not include operating-system cache residency.
- Coloring cancellation occurs between cooperative checks. It cannot interrupt an individual regex call.
- The ordinary coloring-worker destructor signals shutdown without joining it. The standalone process still closes its resources on exit.
- An embedded host needs an explicit worker-lifecycle contract before it can assume immediate reclamation after viewer removal.
- Temporary disk use has no application quota. Full-source capture and transient Git output need their own resource qualification.

The input worker does join during shutdown. The coloring-worker distinction predates this feature and remains unchanged.
No new leak or incorrect ownership cycle was confirmed in the reviewed paths.

## Checks and preserved state

The ordinary suite passed 113 checks, including 61 real-process checks. Four opt-in checks remain excluded.
Both new input-worker checks also passed with the owner-census feature in a separate target directory.
Formatting and ordinary application Clippy passed. The existing vendored Syntect lifetime warning remains unchanged.
Self-review checked notification ordering, bounded failure waits, fixture reuse, and final-owner assertions in the new tests.

The ordinary executable retains SHA-256 `c3b337c3549efab30d7fee3f5089d4f9af733a143e28c091cbbd45a21b462074`.
Application source, the root manifest and lockfile, and vendored runtime source match the preserved source-readiness archive.
The prepared executable, fixtures, reference screens, and prior evidence remain unchanged.
The new tests, documents, logs, and checksums are preserved under `evidence/local-source-hardening-v1/`.

No controlled timing, sustained-memory campaign, GUI automation, remote acquisition, global installation, or public release ran.
The next measurement remains the prepared short local-source capture under its declared AC-power conditions.
