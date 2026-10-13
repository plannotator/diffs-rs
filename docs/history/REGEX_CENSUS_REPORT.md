# Live compiled-pattern ownership: confirmed duplication

Date: 2026-09-13. The diagnostic is complete. No production memory optimization
or new accepted performance series was made in this stage.

## Result

One declared real-PTY process completed the existing showcase trace three times.
All 411 action views and the opening matched the frozen source and style cells.
The viewer exited normally, restored terminal settings, emitted all cleanup
sequences, and completed the census before exit. No sampler error remained after
cleanup. The original style-reference identity and baseline trace semantics match.

| Observed static state at shutdown | Count |
| --- | ---: |
| Loaded grammars / available grammars | 7 / 213 |
| Compiled rule references | 1,218 |
| Uncompiled rule references in loaded grammars | 717 |
| Distinct live compiled owners | 827 |
| Distinct exact compiled patterns | 563 |
| Patterns with multiple live owners across grammars | 254 |
| Extra owners beyond one per exact pattern | 264 |

The loaded grammars were Plain Text, HTML, Markdown, Bash, its built-in-command
grammar, TypeScript, and TypeScriptReact. The first-line cache remained
uninitialized. The census included already-loaded hidden/embedded grammars and
did not initialize the other 206 grammars.

The reduction first deduplicates cell identities, then groups exact pattern
strings. References to a shared cell count once. Uncompiled patterns count zero.
This establishes simultaneous duplication of compiled static roots, not duplicated
bytes. Engine search-cache sizes can differ; temporary backreference substitutions
are excluded. Do not apply the duplicate-root fraction to heap or RSS totals.

## Retained memory attribution

Twenty-five retained source-page profile/accounting files match their original
SHA-256 seal. The largest input's disjoint document structures account for
399,862,618 bytes, or 381.34 MiB. The small showcase's corresponding total is
231,650 bytes, or 0.22 MiB. Source payload in the spool and projection subcategories
are not added to these totals a second time.

The current preserved `b749889e…` candidate's unlogged largest-input profile has
465.77 MiB live at opening and 742.55, 761.12, and 761.20 MiB after three tours.
The last snapshot reports 825.3 MiB physical footprint and 39.7 MiB allocator-zone
fragmentation under that tool's definition. These are overlapping measurements,
not additive owners. Sequential profiler commands are not simultaneous samples.

Its separate stack-logged third-tour heap has 142,457,152 bytes, or 135.86 MiB,
in named `regex_automata`/`fancy_regex` classes. This uses the existing named-class
method and is a partial engine total. The earlier showcase profile has
223,523,136 bytes, or 213.17 MiB, in those classes. That older profile is not
relabeled as a measurement of the current diagnostic build.

None of these separate profiles explains the latest ordinary RSS cohort's abrupt
drops. There is no contemporaneous live-allocation/residency timeline at the
drop. No release, compression, or application saving is attributed to it.

## Implementation and verification

The off-by-default `regex-census` feature borrows initialized grammar/pattern
state. It does not clone compiled owners, compile patterns, load grammars, change
the engine, or introduce a registry. Its shutdown-only worker join is also gated
by that feature. Ordinary matching, cache policy, rendering, and parser limits
remain unchanged.

- Three real-grammar tests pass: cold/shared/independent owners, unchanged
  multiline parser state and operations, and lazy first-line-cache inspection.
- All nine existing library checks, 32 terminal/CLI process checks, and the
  real-worker integration check pass in the diagnostic configuration. Two
  existing opt-in process tests remain ignored. Tests that honor the executable
  override use the preserved diagnostic binary; direct Cargo tests use their
  separate test build.
- Real PTY controls confirm completed output, preserved existing files with a
  detected write error, and no census output from the newly built ordinary
  configuration. Formatting, Python compilation, and ordinary/diagnostic
  application Clippy pass. The existing upstream lifetime-spelling warning stays.
- Review fixed a possible late RSS-error race and added explicit worker/write
  error rejection before the declared tour. Earlier control versions remain.
- An allocator-environment preflight stopped before spawning a process. The
  parent has `MallocNanoZone=0`; the subsequent declared run records and preserves
  it. Stack logging, injected libraries, and delivery tracing were absent.

The diagnostic uses the memory trace's actions but omits its settled holds. It
is not a replacement RSS protocol or responsiveness result. The incidental quit
observation in its process report is not an accepted latency measurement. No
actual terminal pixels were inspected in this stage.

## Next bounded work

Design sharing within one syntax set, not a process-global registry. Preserve
lazy compilation, exact patterns and compiler options, distinct match-rule
actions, caller-owned captures, source-derived delimiters, and owner release.
Resolve clone/serialization behavior and concurrent first-use races explicitly.
Keep the current implementation as the control.

Before adopting it, use real grammar/capture controls and the same frozen views,
then one predeclared allocation comparison with the same allocator environment.
Count registry/key overhead as well as engine allocations. Only a measured owner
reduction with correct output should advance to ordinary responsiveness and
sustained-memory checks. Do not infer a saving from this census or adjust targets.

The combined application candidate remains unpromoted. Existing coloring delays,
memory goals, and physical terminal-paint verification remain open.

## Evidence

- [Declared scope](../../REGEX_CENSUS_PLAN.md)
- [Raw census](../../evidence/regex-census-v1/three-tour/census.json)
- [Process and reduced results](../../evidence/regex-census-v1/three-tour/report.json)
- [Run bindings](../../evidence/regex-census-v1/three-tour/declaration.json)
- [Final process controls](../../evidence/regex-census-v1/process-controls-ready/report.json)
- [Diagnostic build](../../evidence/regex-census-v1/diagnostic/build.json), executable
  `927125341a2fbe149add058f8a4565f667707ff9cbabecc384c3ee4fa81e1e34`
- [Ordinary build](../../evidence/regex-census-v1/ordinary/build.json), executable
  `dd9a750df3d5c75c7737704b549b0ff57a35e1d5e9b697d98ebff88e409641a5`

The preserved `b749889e…` executable and all earlier benchmark evidence remain
untouched. Different build identities are kept distinct; the ordinary build here
has only its opening control, not a new full performance qualification.
