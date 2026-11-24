# Changelog

All notable changes to `diffs-rs` are recorded here. The forked crates in
`crates/` keep their own changelogs.

## 0.2.4 - 2026-11-24

- `Engine::set_tab_width` draws tabs from 1 to 16 cells wide, everywhere code is measured or
  drawn: wrapping, horizontal scrolling, clicks, `Engine::row_at`, changed-word emphasis and
  search matches, on both split sides. `TabWidth::DEFAULT` (4) draws as before.
- `Engine::set_inline_changes` chooses how changed text inside a replaced line is emphasized,
  with the names of Pierre's `lineDiffType`: `InlineChanges::Word` (the default, as before),
  `WordAlt`, `Char` and `None`.
- `Engine::set_whitespace` hides whitespace-only changes as `git diff -b`
  (`Whitespace::IgnoreChange`) or `git diff -w` (`Whitespace::IgnoreAll`). Within each block
  of changes, removed and added lines that match apart from whitespace are drawn as unchanged
  lines, and a hunk with no other change shows only its heading, marked "whitespace only".
  Unpaired lines, hunk boundaries, files and counts stay as the patch has them; see
  `Whitespace` for the limits. `Whitespace::Show`, the default, draws as before.
- `Engine::hunk_is_whitespace_only` and `DiffRow::whitespace_only` give the engine's own
  verdict on what the whitespace setting hides, a hunk drawn as its heading alone or a row
  drawn as one unchanged line, so a host need not repeat the rule.
- Each setting has a getter, can be set before the first file arrives and stays set across
  `Engine::replace`. `InlineChanges` and `Whitespace` are `#[non_exhaustive]`.

### Fixed

- `Engine::reveal`, `Engine::select_range` and search jumps never move the view to show a
  line that is already on screen, however many of them come before the next frame. A second
  reveal before a draw used to think every line was off screen and scroll, so revealing both
  ends of a selected range moved the range to the bottom of the pane.

## 0.2.3 - 2026-11-23

- `Engine::set_split_ratio` shares a split row's width unevenly between the old and the new
  side, from one tenth to nine tenths for the old side; each side keeps at least 20 cells. The
  default, `SplitRatio::EVEN`, draws exactly as before. The cursor's line keeps its screen row,
  wrapping and horizontal scrolling are measured per side, and clicks, `Engine::row_at` and
  `Engine::cursor_row` take the side the line divides. `Engine::split_line` gives the line's
  screen column in the last frame and `Engine::split_ratio_at` the ratio for a column, so a host
  can let the person drag the line. `SplitRatio` keeps its value private; build one with
  `SplitRatio::from_permille` or its constants.
- `Engine::set_layout` changes the layout after opening, `Layout::Auto` included, keeping the
  cursor's line on its screen row as the layout toggle does.

## 0.2.2 - 2026-11-23

- `Engine::cursor_row` says which file and which kind of row the cursor is on, including rows
  with no line: headings, hunk headings, gaps, binary and mode-only notes, empty split cells,
  "No newline" marks and wrapped parts. It also gives the row's hunk (none on revealed context)
  and its original line numbers. `Engine::row_at` answers the same for a screen cell of the
  last frame, the sticky file heading included. Both are read-only and return an
  `Option<DiffRow>`. `DiffRow` and `DiffRowKind` are `#[non_exhaustive]`.

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
