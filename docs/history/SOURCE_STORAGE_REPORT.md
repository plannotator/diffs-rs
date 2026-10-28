# Shared source storage: compatibility checkpoint

Date: 2026-09-22.

The storage and source-reference refactor is implemented.
The current viewer still accepts patches only. No new input command, context expansion, or coloring policy is enabled.

## Scope and starting point

This checkpoint covers the three requested steps: establish the baseline, refactor shared storage, and check compatibility.
It does not complete the entire first stage of [the exact-source feature](../design/EXACT_SOURCE_CONTEXT_SPEC.md).
Complete-source snapshots, disk line indexes, comparison identity, and patch-to-source correspondence remain separate work.

The starting source passed 81 enabled checks, including 52 process checks.
The second process run also passed 52 checks, with a preserved-viewer override for the supported test entry points.
Explicit CLI tests that use the compiled executable path still used the starting-source build in that run.
The paired grid comparison and five-patch exports separately exercised both named executables.
The original source, executable, and logs remain in `evidence/source-foundation-v1/`.

## Implementation

| Boundary | Change |
| --- | --- |
| `src/storage/bytes.rs` | Extract the paged byte store from patch-specific indexing. Separate one append-only writer from shared read handles. |
| `src/storage.rs` | Keep compact patch indexes and original record numbers. Resolve checked references at byte-access boundaries. |
| `src/geometry.rs` | Key cached geometry by backing-store identity, original record number, and width. |
| `src/main.rs` | Use the renamed byte-store return type. The event loop and controls are unchanged. |

`StoreWriter::new` creates a private store and a cache group.
`StoreWriter::sharing` creates a distinct private store within an existing cache group.
Patch bytes, full-file bytes, and future disk indexes can use that same storage implementation and allowance.
The real viewer currently creates only its patch store. The new checks exercise additional stores with real file I/O.

One writer owns append access. Shared `ByteStore` handles cannot overwrite or truncate published bytes.
The writer publishes its new length only after the write succeeds.
Previously published references remain valid during later appends and after the writer closes.
Finishing a raw byte store does not establish source completeness or patch correspondence.

`SourceRef` identifies a checked byte range in one store.
Its private fields prevent callers from constructing an unchecked range.
Reading that reference through another store returns an error.
Store identities are never reused within a process. Counter exhaustion returns an error rather than wrapping.

`SourceId` identifies an original patch record within its store.
It does not use a screen row or display label as identity.
Geometry-cache hits do not decode the compressed byte index.
Original record numbers, source ranges, projection data, and reading anchors retain their existing representation.
There is no new allocation or widened field for every supplied source line.

The shared page cache retains the existing 32 MiB payload allowance and FIFO policy.
Page keys include store identity, so equal offsets in different stores cannot collide.
Dropping the final store owner removes that store's cache entries and closes its private file.
Already borrowed page data remains valid until its reader releases it.
The cache contains no reference back to its stores, so it does not create an ownership cycle.

## Correctness evidence

Evidence directory: `evidence/source-foundation-v1/`.

| Check | Result | Evidence |
| --- | --- | --- |
| Starting source suite | 81 passed, three opt-in checks excluded | `baseline-suite.log` |
| Process suite with preserved-viewer override | 52 passed, with the mixed-target boundary described above | `preserved-process.log` |
| Refactored source suite | 84 passed, four opt-in checks excluded | `qualified-suite.log` |
| Diagnostic-feature source suite | 98 passed, four opt-in checks excluded | `diagnostic-suite.log` |
| Ordinary release process suite | 52 passed | `ordinary-process.log` |
| Explicit preserved-build comparison | 96 complete grids and two canonical streams matched | `ordinary-compatibility.log` |
| Five large patches, preserved build | All source expectations matched | `preserved-corpus.json` |
| Five large patches, current build | All source expectations matched | `current-corpus.json` |
| Application Clippy | Passed with warnings treated as errors | `clippy.log` |
| Diagnostic-feature application Clippy | Passed with warnings treated as errors | `diagnostic-clippy.log` |
| Rust formatting | Passed | `format.log` |

The five-patch check covers 20,847,451 supplied records in each executable.
The largest patch contains 19,299,175 records.
The independent reference checks source bytes, signs, side numbers, file order, change starts, and newline markers.
This check does not establish full-file context, syntax colors, or every possible interior screen.

The complete-grid comparison includes both themes, split/unified layouts, wrapping, width changes, navigation, horizontal movement, mouse input, and sidebar controls.
It compares all emitted cells, including the header and footer, without a presentation exclusion.
Existing independent process expectations still check correct contents rather than only agreement between executables.
Existing failure checks cover partial input, blocked producers, cancellation, signals, and terminal restoration.

The focused storage checks use real private files and the production storage code:

- Retain earlier page readers through append, refresh, and eviction.
- Read distinct full-source stores at identical offsets under one shared payload allowance.
- Reject wrong-store references and invalid or overflowing ranges.
- Reclaim dead-store cache entries without invalidating other stores or borrowed pages.
- Read published references while a real writer appends across page boundaries.
- Preserve all fixture record destinations during progressive publication and both layouts.
- Reuse one geometry cache across documents without returning another document's text.

The three existing opt-in checks remain excluded from the default suite.
The new preserved-build comparison is also opt-in and ran explicitly for this checkpoint.
Historical pre-storage and pre-sidebar comparisons are not substitutes for the current preserved reference.

## Review and resource limits

Self-review covered writer exclusivity, publication ordering, source identity, cache ownership, overflow, and cleanup.
The review replaced a byte-reference geometry key with a store-scoped record key.
This avoids unnecessary compressed-index decoding on cache hits.
It does not establish a measured performance improvement.

No dependency, unsafe code, worker, runtime, or input mode was added.
The implementation uses owned handles, private fields, checked arithmetic, and the existing synchronization primitives.
The sole new process-wide counter supplies unique store identities. It holds no source data or application settings.
The pre-existing vendored Syntect lifetime warning remains unchanged.

Store and cache keys add bookkeeping. They are not added to every patch record.
Cache payload limits exclude table overhead, borrowed pages, active read buffers, and other process allocations.
The storage cache still holds its lock during a bounded page read, as before this refactor.
This checkpoint makes no total-memory, latency, or physical-terminal presentation claim.
The controlled performance campaign and physical Ghostty/Kitty inspection remain separate work.

## Build identities

Preserved reference:

`b06a89565cc4f9e1b466cbf2a2d57d70bbe6cbf0f75051de1039a136e0c7f8f3`

Current ordinary release:

`272cf4bbe3b5bfe34705befc6a8738ec2361862b9f194a84bfc89cbd39fb5dd3`

The default executable remains `target/release/native-diff`.
The ordinary build command is `cargo build --release --locked`.
The checks used Rust 1.96.0 on macOS arm64. This checkpoint does not newly qualify the declared Rust 1.88 minimum.
Diagnostic features use a separate target directory and do not replace the ordinary executable.

## Next boundary

Build complete-source capture and disk line indexes using this storage, then check full patch-to-source correspondence.
Keep source bytes, comparison completeness, and syntax readiness as separate facts.
Do not add source-aware coloring or expansion until those checks pass.
GitHub acquisition and application integration remain later milestones.
