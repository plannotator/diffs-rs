# Changelog

All notable changes to `diffs-rs` are recorded here. The forked crates in
`crates/` keep their own changelogs.

## 0.1.1 - unreleased

- docs: new README, docs moved under docs/.

## 0.1.0 - 2026-10-27

First release.

- `diffs` viewer for Git patches (file or stdin), two files, or two Git commits.
- Split and unified layouts, wrapping, dark and light themes, and a changed-file sidebar.
- Syntax colors from complete old and new sources for file and commit comparisons, with
  incremental context expansion.
- Literal search, source-line selection, save of original bytes, and clipboard copy on macOS.
- `diffs_rs::engine::Engine` for embedding the viewer in a Ratatui application.
