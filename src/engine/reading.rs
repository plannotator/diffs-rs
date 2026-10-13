use super::*;
use crate::reading::{SearchMatch, SearchRead, SearchStart, literal, search::Position};

impl Engine {
    /// Captures diff text and the current reading origin. It does not start a worker.
    /// The caller owns cancellation/join and must reject superseded query replies.
    pub fn search_read(&mut self, query: String) -> Result<SearchRead> {
        literal::validate(&query)?;
        let selected = self.selected_source()?;
        let domain = self.search_domain()?;
        let app = self
            .app
            .as_ref()
            .context("Review source is not available")?;
        let preferred = selected
            .map_or(if app.old_side { Side::Old } else { Side::New }, |source| {
                source.side
            });
        let start = if let Some(row) = app.row(app.cursor.row) {
            if let (Some(selected), RowKind::Code { left, right }) = (selected, row.kind) {
                let source = if selected.side == Side::Old {
                    left
                } else {
                    right
                }
                .context("Selected search source is not available")?;
                SearchStart::At(Position {
                    file: row.file,
                    row: domain.files[row.file]
                        .order
                        .row_for_source(source)
                        .context("Selected source is outside search scope")?,
                    byte: 0,
                })
            } else {
                SearchStart::AfterRow {
                    display: app
                        .view
                        .reading_order(app.doc.files[row.file].clone(), app.split()),
                    row: app.cursor.row - app.view.file_row(&app.doc, row.file, app.split()),
                }
            }
        } else {
            SearchStart::At(Position::default())
        };
        SearchRead::new(domain, query, start, preferred)
    }

    /// Selects a validated match without accepting another query's late reply for the caller.
    pub fn show_search_match(&mut self, found: &SearchMatch) -> Result<()> {
        found.check(self.id)?;
        let app = self.app.as_mut().context("The review is closed")?;
        app.reveal_source(
            found.hit.source,
            found.display.start,
            found.location().side == Side::Old,
        )?;
        app.search_match = Some(found.clone());
        self.changed(InputEffect::Highlight);
        self.track_selection(true)?;
        Ok(())
    }

    pub fn clear_search_match(&mut self) {
        if let Some(app) = &mut self.app
            && app.search_match.take().is_some()
        {
            self.changed(InputEffect::Redraw);
        }
    }
}
