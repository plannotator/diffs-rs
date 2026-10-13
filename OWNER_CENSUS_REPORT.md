# Retained engine and color-cache owners

Date: 2026-09-13. Diagnostic only. No runtime optimization or performance promotion.

The census identifies a concrete next target: repeated compiled fragments inside
the highlighting engine. After three tours, 6,984 inner-engine instances fall
into 1,059 exact pattern/configuration groups. The additional instances in
repeated groups account for **74.21 MiB of approximate strategy storage**. This is
an investigation estimate, not proven recoverable heap or an RSS saving.

The same run matched all 415 frozen terminal views and exited normally. The
color cache also grew in actual entries, with its accounting reconciled at every
checkpoint. Source/display rules, grammars, navigation behavior, and cache limits
are unchanged.

## What remains live

| Boundary | Compiled outer owners | Inner instances | Strategy estimate, MiB | Cached lines | Accounted cache weight, MiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| Opening | 98 | 108 | 7.90 | 43 | 0.009 |
| Tour 1 | 1,068 | 5,459 | 90.75 | 39,901 | 14.36 |
| Tour 2 | 1,174 | 6,921 | 104.16 | 63,704 | 23.23 |
| Tour 3 | 1,201 | 6,984 | 105.82 | 76,431 | 28.14 |

An outer owner is a compiled static grammar expression. Its matching program can
delegate several fragments to separate inner engines. The existing sharing of
identical outer expressions does not merge fragments within different programs.

Every previously observed inner identity retains the same owner, expression,
settings, capture mapping, and strategy estimate. Tour one to tour three adds
1,525 instances and 15.07 MiB of strategy estimates. All 17 loaded grammars remain
the same across those tours: additional rules compile within initialized grammars.
This establishes growth in retained compiled structures. It does not measure
growth inside their search-buffer pools or establish why each rule first ran in
a particular tour.

### Repeated fragments are material

The final census has 276 repeated groups and 5,925 instances beyond one per group.
All instances within each observed group report equal strategy estimates. For
example, one Unicode identifier-character fragment occurs 259 times, and a
whitespace fragment occurs 1,051 times. These are general grammar fragments, not
input-specific code paths.

Groups use the actual compiled fragment, inner syntax settings, and resolved
engine settings. Instruction locations and capture mappings remain recorded per
instance. Grouping does not prove that substituting a shared engine preserves
matching, scratch ownership, concurrency, or lifecycle behavior. Those are design
and verification requirements for a future change.

### Cache growth includes useful entries

The color cache gains 36,530 entries and 13.77 MiB of accounted weight between
tours one and three. Its usable map capacities are 57,344 → 114,688 → 114,688;
FIFO capacities are 65,536 → 65,536 → 131,072. Growth therefore accompanies more
stored line styles, not just an increasing empty container.

The existing counter equals an independent recomputation and the FIFO's summed
entry weights at every observation. All observed endpoints are below the
unchanged 32 MiB limit. These snapshots do not prove an eventual plateau or the
absence of intervening evictions.

The counter includes each occupied inline `LineStyle` and its vector capacities.
It is not an exclusive heap subtotal. Map capacity means usable slots, not raw
bucket allocation bytes. Do not add the full backing-table allocation to this
counter: occupied inline values would be counted twice. Keys, spare slots, FIFO
storage, and allocator overhead still require separate accounting.

## Scope and measurement limits

The off-by-default `owner-census` feature visits initialized syntax cells and
borrowed inner engines. It neither compiles nor searches, clones engines, loads
grammars, or checks out search scratch. A small pinned `fancy-regex 0.16.2` patch
adds only the borrowed inspection API; matching and compilation remain unchanged.

The worker observes engines between parser requests. After its acknowledgement,
the main loop observes the style cache. Both intervals are recorded. This is a
sequential pair, not an atomic heap snapshot. At most eight requests are allowed
per process. Output uses new files and never enters the terminal stream.

Strategy estimates come from pinned `regex-automata 0.4.18`. They exclude search
pools, outer VM storage, pattern strings, and allocator overhead. The application
does not clone inner handles; deduplicating shared outer owners avoids repeating
their instance estimates. The dependency's estimates still are not exact
allocator accounting.

**Search-buffer bytes remain unavailable.** The private pool has no safe,
non-creating inspection API. This stage adds no unsafe access, pool manipulation,
second engine fork, or search-path instrumentation to obtain that number.

The prior [allocation diagnostic](RETAINED_MEMORY_REPORT.md) and this census use
different observation methods and processes. Their subtotals must not be
subtracted to invent an exact scratch total or recovery estimate. No ordinary
latency, sustained-RSS, full-corpus performance, or physical terminal-paint result
was accepted here. Diagnostic pauses and allocations prevent such a claim.

## Verification and retained failures

The ordinary and diagnostic releases each pass nine library checks, 32 actual
terminal/CLI checks, and one real worker lifecycle check. The diagnostic also
passes eight real grammar/regex checks, including exact capture boundaries,
configured VM versus wrapped engines, cold cells, shared owners, and unchanged
multiline parser operations. Two existing opt-in corpus process tests remain
ignored; this stage does not claim a fresh five-patch run.

Seven final process controls pass: startup inspection during loading/highlighting;
eight identical settled snapshots followed by rejection of a ninth; collisions at
the worker file, application file, and ready marker; feature-disabled behavior;
and diagnostic behavior without an output request. Four controls intentionally
exit one and are recorded as such. Existing files survive, incomplete writes do
not gain a new ready marker, and every process group is gone.

One declared census process follows the unchanged 138-action addition-heavy trace
for three tours. The opening plus 414 action views pass exact frozen text/color
assertions. Exit zero, terminal restoration, sampler shutdown before quit, and
process-group cleanup all pass. An independent reviewer and the offline audit
reconcile the raw counts, stable identities, view sequence, hashes, and cleanup.
The output stream is hashed and its tail retained, not archived in full; this
audit cannot independently replay every actual screen or establish Ghostty paint.

Two earlier control attempts remain: a preflight rejected inherited
`MallocNanoZone=0` before starting a viewer; a startup detector missed the title's
leading space and was stopped with cleanup. The runner now records and preserves
the inherited allocator setting, as the existing tools do, and recognizes the
title padding. The actual three-tour census ran once, after controls passed.
An explicit optimized-Python rejection is also retained so disabled assertions
cannot yield a false pass. Formatting and application Clippy pass; two warnings
in unchanged upstream dependency code remain.

## Next bounded work

Scope sharing of identical inner compiled fragments first. Keep sharing scoped
to an explicit syntax-set/compiler owner, not a permanent global cache. Preserve
the full compilation identity, VM instructions, capture mappings, independent
old/new parser states, search behavior, error handling, and release semantics.
Determine whether immutable strategy sharing or shared handles is appropriate
before changing the representation.

Require real capture, lookaround, Unicode, configured-expression, concurrency,
and lifetime checks; existing source/cell contracts; then a declared allocation
comparison and ordinary responsiveness/memory qualification. Keep the candidate
only if the measured benefit survives those checks. Do not treat 74.21 MiB as a
target, change grammar coverage, or lower cache limits to obtain it.

Keep cache-container overhead and unmeasured search pools on the list, but do not
combine them into the first optimization. The current candidate remains
unpromoted; previous coloring delays and physical terminal verification remain open.

## Evidence

- [Declaration](evidence/owner-census-v1/PLAN.md), [offline audit](evidence/owner-census-v1/audit.json), and [audit program](evidence/owner-census-v1/audit.py).
- [Raw observations and process record](evidence/owner-census-v1/census/process/process.json), [bound run inputs](evidence/owner-census-v1/census/declaration.json), and [completed controls](evidence/owner-census-v1/controls-3/report.json).
- Ordinary control `18177558…` and diagnostic `81962762…`, each preserved with its exact source package under `evidence/owner-census-v1/`. The earlier ordinary `da5754e8…` executable is unchanged.
- [Application feature](src/owner_census.rs), [borrowed engine inspection](vendor/fancy-regex/src/owner_census.rs), and [vendor provenance](vendor/fancy-regex/NATIVE_DIFF_PATCH.md).
