# Documentation

## Guides

- [embedding.md](embedding.md): embed the diff engine in a Ratatui application.
- [release.md](release.md): publish the crates.
- [measurements/](measurements/README.md): large-patch timing and memory results, and how they
  were measured.

## Design

[design/](design/) has the specifications the viewer was built against:

- [SPEC.md](design/SPEC.md): product behavior, controls, and presentation.
- [LARGE_DIFFS.md](design/LARGE_DIFFS.md): the large-patch workload and its targets.
- [EXACT_SOURCE_CONTEXT_SPEC.md](design/EXACT_SOURCE_CONTEXT_SPEC.md): full-source coloring and
  context expansion.
- [INPUT_SCENARIO_ANALYSIS.md](design/INPUT_SCENARIO_ANALYSIS.md): input sources, including
  proposed GitHub pull request support.
- [SIDEBAR_SPEC.md](design/SIDEBAR_SPEC.md): the changed-file sidebar.
- [ACCEPTANCE.md](design/ACCEPTANCE.md): the acceptance checklist.

Some of these describe features that are not implemented yet.

## History

[history/](history/README.md) keeps the dated plans, reports, and reviews from development
before the first release, including the full development log in
[CURRENT_STATE.md](history/CURRENT_STATE.md). They record what was measured or decided at
the time and are not current instructions.
