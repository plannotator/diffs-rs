//! A review owns its workers and view state, but never the host terminal.

mod reading;
mod selection;

use std::{
    fs::File,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, SyncSender},
    },
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail, ensure};
use crossterm::event::Event;
use ratatui::{Frame, layout::Rect, widgets::Paragraph};

pub use crate::app::{Command, InputEffect, Layout, SidebarPreference};
use crate::{
    app::App,
    input::{self, LocalInput, PatchInput, SourceState},
    model::RowKind,
    render, storage,
    text::{safe_label, safe_text},
    theme::Appearance,
};

pub enum Input {
    Patch(PathBuf),
    /// Takes exclusive ownership of the shared read cursor and status flags until close.
    Stream {
        file: File,
        label: String,
    },
    Local(LocalInput),
}

#[derive(Clone, Copy)]
pub struct Options {
    pub layout: Layout,
    pub appearance: Appearance,
    pub sidebar: SidebarPreference,
    pub wrap: bool,
    pub line_numbers: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            layout: Layout::Auto,
            appearance: Appearance::Dark,
            sidebar: SidebarPreference::Auto,
            wrap: false,
            line_numbers: true,
        }
    }
}

/// One open occurrence within this process, not a persistent revision identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ReviewId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Old,
    New,
}

/// Original one-based source lines of the selected record, never display-row or byte offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    pub review: ReviewId,
    pub file: usize,
    pub side: Side,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
}

pub struct Status<'a> {
    pub closed: bool,
    pub loading: bool,
    pub files: usize,
    pub coloring: bool,
    pub preparing_source: bool,
    pub source_failed: bool,
    pub error: Option<&'a str>,
}

/// Caller-owned controls to display alongside the engine's existing hints and help.
pub struct ReadingControls<'a> {
    pub hint: &'a str,
    pub help: &'a str,
    pub status: Option<&'a str>,
}

pub struct Engine {
    id: ReviewId,
    options: Options,
    label: String,
    app: Option<App>,
    input: Option<input::Worker>,
    full_sources: bool,
    wake: SyncSender<()>,
    loading: bool,
    error: Option<String>,
    source_failed: bool,
    source_reads: Arc<AtomicBool>,
    dirty: bool,
    request: bool,
    progress: bool,
    last_draw: Instant,
    draw_pending: bool,
    area: Rect,
}

impl Engine {
    pub fn open(input: Input, options: Options, wake: SyncSender<()>) -> Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let id = ReviewId(
            NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
                .map_err(|_| anyhow::anyhow!("Review identities exhausted"))?,
        );
        let full_sources = matches!(&input, Input::Local(_));
        let (label, worker) = match input {
            Input::Patch(path) => {
                let label = safe_label(path.to_string_lossy().as_bytes());
                (
                    label,
                    input::Worker::start_patch(PatchInput::Path(path), wake.clone()),
                )
            }
            Input::Stream { file, label } => (
                safe_text(label.as_bytes()),
                input::Worker::start_patch(PatchInput::Stream(file), wake.clone()),
            ),
            Input::Local(input) => (input.label(), input::Worker::start(input, wake.clone())),
        };
        Ok(Self {
            id,
            options,
            label,
            app: None,
            input: Some(worker),
            full_sources,
            wake,
            loading: true,
            error: None,
            source_failed: false,
            source_reads: Arc::new(AtomicBool::new(true)),
            dirty: true,
            request: true,
            progress: false,
            last_draw: Instant::now(),
            draw_pending: false,
            area: Rect::default(),
        })
    }

    pub fn review_id(&self) -> ReviewId {
        self.id
    }

    pub fn options(&self) -> Options {
        self.app.as_ref().map_or(self.options, |app| Options {
            layout: app.layout,
            appearance: app.appearance,
            sidebar: app.sidebar.preference,
            wrap: app.wrap,
            line_numbers: app.numbers,
        })
    }

    pub fn file(&self, index: usize) -> Option<&storage::FileMetadata> {
        self.app
            .as_ref()?
            .doc
            .files
            .get(index)
            .map(|file| &file.meta)
    }

    /// Prepares an owned source job without reading source bodies on the input thread.
    /// Streaming rejects missing lines unless checked full source covers the range.
    pub fn source_read(
        &mut self,
        range: crate::reading::SourceRange,
    ) -> Result<crate::reading::SourceRead> {
        ensure!(self.input.is_some(), "The review is closed");
        ensure!(
            range.review == self.id,
            "Source range belongs to another review"
        );
        let app = self
            .app
            .as_mut()
            .context("Review source is not available")?;
        let file = app
            .doc
            .files
            .get(range.file)
            .context("Source file is not available")?;
        let pair = match app
            .sources
            .as_ref()
            .and_then(|sources| sources.get(range.file))
        {
            Some(SourceState::Checked(pair)) => Some(pair),
            _ => None,
        };
        let pair = if let Some(pair) = pair {
            Some((pair, app.source_reads.lease(range.file)))
        } else {
            None
        };
        crate::reading::SourceRead::new(range, file.clone(), pair, self.source_reads.clone())
    }

    pub(crate) fn search_domain(&mut self) -> Result<crate::reading::domain::Domain> {
        ensure!(self.input.is_some(), "The review is closed");
        ensure!(
            !self.loading,
            "Wait for the review to finish loading before searching"
        );
        ensure!(self.error.is_none(), "Cannot search an incomplete review");
        let app = self
            .app
            .as_mut()
            .context("Review source is not available")?;
        crate::reading::domain::Domain::capture(
            self.id,
            self.source_reads.clone(),
            &app.doc,
            &mut app.view,
            app.sources.as_deref(),
        )
    }

    pub fn status(&self) -> Status<'_> {
        let mut coloring = false;
        let mut preparing_source = false;
        if let Some(app) = &self.app {
            for row in app
                .visible
                .iter()
                .filter_map(|(position, _)| app.row(position.row))
            {
                preparing_source |= matches!(
                    app.sources
                        .as_ref()
                        .and_then(|sources| sources.get(row.file)),
                    Some(SourceState::Pending)
                );
                if let RowKind::Code { left, right } = row.kind {
                    coloring |= left
                        .into_iter()
                        .chain(right)
                        .any(|id| app.cache.get(id).is_none());
                }
            }
        }
        Status {
            closed: self.input.is_none(),
            loading: self.loading,
            files: self.app.as_ref().map_or(0, |app| app.doc.files.len()),
            coloring,
            preparing_source,
            source_failed: self.source_failed,
            error: self.error.as_deref(),
        }
    }

    pub fn selected_source(&self) -> Result<Option<SourceLocation>> {
        let Some(app) = &self.app else {
            return Ok(None);
        };
        let Some(row) = app.row(app.cursor.row) else {
            return Ok(None);
        };
        let Some((id, old)) = app.source_at_cursor(None)? else {
            return Ok(None);
        };
        let info = app.line_info(id)?;
        Ok(Some(SourceLocation {
            review: self.id,
            file: row.file,
            side: if old { Side::Old } else { Side::New },
            old_line: info.old,
            new_line: info.new,
        }))
    }

    fn app(&mut self) -> Result<&mut App> {
        if self.app.is_none() {
            let mut app = App::new(
                storage::Document::default(),
                self.label.clone(),
                self.options.layout,
                self.options.appearance,
                self.wake.clone(),
            )?;
            app.set_sidebar_preference(self.options.sidebar);
            app.wrap = self.options.wrap;
            app.numbers = self.options.line_numbers;
            if self.full_sources {
                app.sources = Some(Vec::new());
            }
            self.app = Some(app);
        }
        Ok(self.app.as_mut().unwrap())
    }

    /// Clear the host wake token before calling this. Each drain is bounded so input can run.
    pub fn poll(&mut self) -> Result<()> {
        if self.input.is_none() {
            return Ok(());
        }
        if (self.loading || self.full_sources) && self.error.is_none() {
            for _ in 0..64 {
                match self.input.as_ref().unwrap().try_reply() {
                    Ok(Ok(input::Message::File(file))) => {
                        let app = self.app()?;
                        let exposes_more = app
                            .visible
                            .last()
                            .is_none_or(|(p, _)| p.row + 1 >= app.row_len());
                        app.doc.append(file)?;
                        if let Some(sources) = &mut app.sources {
                            sources.push(SourceState::Pending);
                        }
                        self.request |= exposes_more;
                        self.dirty |= exposes_more;
                        self.progress = true;
                    }
                    Ok(Ok(input::Message::Complete { files })) => {
                        crate::delivery_event!("indexed", 0, 0);
                        ensure!(
                            self.app()?.doc.files.len() == files,
                            "Input completion does not match the published files"
                        );
                        self.loading = false;
                        self.dirty = true;
                        break;
                    }
                    Ok(Ok(input::Message::Source { file, state })) => {
                        self.source_failed |= matches!(state, SourceState::Unavailable(_));
                        self.app
                            .as_mut()
                            .context("Source reply arrived before the patch")?
                            .set_source(file, state)?;
                        self.request = true;
                        self.dirty = true;
                    }
                    Ok(Err(error)) => {
                        self.fail_input(format!("{error:#}"));
                        break;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.fail_input(
                            "The input worker stopped before completing its work.".into(),
                        );
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                }
            }
        }
        if let Some(app) = &mut self.app {
            app.loading = self.loading;
            for _ in 0..64 {
                let reply = match app.worker.try_reply() {
                    Ok(reply) => reply?,
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        bail!("The syntax worker stopped before the viewer closed")
                    }
                };
                #[cfg(feature = "delivery-trace")]
                crate::delivery_trace::source_event("drain", reply.generation, Some(reply.id));
                let id = reply.id;
                let failed = reply.source_error.is_some();
                if app.accept_highlight(reply)? {
                    self.source_failed |= failed;
                    self.request |= failed;
                    self.dirty |= failed || app.visible.iter().any(|(p, _)| {
                        matches!(app.row(p.row).map(|r| r.kind), Some(RowKind::Code { left, right }) if left == Some(id) || right == Some(id))
                    });
                }
            }
        }
        Ok(())
    }

    fn fail_input(&mut self, error: String) {
        self.error = Some(safe_text(error.as_bytes()));
        self.loading = false;
        self.dirty = true;
    }

    pub fn needs_draw(&self) -> bool {
        self.dirty || (self.progress && self.last_draw.elapsed() >= Duration::from_millis(100))
    }

    /// Draw only into the intersection with the frame. The supplied pane controls reflow.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
        self.draw_with_controls(frame, area, None)
    }

    pub fn draw_with_controls(
        &mut self,
        frame: &mut Frame<'_>,
        area: Rect,
        controls: Option<&ReadingControls<'_>>,
    ) -> Result<()> {
        self.draw_pending = false;
        let area = area.intersection(frame.area());
        self.request |= (self.area.width, self.area.height) != (area.width, area.height);
        self.area = area;
        if let Some(app) = &mut self.app {
            let previous_size = (app.width, app.height);
            render::draw_with_controls(frame, app, area, self.error.as_deref(), controls)?;
            self.request |= previous_size != (app.width, app.height);
        } else {
            let text = self
                .error
                .as_ref()
                .map(|error| format!("Cannot open patch\n\n{error}\n\nq quit"))
                .unwrap_or_else(|| {
                    if self.input.is_some() {
                        "Loading patch…\n\nq quit · Ctrl-C cancel".into()
                    } else {
                        "Review closed".into()
                    }
                });
            frame.render_widget(Paragraph::new(text), area);
        }
        self.draw_pending = true;
        Ok(())
    }

    /// Call after a successful host terminal draw, before handling more events.
    /// Visible work starts after the text frame has been written, not while it is being composed.
    pub fn after_draw(&mut self) {
        if !std::mem::take(&mut self.draw_pending) {
            return;
        }
        if let Some(app) = &self.app {
            if self.full_sources && !self.loading {
                let file = app
                    .row(app.cursor.row)
                    .map(|row| row.file)
                    .into_iter()
                    .chain(
                        app.visible
                            .iter()
                            .filter_map(|(position, _)| app.row(position.row).map(|row| row.file)),
                    )
                    .find(|id| {
                        matches!(
                            app.sources.as_ref().and_then(|sources| sources.get(*id)),
                            Some(SourceState::Pending)
                        )
                    });
                if let Some(worker) = &self.input {
                    worker.request(file.unwrap_or(usize::MAX));
                }
            }
            crate::delivery_event!("frame", app.epoch, 0);
            if self.request {
                app.request_highlights();
            }
        } else {
            crate::delivery_event!("frame", 0, 0);
        }
        self.request = false;
        self.dirty = false;
        self.progress = false;
        self.last_draw = Instant::now();
    }

    /// The host routes keyboard focus; mouse coordinates remain absolute terminal cells.
    pub fn handle_event(&mut self, event: Event, focused: bool) -> Result<InputEffect> {
        if self.input.is_none() {
            return Ok(InputEffect::Ignored);
        }
        if matches!(event, Event::Resize(..)) {
            self.dirty = true;
            self.request = true;
            return Ok(InputEffect::Redraw);
        }
        if matches!(event, Event::Key(_)) && !focused {
            return Ok(InputEffect::Ignored);
        }
        if let Event::Mouse(mouse) = &event
            && !self.area.contains((mouse.column, mouse.row).into())
        {
            return Ok(InputEffect::Ignored);
        }
        let shift_click = matches!(&event, Event::Mouse(mouse)
            if mouse.kind == crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left)
            && mouse.modifiers.contains(crossterm::event::KeyModifiers::SHIFT)
            && self.app.as_ref().is_some_and(|app| !app.help && app.sidebar.diff.contains((mouse.column, mouse.row).into())));
        if shift_click {
            if self.selection().is_none() {
                self.selection_command(Command::SelectLines)?;
            }
            if let Some(selection) = self.app.as_mut().and_then(|app| app.selection.as_mut()) {
                selection.extending = true;
            }
        }
        let extend = selection::extends(&event)
            && match &event {
                Event::Mouse(mouse)
                    if mouse.kind
                        == crossterm::event::MouseEventKind::Down(
                            crossterm::event::MouseButton::Left,
                        ) =>
                {
                    self.app.as_ref().is_some_and(|app| {
                        app.sidebar.diff.contains((mouse.column, mouse.row).into())
                            && app
                                .visible
                                .iter()
                                .find(|(_, y)| *y == mouse.row - app.sidebar.diff.y + 1)
                                .and_then(|(position, _)| app.row(position.row))
                                .is_some_and(|row| matches!(row.kind, RowKind::Code { .. }))
                    })
                }
                _ => true,
            };
        let Some(app) = &mut self.app else {
            return Ok(InputEffect::Ignored);
        };
        let effect = app.handle_event(event)?;
        self.changed(effect);
        self.track_selection(extend && effect != InputEffect::Ignored)?;
        if shift_click
            && let Some(selection) = self.app.as_mut().and_then(|app| app.selection.as_mut())
        {
            selection.extending = false;
        }
        Ok(effect)
    }

    pub fn execute(&mut self, command: Command) -> Result<InputEffect> {
        if matches!(command, Command::SelectLines | Command::ClearSelection) {
            return self.selection_command(command);
        }
        let Some(app) = &mut self.app else {
            return Ok(InputEffect::Ignored);
        };
        let effect = app.execute(command)?;
        self.changed(effect);
        self.track_selection(selection::moves(command) && effect != InputEffect::Ignored)?;
        Ok(effect)
    }

    fn changed(&mut self, effect: InputEffect) {
        self.dirty |= effect != InputEffect::Ignored;
        self.request |= effect == InputEffect::Highlight;
    }

    /// Close the previous review before creating fresh channels, identity and workers.
    pub fn replace(&mut self, input: Input) -> Result<()> {
        let options = self.options();
        self.close()?;
        *self = Self::open(input, options, self.wake.clone())?;
        Ok(())
    }

    /// Joins engine workers and releases its view/source ownership.
    /// Caller-owned source jobs are revoked but must be joined and dropped by their owner.
    /// Individual parser calls are not interruptible.
    pub fn close(&mut self) -> Result<()> {
        self.source_reads.store(false, Ordering::Release);
        self.options = self.options();
        if let Some(app) = &self.app {
            app.worker.cancel();
        }
        let input = self
            .input
            .take()
            .map(|mut worker| worker.close())
            .transpose();
        let colors = self.app.as_mut().map(|app| app.worker.close()).transpose();
        self.app = None;
        self.loading = false;
        self.error = None;
        self.source_failed = false;
        self.progress = false;
        self.request = false;
        self.draw_pending = false;
        self.dirty = true;
        crate::cleanup::finish(input.map(|_| ()), colors.map(|_| ()))
    }

    #[cfg(feature = "owner-census")]
    pub fn poll_census(&self, capture: &mut crate::owner_census::Capture) -> Result<()> {
        self.app.as_ref().map_or(Ok(()), |app| capture.poll(app))
    }

    #[cfg(feature = "owner-census")]
    pub fn request_census(&self, capture: &mut crate::owner_census::Capture) -> Result<()> {
        self.app.as_ref().map_or(Ok(()), |app| capture.request(app))
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests;
