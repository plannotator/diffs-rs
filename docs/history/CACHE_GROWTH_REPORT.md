# Cache admission and capacity

Completed 2026-09-14. The diagnostic captured history-related capacity growth
without a new peak in live entries. Both three-tour workloads and the final
audit passed. The normal executable and cache policy are unchanged.

## P2: lookup growth without a new live-entry peak

The largest-patch first tour captured a capacity high-water increase without a
new peak in live entries. Earlier capacity reached 229,376 entries, and the live
population peaked at 147,412 entries. Admission 1,382,344 produced this record:

| Boundary | Live entries | Reported capacity | Charged payload, bytes |
| --- | ---: | ---: | ---: |
| Before insertion | 123,296 | 123,296 | 33,554,424 |
| After insertion | 123,297 | 458,752 | 33,555,008 |
| After eviction | 123,295 | 458,752 | 33,554,368 |

The incoming style charged 584 bytes. Two evictions released 640 charged bytes.
The 32 MiB budget and final survivor policy did not change.
The earlier capacity high water already exceeded the new live count.
The doubled high water was not required by a new population peak.
[Raw first-tour snapshot](../../evidence/cache-growth-v1/largest/process/app-01.json).

This event supports deletion/reinsertion history as a growth cause in the current
cache. It is not merely an unobserved large interior population.
The diagnostic records public capacity, not table allocation bytes or RSS.
It does not retrospectively identify the exact trigger in earlier instrumented
or ordinary processes.

## Completed tours

Each workload ran once, with three complete tours. These are diagnostic counts,
not ordinary memory or latency measurements. Peak live entries include the
temporary insertion state before eviction.

| Workload and boundary | Live entries | Peak live entries | Reported capacity | Capacity high water |
| --- | ---: | ---: | ---: | ---: |
| Bun, tour 1 | 39,722 | 39,722 | 57,344 | 57,344 |
| Bun, tour 2 | 63,051 | 63,051 | 114,688 | 114,688 |
| Bun, tour 3 | 75,063 | 75,063 | 114,688 | 114,688 |
| Largest, tour 1 | 108,327 | 147,412 | 456,743 | 458,752 |
| Largest, tour 2 | 108,870 | 147,412 | 420,232 | 458,752 |
| Largest, tour 3 | 108,776 | 147,412 | 401,612 | 458,752 |

Bun had no evictions. Its capacity increases accompanied new live-entry peaks.
The largest trace recorded 4,636,214 admissions and 4,527,438 evictions.
It recorded 865,310 insertion-capacity increases, but only 18 new capacity records
since the empty cache. Exactly one new record occurred without a new live-entry
peak: the event described above.

Neither the live-entry peak nor the capacity high water increased on later tours.
Reported capacity decreased during reuse, but that decrease does not demonstrate
released table storage. Three tours do not establish an indefinite memory plateau.
Random hashing, diagnostic pauses, and asynchronous work can change admission
counts and transition timing in another process.

## Standard-library interpretation

The diagnostic build uses Rust 1.96.0, commit
`ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96`. Its
[standard-library manifest](https://github.com/rust-lang/rust/blob/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/Cargo.toml)
selects hashbrown 0.16.1.

That implementation reports live entries plus remaining insertion capacity.
Removal can reduce reported capacity without releasing the table.
Reusing deleted slots can increase reported capacity without allocation.
The cache's insertion path reserves space before it selects a slot.
When remaining insertion capacity is zero, that path can rebuild or grow the table.
[Pinned capacity and insertion implementation](https://github.com/rust-lang/hashbrown/blob/v0.16.1/src/raw/mod.rs).

The pinned reserve branch rebuilds in place only at or below its half-capacity boundary.
The observed insertion held 123,297 entries, more than half the earlier 229,376 capacity.
Its larger capacity is consistent with that growth branch.
This is an interpretation of pinned source and observed states, not a native
allocation measurement. The behavior does not establish a table defect or leak.

Evicting first cannot guarantee a smaller table. Removal can leave insertion
capacity exhausted. The final 123,295 entries also exceed the half-capacity boundary.
A counterfactual run needs its own hash history and workload checks.

The standard `Entry` API is not a remedy for this capacity mechanism on this
toolchain. It can remove redundant key hashing, but its vacant path also reserves
space before insertion. Standard `Entry` differs from hashbrown's public entry API.
[Standard-library wrapper](https://github.com/rust-lang/rust/blob/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/collections/hash/map.rs),
[compiler-compatible entry implementation](https://github.com/rust-lang/hashbrown/blob/v0.16.1/src/rustc_entry.rs).

## Diagnostic and correctness

The observer uses 392 bytes of fixed-size counters in the diagnostic build.
It observes entry count, capacity, and payload at three admission boundaries.
It retains cumulative classifications and two latest-event records, not an
unbounded event log. It adds no token copies, heap allocation, timing calls, or
output to the admission path. Counters reset with the cache on theme changes.

All observer code is behind `owner-census`. The ordinary executable remains
`1b1ee242cdd50f18b8ea12a787fcdf2f69c77fcece49ebd4d6009e2ff01c7d78`.
There is no diff-source, grammar, rendering, cache-budget, or eviction-policy change.

Formatting and all-feature application Clippy passed. The existing vendored
syntect lifetime warning remains. The ordinary cache test passed.
The reviewed diagnostic suite passed 14 library tests, 32 real-process contracts,
two engine tests, eight regex tests, and one real-worker test.
Two existing opt-in process tests remained excluded.

The first test covered actual allocations, duplicate IDs, multiple evictions,
oversized entries, reinsertion, and exact counters. Read-only review identified
a missing churn classification check before any corpus diagnostic ran.
The added test compares 16,384 real cache admissions with an independent real
hash table that shares the existing randomized hasher.
It checks every capacity classification and witness without a fixed hash seed
or exact growth-threshold expectation. The affected release checks then passed.
Both build archives and both sets of check records remain preserved.

Seven real-process controls and both corpus workloads passed. The audit checked
983 ordered corpus views, eight corpus snapshots, cumulative counter consistency,
terminal restoration, sampler shutdown, and process-group cleanup.
Three altered-record controls failed as expected. The reader also accepted both
earlier storage schemas.

The first offline audit failed while hashing its inputs because a glob included
a collision-test directory ending in `.json`. The corrected audit selected files
and passed. The original auditor remains archived. No viewer workload repeated.
[Audit correction](../../evidence/cache-growth-v1/AUDIT_NOTE.md).

The runs were serial on the shared local host, without a dedicated-host reservation.
There was no allocation profiler, allocator change, forced shrinking, GUI capture,
installation, or external outreach. The earlier queue-stage evidence remains unchanged.

## Next design question

Model smaller lookup values before a broader cache redesign. A boxed `LineStyle`
is the first simple representation to assess against the current inline value.
It reduces each unused lookup slot's value extent, but adds one allocation and
an indirection per live style. No net memory or speed improvement is established.

Keep `LineStyle`, its vector capacities, and its charged weight unchanged in that
model. This differs from the rejected conversion of style vectors into boxed
slices. Preserve duplicates, FIFO order, final survivors, side distinctions,
tokens, and changed spans. Do not select a dense slot store, allocator change,
smaller budget, forced shrinking, or input-specific rule without separate evidence.

General index storage, coloring latency, and physical terminal presentation
remain separate work. This diagnostic does not repeat or replace ordinary
responsiveness and sustained-memory qualification.

Before accepting a representation change, compare identical retained styles and
FIFO survivors, then recheck real terminal behavior, responsiveness, and sustained
memory across the existing five-patch workload. Extra allocation and indirection
costs must remain visible in that decision.

## Evidence

- [Declared scope](../../evidence/cache-growth-v1/PLAN.md) and [frozen run identities](../../evidence/cache-growth-v1/freeze.json).
- [Initial checks](../../evidence/cache-growth-v1/checks/report.json) and
  [reviewed release checks](../../evidence/cache-growth-v1/review-checks/report.json).
- [All nine diagnostic processes](../../evidence/cache-growth-v1/run-result.json) and
  [complete raw-record audit](../../evidence/cache-growth-v1/audit.json).
- [Source and evidence seal](../../evidence/cache-growth-v1/seal.json).

The measured diagnostic is
`1a99e464961faaa501447ae45ee98bad6bf2bb2426e11dd1c5263bf486b2dea7`.
Its [build manifest](../../evidence/cache-growth-v1/diagnostic-reviewed/build.json) and
adjacent archive preserve the recipe and exact source. It is not an ordinary
performance build.
