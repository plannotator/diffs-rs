# Current-build measurement readiness

Updated 2026-09-22. Current correctness qualification is complete.
The remaining gates are the role-specific frozen run declaration and an exclusive AC measurement window.
No accepted performance campaign started.

## Corrected-build follow-up

The large-patch review found an inline-emphasis inconsistency between replacement sides.
The correction uses one old-to-new comparison direction for both sides.
Source text, formatting, layout, syntax colors, and size limits remain unchanged.

The corrected ordinary executable passed 70 normal checks, 87 diagnostic checks, and 12 standalone rendering contracts.
It passed all 39 focused views with unchanged reviewed expectations and saved-output replay.
Independent checks passed for all 842 sampled paired sides.
All 724 actual large-patch views matched the reviewed expectations, including exact source maps and sequential syntax colors.
The 83 applicable deliberate-corruption controls rejected their corrupt inputs.

The fresh ordinary measurement candidate has SHA-256:

`3018ed9fad672e329c6901eedf824b4970aff3f8018ef1d8c49f1eb7249e9c0a`

It passed all 12 rendering contracts and 39 focused views.
Its compiler, SDK, environment, Cargo configuration, and feature tree match the preserved ordinary reference.
Its source matches the current default build, whose SHA-256 is:

`6cf039592f9bc400ee9e90ea510eb4b2daa48ebc24ae1b75371d7d6cc16e2eb3`

The host reported AC power at the follow-up checks.
Correctness and build preparation ran without an exclusive measurement window, so their timings do not count as performance evidence.
The [correction report](../../evidence/inline-pair-v1/REPORT.md) records the complete qualification and a resolved Cargo build-guard incident.
The [next checkpoint](../../evidence/inline-pair-v1/NEXT.md) defines the existing comparison schedules and acceptance conditions.
It supersedes the earlier next-action list for execution order.

## Original 2026-09-21 checkpoint

The remaining sections preserve the earlier preparation state and historical executable identities.
The corrected-build follow-up supersedes statements that the large-patch review remains unfinished.

## What passed

All five cached workload patches match their pinned SHA-256 values.
Their existing memory traces retain the original public actions, three tours, and one-second holds.
The existing focused packet retains all 39 action windows and passes its identity checks.
No historical evidence, workload, target, or measurement algorithm changed.

The current source differs from the earlier measured candidate in four build inputs:

- `src/app.rs`
- `src/highlight.rs`
- `src/highlight/inline.rs`
- `src/theme.rs`

These differences contain cached-destination cancellation, inline emphasis, and alignment-gap contrast.
A future combined comparison cannot attribute every difference to cancellation alone.

## Local build and installation

An isolated copy contains all 2,065 build inputs with matching hashes.
The normal release build succeeded with an empty target directory and no diagnostic features.
It used Rust 1.96.0, the macOS 26.5 SDK, and cached dependency sources.
This is not a clean-machine dependency-download check.

`cargo install --path` succeeded in an isolated prefix.
The installed executable passed all 11 rendering contracts through real terminal processes.
From an empty working directory, it also passed version and independent canonical-source checks with a minimal environment.
Nonterminal interactive use returned an error without terminal escape output.
Its dynamic library links point only to macOS system libraries and frameworks.

The normal executable remains unchanged:

`6111c3340f1c42185b7091891205af9072509a806d0d22321a57c066f9aab5f5`

The isolated build and installed executable share this hash:

`b827b9187d4ecca3281ad954b07a686d5dad2a6418657aa1c735a983c33cf81e`

The different build hashes do not establish byte-for-byte reproducibility.
No global installation, PATH change, dependency update, or replacement of the normal executable occurred.

No package in the inspected metadata declares a Rust requirement above 1.88.
The initial installation check used Rust 1.96.0. The battery-powered follow-up also qualified Rust 1.88.0.
Other platforms, physical terminals, multiplexers, security advisories, and complete distribution notices remain open.

## Why the old expectations cannot be reused unchanged

The first unscored rehearsal stopped at the dense-line opening view.
Its text matched exactly. Three absent-side panes differed only in their gray background.
The frozen expectation used the preceding gap color.

A separate expectation copy changes only independently identified absent-side gaps.
It preserves the text, gutters, source positions, other colors, and public actions.
The deep-navigation and dense-line cases then passed all 12 action windows with exact cells and saved-packet replay.

The next case, many small files, stopped at its opening view.
All source and position checks matched. The 96 changed cells contain only unchanged `();` punctuation.
Only their backgrounds differ. Syntax foregrounds and other cell properties match.
This is the accepted punctuation-aware emphasis change, not a source-content failure.
The remaining two focused cases did not run after that stop.

Both stopped rehearsals restored terminal modes and stopped their sampling threads.
The complete raw output and failures remain available. No timings from these battery-powered rehearsals count as performance results.

## Next actions

1. Preserve the old expectations for the old executable.
2. Prepare separate current-build expectations with the same fixtures, actions, dimensions, and hold periods.
3. Review every changed cell against the accepted gap and inline contracts.
4. Retain independent source checks, sequential syntax checks, and deliberate-corruption controls.
5. Stop on any unexplained text, position, foreground, or background difference.
6. Prepare matched ordinary build recipes and check both exact executables.
7. Freeze the reviewed identities and expectations before measurement.
8. With AC power and exclusive workload coordination, run the existing serial schedules.
9. Report text readiness, complete coloring, responsiveness, peak RSS, and settled RSS separately.

Expected cells must not come solely from the current viewer output.
Different accepted presentation rules require separate exact expectations, not a relaxed comparison that ignores colors.
The five-patch expectation migration remains unfinished. The complete current-build focused rehearsal now passes.
The capture host reported battery power. No accepted performance run started.

## Battery-powered follow-up

The current executable passed all 39 focused views with reviewed exact expectations.
The expected inline spans come from explicit source fragments, not captured viewer output.
The gap changes affect only independently identified absent-side cells.
Source text, source positions, syntax foregrounds, and unrelated cell attributes remain exact.
Saved terminal output reproduced all accepted observations.

Twenty color-corruption checks and five additional saved-output corruption checks passed.
The first hand-written expectation file contained incorrect numeric offsets. Its failed rehearsal remains preserved.
The corrected file derives offsets from exact source fragments.
The accepted packet is `focused-current-v2/packet.json` in the follow-up evidence folder.

Two ordinary comparison builds now share the compiler, SDK, environment, feature tree, and build recipe.
Each uses an empty target directory and no diagnostic features.
Each executable passed all 39 focused views against its own exact presentation answers.
The reference retains the earlier presentation rules. The current build uses the reviewed new rules.
Neither executable replaced the normal application.

| Prepared role | Executable SHA-256 |
| --- | --- |
| Reference | `95229cefda892c6567c6d1bd1522e0ebcfcd77877da116bf3f3cf724f88da36e` |
| Current source | `8d2e69aeda6437da7f4e5f0290fc5fe41da3ae1639611347dac952297a8108f5` |

All 724 historical large-patch views passed independent source and sequential-style checks.
All 83 applicable deliberate corruptions failed as expected.
These offline checks do not establish current-build behavior on those views.

The inline inventory contains 397 visible replacement pairs.
An independent ASCII-token analysis found unique spans for 240 pairs and multiple optimal alignments for 157 pairs.
Another 29 emphasized source lines lack a visible peer in the inspected captures.
These counts identify the remaining review work. Multiple valid alignments do not establish a viewer defect.
No large-patch expectations were silently regenerated or accepted from current output alone.

The [release-readiness checkpoint](../../RELEASE_READINESS.md) records the minimum-compiler and dependency checks.
All 2,065 active build inputs and the normal executable remain unchanged.
The battery-powered captures retain raw diagnostic fields, but none counts as accepted performance evidence.

## Evidence

The local records are outside the product source:

`/Users/ramos/Documents/Codex/2026-09-21/native-diff-readiness/`

The folder contains the build recipe and source archive, install checks, workload identities, stopped rehearsals, and mismatch analysis.
The follow-up evidence is in:

`/Users/ramos/Documents/Codex/2026-09-21/native-diff-battery-readiness/`

It contains the reviewed focused packet, raw captures, matched builds, minimum-compiler checks, and offline large-patch inventories.
The [current project state](../../CURRENT_STATE.md) owns the overall status.
