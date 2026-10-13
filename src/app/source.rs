use std::sync::Arc;

use anyhow::{Context, Result, ensure};

use super::App;
use crate::{
    highlight::{FileSource, Reply},
    input::SourceState,
    model::RowKind,
    storage::FileIndex,
    view::LineId,
};

pub(crate) struct SourceStatus {
    pub text: String,
    pub attention: bool,
}

impl App {
    pub(crate) fn source_at_cursor(&self, side: Option<bool>) -> Result<Option<(LineId, bool)>> {
        let Some(row) = self.row(self.cursor.row) else {
            return Ok(None);
        };
        let RowKind::Code { left, right } = row.kind else {
            return Ok(None);
        };
        let (id, old) = if let Some(old) = side {
            (if old { left } else { right }, old)
        } else if self.split() {
            (if self.old_side { left } else { right }, self.old_side)
        } else if right.is_none() {
            (left, true)
        } else {
            (right, left == right && self.old_side)
        };
        let Some(id) = id else {
            return Ok(None);
        };
        if !crate::render::source_has_part(self, id, self.cursor.part, old)? {
            return Ok(None);
        }
        Ok(Some((id, old)))
    }

    pub(crate) fn line_file(&self, id: LineId) -> &Arc<FileIndex> {
        match id {
            LineId::Patch(id) => self.doc.file_for_source(id),
            LineId::Context { file, .. } => &self.doc.files[file],
        }
    }

    pub(crate) fn line_info(&self, id: LineId) -> Result<crate::storage::LineInfo> {
        match id {
            LineId::Patch(id) => Ok(self.doc.info(id)),
            LineId::Context { file, old, new } => self
                .checked_source(file)?
                .context_info(old.get(), new.get()),
        }
    }

    fn checked_source(&self, file: usize) -> Result<&crate::source::CheckedPair> {
        match self.sources.as_ref().and_then(|sources| sources.get(file)) {
            Some(SourceState::Checked(pair)) => Ok(pair),
            _ => anyhow::bail!("Context requires checked source"),
        }
    }

    pub(crate) fn line_geometry(
        &self,
        id: LineId,
        width: Option<usize>,
    ) -> Result<std::rc::Rc<crate::geometry::Line>> {
        match id {
            LineId::Patch(id) => self.geometry.get(&self.doc, id, width),
            LineId::Context { file, old, new } => {
                let pair = self.checked_source(file)?;
                pair.context_info(old.get(), new.get())?;
                let source = pair.new_source().context("Checked context has no source")?;
                let (store, reference) = source.stored_line(new.get())?;
                self.geometry.get_source(store, reference, width)
            }
        }
    }

    pub(crate) fn line_digits(&self, file: usize) -> usize {
        let mut digits = self.doc.files[file].digits;
        if let Some(SourceState::Checked(pair)) =
            self.sources.as_ref().and_then(|sources| sources.get(file))
        {
            let lines = pair
                .old()
                .into_iter()
                .chain(pair.new_source())
                .map(|source| source.lines())
                .max()
                .unwrap_or(0);
            digits = digits.max(lines.checked_ilog10().unwrap_or(0) as usize + 1);
        }
        digits
    }

    pub(super) fn coloring_source(&self, file: &Arc<FileIndex>) -> FileSource {
        match &self.sources {
            None => FileSource::Patch(file.clone()),
            Some(sources) => match sources.get(file.id) {
                Some(SourceState::Checked(pair)) => FileSource::Complete(pair.clone()),
                _ => FileSource::Plain(file.clone()),
            },
        }
    }

    pub fn set_source(&mut self, file: usize, state: SourceState) -> Result<()> {
        let entry = self
            .doc
            .files
            .get(file)
            .context("Source reply is outside the current review")?;
        if let SourceState::Checked(pair) = &state {
            ensure!(
                Arc::ptr_eq(entry, pair.file()),
                "Source reply belongs to a different patch entry"
            );
        }
        ensure!(
            self.sources
                .as_ref()
                .is_some_and(|sources| file < sources.len()),
            "Source reply is outside the current review"
        );
        let same_source = matches!(
            (self.sources.as_ref().and_then(|sources| sources.get(file)), &state),
            (Some(SourceState::Checked(previous)), SourceState::Checked(next))
                if Arc::ptr_eq(previous, next)
        );
        let split = self.split();
        let cursor_file = self.row(self.cursor.row).map(|row| row.file);
        let digits = cursor_file.map(|file| self.line_digits(file));
        let cursor_source = self.anchor();
        let top_source = self.row(self.top.row).and_then(|row| match row.kind {
            RowKind::Code { left, right } => left.or(right),
            _ => None,
        });
        let cursor_row = self.view.original_row(&self.doc, self.cursor.row, split);
        let top_row = self.view.original_row(&self.doc, self.top.row, split);
        let removing = !matches!(state, SourceState::Checked(_));
        if removing && matches!(cursor_source, Some(LineId::Context { file: id, .. }) if id == file)
        {
            self.reading_anchor = None;
        } else {
            self.remember_position(None)?;
        }
        self.view.set(
            &self.doc,
            file,
            match &state {
                SourceState::Checked(pair) => Some(pair),
                _ => None,
            },
        )?;
        if !same_source {
            self.source_reads.invalidate(file);
            if self
                .selection
                .as_ref()
                .is_some_and(|selection| selection.range.file == file && selection.checked)
            {
                self.selection = None;
            }
        }
        if let Some(selection) = &mut self.selection
            && selection.range.file == file
            && matches!(state, SourceState::Checked(_))
        {
            selection.checked = true;
        }
        self.cache.invalidate(&self.doc.files[file]);
        if let Some(sources) = &mut self.sources {
            sources[file] = state;
        }
        self.cursor.row = cursor_source
            .and_then(|id| self.view.source_row(&self.doc, id, split))
            .unwrap_or_else(|| self.view.patch_row(&self.doc, cursor_row, split));
        self.top.row = top_source
            .and_then(|id| self.view.source_row(&self.doc, id, split))
            .unwrap_or_else(|| self.view.patch_row(&self.doc, top_row, split));
        self.cursor.part = self
            .cursor
            .part
            .min(self.row_height(self.cursor.row)?.saturating_sub(1));
        self.top.part = self
            .top
            .part
            .min(self.row_height(self.top.row)?.saturating_sub(1));
        if self.reading_anchor.is_some() || digits != cursor_file.map(|file| self.line_digits(file))
        {
            self.reflow(self.layout, self.width, self.height)?;
        }
        self.visible.clear();
        self.worker.cancel();
        Ok(())
    }

    pub fn accept_highlight(&mut self, reply: Reply) -> Result<bool> {
        if reply.epoch != self.epoch
            || match reply.id {
                LineId::Patch(id) => id >= self.doc.source_len(),
                LineId::Context { file, .. } => file >= self.doc.files.len(),
            }
        {
            return Ok(false);
        }
        let file = self.line_file(reply.id);
        if reply.origin != self.coloring_source(file).origin() {
            return Ok(false);
        }
        if let Some(reason) = reply.source_error {
            self.set_source(file.id, SourceState::Unavailable(reason))?;
            return Ok(true);
        }
        self.cache.insert(reply.id, reply.style);
        Ok(true)
    }

    /// Describes the source of the cursor's file. Other files visible on the same screen keep
    /// their own state, which the footer reports once the cursor reaches them.
    pub(crate) fn source_status(&self) -> Option<SourceStatus> {
        let sources = self.sources.as_ref()?;
        let cursor = self.row(self.cursor.row);
        let file = cursor.map(|row| row.file);
        let (mut preparing, mut checked) = (false, false);
        match file.and_then(|file| sources.get(file)) {
            Some(SourceState::Unavailable(reason)) => {
                return Some(SourceStatus {
                    text: format!("Source unavailable: {reason} · patch view retained"),
                    attention: true,
                });
            }
            Some(SourceState::Pending) => preparing = true,
            Some(SourceState::Checked(_)) => checked = true,
            _ => {}
        }
        let mut pending = false;
        let mut fallback = None;
        for row in self
            .visible
            .iter()
            .filter_map(|(position, _)| self.row(position.row))
            .filter(|row| Some(row.file) == file)
            .chain(cursor)
        {
            if let RowKind::Code { left, right } = row.kind {
                for id in left.into_iter().chain(right) {
                    pending |= self.cache.get(id).is_none();
                    if let Some(reason) = self.cache.get(id).and_then(|style| style.fallback_reason)
                    {
                        fallback.get_or_insert(reason);
                    }
                }
            }
        }
        let text = if let Some(reason) = fallback {
            reason.status().into()
        } else if preparing {
            "Preparing source · plain syntax".into()
        } else if checked && pending {
            "Source checked · coloring visible code…".into()
        } else if checked {
            "Source checked · complete-context coloring".into()
        } else {
            "Metadata only".into()
        };
        Some(SourceStatus {
            text,
            attention: fallback.is_some() || preparing || checked && pending,
        })
    }
}
