# Source-run storage and parser checkpoints

## Changes

Two general changes are implemented. Neither depends on a patch name, language,
repository, benchmark size, or performance target.

1. Private source-run line numbers use `Option<NonZeroU32>`. The input validator
   already requires positive line numbers. Public values, the complete positive
   `u32` domain, source records, and display mappings are unchanged.
2. The existing syntax worker retains bounded parser checkpoints. Backward
   movement and uncached revisits can resume from a compatible checkpoint instead
   of parsing the whole supplied prefix again. The closest eligible active
   continuation still takes precedence. First-cold and theme-invalidated
   prefixes still require normal parsing.

The [stage specification](NEXT_PERFORMANCE_STAGE.md) predates implementation.
The [private runbook](evidence/source-run-v1/RUNBOOK.md) and
[checkpoint experiment](evidence/source-run-v1/CHECKPOINT_EXPERIMENT.md) retain
the fixed workload, limits, failed attempts, and candidate-selection evidence.

## Storage result

The representation change reduces source-run payload on all five large patches.
On the largest patch, it saves **21,030,688 bytes (20.06 MiB)**. Other owners and
counts remain unchanged; projection totals decrease by the same amount. Total
accounted document payload
falls from 536,750,670 to 515,719,982 bytes.

Separate opening allocation profiles show live heap falling from 753,615,440
to 729,425,808 bytes: **23.07 MiB less**. The projection allocation class explains
almost all of that difference. These profiles compare the ordinary pre-change
build with the representation-only build; they do not measure checkpoint storage.

Payload bytes, allocation-instrumented live heap, and ordinary sustained RSS are
different measurements. The first two results are not a claim that total process
memory fell by the same amount.

## Repeated-coloring result

Five alternating fresh-process pairs used the unchanged deep-navigation driver.
All ten processes completed with checked destinations, colors, and clean exits.
The reference already includes the compact source-run change; this comparison
isolates the added checkpoint mechanism.

| Coloring milestone | Reference median / maximum | Checkpoint median / maximum |
| --- | ---: | ---: |
| Opening | 111.53 / 113.29 ms | 110.18 / 111.20 ms |
| First cold end visit | 1692.37 / 1708.23 ms | 1698.96 / 1739.49 ms |
| Backward uncached view | 1740.72 / 1784.51 ms | 43.87 / 44.98 ms |
| Theme change | 1672.63 / 1738.68 ms | 1678.02 / 1775.69 ms |
| Uncached forward exposure | 31.68 / 33.24 ms | 30.25 / 31.00 ms |

Correct-text delivery remained separate: backward-view medians were 3.44 and
3.36 ms. Largest observed process peaks were 65.41 and 65.81 MiB on this small
fixture. Cold/theme maxima were slower in the candidate; the series does not
establish latency equivalence or faster cold parsing.

The representation-only comparison also retains its small mixed responsiveness
differences in `evidence/source-run-v1/paired-challenges.json`. It is a storage
improvement, not evidence of faster interaction everywhere.

### Natural eviction and rapid navigation

An unchanged diagnostic workload fills the normal style cache with dense lines,
then revisits a previously colored deep file. It also sends a burst of ordinary
downward navigation while a newly exposed dense line is being colored.

The reference revealed canceled publication followed by a backward reset: it
reparsed 1519 dense predecessors. With checkpoints, the worker restored at 1280
and processed the remaining 239 predecessors. Settled coloring fell from
69.38 to 11.72 seconds. The subsequent naturally evicted deep revisit replayed
178 rather than 11,954 predecessors; coloring fell from 1785.91 to 58.94 ms.
Source checks, deep-view cell equality, identity checks, and clean quit passed.
All 40 saved settled screens also match byte-for-byte across the two builds.

These are single instrumented mechanism observations, not ordinary latency
acceptance. Dense first-time coloring remains expensive. The earlier reference
attempt exceeded its 30-second observer deadline and remains recorded as failed;
one declared 180-second observation followed with the same actions and budgets.
That watchdog is not a relaxed performance target.

## Correctness and ownership

- Complete independent source checks pass on the final ordinary executable for
  all five patches, including all 19,299,175 supplied records in the largest.
- The representation change passed complete projection comparisons across all
  five patches, five structural challenges, and exhaustive small patterns. The
  final candidate retains that same projection source. These comparisons include
  inverse positions, partners, rows, and navigation boundaries.
- All 33 real-process tests and five library checks pass. New coverage verifies
  numeric endpoints and actual parser restoration through captured delimiters,
  private cleared scopes, embedded JavaScript, both themes, and normal cache
  eviction/non-admission. Existing process cases now force backward restoration
  across separate old/new delimiters and an invalidated old side. Pending
  highlighting, rapid resize, source anchors, and cleanup checks also pass.
- All 82 vendored parsing checks pass. Formatting and application Clippy pass;
  the dependency's existing lifetime-spelling warning remains.
- A separate 20,001-record process diagnostic checks 16,344 foreground and
  51,994 background cells against the unchanged sequential oracle. It records
  14 checkpoint evictions and a restore under normal limits, reaching 64 entries
  and 42,040 owned bytes. Oversized non-admission uses real custom-grammar parser
  state in a component test, not a claimed occurrence in the five-patch corpus.

Each checkpoint owns both complete parser states, both complete scope stacks
(including private cleared scopes), and each side's validity. Document/grammar
ownership remains worker-local; identity includes file, hunk, epoch, and theme.
Width, wrapping, and layout do not invalidate syntax. No grammar, source guard,
fallback rule, word comparison, or existing cache budget was weakened.

Admission occurs every 256 supplied records relative to the hunk start. Limits
are 64 FIFO entries, 16 MiB including containers, and 256 KiB per entry. Checked
nested-capacity accounting precedes cloning; eviction occurs before admission,
and actual clone capacities are checked afterward. Oversized or unaccountable
state continues parsing without retention. Cancellation is checked before and
after cloning/restoration. Allocator overhead, active parser/color state, and
temporary clones remain outside the retained-cache limit.

Independent read-only review found no runtime defect requiring a change. Tests
do not deterministically interrupt inside the short clone operation. Diagnostic
`checkpoint_capture` counts completed clone attempts, not necessarily admissions;
`checkpoint_restore` excludes deferred color reconstruction, which remains in
`visible_colors`. The frozen analyzer accepts the added checkpoint reason, but
its older explanatory prose omits it. Checkpoint durations are part of its
unattributed remainder, not another disjoint phase to add twice.

## Full five-patch checkpoint

All **25 latency processes and five three-tour memory processes completed** with
matching checked views and clean exits. Fresh captures passed the unchanged
sequential oracle, corruption controls, and historical workload-semantics checks.
This is a completed measurement series, not a pass of every performance target.

| Patch | Third-tour settled maximum | Memory-run process peak | Memory target |
| --- | ---: | ---: | --- |
| Pierre | 272.0 MiB | 272.0 MiB | Miss |
| Ghostty | 113.6 MiB | 113.6 MiB | Pass |
| Node | 253.5 MiB | 253.5 MiB | Miss |
| Bun | 351.9 MiB | 351.9 MiB | Miss |
| Linux | 1021.9 MiB | 1051.1 MiB | Miss |

The unchanged uniform development policy is 128 MiB settled / 192 MiB peak for
every patch. These goals do not limit supported input. The largest patch's
tour maxima were 1001.1, 1021.8, and 1021.9 MiB. Three tours do not prove an
indefinite plateau or leak freedom.

Across the 25 latency processes, first text appeared within 12.04 ms and opening
coloring within 87.91 ms. All five met the first-text, opening-color, and quit
guards. Pierre's distant-coloring maximum was 280.68 ms (200 ms guard); the other
four patches met all navigation targets. The largest patch's maximum
complete-index observation was 4257.54 ms (2000 ms guard). All category p95s and
other maxima remain in `five-patch/checkpoint.json`. These trace results do not
replace the slower cold-prefix and dense-line diagnostics above.

This is the first full ordinary-build baseline for the combined candidate. The
historical 1052.5 MiB settled observation belongs to a previous test-target build;
it is not a paired control for attributing the current RSS difference. The
controlled allocation profiles establish the representation's live-heap saving,
not a causal sustained-RSS reduction. Memory remains an open engineering problem.

**Decision:** retain the compact source-run representation and bounded checkpoints.
The former reduces a demonstrated owner without changing source/projection
answers. The latter reduces repeated predecessor work with matching styles,
small observed checkpoint storage, and no material cold-path change established
by the fixed paired series. Slower individual cold/theme observations remain
reported; this is not a latency-equivalence claim. No target was relaxed.

## Reproduction and remaining boundaries

The ordinary measured executable is
[checkpoint-final/native-diff](evidence/source-run-v1/checkpoint-final/native-diff),
SHA-256 `61a3dbea242289b4fbfa3dc32b99950a4c5ff56a064a75c54039a9d6e36add23`.
Its build manifest includes source hashes, dependency features, exact command,
toolchain, SDK, and environment settings; its source archive is self-contained.
Tests and diagnostic builds use separate target directories.

A separate repeat build is not byte-identical: 48 bytes differ, confined to the
linker UUID and corresponding first-page signature digest. A read-only Mach-O
comparison verifies all code-page hashes and equality of every other byte.
Neither executable was rewritten or normalized. Every trial keeps its actual
binary hash. Earlier unmeasured build attempts remain preserved.

Actual Ghostty pixels, frame pacing, and trackpad behavior remain unverified.
PTY delivery is not physical input-to-paint latency. Full visual parity with
Pierre and a total-memory guarantee are not established by these changes.

Remaining memory work should target the measured document-index and compiled
grammar owners under the existing conditional specification. Cold/dense coloring
needs separate investigation; adding more checkpoints cannot eliminate a first
parse. No additional optimization or benchmark run starts automatically from
this report.
