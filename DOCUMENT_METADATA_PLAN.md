# Retained document metadata

Status: scoped investigation, 2026-09-14. No storage change is accepted here.
Start after the active cache-value qualification closes and the terminal visual
check has a recorded result or an explicit capture blocker.

## Question

Can immutable file and hunk metadata retain less memory without changing lookup,
publication, or displayed content?

`storage::read` currently moves the parser's `FileDiff` into each published
`Arc<FileIndex>`. Its five strings, hunk vector, and hunk-header strings retain
growable storage even though the published file is read-only. Source spans and
both layout projections already have compact representations. Those indexes are
not part of this proposal.

The [earlier largest-input accounting](evidence/compact-projection-v1/candidate-index.json)
recorded 67,584,706 bytes of metadata allocations. That figure includes useful
data, not just unused capacity. On that 64-bit target, removing one capacity word
from six descriptors per file and one per hunk suggests approximately 6.74 MiB:
`(76,872 * 6 + 421,819) * 8` bytes. This estimate is neither an allocation profile
nor an RSS result. The unused buffer capacity has not yet been counted separately.

## First step: count before changing storage

Use one small diagnostic that borrows completed `FileIndex` values. Count each
allocation once across all five pinned patches. Do not clone metadata or add an
observer to rendering or highlighting.

Record:

- File and hunk counts, plus the largest per-file hunk count.
- Hunk-vector lengths and capacities, with the current descriptor size.
- Length and capacity totals for the five file strings and all hunk headers.
- The existing disjoint source, projection, metadata, and file-descriptor totals.
- Executable, source, and input hashes; the diagnostic's exact counting rules.

Reconcile the new counts with `FileIndex::metadata_bytes`. Keep descriptor,
string-buffer, and unused-capacity totals separate. Exclude allocator rounding,
allocation headers, and RSS from these arithmetic claims.

Use the results to decide whether an implementation is worth its added types and
publication cost. Stop after the counts if the opportunity is too small to justify
that complexity. Do not select a different design merely to reach a memory target.

## Conditional implementation

Keep the mutable parser model unchanged. At publication, consume it into a small
retained metadata type with `Box<str>` labels and a boxed hunk slice. Each retained
hunk has a boxed header and the same numeric fields. Use an exhaustive conversion;
do not reconstruct labels, normalize text, or silently omit fields.

Reads must remain direct borrowed string/slice access. Preserve `u32` source line
numbers and `usize` ranges and bases. Preserve empty files, metadata-only files,
file order, status, paths, modes, counts, and hunk boundaries. Do not change source
bytes, line pairing, newline flags, syntax coverage, or navigation behavior.

The conversion may allocate and move the hunk array while the old array is still
live. Assess this transient cost as well as retained storage. Do not add string
interning, compressed labels, custom allocators, periodic shrinking, disk-backed
indexes, or an input-specific path to this change.

## Verification and decision

1. Preserve the selected reference source and ordinary executable before editing.
2. Run formatting and application Clippy. Use the existing real-process contracts
   and complete projection comparison, including metadata-only files, unusual
   labels, high source bases, and exact navigation destinations. Add a focused
   boundary case only where the existing checks leave a real gap.
3. Recheck all supplied source records against the independent five-patch source
   expectations. Check labels and ranges separately; a source digest does not
   establish metadata correctness.
4. Repeat the counts. Require unchanged semantic counts and reconcile all changed
   extents. A smaller descriptor does not prove a smaller process.
5. Run the existing serial ordinary opening/interaction and three-tour memory
   comparisons. Include full-index completion, resize, cache revisits, and exit.
   Preserve both transient peaks and held-view samples, including adverse results.

A correctness failure rejects the candidate. Unexplained responsiveness or memory
regressions require investigation before acceptance. Do not pool this experiment
with the cache-value comparison. Record a keep/reject decision, exact build
identities, and unresolved limits before starting another storage change.
