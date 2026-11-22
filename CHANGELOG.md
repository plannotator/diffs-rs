# Changelog

All notable changes to `diffs-rs` are recorded here. The forked crates in
`crates/` keep their own changelogs.

## 0.2.1 - 2026-11-22

- `Engine::set_file_marks` lets the host mark files in the Files sidebar, for example as
  viewed. A `FileMark` names a file by path and carries an optional `Badge` of one or two
  cells in a `BadgeColor` from the theme, and a `dimmed` flag that mutes the row. The badge
  has its own column at the right of every row, so rows stay aligned, and a folder shows a
  badge when every file under it has the same one. Hosts that never call it, and the `diffs`
  viewer, look as before.
- `Engine::set_terminal_background(true)` draws the diff on the terminal's own background
  and text color: empty rows, plain context lines, gaps, file headers and the Files sidebar
  get no background, and plain text uses the terminal's default foreground. Added and
  deleted lines, changed words and hunk headers keep explicit tints from the palette of
  `Options::appearance`, adjusted to read on lighter or bluer dark terminals. Off by
  default; the `diffs` viewer does not change.

## 0.2.0 - 2026-11-15

- `Options.chrome`: set it to `false` and the engine draws no label row, footer or help
  overlay, so the host owns those rows. Adding this field breaks hosts that build `Options`
  without `..Options::default()`.
- `Engine::set_markers` draws host markers in the gutter for ranges of source lines.
- `Engine::reveal` moves the cursor to a source line, and `Engine::select_range` selects a
  range of source lines as the user would.
- The Files sidebar and the file headers color each status by kind (added, modified, deleted,
  renamed or copied) and show `+N` in the addition color and `−N` in the deletion color, as
  Hunk does. Folder totals use the same count colors. `theme::Colors` gains `modified` and
  `renamed`, which breaks code that builds `Colors` by hand.
- Search scans the patch bytes directly instead of reading each source line on its own. On
  the 711 MB Linux v6.0…v7.0 patch the first match and a wrapping F3 take about 60 ms
  instead of 3–9 s. Results do not change.

## 0.1.1 - 2026-10-27

- docs: new README, docs moved under docs/.

## 0.1.0 - 2026-10-27

First release.

- `diffs` viewer for Git patches (file or stdin), two files, or two Git commits.
- Split and unified layouts, wrapping, dark and light themes, and a changed-file sidebar.
- Syntax colors from complete old and new sources for file and commit comparisons, with
  incremental context expansion.
- Literal search, source-line selection, save of original bytes, and clipboard copy on macOS.
- `diffs_rs::engine::Engine` for embedding the viewer in a Ratatui application.
