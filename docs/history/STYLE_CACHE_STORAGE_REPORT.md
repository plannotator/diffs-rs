# Whole style-cache accounting

Current status: qualified and integrated on 2026-09-27.
The [current state](CURRENT_STATE.md#current-checkpoint) identifies the normal executable and its completed integration checks.
The preserved candidate completed its ordinary comparison and Rust 1.88 checks before integration.
Those measurements do not describe a new comparison of the integrated executable.

## Original isolated checkpoint

Date: 2026-09-26.

The isolated prototype reports the cache's retained requested storage without counting any owned allocation twice.
It does not reduce memory, correct history-dependent table growth, or enforce a total-storage limit.
The normal source and executable remain unchanged.

The [plan](STYLE_CACHE_STORAGE_PLAN.md) defines the ownership boundaries and unchanged behavior.
The verification report (temporary verification record `2026-09-26-cache-storage.pdKhDF/report.html`, not retained) links this checkpoint to its evidence.
The isolated source, preserved executables, logs, and records are in the run directory (temporary verification record `2026-09-26-cache-storage.pdKhDF`, not retained).

## Implementation

The style cache uses hashbrown 0.16.1 with the standard randomized hasher in the isolated copy.
The package already exists in the lockfile, but the direct dependency and container change still require qualification.
The dependency declaration requests no default features.
Existing transitive dependencies already enable those defaults, as the saved feature tree records.

The diagnostic uses the pinned public allocation-size API.
It adds table storage, retained vector capacity, and FIFO capacity exactly once.
It reports inline cache storage separately, including the diagnostic counter extent.
Occupied keys, occupied style values, and the existing style charge remain overlapping views, not additional allocations.

The existing 32 MiB style charge, duplicate handling, insertion order, eviction, and source invalidation remain unchanged.
Token collection, token contents, old/new sides, inline emphasis, and source identities remain unchanged.
No application unsafe code, custom allocator, fixed hash seed, or input-specific policy was added.

These totals exclude allocator rounding, allocator metadata, retained free pages, transient growth, other owners, and process RSS.
The API supplies profiling information, not a portable admission-control guarantee.

## Checks completed

- 124 ordinary checks passed, including 61 real-terminal process checks.
- 140 diagnostic checks passed, including the same 61 terminal contracts in the diagnostic configuration.
- Four opt-in checks remained excluded from each main suite.
- The compatible-baseline check ran separately against preserved executables.
  It matched 96 full terminal grids and two canonical source streams.
- Real allocations covered populated, duplicated, evicted-empty, and reset caches.
- Ten real source-transition records reconciled patch/context ownership, replacement, revocation, stale replies, unrelated-file survival, and theme reset.
- Seven real-process diagnostic controls passed, including eight stable observations, the observation limit, collisions, disabled diagnostics, and terminal cleanup.
- The reader rejected 19 storage-corruption cases and 40 activity-corruption cases.
  All ten source-transition records also passed the 19 storage-corruption cases.
- Four historical storage formats remained readable without invented allocation totals.
- Formatting and strict application Clippy passed. The existing vendored Syntect lifetime warning remains.

The grid comparison covers layouts, themes, wrapping, navigation, mouse input, sidebar behavior, and four terminal widths.
It checks emitted terminal cells, not physical paint timing or scrolling feel.
The completed five-patch comparison cohorts were not repeated.

## Example of the new distinction

The small source-transition case reported these requested byte extents:

| State | Entries | Existing style charge | Table | Vectors | FIFO | Reported heap |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Checked sources | 18 | 3,856 | 3,112 | 2,416 | 512 | 6,040 |
| One source invalidated | 9 | 1,928 | 3,112 | 1,208 | 512 | 4,832 |
| Full reset | 0 | 0 | 0 | 0 | 0 | 0 |

Invalidation removes the selected styles but retains the table and FIFO allocations.
The existing style charge therefore differs from total requested heap storage.
The diagnostic build's inline cache occupies another 536 bytes, including 448 bytes of diagnostic counters.
These small-case observations are accounting examples, not large-patch memory results.

## Review and evidence integrity

Independent review found a portability assumption in the existing capacity-classification test.
That test compared exact capacities with the compiler's standard map.
It now uses the pinned container for representation-specific capacity observations.
Independent survivor, source-transition, and terminal-grid checks remain separate.

This correction changes a cfg(test)-only import after preservation of the two release builds.
The integrity record identifies that delta.
The original build archives and binaries remain intact, alongside a final qualified-source archive.

Two setup failures remain preserved:

1. The source-transition export succeeded, then its command reporter collided with the exported JSON filename.
   The wrapper resumed after the successful test. No record was overwritten.
2. The isolated fixture path changed the title text in the first diagnostic control.
   That attempt stopped before a matching view, restored the terminal, and removed its process group.
   The replacement attempt used the original fixture argument. Expected cells and source bytes remained unchanged.

The evidence audit passed.
It checked build archives, executable hashes, source differences, successful controls, historical records, and cleanup from the interrupted attempt.
The reference source inventory comes from a preserved diagnostic build with matching source.
The ordinary reference executable has its own checked identity. That diagnostic recipe is not its ordinary build recipe.

| Executable | SHA-256 |
| --- | --- |
| Unchanged normal viewer | `53d28a87bd1669910adfc6f28aedb20063ddb9db270310ad056da5a20a3bd838` |
| Isolated ordinary prototype | `4132c42a0488161370783b8094468136b28478af186db4cf0b67c46106b8de35` |
| Isolated diagnostic prototype | `d9ffb9fbbb16231cebda178d6237c2e8de43d240b5e11d1dcf6796ed27e17357` |

## Original next gate

The prototype remains isolated.
The exact Rust 1.88 toolchain is not currently installed, so this checkpoint does not claim minimum-compiler qualification.
The dependency's declared minimum version is compatible, but that is not a completed build.

The next declaration must cover ordinary responsiveness and retained storage against the preserved normal viewer.
It must keep source/color expectations, workloads, and the style-charge policy unchanged.
A bounded owner observation can then connect the new component totals to real cache churn.
No performance improvement, production adoption, new budget, or new storage policy is approved by this checkpoint.

## Integration and bounded owner observations

Date: 2026-09-27.

The ordinary qualification decision (temporary verification record `2026-09-27-cache-resume.6qXEpJ/DECISION.md`, not retained) accepted the accounting foundation for integration.
It retained mixed timing results, the higher final Linux RSS, different power conditions, and the reference-to-candidate time gap.
The Rust 1.88 ordinary checks passed before integration.
These results supersede the original pending gates, without a performance-neutrality or memory-saving claim.

The integration contains exactly the nine qualified source and reader changes, including the documented test-only correction.
The previous files and executable remain in the rollback archive.
The current normal executable has SHA-256 `9350f499bda0f9442234ce856992ceb3756e0a9324302ee2a6dd62ef04dfa630`.
The separately preserved diagnostic executable has SHA-256 `b7cb65c0f9f3db36efe9e66893ff4b3287950dd025404a1d7bd2ac06e59658b6`.

Integration checks:

- The ordinary suite passed 124 checks. The diagnostic suite passed 140 checks.
- Each suite included 61 process checks and excluded four opt-in checks.
- The separate process-suite rerun passed 61 checks with mixed executable selection.
  Override-aware helpers used the normal executable. Other helpers used Cargo's test-built executable from the same source.
- The explicit-executable comparison matched 96 complete terminal grids and two canonical source streams against the qualified candidate.
- Ten real source-transition records passed the storage reader and its 19 corruption cases per record.
- The activity reader rejected 40 corruption cases.
- Formatting and strict application Clippy passed. The existing vendored Syntect lifetime warning remains.

The executable-selection note (temporary verification record `2026-09-27-cache-integration.IhX6h6/TEST_SCOPE.md`, not retained) records the process-suite limitation.
Consistent executable selection is a separate harness improvement. No helper or expectation changed during this exact integration.

The diagnostic observation used the existing Bun and Linux traces, serially, for three tours each.
Seven small real-process controls passed before those captures.
The captures passed 418 Bun views and 616 Linux views, with four snapshots per process.
Every snapshot passed component reconciliation and the 19 storage-corruption cases.
Both viewers exited cleanly, restored terminal state, and left no process group.
Power records showed AC at both boundaries. The sleep log contained no interruption.

Final retained requested storage, in MiB:

| Input | Style charge | Table | Vectors | FIFO | Reported heap |
| --- | ---: | ---: | ---: | ---: | ---: |
| Bun | 31.999 | 12.125 | 25.515 | 2.000 | 39.640 |
| Linux | 32.000 | 48.500 | 23.350 | 4.000 | 75.850 |

The heap column adds only the table, vectors, and FIFO. The style charge overlaps these owners and must not be added again.
The diagnostic inline cache occupies another 536 bytes, including 448 diagnostic bytes.
These counts exclude allocator overhead, retained free pages, transient allocations, and other owners. They do not partition process RSS.

Linux's final snapshot contained 113,370 styles, below its 148,084-entry population peak.
Its recorded capacity high-water increase occurred during eviction, without a new entry peak.
The table allocation was 50,855,944 bytes at each end-of-tour snapshot.
Activity records contain public capacity observations, not allocation-size measurements at every insertion.
Thus, the records identify retained table storage and a growth mechanism, not exact per-event allocation attribution.

Bun's last capacity high-water increase accompanied a new population peak, before budget eviction started.
Its final table occupied 12,713,992 bytes.
Neither capture proves an indefinite plateau, a leak, or a causal difference from an older run.
No ordinary timing or sustained-RSS comparison of the integrated executable ran in this checkpoint.

The evidence directory (temporary verification record `2026-09-27-cache-integration.IhX6h6`, not retained) preserves the commands, raw snapshots, environment records, and audits.
The [next candidate scope](STYLE_CACHE_STORAGE_PLAN.md#next-isolated-candidate-public-entry-insertion) uses only the public entry API.
That scope was unimplemented at the integration checkpoint. Its completed experiment follows.
Budgets, eviction, token storage, and source behavior remain unchanged.

## Public-entry insertion experiment

Date: 2026-09-27. Result: complete without adoption.

The isolated candidate uses the public entry API for duplicate lookup and insertion.
It changes no cache budget, style weight, token storage, source identity, or eviction order.
The normal source and executable remain unchanged.

The candidate passed 125 ordinary checks and 141 diagnostic checks.
Each suite included 61 enabled process checks and excluded four opt-in checks.
An independent FIFO model checked real style allocations, mixed source identities, duplicate ownership, and exact survivors.
The existing source-transition checks covered replacement, revocation, stale replies, reset, and unrelated-file survival.
Formatting and strict application Clippy passed.

The preserved ordinary candidate matched 96 complete normal-viewer grids and two canonical source streams.
Those operations selected both executable paths explicitly.
The full suites used Cargo test-built executables. They do not prove that every case used the preserved ordinary candidate.
The existing mixed-selection harness gap remains separate work.

Seven small diagnostic controls passed before the two serial candidate captures.
The captures passed 1,034 expected views and eight owner snapshots across three tours per input.
Both processes restored terminal state and left no process group.
No capture was repeated, and no sleep interruption occurred.
Power records showed AC at the observation boundaries, without an AC requirement.

| Input | Reference table MiB | Candidate table MiB | Reference cache heap MiB | Candidate cache heap MiB |
| --- | ---: | ---: | ---: | ---: |
| Bun | 12.125 | 12.125 | 39.640 | 39.638 |
| Linux | 48.500 | 48.500 | 75.850 | 75.846 |

Table allocations matched at every corresponding snapshot. Final FIFO capacities also matched.
The final heap differences were only 1,704 bytes for Bun and 5,112 bytes for Linux.
They came from retained vector capacities, with different entry counts.
These observations do not establish a causal or meaningful memory improvement.
The reference observations are preserved earlier captures, not simultaneous or same-seed controls.

The larger Linux table still arose during eviction without a new population peak.
Thus, the public entry API alone did not solve the observed retention problem.
The candidate remains isolated. No ordinary performance campaign or production adoption follows this result.
This decision does not establish a general speed regression or defect in the entry API.
The component counts are not process RSS or a total-memory guarantee.

The decision and evidence (temporary verification record `2026-09-27-entry-insertion.kPXXk5/DECISION.md`, not retained) preserve all outcomes.
The next harness task (temporary verification record `2026-09-27-entry-insertion.kPXXk5/FOLLOW_UP.md`, not retained) closes executable selection before another storage experiment.
No new cache representation or retention policy is selected.
