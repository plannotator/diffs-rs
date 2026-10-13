# Complete-source saved-state reuse

Date: 2026-09-22. Implementation and correctness checks are complete.
The original ordinary comparison stopped because the Mac changed to battery power.
The [completion report](LOCAL_SOURCE_REPLAY_COMPARISON.md) records the eight resumed runs, all eighteen outcomes, and the remaining limits.
The sections below preserve the original interruption checkpoint.

## Change

The worker can reuse compatible parser checkpoints across hidden gaps and theme changes.
It still parses unknown source prefixes. An uninterrupted cold opening has no later checkpoint to reuse.

Checkpoint compatibility uses the immutable source origin, file, and parser scope.
Active color state still includes the request epoch and appearance. Restored state rebuilds colors for the current theme.
Old and new source sides remain separate. A different checked source pair cannot reuse state from the previous pair.

Before a requested line, the reader can restore an earlier checkpoint inside the hidden gap.
The reader and parser move together within the same snapshot. The requested line itself is never skipped.
Cancellation, fallback, source-error handling, and reply acceptance retain their existing checks.

The change affects three existing highlighting modules. It changes no source bytes, diff layout, navigation, dependency, worker count, or cache allowance.
The checkpoint limits remain 64 entries, 16 MiB total, and 256 KiB per entry.
These limits do not establish a total-process memory guarantee.

## Correctness and work reduction

The final ordinary suite passed **115 checks**, including **61 real-process contracts**. Four opt-in checks remain excluded.
The final combined `delivery-trace,owner-census` suite passed **133 checks**, with the same four exclusions.
Formatting and application Clippy passed. The existing vendored Syntect lifetime warning remains unchanged.

The new checks use real source capture, the production worker, and uninterrupted full-file color references.
They cover multiple gaps, checkpoint boundaries, both theme transitions, and source replacement for the exact same patch entry.
The existing checks cover divergent side syntax, embedded languages, different extensions, fallback, eviction, cancellation, queue saturation, and cleanup.
The private-scope and captured-delimiter check now restores across themes and checks duplicate suppression across epochs.

The reuse check failed before the implementation, then passed after it.
Its separate diagnostic process recorded these exact counts across both sides:

| Request over the same sparse source | Parsed source lines | Published styles |
| --- | ---: | ---: |
| First request | 11,266 | 12 |
| Same-theme revisit | 2,056 | 12 |
| Light-theme revisit | 2,056 | 12 |
| Dark-theme revisit | 2,056 | 12 |

Every published token matched its sequential reference. No trace records were dropped.
These are work counts, not latency measurements. The trace durations from the battery-powered count check are not timing evidence.
The bounded fixture retains its useful checkpoints. Larger inputs can evict checkpoints and require more replay.

The ordinary executable passed all **33 complete reference views** across three local-source cases.
The two prepared cases retain their existing independently checked source and color expectations.
A separate 2,204-line Rust source case uses 13 comment additions and a sequential color reference.
Its second page action changes the viewport. Its first page action changes only selection.

## Interrupted ordinary comparison

The declaration specifies three paired trials per case, with reversed executable order in the middle trial.
Each process uses eleven two-second action windows. Expensive screen checks run after process exit.
The capture retains raw output, input boundaries, process resources, and AC checks.

**Ten of eighteen runs completed.** All 110 completed views matched exactly. All ten processes passed terminal and spool cleanup checks.
Those runs recorded 120 AC checks. The next run completed three action windows before an AC check detected battery power.
The driver stopped that process. Its partial output remains saved and contributes no completed timing sample.
Its private spool directory is empty.

The Mac also had unrelated application activity. This was not an isolated timing environment.
The incomplete prefix contains two paired trials for each prepared case and one pair for the real-source case.
The partial summary retains every completed value, including slower opening observations for the changed build.
It does not establish a general speedup, a cold-opening improvement, or sustained-memory behavior.

The small cases showed no large process-memory increase in the completed observations.
Median OS peak RSS was 29.16 MiB for both builds in the two-file case.
The Git case recorded 33.29 MiB before and 34.08 MiB after. The single real-source pair recorded 17.02 and 16.67 MiB.
These are short-run observations on small inputs, not results for the largest patch or a memory plateau.
The complete raw values and timing observations remain in [the partial summary](evidence/local-source-replay-v1/partial-summary.json).

## Build and evidence

Current ordinary executable: `target/release/native-diff`.

SHA-256: `3f87c079518ee654053658a7015e56319a5c377a05ce568f45a767b456be5a15`.

Preserved prior executable SHA-256: `c3b337c3549efab30d7fee3f5089d4f9af733a143e28c091cbbd45a21b462074`.

The [evidence directory](evidence/local-source-replay-v1/) contains both ordinary executables, source archives, test logs, references, raw captures, and the worker trace.
The [implementation review](evidence/local-source-replay-v1/REVIEW.md) records ownership, restoration, test additions, and preparation corrections.
The [implementation plan](LOCAL_SOURCE_REPLAY_PLAN.md) defines the unchanged scope and invariants.

## Next checkpoint

On AC power, resume the eight remaining declared runs. Preserve the interrupted attempt separately and retain the same executable hashes, inputs, references, and action order.
Do not repeat the ten completed runs to replace their values. A new capture declaration must record the interruption and resumption conditions.
After the sequence completes, assess the whole series before accepting any timing conclusion.

Broader qualification still needs repeated source-aware expansion, cache pressure, active-work interruption, sustained memory, and physical terminal inspection.
The existing five-patch campaign, GitHub acquisition, and release qualification remain separate tasks.
