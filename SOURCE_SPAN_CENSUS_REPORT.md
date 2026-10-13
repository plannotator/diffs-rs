# Source-span census

Date: 2026-09-23. Complete. No production implementation changed.

## Outcome

The census checked all 20,847,451 source records across the five pinned patches.
The largest patch supports a conditional reduction of 35.05 MiB in accounted storage.
That value includes the extra file descriptors, but excludes allocator overhead.
It is not a measured heap, RSS, or speed improvement.

The candidate merits an isolated prototype, not adoption.
The current ordinary executable and all 2,083 runtime/build inputs remain unchanged.
The [original plan](SOURCE_SPAN_CENSUS_PLAN.md) remains frozen as the pre-capture declaration.

## Counts and conditional model

An eligible file uses compact offsets and has no record longer than 65,535 bytes.
Lengths count stored source bytes, including any final newline.
Every record must fit. One longer record keeps the entire file on the existing representation.
Empty files qualify, but do not add payload savings or allocations.

| Input | Eligible files / all files | Eligible records / all records | Net accounted reduction | Additional allocations |
| --- | ---: | ---: | ---: | ---: |
| Pierre | 73 / 73 | 18,420 / 18,420 | 0.033 MiB | 69 |
| Ghostty | 25 / 25 | 93,579 / 93,579 | 0.178 MiB | 25 |
| Node | 3,420 / 3,420 | 419,306 / 419,306 | 0.721 MiB | 3,411 |
| Bun | 2,188 / 2,188 | 1,016,971 / 1,016,971 | 1.890 MiB | 1,819 |
| Linux | 76,870 / 76,872 | 19,299,167 / 19,299,175 | 35.051 MiB | 75,238 |

Linux contains two records longer than 65,535 bytes.
Their two files contain eight records in total, so the model excludes all eight from savings.
Its longest stored record has 357,708 bytes.
All corpus files use the existing compact-offset representation. None requires the wide fallback.

The model uses separate boxed `u32` offsets and `u16` lengths for eligible files.
It retains the current compact and wide alternatives, with unchanged newline flags.
The compiler reports these sizes on the recorded ARM64 macOS toolchain:

- Current span descriptor: 40 bytes. Modeled descriptor: 64 bytes.
- Current file descriptor: 360 bytes. Modeled descriptor: 384 bytes.
- A `(u32, u16)` tuple: 8 bytes, not 6 bytes.
- Separate offset and length elements: 4 and 2 bytes, with alignments of 4 and 2 bytes.

The model charges 24 extra descriptor bytes to every file, including empty and ineligible files.
It adds one allocation per eligible nonempty file.
On Linux, 38,598,334 fewer payload bytes minus 1,844,928 extra descriptor bytes gives 36,753,406 fewer accounted bytes.

Small inputs can become larger.
The empty-file fixture gains 48 accounted bytes. The metadata fixture gains 114 bytes.
Extra allocation metadata, size-class rounding, and access costs can further reduce the benefit.
The census does not measure those costs.

## Checks

The diagnostic borrows published spans without source-byte reads, record clones, syntax loading, or mutation.
It checks every source range against its spool extent.
All five corpus file counts, record counts, and source-index extents match the preserved ordinary summary.
The source cache stays empty for every census input.

Four existing correctness fixtures and one boundary fixture pass exact aggregate checks.
Their expectations use ordinary canonical output checked against the independent patch decoder.
The boundary fixture includes empty files, empty records, tabs, Unicode, newline markers, and mixed length eligibility.
It distinguishes 15 individually short records from 14 records in eligible files.

Three real-constructor checks pass, including the two existing storage checks.
They cover lengths of 65,535, 65,536, and `u32::MAX`, offsets beyond `u32::MAX`, overflow rejection, and flags across byte boundaries.
Numeric controls cover wide ranges without a multi-gigabyte physical fixture.
Wide storage retains absolute offsets, so its diagnostic reports absolute widths rather than reconstructing an unavailable file-relative base.

Both the ordinary executable and diagnostic reject the two malformed patch controls: four process checks, with no crashes.
Formatting and diagnostic Clippy checks pass. The existing vendored lifetime warning remains unchanged.
An independent read-only review found no unresolved material errors in the completed census or model arithmetic.
The full viewer suite and timing campaign did not repeat because production code did not change.

## Preserved correction and evidence

The first attempt completed all five corpus inputs and four existing fixtures, then rejected the new boundary fixture.
Its empty-file header lacked the required index line.
The corrected fixture adds only that line. No parser behavior changed, and no completed corpus census repeated.

The first driver also omitted checks for two declared reference hashes.
The completion checks those hashes, helpers, source, executables, and all completed input identities before and after the controls.
The failed attempt and original driver remain intact.

- [Declaration](evidence/source-span-census-v1/declaration.json) and [isolated source archive](evidence/source-span-census-v1/capture-001/diagnostic-source.tar.gz).
- [Original counts and preserved failure](evidence/source-span-census-v1/capture-001/results.json).
- [Completed controls, identity checks, and model reconciliation](evidence/source-span-census-v1/controls-002/completion.json).
- [Constructor checks](evidence/source-span-census-v1/capture-001/tests.log) and [diagnostic Clippy log](evidence/source-span-census-v1/capture-001/clippy.log).

Ordinary executable SHA-256: `1ab24c0c70c45a5b480b83e111fa909d33ed56f4f4af9510b30dad72558f2e4e`.
Diagnostic SHA-256: `a86e229bc07978dd29a4c78c0fa58216ec58f4b6b8be72ddc2f2fcaeb9c7053c`.
Compiler: Rust 1.96.0, `ac68faa20`, target `aarch64-apple-darwin`.

## Next decision

The next implementation candidate is exact shorter-length storage with the full existing fallback.
An isolated prototype must preserve complete source identity, flags, projection, and error handling before any timing comparison.
The current parser, renderer, cache limits, grammars, workload, and independent expectations remain fixed.
Real-process checks must cover navigation, wrapping, resizing, source expansion, cancellation, and cleanup.

Adoption requires ordinary-build memory and responsiveness evidence, including small-file costs and construction costs.
If extra allocations or access costs outweigh the benefit, the prototype remains unadopted.
This representation does not address continued style-cache growth or complete-source qualification.
Those concerns remain separate. No cache-policy change belongs in this candidate.
