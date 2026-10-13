# Live compiled-pattern ownership census

Date: 2026-09-13. Diagnostic scope only, declared before implementation or runs.

## Question and retained evidence

Does a completed viewing session retain multiple compiled regex cells for the
same exact static pattern across loaded grammars? The existing temporary sharing
map covers one grammar, not the whole syntax set. Source inspection establishes
that duplication is possible. It does not establish the live count or byte cost.

The preserved source-page candidate's largest-input, unlogged allocation profile
reports 465.77 MiB live at opening and 742.55, 761.12, and 761.20 MiB after its
three tours. Its disjoint document accounting is 381.34 MiB. The small showcase
document accounts for 0.22 MiB; earlier named allocation profiles identify its
compiled highlighting engine as a substantial separate owner. These observations
justify inspecting syntax ownership independently of document-index work.

Those allocation profiles are separate processes from the latest RSS cohort.
They cannot explain that cohort's abrupt RSS drops. No compression, reclamation,
or application saving is attributed to those drops.

## Implementation boundary

- Add an off-by-default `regex-census` feature to the application and vendored
  parsing library. Normal matching, grammar loading, caches, and parser policy
  stay unchanged. No global registry, new engine, or production optimization.
- Inspect only initialized lazy-context cells through `OnceCell::get`. Never
  load a grammar or compile a pattern to inspect it. Include hidden and embedded
  grammars already loaded by ordinary parsing.
- Borrow pattern strings and record compiled-cell pointer identities without
  cloning regex owners. Record uncompiled rules with a null identity. Enumerate
  an already-initialized first-line cache separately; never initialize it.
- Take one snapshot on worker shutdown, while its syntax set is still alive and
  no parser call is running. In diagnostic builds only, join that worker during
  cleanup so process exit cannot race the file write. Release the mailbox lock
  before joining. Retain the existing external timeout and cleanup protections.
- A diagnostic environment variable selects a new output file. Write once,
  explicitly flush, and report errors. Missing or malformed output fails the
  diagnostic even if the viewer exits successfully.
- Reduce offline: count distinct initialized owners before grouping exact
  pattern strings. Multiple rule references to one owner are not duplicates.
  Owner addresses are meaningful within this one snapshot only.
- Temporary backreference substitutions are outside this static-owner census.
  Duplicate roots do not establish duplicated engine bytes, recoverable memory,
  identical search-cache capacities, or an RSS saving.

## Verification and fixed run

First test real grammar parsing: cold grammars remain cold; repeated rules in one
grammar share a compiled cell; the same rule in two independently loaded grammars
has two cells. Repeated inspection must not change the result or parser output.
Test first-line-cache inspection separately. Exercise diagnostic process shutdown
and file completion through a real PTY, including file-write failure detection.
Run formatting and Clippy on ordinary and diagnostic application configurations.

Preserve a diagnostic release executable with its source, toolchain, dependency
features, and hash. Run exactly one three-tour process on the existing showcase
memory trace, with its frozen source/style expectations and checked cleanup.
Use the same input actions, without allocator logging or external heap snapshots.
This is an ownership census, not a new accepted latency or sustained-RSS series.
Keep the ordinary `b749889e…` binary and all earlier evidence untouched.

Stop on a failed view, safety limit, or invalid census. Preserve failures instead
of retrying toward a preferred result. A negative duplication result is valid.
Any broader sharing implementation requires a separate design and byte-cost
attribution; it is not automatically authorized by a positive count.

## Preflight clarification, before the three-tour process

The guarded process-control preflight stopped before spawning because the parent
environment contains `MallocNanoZone=0`. Retain and record that existing allocator
setting. Do not unset it or infer that earlier profiles used the same setting.
Continue to reject inherited stack logging, scribbling, guard edges, injected
libraries, and delivery tracing. The failed preflight and earlier control helpers
remain preserved. This clarification changes no viewer behavior or matching rule.
