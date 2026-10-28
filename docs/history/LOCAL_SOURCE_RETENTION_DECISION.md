# Checkpoint-retention decision

Date: 2026-09-23. Scope: request-aware checkpoint retention, with the accepted Git wait unchanged in both versions.

Decision: adopt the bounded policy for reduced repeated parser work, with the observed smaller-workload costs recorded below.
The ordinary comparison, diagnostic gates, and normal-build qualification passed.

## Implementation and review

The coloring worker protects compatible saved states that the current complete-source file job can reuse.
Unprotected entries keep FIFO order. Admission checks capacity before eviction.
If no unprotected capacity remains, parsing continues without another checkpoint.
The scope guard clears protection on normal completion, cancellation, errors, and unwind.

The policy uses source identity and requested coordinates. It does not inspect filenames, languages, workload names, or timing targets.
The limits remain 64 entries, 16 MiB in aggregate, and 256 KiB per entry.
The added flags occupy 512 bytes across the allocated slots on this host. Existing accounting includes those bytes.
The change adds no dependency, source copy, worker, public control, or second cache.

Protection can reduce new checkpoint coverage during a job. It does not guarantee a maximum replay distance or faster future navigation.
Cold opening still parses unknown source prefixes. Patch-only rendering and source acquisition retain their existing behavior.

The independent review found no actionable correctness or ownership defect. `REVIEW.md` records its reasoning and coverage limits.
The checks cover entry pressure and oversized real parser states, but not the narrow partially protected aggregate-byte boundary.
The unchanged count and per-entry limits make that boundary require almost 64 maximal entries.
The reviewed aggregate check remains necessary. The report does not claim complete boundary coverage.

## Correctness before capture

The ordinary candidate passed all 120 enabled checks, including 61 terminal-process contracts. Four opt-in checks remained excluded.
The diagnostic baseline passed 80 selected checks. The diagnostic candidate passed 83, including the parser-work regression.
Formatting and application Clippy passed for all three copies. The existing vendored Syntect lifetime warning remains unchanged.

The new real-worker checks compare divergent old/new syntax states against independent sequential colors.
They cover sparse pressure, revisits, both themes, a full reply queue, cancellation, and release of the previous complete source.
The earlier FIFO negative control fails the saved-work assertion. The retained policy passes it without changing expected tokens.

The integrity check found exactly three runtime differences: `src/highlight.rs`, `src/highlight/checkpoint.rs`, and `src/highlight/full_source.rs`.
The candidate also adds `tests/source_retention.rs`. Git handling is identical within both pairs.
Before capture, all 2,167 archived product files matched the normal source. Dependencies, fixtures, and expected source bytes remained unchanged.

## Targeted diagnostic

Six processes completed twelve actions each. All 72 grids matched independently checked source and color references.
The middle input reversed role order. Every action used a two-second window, with AC checks at each boundary.
Saved output replay reproduced text and complete-cell milestones. Terminal restoration, private-file cleanup, RSS containment, and sampler shutdown passed.
No diagnostic events or request summaries were lost.

The sequence separates first endpoint arrival, return, and warm endpoint revisit.
It also separates gap selection from Enter, then observes unified layout, narrow/wide resizing, and both themes.
Each Enter action contained one input event and revealed additional independently checked source coordinates.

| Action | Baseline work | Candidate work |
| --- | ---: | ---: |
| Deep-source opening | 23,926 parsed lines | 23,926 parsed lines |
| Deep-source light theme | 23,922 parsed lines | 6,591 parsed lines |
| Deep-source dark theme | 23,922 parsed lines | 806 parsed lines |
| Deep-source and Git expansion | 192 parsed lines each | 192 parsed lines each |
| Rust-source expansion | 64 parsed lines | 64 parsed lines |
| Unified layout and both width changes | No coloring request | No coloring request |

Candidate checkpoint planning took at most 6.917 microseconds per action in this sequence.
The isolated Enter handler took 28–49 microseconds across both roles and all inputs.
Its complete-cell receipt took approximately 2.0–7.3 ms, including drawing and coloring.
These observations locate the work. They do not establish an ordinary-build speed improvement.

Cached layout and resize differences occur without checkpoint planning or parser work.
Drawing accounts for most measured synchronous work in those actions.
That excludes direct checkpoint planning as their cause, not every indirect or host effect.

The first deep-source endpoint was already styled during opening. It is not a cold-coloring observation.
The Git endpoint required two additional Git commands. Its warm revisit required neither Git nor coloring.
The Rust endpoint required parsing on first arrival, then no coloring on the warm revisit.
Git intermediate parsing differed by 160 lines during first arrival. Source-readiness scheduling can cancel different amounts of intermediate work.
Final cells still matched. No claim relies on equal intermediate Git work.

Trace-to-receipt clock-offset spread was 0.154–1.911 ms.
No attributed parser request or phase crossed an action boundary. Work totals match the complete trace.
The clock alignment supports action attribution, not an exact physical-paint timestamp.

## Ordinary comparison

Eighteen complete processes passed all 198 expected grids and offline replay.
The series uses three pairs per input and eleven existing actions per process.
The middle trial reverses role order. The ordinary builds contain no new attribution timers.
No owned build or competing capture overlapped either measurement series.

The fourth attempt stopped during opening because the RSS sampler could not read one child process state.
The first three completed runs remain included. A separate declaration resumes the interrupted slot and preserves the remaining schedule.
All nineteen attempts remain in the packet. The incomplete attempt contributes no timing or memory summary.
`RECOVERY.md` records the raw observation and cleanup limits. No capture helper or acceptance rule changed.
The interrupted pair has a longer separation between roles. These are shared-workstation observations, not an isolated causal estimate.

These medians measure milliseconds to reference cells at PTY receipt, not physical terminal paint.

| Input and action | Baseline | Candidate | Paired direction |
| --- | ---: | ---: | --- |
| Deep-source opening | 764.67 | 788.71 | Mixed |
| Deep-source light theme | 522.81 | 147.32 | Faster in all three pairs |
| Deep-source dark theme | 536.62 | 21.70 | Faster in all three pairs |
| Git opening | 336.43 | 300.40 | Mixed |
| Rust-source opening | 185.36 | 198.42 | Slower in all three pairs |
| Rust-source page scroll | 36.24 | 38.65 | Slower in all three pairs |
| Rust-source light theme | 20.02 | 27.95 | Slower in all three pairs |
| Rust-source expansion sequence | 25.06 | 22.93 | Mixed |
| Rust-source wide resize | 6.68 | 11.11 | Slower in all three pairs |

Opening text medians were 224.43 / 238.30 ms for deep source, 291.78 / 258.56 ms for Git, and 157.31 / 161.64 ms for Rust.
Deep-source complete-cell changes were +91.14, -19.88, and +24.04 ms within pairs.
Rust-source opening costs were +7.21, +13.06, and +2.47 ms. Its page-scroll costs ranged from 2.41 to 7.55 ms.
The first-terminal-output interval does not explain every opening cost. The report does not assign all remaining delay to a proven cause.

Git start navigation rose from 2.05 to 3.76 ms. Git narrow resize rose from 2.05 to 2.49 ms.
Both were slower in every pair. Rust start navigation rose from 20.42 to 23.40 ms, also slower in every pair.
Other navigation, theme, and layout observations were mixed. `ordinary-analysis.json` retains every action and individual observation.

| Input | Process CPU median, before / after | Viewer peak RSS median, before / after | Final-window group RSS median, before / after |
| --- | ---: | ---: | ---: |
| Deep source | 1.644 / 0.769 s | 28.92 / 28.92 MiB | 28.92 / 28.92 MiB |
| Git | 0.287 / 0.293 s | 33.19 / 33.33 MiB | 33.19 / 33.33 MiB |
| Rust source | 0.223 / 0.238 s | 16.86 / 17.11 MiB | 16.86 / 17.11 MiB |

Process CPU is accumulated user and system CPU, not elapsed time to a visible response.
The deep-source process uses less CPU. The smaller inputs show higher CPU medians and slightly higher memory observations.
These results do not establish a general CPU or memory reduction.

## Adoption decision

The policy preserves exact output and ownership while reducing demonstrated self-eviction during sparse replay.
Earlier real-worker probes cover sparse, dense, backward, alternating-source, and unequal-side patterns.
The current comparison uses the same Git wait in both roles. Both large-source recoloring actions improve in every pair.
The implementation remains three runtime files, with no increased allowance or additional persistent store.

Adoption accepts the measured smaller-input costs above. It does not dismiss those costs as noise or promise faster opening.
The diagnostic locates microsecond planning and unchanged cached-layout work, but does not explain every ordinary timing difference.
The accepted benefit is less repeated parser work and faster recoloring under the covered pressure pattern.
It is not a universal responsiveness, cold-opening, or total-memory improvement.

The normal rebuild passed all 120 enabled checks, formatting, and application Clippy.
It then passed three separate correctness tours: 33 complete reference grids, offline replay, and cleanup checks.
All 2,168 archived source, test, dependency, fixture, asset, and tool files match the qualified ordinary candidate.
The normal executable has a different hash from the isolated candidate despite matching source.
The paired timing results belong to the isolated candidate. The normal rebuild's tours remain separate and do not add another paired performance claim.

## Next checkpoint

The retention decision is complete. No additional cache or scheduling change follows from this result alone.
Next comes integrated local-source qualification with actual viewport movement, repeated expansion, revisits, and interruption during active coloring.
The existing reference checks and capture tools remain the starting point. Expected source identities must survive each action.
That workload must separate first text, complete coloring, responsiveness, and sustained process memory.
The current-build five-patch campaign remains separate from complete-source qualification.
Physical terminal checks and remote acquisition remain unfinished.

## Evidence and limits

The packet is `evidence/local-source-retention-decision-v1/` in the project directory.
Its `PLAN.md` records the declaration before capture. Its nested `evidence/` directory contains binaries, archives, logs, declarations, captures, and analyses.
The scripts retain machine-local paths and reuse the preceding frozen capture helpers. They are experiment records, not installation commands.
The existing measurement environment supplies the Python modules. No packages or global installations changed.

This is a shared-workstation comparison with warm files and fresh processes.
PTY receipt is not physical terminal paint. Short RSS observations do not establish sustained memory, live heap, or a total-memory bound.
The larger five-patch campaign, active-work interruption, and physical terminal checks remain separate work.

The earlier prototype's slower Git/Rust openings and deep-opening outlier remain valid adverse observations.
This series uses the same Git wait in both roles. Its results cannot credit retention with the earlier notification benefit.
The earlier opening costs are not relabeled as fully explained or discarded.

The ordinary baseline is `fd13e46f331f41965b4addce77223c6f8a492f0622b4db2a24fe89ed37336b3c`.
The ordinary candidate is `068b4cf062aff564f36a053f4414e1d9d15ce6fdb2307f59d34ece0e4c9ae415`.
The diagnostic baseline is `8dcfddcd87a1ad4de44d4a61f56d945e5adcd3504629d189481879ffcab04579`.
The diagnostic candidate is `1bf9d0becd96f5c7c3d8535fdabd6091fd7fb53f1e383ec11a8d9a8534c65079`.
The adopted normal executable is `1ab24c0c70c45a5b480b83e111fa909d33ed56f4f4af9510b30dad72558f2e4e`.

The original declarations pin the product before adoption. Their old product hash resolves to the preserved baseline executable.
The adopted declaration identifies the final normal build separately. Earlier sealed evidence remains unchanged.
