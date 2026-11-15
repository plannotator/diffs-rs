use super::*;
use crate::reading::{LineSelection, SourceRange};
use crossterm::event::{MouseButton, MouseEventKind};

pub(super) fn moves(command: Command) -> bool {
    matches!(
        command,
        Command::Down
            | Command::Up
            | Command::PageDown
            | Command::PageUp
            | Command::FirstSource
            | Command::LastSource
            | Command::NextFile
            | Command::PreviousFile
            | Command::NextHunk
            | Command::PreviousHunk
    )
}

pub(super) fn extends(event: &Event) -> bool {
    match event {
        Event::Key(key) => Command::from_key(*key).is_some_and(moves),
        Event::Mouse(mouse) => matches!(
            mouse.kind,
            MouseEventKind::ScrollDown
                | MouseEventKind::ScrollUp
                | MouseEventKind::Down(MouseButton::Left)
        ),
        _ => false,
    }
}

impl Engine {
    /// The selected original-side range. It can include unavailable patch-only gaps.
    /// `source_read` rejects those gaps when streaming, before visible publication.
    pub fn selection(&self) -> Option<SourceRange> {
        self.app
            .as_ref()?
            .selection
            .as_ref()
            .map(|selection| selection.range)
    }

    pub fn selection_extending(&self) -> bool {
        self.app
            .as_ref()
            .and_then(|app| app.selection.as_ref())
            .is_some_and(|selection| selection.extending)
    }

    /// Scrolls the original line on one side into view and puts the cursor on it.
    /// Fails when the line is not displayed: outside the file, or in an unrevealed gap.
    /// A selection in another file or on the other side is cleared, as cursor moves do.
    pub fn reveal(&mut self, file: usize, side: Side, line: u32) -> Result<()> {
        let id = self.displayed(file, side, line)?;
        let app = self.app.as_mut().context("The review has no files yet")?;
        app.focus_diff();
        app.reveal_source(id, 0, side == Side::Old)?;
        self.changed(InputEffect::Highlight);
        self.track_selection(false)
    }

    /// Selects `range.first..=range.last` as if the user had selected it, ending with the
    /// cursor on `range.last`. Both endpoints must be displayed source lines; the lines
    /// between them can include gaps, as a user selection can.
    pub fn select_range(&mut self, range: SourceRange) -> Result<()> {
        ensure!(
            range.review == self.id,
            "Source range belongs to another review"
        );
        ensure!(
            range.first != 0 && range.first <= range.last,
            "Invalid source line range {}..={}",
            range.first,
            range.last
        );
        self.displayed(range.file, range.side, range.first)?;
        let last = self.displayed(range.file, range.side, range.last)?;
        let app = self.app.as_mut().context("The review has no files yet")?;
        app.focus_diff();
        app.reveal_source(last, 0, range.side == Side::Old)?;
        let checked = matches!(
            app.sources
                .as_ref()
                .and_then(|sources| sources.get(range.file)),
            Some(SourceState::Checked(_))
        );
        app.selection = Some(LineSelection {
            range,
            anchor: range.first,
            extending: false,
            checked,
        });
        self.changed(InputEffect::Highlight);
        Ok(())
    }

    fn displayed(&self, file: usize, side: Side, line: u32) -> Result<crate::view::LineId> {
        ensure!(self.input.is_some(), "The review is closed");
        let app = self.app.as_ref().context("The review has no files yet")?;
        ensure!(
            file < app.doc.files.len(),
            "File {file} is not in the review"
        );
        app.displayed_source(file, side == Side::Old, line)
            .with_context(|| {
                format!(
                    "{} line {line} of file {file} is not in the diff",
                    match side {
                        Side::Old => "Old",
                        Side::New => "New",
                    }
                )
            })
    }

    pub(super) fn selection_command(&mut self, command: Command) -> Result<InputEffect> {
        let Some(app) = &mut self.app else {
            return Ok(InputEffect::Ignored);
        };
        if app.help || app.sidebar.focused() || app.sidebar.diff.is_empty() {
            return Ok(InputEffect::Ignored);
        }
        if command == Command::ClearSelection {
            if app.selection.take().is_none() {
                return Ok(InputEffect::Ignored);
            }
        } else if let Some(selection) = &mut app.selection
            && selection.extending
        {
            selection.extending = false;
        } else {
            let Some(location) = self.selected_source()? else {
                return Ok(InputEffect::Ignored);
            };
            let line = match location.side {
                Side::Old => location.old_line,
                Side::New => location.new_line,
            }
            .context("The selected side has no source line")?;
            let app = self
                .app
                .as_mut()
                .context("The review has no viewer state")?;
            app.old_side = location.side == Side::Old;
            let checked = matches!(
                app.sources
                    .as_ref()
                    .and_then(|sources| sources.get(location.file)),
                Some(SourceState::Checked(_))
            );
            app.selection = Some(LineSelection {
                range: SourceRange {
                    review: self.id,
                    file: location.file,
                    side: location.side,
                    first: line,
                    last: line,
                },
                anchor: line,
                extending: true,
                checked,
            });
        }
        self.changed(InputEffect::Redraw);
        Ok(InputEffect::Redraw)
    }

    pub(super) fn track_selection(&mut self, extend: bool) -> Result<()> {
        let Some(app) = &mut self.app else {
            return Ok(());
        };
        let Some(mut selection) = app.selection.take() else {
            return Ok(());
        };
        if app.row(app.cursor.row).map(|row| row.file) != Some(selection.range.file) {
            self.changed(InputEffect::Redraw);
            return Ok(());
        }
        let before = selection.range;
        if extend
            && selection.extending
            && !app.help
            && !app.sidebar.focused()
            && let Some((id, old)) =
                app.source_at_cursor(Some(selection.range.side == Side::Old))?
        {
            let info = app.line_info(id)?;
            let line = if old { info.old } else { info.new }
                .context("Selected range endpoint has no source")?;
            selection.range.first = selection.anchor.min(line);
            selection.range.last = selection.anchor.max(line);
        }
        let changed = before != selection.range;
        app.selection = Some(selection);
        if changed {
            self.changed(InputEffect::Redraw);
        }
        Ok(())
    }
}
