# Retained-owner accounting

The optional `owner-census` build records four additional owners at the existing
F12 diagnostic boundary. Ordinary builds do not include these observations.
Cache budgets, admission, eviction, parser replay, and rendering are unchanged.

Both application and syntax JSON records have an additive `retained` object with
schema 1. Historical records without that object remain readable, but do not
provide these measurements. The reader validates new records when present.

## Owners and limits

- Source pages: one shared cache for the review's patch, full-source, and index
  stores. The payload is the cached byte slices, counted once, not once per file.
  Page values, occupied map elements, and FIFO extent are separate from payload.
  An application without a published file reports `null`, not zero occupancy.
- Geometry: cached display text and its wrap/checkpoint boundaries. The charge
  includes each line value and vector capacities. Text bytes are a subset of that
  charge, not an additional allocation. Occupied map elements and FIFO extent are
  separate. Transient lines that were not admitted are not counted.
- Live parser state: patch continuation and each complete-source side are
  reported separately. Parser stack/capture capacities and highlighter
  style/scope capacities are separate heap counts. Inline state sizes are
  separate and include empty option storage. Unknown heap counts are `null`.
- Checkpoints: current entry count, deque capacity, charged heap, independently
  recomputed heap, and the existing retained charge. The retained charge already
  includes deque capacity and the cache value. Do not add those components again.

All methods borrow existing owners. They do not fetch source pages, construct
geometry, parse text, clone parser states, or retain reference-counted values.
Source-cache locking and geometry borrowing failures stop the observation.
The worker observes only between requests, after temporary checkpoint protection
has ended. Normal request priority and shutdown behavior remain unchanged.

Worker and application measurements have separate, ordered time intervals.
They are not an atomic process snapshot. Application intervals include cache,
context-view, page, and geometry inspection before the end timestamp.

These are Rust value extents and existing budget charges, not allocator usable
sizes or total memory. They omit unused hash-table buckets, reference-count
headers, allocator overhead, stacks, request-local buffers, syntax repository
metadata, and source-derived regex engine storage. Static compiled-engine
estimates remain in the separate existing census. Diagnostic serialization and
native inspection can affect process residency. Do not subtract these partial
counts from RSS and assign the remainder to an owner.

## Verification

The storage test fills the real shared cache through eviction, compares
observations through different stores, and checks owner release. The geometry
test uses tabs, Unicode, and multiple wrap widths and checks unchanged allocation
addresses and reference counts. The checkpoint restoration test inspects real
captured parser states before comparing subsequent colors to sequential parsing.
Real-terminal controls check repeated idle snapshots and unchanged terminal cells.
Complete-source captures also check exact source text, side, line, and foreground
colors against the preserved sequential reference.

## Settled observation boundary

The PTY harness can explicitly disable background RSS with
`background_rss=False`. Its existing hold helper can then use
`foreground_rss=True` to collect the same strict process-group queries inside
an already-validated settled hold. The helper rejects a live or previously
started background sampler and rejects sampling after shutdown.

This mode measures held RSS only. It has no startup, source-acquisition,
navigation, or peak-memory coverage, and no sampled safety stop outside holds.
Use it only for declared bounded owner observations with a readiness watchdog,
not for performance or peak-memory qualification. Synchronous queries can delay
PTY consumption by their existing timeout, so these captures do not measure
responsiveness.

Within each hold, keep every raw query. Errors and unavailable values fail the
hold; they are not converted to zero or retried. Apply the existing sampled
memory stop and require at least five fully contained queries. Record exact
start, deadline, end, and query indices. Default harness callers still sample
in the background. The strict RSS parser and its protocol are unchanged.
