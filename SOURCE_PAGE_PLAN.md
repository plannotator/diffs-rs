# Next memory task: source-page ownership

Date: 2026-09-12. Scoped, not implemented or measured.

## Why this next

The existing index-heavy three-tour allocation profile contains 512 live source
page allocations totaling 41,943,040 bytes: 40 MiB for 32 MiB of page payload.
The stack resolves to `Spool::read`. This is historical instrumented evidence
from executable `a7a84a94…`, not a measurement of today's delivery candidate.

The current [storage implementation](src/storage.rs) still builds a byte vector
and converts it to `Arc<[u8]>`. The shared header sits in the same allocation as
the bytes. The observed full-page allocation class is 80 KiB for a 64 KiB page.
Separating the shared owner from its immutable byte allocation is a concrete,
small experiment. It may avoid that allocation rounding and the vector-to-Arc
copy. It adds a small allocation and an indirection; measure the net effect.

This opportunity is roughly 8 MiB before new-owner overhead on the observed
full cache, not a solution to the largest document's total memory. The design
must work identically for every input and must not depend on allocator buckets.

## One-owner implementation

1. Preserve the current `611805a3…` build and source as the unchanged experimental
   control. Its unaccepted status remains explicit. Isolate the page delta from
   compact projection and delivery; do not claim this work accepts those changes.
2. Evaluate a private shared page owner containing an immutable boxed byte slice
   (`Arc<Page>` with `Box<[u8]>`), or the equally small direct equivalent. Keep the
   large byte allocation separate from the reference-count header. Read into the
   final byte buffer where practical. Avoid a reusable buffer pool, new trait,
   custom allocator, unsafe code, platform branch, or generic cache framework.
3. Leave `Spool::read` and its owned byte result unchanged. Page handles stay
   private. Preserve the 64 KiB page size, 32 MiB payload policy, insertion-order
   eviction, locking, page keys, published-length ordering, errors, and offsets.
4. Preserve partial-page refresh: an older short page can remain alive for a
   reader while a later append publishes a longer immutable replacement. Never
   mutate a buffer already visible to a reader. Cache eviction drops only its
   ownership; the final reader controls release.

Do not add extra copies or retain both representations. Payload accounting stays
payload accounting; separately count page owners, buffers, capacities, and live
allocations. A cache limit is not a process-memory bound.

## Verification that earns the change

Use real file-backed storage and real viewer processes, not mocked storage.

- Add a small coherent storage regression test for byte-exact reads across a
  page boundary, a cached partial page followed by append/refresh, and an old
  reader surviving replacement and eviction. Verify final-owner release with
  Rust ownership observations, not allocator internals or timing assertions.
- Reuse the existing progressive-input and input-replacement process contracts.
  Exercise cache eviction/revisit with a deterministic patch larger than the
  payload cache and independently expected bytes; do not use a repository name
  to select behavior. Check successful and failed-input cleanup.
- Before retention, run the existing real-process, source/projection, Unicode,
  position/resize, old/new syntax, and storage checks. Verify complete supplied
  source for all five corpus patches. Run formatting and application Clippy.

## Bounded measurement and decision

Freeze the exact commands, toolchain, source, workloads, expected views, binaries,
and trial order before execution. Build plain reference/candidate executables in
separate preserved locations. Do not alter earlier recorded experiments.

First use one serial allocation-instrumented control/candidate pair on the same
existing index-heavy three-tour trace. Confirm both have the same cached payload
at the checked phases. Account for the separate small owners and byte buffers;
retain total live heap, allocation count, and profiling intervals. A smaller Rust
type or a predicted 8 MiB saving is not sufficient. If the owner does not shrink
in practice, or the representation needs broader changes, stop and reject it.

If the ownership result and correctness checks pass, use the existing declared
ordinary responsiveness comparison with capture separated from expensive cell
validation. Then run the normal five-patch latency and three-tour memory
checkpoint once. Keep text and coloring milestones separate; include fresh opening,
navigation, revisits, resizing, and cleanup as the existing workload specifies.
Do not substitute allocation-instrumented timings for ordinary performance.
Retain all slower observations and finite-tour growth. No repeat-until-pass loop.

Accept only a net owner reduction without source/presentation changes or an
unexplained material responsiveness regression. Attribute the page delta against
its immediate control; the combined development build still needs an explicit
promotion decision. If inconclusive, keep the proposal unaccepted rather than
adjusting workload, budgets, or source behavior.

## After this bounded task

Compiled syntax remains the larger cross-input opportunity: earlier profiles
show about 213 MiB of named regex allocations after three tours. Before changing
sharing, count distinct live compiled cells for exact patterns across grammars
actually used by that trace. A static inventory of every bundled grammar does
not establish duplicate live allocation. Preserve lazy compilation, independent
parser/capture state, and exclusion of source-derived expressions. Account for
registry lifetime and keys; do not introduce a persistent registry without a net
ownership case. Stop at a no-change finding if duplicate ownership is immaterial.

Actual terminal pixels, smoothness, and visual parity remain separate work. Neither
this proposal nor the recent PTY attribution closes them.
