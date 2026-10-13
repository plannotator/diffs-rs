use std::sync::{Arc, mpsc::SyncSender};

use anyhow::{Context, Result};
use crossterm::event::{Event, KeyCode, KeyModifiers};
use native_diff::{
    engine::{Command, Engine, InputEffect, ReadingControls, Side},
    reading::{SearchDirection, SearchMatch, SearchRead, SourceRange, SourceRead},
    text::safe_label,
};
use ratatui::Frame;

use crate::{
    clipboard,
    reading_worker::{Output, Worker, cleanup_failed},
};

mod prompt;
use prompt::{Action, Mode, Prompt};

const COPY_INTERRUPTED: &str = "Copy interrupted; clipboard contents may have changed";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pending {
    Search(u64),
    Save(u64),
    Copy(u64),
    Deliver(u64),
}

impl Pending {
    fn id(self) -> u64 {
        match self {
            Self::Search(id) | Self::Save(id) | Self::Copy(id) | Self::Deliver(id) => id,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Search(_) => "Search",
            Self::Save(_) => "Save",
            Self::Copy(_) | Self::Deliver(_) => "Copy",
        }
    }

    fn status(self) -> &'static str {
        match self {
            Self::Search(_) => "Searching diff text… · Esc cancels",
            Self::Save(_) => "Preparing save… · Esc cancels",
            Self::Copy(_) => "Preparing copy… · Esc cancels",
            Self::Deliver(_) => "Copying… · Esc stops delivery",
        }
    }
}

pub struct Reading {
    wake: SyncSender<()>,
    worker: Option<Worker>,
    prompt: Option<Prompt>,
    query: String,
    save_path: String,
    read: Option<Arc<SearchRead>>,
    current: Option<SearchMatch>,
    pending: Option<Pending>,
    notice: Option<String>,
    selection: Option<(SourceRange, bool)>,
    selection_status: Option<String>,
    dirty: bool,
}

impl Reading {
    pub fn new(wake: SyncSender<()>) -> Self {
        Self {
            wake,
            worker: None,
            prompt: None,
            query: String::new(),
            save_path: String::new(),
            read: None,
            current: None,
            pending: None,
            notice: None,
            selection: None,
            selection_status: None,
            dirty: false,
        }
    }

    pub fn needs_draw(&self) -> bool {
        self.dirty
    }

    pub fn after_draw(&mut self) {
        self.dirty = false;
    }

    pub fn controls(&self, width: u16) -> ReadingControls<'_> {
        ReadingControls {
            hint: if width < 160 {
                "/ search  V select"
            } else {
                "/ search  F3/Shift-F3 next/prev  V select  y copy  Ctrl-S save"
            },
            help: "  READING · original diff text\n  /              Literal search; Enter submits\n  F3 / Shift-F3  Next / previous match; wraps once\n  V              Begin line selection / fix range\n  Shift-click    Extend range on the same file and side\n  y              Copy range/current line (UTF-8, max 16 MiB)\n  Ctrl-S         Save range/current line to a new file\n  Esc            Cancel prompt/work, or clear range",
            status: if self.selection.is_some() {
                self.selection_status.as_deref()
            } else if let Some(pending) = self.pending {
                Some(pending.status())
            } else {
                self.notice.as_deref()
            },
        }
    }

    pub fn draw_prompt(&self, frame: &mut Frame<'_>) {
        if let Some(prompt) = &self.prompt {
            prompt.draw(frame);
        }
    }

    fn cancel(&mut self) -> Result<()> {
        if let Some(worker) = &mut self.worker {
            worker.cancel()?;
        }
        self.pending = None;
        Ok(())
    }

    fn open_prompt(&mut self, engine: &mut Engine) -> Result<()> {
        self.show_prompt(engine, Prompt::default())
    }

    fn show_prompt(&mut self, engine: &mut Engine, mut prompt: Prompt) -> Result<()> {
        let interrupted = matches!(self.pending, Some(Pending::Deliver(_)));
        self.cancel()?;
        engine.execute(Command::Back)?;
        engine.execute(Command::Back)?;
        if interrupted {
            prompt.set_error(COPY_INTERRUPTED.into());
        }
        self.prompt = Some(prompt);
        self.notice = interrupted.then(|| COPY_INTERRUPTED.into());
        self.dirty = true;
        Ok(())
    }

    fn submit(&mut self, direction: SearchDirection) -> Result<()> {
        let Some(read) = &self.read else {
            return Ok(());
        };
        if self.worker.is_none() {
            self.worker = Some(Worker::start(self.wake.clone())?);
        }
        self.pending = Some(Pending::Search(self.worker.as_mut().unwrap().submit(
            read.clone(),
            self.current.clone(),
            direction,
        )?));
        self.notice = None;
        self.dirty = true;
        Ok(())
    }

    fn begin(&mut self, engine: &mut Engine, query: String) -> Result<()> {
        let read = engine.search_read(query)?;
        self.query = read.query().into();
        self.read = Some(Arc::new(read));
        self.current = None;
        engine.clear_search_match();
        self.submit(SearchDirection::Forward)
    }

    fn source(engine: &mut Engine) -> Result<SourceRead> {
        let range = if let Some(range) = engine.selection() {
            range
        } else {
            let at = engine
                .selected_source()?
                .context("Choose a source line in the diff, not a header, gap or padding")?;
            let line = match at.side {
                Side::Old => at.old_line,
                Side::New => at.new_line,
            }
            .context("The selected side has no source line")?;
            SourceRange {
                review: at.review,
                file: at.file,
                side: at.side,
                first: line,
                last: line,
            }
        };
        engine.source_read(range)
    }

    fn open_save(&mut self, engine: &mut Engine) -> Result<()> {
        let source = Self::source(engine);
        self.dirty = true;
        match source {
            Ok(source) => {
                self.show_prompt(
                    engine,
                    Prompt::save(Arc::new(source), self.save_path.clone()),
                )?;
            }
            Err(error) => self.notice = Some(safe_label(format!("{error:#}").as_bytes())),
        }
        Ok(())
    }

    fn save(&mut self, read: Arc<SourceRead>, path: String) -> Result<()> {
        read.check()?;
        if self.worker.is_none() {
            self.worker = Some(Worker::start(self.wake.clone())?);
        }
        self.save_path = path;
        self.pending = Some(Pending::Save(
            self.worker
                .as_mut()
                .unwrap()
                .save(read, self.save_path.clone().into())?,
        ));
        self.notice = None;
        self.dirty = true;
        Ok(())
    }

    fn copy(&mut self, engine: &mut Engine) -> Result<()> {
        if matches!(self.pending, Some(Pending::Copy(_) | Pending::Deliver(_))) {
            return Ok(());
        }
        let source = clipboard::available().and_then(|()| Self::source(engine));
        self.dirty = true;
        match source {
            Ok(source) => {
                if self.worker.is_none() {
                    self.worker = Some(Worker::start(self.wake.clone())?);
                }
                self.pending = Some(Pending::Copy(
                    self.worker.as_mut().unwrap().copy(Arc::new(source))?,
                ));
                self.notice = None;
            }
            Err(error) => self.notice = Some(safe_label(format!("{error:#}").as_bytes())),
        }
        Ok(())
    }

    pub fn handle(&mut self, event: &Event, engine: &mut Engine) -> Result<bool> {
        if let Some(prompt) = &mut self.prompt {
            if matches!(event, Event::Resize(..)) {
                return Ok(false);
            }
            let action = prompt.handle(event);
            self.dirty |= !matches!(action, Action::Unchanged);
            match action {
                Action::Cancel => self.prompt = None,
                Action::Submit if prompt.text().is_empty() => self.prompt = None,
                Action::Submit => {
                    let text = prompt.text().to_owned();
                    let source = match &prompt.mode {
                        Mode::Search => None,
                        Mode::Save(read) => Some(read.clone()),
                    };
                    let result = match source {
                        Some(read) => self.save(read, text),
                        None => self.begin(engine, text),
                    };
                    match result {
                        Ok(()) => self.prompt = None,
                        Err(error) if cleanup_failed(&error) => return Err(error),
                        Err(error) => self
                            .prompt
                            .as_mut()
                            .unwrap()
                            .set_error(format!("{error:#}")),
                    }
                }
                Action::Edit | Action::Unchanged => {}
            }
            return Ok(true);
        }
        if matches!(event, Event::Paste(_)) {
            return Ok(true);
        }
        if let Event::Key(key) = event {
            if key.code == KeyCode::Char('y') && key.modifiers == KeyModifiers::NONE {
                self.copy(engine)?;
                return Ok(true);
            }
            if key.code == KeyCode::Char('s') && key.modifiers == KeyModifiers::CONTROL {
                self.open_save(engine)?;
                return Ok(true);
            }
            if key.code == KeyCode::Char('V')
                && !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
            {
                self.notice = if engine.execute(Command::SelectLines)? == InputEffect::Ignored {
                    Some("Choose a source line in the diff, not a header, gap or padding".into())
                } else {
                    None
                };
                self.dirty = true;
                return Ok(true);
            }
            if key.code == KeyCode::Char('/')
                && !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
            {
                self.open_prompt(engine)?;
                return Ok(true);
            }
            if key.code == KeyCode::F(3) {
                if self.pending.is_some() {
                    return Ok(true);
                }
                if self.query.is_empty() {
                    self.open_prompt(engine)?;
                } else {
                    let direction = if key.modifiers.contains(KeyModifiers::SHIFT) {
                        SearchDirection::Backward
                    } else {
                        SearchDirection::Forward
                    };
                    if self.read.is_none() {
                        match engine.search_read(self.query.clone()) {
                            Ok(read) => self.read = Some(Arc::new(read)),
                            Err(error) => {
                                self.notice = Some(safe_label(format!("{error:#}").as_bytes()));
                                self.dirty = true;
                                return Ok(true);
                            }
                        }
                    }
                    self.submit(direction)?;
                }
                return Ok(true);
            }
            if key.code == KeyCode::Esc
                && let Some(pending) = self.pending
            {
                self.cancel()?;
                self.notice = Some(if matches!(pending, Pending::Deliver(_)) {
                    COPY_INTERRUPTED.into()
                } else {
                    format!("{} canceled", pending.name())
                });
                self.dirty = true;
                return Ok(true);
            }
            if key.code == KeyCode::Esc
                && engine.execute(Command::ClearSelection)? != InputEffect::Ignored
            {
                self.selection = None;
                self.selection_status = None;
                self.notice = Some("Selection cleared".into());
                self.dirty = true;
                return Ok(true);
            }
        }
        if self.notice.take().is_some() {
            self.dirty = true;
        }
        Ok(false)
    }

    pub fn poll(&mut self, engine: &mut Engine) -> Result<()> {
        self.poll_worker(engine)?;
        let selection = engine
            .selection()
            .map(|range| (range, engine.selection_extending()));
        if self.selection != selection {
            if self.selection.is_some() && selection.is_none() {
                if let Some(notice) = &mut self.notice {
                    notice.push_str(" · selection cleared");
                } else {
                    self.notice = Some("Selection cleared · file, side, or source changed".into());
                }
            }
            self.selection = selection;
            self.dirty = true;
        }
        if self.dirty {
            self.selection_status = selection.map(|(range, extending)| {
                let mode = if extending { "Selecting" } else { "Selected" };
                let side = if range.side == Side::Old {
                    "old"
                } else {
                    "new"
                };
                let mut status = format!("{mode} {side} {}–{}", range.first, range.last);
                let notice = if let Some(pending) = self.pending {
                    Some(pending.status())
                } else {
                    self.notice.as_deref()
                };
                if let Some(notice) = notice {
                    status.push_str(" · ");
                    status.push_str(notice);
                }
                status
            });
        }
        Ok(())
    }

    fn poll_worker(&mut self, engine: &mut Engine) -> Result<()> {
        if self.read.as_ref().is_some_and(|read| read.check().is_err()) {
            if matches!(self.pending, Some(Pending::Search(_))) {
                self.cancel()?;
            }
            self.read = None;
            self.current = None;
            engine.clear_search_match();
            self.notice = Some("Search scope changed · F3 restarts; / new query".into());
            self.dirty = true;
        }
        let reply = self
            .worker
            .as_ref()
            .map(Worker::poll)
            .transpose()?
            .flatten();
        let Some(reply) = reply else {
            return Ok(());
        };
        if self.pending.map(Pending::id) != Some(reply.id) {
            return reply.discard();
        }
        let pending = self.pending.take().unwrap();
        self.dirty = true;
        match reply.result {
            Ok(Output::Search(Some(found))) => {
                engine.execute(Command::Back)?;
                engine.execute(Command::Back)?;
                match engine.show_search_match(&found) {
                    Ok(()) => {
                        let at = found.location();
                        let (side, line) = if at.side == Side::Old {
                            ("old", at.old_line)
                        } else {
                            ("new", at.new_line)
                        };
                        let line = line.context("Search match has no original source line")?;
                        self.notice = Some(format!(
                            "Match · {side} line {line}{}",
                            if found.wrapped() { " · wrapped" } else { "" }
                        ));
                        self.current = Some(found);
                    }
                    Err(error) => {
                        self.notice =
                            Some(safe_label(format!("Search failed: {error:#}").as_bytes()))
                    }
                }
            }
            Ok(Output::Search(None)) => self.notice = Some("No match in diff text".into()),
            Ok(Output::Save(prepared)) => match prepared.publish(engine.review_id()) {
                Ok(saved) => {
                    let notice = format!(
                        "Saved {} bytes to {}",
                        saved.bytes,
                        saved.destination.display()
                    );
                    saved.cleanup.with_context(|| notice.clone())?;
                    self.notice = Some(safe_label(notice.as_bytes()));
                }
                Err(error) if cleanup_failed(&error) => return Err(error),
                Err(error) => {
                    self.notice = Some(safe_label(
                        format!("Save failed: {error:#} · Ctrl-S retries").as_bytes(),
                    ))
                }
            },
            Ok(Output::Copy(prepared)) => {
                let result = clipboard::command(prepared.bytes()).and_then(|command| {
                    self.worker.as_mut().unwrap().deliver(
                        reply.id,
                        prepared,
                        engine.review_id(),
                        command,
                    )
                });
                match result {
                    Ok(()) => self.pending = Some(Pending::Deliver(reply.id)),
                    Err(error) => {
                        self.notice = Some(safe_label(
                            format!("Copy failed: {error:#} · Ctrl-S saves instead").as_bytes(),
                        ))
                    }
                }
            }
            Ok(Output::Copied(receipt)) => {
                self.notice = Some(match receipt.check(engine.review_id()) {
                    Ok(()) => format!("Copied {} bytes as plain text", receipt.bytes),
                    Err(_) => "Copy source changed; clipboard contents may have changed".into(),
                });
            }
            Err(error) if cleanup_failed(&error) => return Err(error),
            Err(error) => {
                let fallback = if matches!(pending, Pending::Copy(_) | Pending::Deliver(_)) {
                    " · Ctrl-S saves instead"
                } else {
                    ""
                };
                self.notice = Some(safe_label(
                    format!("{} failed: {error:#}{fallback}", pending.name()).as_bytes(),
                ))
            }
        }
        Ok(())
    }

    pub fn close(&mut self) -> Result<()> {
        let result = self
            .worker
            .take()
            .map_or(Ok(()), |mut worker| worker.close());
        self.pending = None;
        self.read = None;
        self.current = None;
        self.prompt = None;
        self.selection = None;
        self.selection_status = None;
        result
    }
}

impl Drop for Reading {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests;
