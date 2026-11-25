# Changelog

All notable changes to `diffs-rs` are recorded here. The forked crates in
`crates/` keep their own changelogs.

## Unreleased

### Added

- Insets: `Engine::set_insets` keeps blank rows under a line (`InsetAnchor::Line`) or a file's
  heading (`InsetAnchor::FileHeading`) for the host to draw into, such as review comments shown
  inline. An `Inset` is built with `Inset::new` and `Inset::with_span`. `Inset::span` (`InsetSpan::Full` or `InsetSpan::Side(side)`) places an inset across
  a split diff or in one side's column: side insets under the same row stack side by side, in
  blocks between full ones; unified, every inset stacks in the order given. The engine draws only the
  background, the line between the sides and the cursor's mark there; markers, the selection
  and search matches never show on them.
- `Engine::inset_areas` reports where each visible inset was drawn in the last frame: its rows
  on screen over the columns it takes, how many are cut above the pane, and whether the cursor
  is on it. `Engine::inset_width` and `Engine::inset_width_in` give that width for any span,
  with no frame needed, to wrap an inset's content to before setting its height. `Engine::insets` returns the set and `Engine::hidden_insets` the insets whose line is
  not displayed: outside the diff, in an unrevealed gap or in a hunk hidden as whitespace only.
- Setting insets never moves what is read: the source line on the cursor's screen row stays on
  that row and the horizontal scroll is kept, while insets above grow and shrink out of the
  top of the pane. With the cursor on an inset, the line it hangs under stays put while the
  inset grows, shrinks or goes. At the very start of the diff the top stays instead, so insets
  under the first heading come in under it; setting and removing the same insets is an exact
  round trip. Every reflow keeps working as before with insets present, the cursor on an
  inset included. Scrolling, paging, `G`, the last page and the sticky heading count inset rows.
- `DiffRowKind::Inset { id }` from `Engine::cursor_row` and `Engine::row_at`. By default the
  cursor stops once on each inset, a line move onto one shows all of it that fits, and passing
  through insets never changes the side the cursor has on code;
  `Engine::set_inset_stops(false)` makes line moves pass over them. A selection across an inset holds only its source lines.
- `Side` implements `Hash`.

With no insets set, every frame is drawn as before.

## 0.2.5 - 2026-11-24

### Fixed

- A reflow never scrolls the view sideways. A resize, `Engine::set_layout`,
  `Engine::set_split_ratio`, `Engine::set_tab_width`, `Engine::set_whitespace`, the line-number
  and wrapping toggles and expanding context used to make the column of the last click the
  first one shown when it fell outside the narrower pane, and with the cursor on a wrapped
  part, turning wrapping off scrolled to that part's text. A host without horizontal scrolling
  then showed only the ends of every line, with no way back. This happened with a click in the
  empty half of a split row, on a "No newline at end of file" row, and on any wrapped part.
  The horizontal scroll is now kept, and only lowered when the lines shown no longer reach it.
- Turning wrapping off keeps the top row at the top, on its first screen row. With the cursor
  on a wrapped part, the top used to move to an earlier line. Turning it straight back on, with
  nothing done in between, returns to the wrapped view it left.
- Near the end of the diff, a reflow or turning wrapping off adds no empty rows under the
  last row: when there were none, the top is raised to the last full page if needed, and the
  cursor stays on screen. A taller pane, a change to unified, hidden whitespace or unwrapping
  used to leave the rows under the end empty. A view already scrolled past the end, by the
  wheel or a jump to a short last file, keeps the cursor's screen row as before, and so does
  the standalone viewer (`set_keep_text_in_view(true)`).

### Changed

- By default only `Command::Left` and `Command::Right`, and `Engine::reveal`,
  `Engine::select_range` and search jumps when what they show is out of view, move the view
  sideways. A reflow no longer scrolls left either: toggling line numbers with the cursor on a
  line shorter than the scroll used to jump to its end, and now keeps the scroll. The recorded
  default frames change only at `ToggleNumbers` and the frame after it: on a scrolled view
  (`fixtures/review.patch` and `fixtures/position/review.patch`), and in `git-output/log.patch`,
  whose cursor is then on its "No newline at end of file" mark and now stays there.
- A reflow with the cursor on a "No newline at end of file" mark keeps it on the mark. It used
  to move to the line above, one row up, which moved the top row down by one.
- `Engine::set_keep_text_in_view(true)` keeps the old behaviour for a host that scrolls
  sideways itself: a reflow scrolls to keep the text last clicked, or the first column shown
  on the cursor's line, in view, and turning wrapping off keeps the cursor's line on its screen
  row. The standalone viewer turns it on, so it behaves as before. It stays set across
  `Engine::replace`.

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
