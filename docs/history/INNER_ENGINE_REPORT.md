# Scoped inner-engine sharing

Implemented and checked on 2026-09-13. The preserved ordinary candidate is
`9f16c2c92a9efd2d3cf77e5c0a5a41d314124099cafa577ad36116ed83a1b657`.
It passes the focused correctness, allocation, and responsiveness gates.
It is not a release promotion or a completed ordinary-memory qualification.

## Change and ownership

Identical compiled regex fragments now share complete engine handles and their
thread-safe search-buffer pools within one syntax-set family. Cache identity is
the exact cooked expression, eleven syntax settings, and both compiler limits.
Entries hold weak references. Compilation occurs outside the registry lock;
matching does not look up the registry. Ordinary regex cloning still makes an
independent pool.

Captures, VM mappings, old/new parser state, and rule actions remain private.
Static rules receive cache admission; source-derived delimiters, validation, and
first-line compilation do not. No application source/index/render/navigation
code, grammar, cache limit, or highlighting guard changed. The application opts
into a small feature-gated patch in the pinned dependencies. No fixture-specific
runtime behavior was added.

The [predeclared plan](INNER_ENGINE_PLAN.md), vendored provenance notes, and
[source packages](../../evidence/inner-engine-v1/candidate/build-source.tar.gz) record
the implementation. The independent review found no ownership or matching defect;
its Unicode, admission, and provenance gaps were addressed before qualification.

## Correctness checks

- 9 application library tests, 32 real terminal/CLI tests, 8 grammar/census tests,
  2 shared-engine integration tests, and 1 real worker test passed. Two existing
  opt-in process tests remain ignored.
- The engine dependency passed 231 tests with sharing enabled and 229 with its
  default features. Its no-std library check passed. Tests cover real capture
  offsets, multibyte Unicode, unequal VM capture mappings, configuration/error
  isolation, concurrent compilation/search, independent families, ordinary
  clones, and final-owner release.
- 111 available syntax-library tests passed, including actual source-derived
  delimiter admission and clone/serialization checks. The initial full suite
  failed one theme-loading test because its upstream theme submodule is absent.
  That failure and the exact final exclusion remain in the
  [development record](../../evidence/inner-engine-v1/DEVELOPMENT.md); the full upstream
  suite is not reported as passing.
- All five patches matched the independent canonical source expectations:
  20,847,451 supplied source records. Full split/unified projection, source
  partners, inverse lookups, and boundary probes matched the preserved model.
  The projection check also covered 2,888 generated cases and 716 cases rejected
  by both parsers.
- The census matched 415 frozen terminal views and passed its seven process
  controls. Both allocation processes matched the same 415 views each. All normal
  exits restored terminal state. Formatting and application Clippy passed;
  existing dependency warnings remain.

## Retained allocation result

One ordinary reference and one ordinary candidate were profiled, in that order,
on the unchanged addition-heavy patch and three-tour trace. Both inherited
`MallocNanoZone=0`; allocation profiling alone enabled `MallocStackLogging=1`.
No failed profile or replacement pair was discarded.

| Snapshot | Reference live heap | Candidate live heap | Net reduction |
|---|---:|---:|---:|
| Opening | 24.79 MiB | 23.12 MiB | 1.67 MiB |
| Tour 1 | 254.06 MiB | 131.33 MiB | 122.73 MiB |
| Tour 2 | 297.41 MiB | 141.61 MiB | 155.80 MiB |
| Tour 3 | 313.71 MiB | 151.46 MiB | 162.25 MiB |

These are reconciled native live-heap totals, including registry overhead, under
allocation instrumentation. Partial engine-named allocation classes fall by
139.37 MiB at tour three. They support the ownership attribution but do not form
a complete owner partition. The earlier 74.21 MiB strategy estimate was neither
a target nor a prediction of this allocator result. Do not add strategy
estimates, stack totals, and allocator totals together.

The separate candidate census ends with 7,061 engine references using 1,059
handles, versus 6,984 separate instances in the earlier census. All 6,984 prior
semantic references, compiler data, and capture mappings remain unchanged;
77 additional references were compiled during background work. The final style
cache has 78,025 entries / 28.83 MiB of accounted weight, versus the prior
76,431 entries / 28.14 MiB. Visible workloads match, but internal background
progress is not claimed to be identical. Do not subtract cache weights from the
separate allocation profiles.

Residual growth remains: candidate logged live heap rises from 131.33 to
151.46 MiB across tours. This stage does not establish a plateau, a total-memory
bound, ordinary RSS savings, or a new largest-patch peak.

## Focused ordinary responsiveness

The existing capture-first gate passed, including exact disk replay and damaged
recording controls. Five alternating pairs then completed 40 visual sessions and
280 action screens. Ten joined-emoji case entries remain explicitly source-only
because of the existing decoder limitation. The binaries, workload, observer,
and expected screens stayed frozen; there were no extra repetitions.

The complete [28-row summary](../../evidence/inner-engine-v1/paired/summary.json) retains
all five observations per build, content/color/readiness milestones, and maxima.
Examples of body-color receipt medians:

| Case/action | Reference | Candidate |
|---|---:|---:|
| Many small files: opening | 16.95 ms | 11.29 ms |
| Continuous file: opening | 31.15 ms | 13.86 ms |
| Continuous file: end jump | 64.16 ms | 18.21 ms |
| Real repository: opening | 36.55 ms | 22.44 ms |
| Guard: light theme | 59.39 ms | 61.16 ms |

The guard/light theme case is a small unresolved regression in this cohort.
Exact-ready median rises from 103.24 to 107.34 ms, and its maximum rises from
106.48 to 110.35 ms. The candidate is slower in four of five matched pairs;
its cause is not established. Other slower results are also retained, including
the real-repository dark-theme maximum of 10.20 versus 8.22 ms. Do not call the
candidate uniformly faster or attribute slower readings to noise without evidence.

The review judgment is that these small delays do not block this bounded stage.
They remain open. These are terminal receipt observations, not physical paint
measurements or large-corpus responsiveness qualification. The short capture
runner does not retain ordinary memory samples. The
[pre-capture clarification](../../evidence/inner-engine-v1/QUALIFICATION_LIMITS.md)
corrects the plan's overly broad ordinary-memory wording and records the
optimized-Python guard correction without rewriting the frozen profile driver.

## Next boundary

Run the existing full five-patch checkpoint on this preserved candidate, with
separate text/coloring milestones and ordinary three-tour memory holds. Keep the
same source expectations and workload. That is needed before making current
opening, scrolling, resizing, sustained-RSS, or largest-patch peak claims.

Physical Ghostty/Kitty presentation remains unverified. Earlier distant-coloring
and large-input performance issues are not declared resolved by this smaller
comparison. No further runtime change is justified by a target number alone.

All viewer, build, profile, and comparison work from this stage has finished.
The [offline audit](../../evidence/inner-engine-v1/audit.json) reconciles source bindings,
ownership, allocation evidence, and field accounting. The stage's checksum
manifest and supporting-source archive preserve its reports, tests, and drivers.
