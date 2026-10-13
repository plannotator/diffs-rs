# Complete-context coloring checkpoint

Date: 2026-09-22.

The two local source commands now color checked files from their complete preceding contents.
Context expansion remains a separate, unimplemented stage.

## Behavior

Each side starts at its source beginning or a compatible parser checkpoint.
Hidden lines advance the parser, including comments, strings, heredocs, and embedded-language boundaries.
Patch boundaries do not reset complete-source state. Old and new sides use their own filenames and parser states.
Unified shared context keeps the existing new-side policy. Unified deletion rows retain old-side colors.

Pending and unavailable sources use plain syntax. Change backgrounds and valid inline emphasis remain available.
The footer reports source preparation, pending coloring, complete-context coloring, or a specific styling fallback.
Unsupported languages, oversized lines, and parser failures do not change source text or diff structure.
An oversized line invalidates later syntax on its side through EOF. The other side remains independent.
Earlier positions can still use parser states before the failure.

Source read failure retains the patch view, records a source error, and makes normal quit return nonzero.
The existing patch-only command retains its colors and missing-context policy. It does not acquire full source implicitly.
No source formatter, new line pairing, navigation command, or context-expansion action was added.

## Structure and review

`highlight/full_source.rs` owns complete-source replay and request-local readers.
`highlight/input.rs` distinguishes patch, plain, and checked-source inputs.
The existing worker, replaceable request mailbox, bounded reply channel, grammar engine, and themes remain in use.

A checked-pair identity binds immutable snapshots to the original patch entry and its filenames.
Checkpoint keys distinguish original patch sections from complete-source sides. Both include origin and theme identity.
The application rejects queued styles from a previous source origin or theme.
Source readiness invalidates that file's cached styles, not its source numbers, geometry, or reading position.

The self-review removed redundant parser-validity state and made checkpoint scope explicit.
It also checked cancellation, ownership, source failures, stale replies, and shared cache accounting.
Checkpoint capture uses the existing ownership estimates and rejection limits for both replay paths.
Cargo manifests, resolved dependencies, vendored engines, source parsing, and projection rules are unchanged.
There is no new unsafe code, thread, runtime, allocator, or cache allowance.

Hidden replay uses bounded sequential buffers. It does not retain a RAM index or a style vector for each hidden line.
Complete-source replay and patch replay share the existing 16 MiB, 64-entry checkpoint allowance.
Source pages, geometry, and styles retain their existing allowances. These limits do not establish total process memory.
Cancellation occurs between source lines and I/O chunks. An individual regex call remains nonpreemptible.

## Correctness evidence

The final source passed 102 ordinary checks and 116 diagnostic checks. Each default suite excluded four opt-in checks.
The diagnostic suite used `owner-census` in a separate Cargo target directory.
Ordinary and all-feature application Clippy passed with warnings treated as errors. Formatting passed.
The existing vendored Syntect lifetime warning remains unchanged.

The exact ordinary executable passed all 57 enabled process checks.
It matched the preserved input-only executable across 96 complete terminal grids and two canonical source streams.
All five large patches matched independent expectations for 20,847,451 supplied source records.
These large-patch checks cover patch-source preservation, not complete-source availability for downloaded diffs.

Five new worker integration checks use actual files, Git generation, source snapshots, and the production syntax worker.
They compare requested tokens against uncached complete-file passes in both themes.
Explicit keyword and literal colors supplement the same-engine reference. Patch-only negative controls differ where preceding context is missing.
Coverage includes separate old/new state, hidden language boundaries, checkpoint eviction, different filename extensions, CRLF, unsafe bytes, and missing final newlines.
An 8 MiB hidden line checks side-specific fallback across later patch sections and backward visits.
Real queued replies check source readiness, unrelated review identity, theme changes, bounded delivery, and source release after cancellation.

Two new terminal-process checks cover visible colors, navigation, wrapping, layouts, themes, width changes, cancellation, terminal restoration, and private-file cleanup.
The Git case changes controls during replay through 120,000 hidden lines.
The broader suite retains existing exact source, geometry, file-tree, helper-failure, and cleanup checks.

Early terminal-check failures came from test expectations, not an adopted viewer workaround.
The checker initially treated section-header text as source code and expected new-side colors on unified deletion rows.
The corrected assertions identify source rows and the proper side explicitly.

## Preserved build

The ordinary executable is `target/release/native-diff`, also preserved as `evidence/full-source-color-v1/native-diff`.
Its SHA-256 is `1a26004939aa87f3eba258813f60aadbb05cab36b5df3b2e381c73a9ee91c1f0`.
The reference executable is `evidence/full-source-color-v1/baseline-native-diff`.
Its SHA-256 is `a8d4595017911c2a149ab05b1de0ab7cfde216b2c7d85c845d862fcea83d9414`.

The evidence directory retains the source archive, executables, intermediate results, final qualification logs, and hash manifest.
The `pre-review-native-diff` artifact is an intermediate build, not the final executable.
Final-source suite logs are `qualified-suite-2.log` and `diagnostic-suite-2.log`.
Final-executable results use the `ordinary-process-final`, `ordinary-compatibility-final`, and `current-corpus-final` filenames.
Earlier local-source, storage, and sidebar manifests retain all 580 checked entries unchanged.

## Limits and next stage

Qualification used macOS arm64, Rust 1.96.0, and Apple Git 2.50.1.
The declared Rust 1.88 minimum was not newly qualified. No GUI terminal was opened or recorded.
Same-engine token equality does not prove universal grammar accuracy. Terminal cells do not prove physical pixels or frame pacing.
No current-build opening, coloring latency, sustained-memory, or helper-resource claim follows from these checks.
The longer performance campaign remains separate.

Next comes incremental context expansion from these checked snapshots, with stable source positions and unchanged diff totals.
GitHub acquisition, application integration, search, copy, and release qualification remain separate work.
