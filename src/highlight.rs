use std::{
    collections::{VecDeque, hash_map::RandomState},
    ops::Range,
    path::Path,
    sync::{Arc, Condvar, Mutex, mpsc},
    thread,
};

use ratatui::style::Color;
use syntect::{
    highlighting::{HighlightIterator, HighlightState, Highlighter},
    parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet},
};

#[cfg(feature = "delivery-trace")]
use crate::delivery_trace::{Outcome, ReplayReason, RequestWork};
#[cfg(feature = "delivery-trace")]
use std::time::Instant;

use crate::{
    model::Kind,
    storage::FileIndex,
    text::display_text,
    theme::{self, Appearance},
    view::LineId,
};

mod checkpoint;
use checkpoint::Checkpoints;
mod full_source;
mod input;
pub use input::{FileSource, Origin};

mod inline;
pub use inline::{ChangeSide, changed_spans};

#[cfg(feature = "owner-census")]
mod cache_activity;
#[cfg(feature = "owner-census")]
mod census;

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub range: Range<usize>,
    pub color: Color,
}

#[derive(Clone, Debug, Default)]
pub struct LineStyle {
    pub tokens: Vec<Token>,
    pub old_tokens: Option<Vec<Token>>,
    pub changed: Vec<Range<usize>>,
    pub fallback: bool,
    pub fallback_reason: Option<Fallback>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fallback {
    UnsupportedLanguage,
    LongLine,
    ParserFailure,
}

impl Fallback {
    pub fn status(self) -> &'static str {
        match self {
            Self::UnsupportedLanguage => "Plain syntax · unsupported language",
            Self::LongLine => "Plain syntax · oversized line in source prefix",
            Self::ParserFailure => "Plain syntax · source parser failed",
        }
    }
}

impl LineStyle {
    fn cache_weight(&self) -> usize {
        size_of::<Self>()
            + self.tokens.capacity() * size_of::<Token>()
            + self
                .old_tokens
                .as_ref()
                .map_or(0, |tokens| tokens.capacity() * size_of::<Token>())
            + self.changed.capacity() * size_of::<Range<usize>>()
    }

    pub fn tokens_for(&self, old: bool) -> &[Token] {
        if old {
            self.old_tokens.as_deref().unwrap_or(&self.tokens)
        } else {
            &self.tokens
        }
    }
}

fn tokens(fragments: Vec<(syntect::highlighting::Style, &str)>) -> Vec<Token> {
    let mut byte = 0;
    fragments
        .into_iter()
        .map(|(style, text)| {
            let start = byte;
            byte += text.len();
            Token {
                range: start..byte,
                color: Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b),
            }
        })
        .collect()
}

struct Side {
    parse: ParseState,
    highlight: HighlightState,
    needs_colors: bool,
    fallback: Option<Fallback>,
}

impl Side {
    fn new(syntax: &SyntaxReference, highlighter: &Highlighter<'_>) -> Self {
        Self {
            parse: ParseState::new(syntax),
            highlight: HighlightState::new(highlighter, ScopeStack::new()),
            needs_colors: false,
            fallback: None,
        }
    }

    fn advance(
        &mut self,
        text: &str,
        syntaxes: &SyntaxSet,
        highlighter: &Highlighter<'_>,
        visible: bool,
        #[cfg(feature = "delivery-trace")] work: &mut RequestWork,
        #[cfg(feature = "delivery-trace")] source_side: (LineId, bool),
    ) -> Option<Vec<Token>> {
        if self.fallback.is_some() {
            return None;
        }
        #[cfg(feature = "delivery-trace")]
        let initialization = syntect::parsing::syntax_timing::Window::parser_call();
        #[cfg(feature = "delivery-trace")]
        let parse_start = Instant::now();
        let parsed = self.parse.parse_line(text, syntaxes);
        #[cfg(feature = "delivery-trace")]
        let parse_elapsed = parse_start.elapsed();
        #[cfg(feature = "delivery-trace")]
        work.parsed(
            parse_elapsed,
            visible,
            source_side.0,
            source_side.1,
            initialization.finish(),
        );
        let ops = match parsed {
            Ok(ops) => ops,
            Err(_) => {
                self.fallback = Some(Fallback::ParserFailure);
                return None;
            }
        };
        #[cfg(feature = "delivery-trace")]
        let scope_start = Instant::now();
        if visible {
            if self.needs_colors {
                let path = std::mem::replace(&mut self.highlight.path, ScopeStack::new());
                self.highlight = HighlightState::new(highlighter, path);
                self.needs_colors = false;
            }
            let colored = tokens(
                HighlightIterator::new(&mut self.highlight, &ops, text, highlighter).collect(),
            );
            #[cfg(feature = "delivery-trace")]
            work.summary.visible_colors.record(scope_start);
            Some(colored)
        } else {
            self.needs_colors = true;
            for (_, op) in ops {
                if self.highlight.path.apply(&op).is_err() {
                    self.fallback = Some(Fallback::ParserFailure);
                    #[cfg(feature = "delivery-trace")]
                    work.summary.replay_scopes.record(scope_start);
                    return None;
                }
            }
            #[cfg(feature = "delivery-trace")]
            work.summary.replay_scopes.record(scope_start);
            Some(Vec::new())
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SyntaxScope {
    PatchHunk(usize),
    SourceSide { old: bool },
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SyntaxIdentity {
    origin: Origin,
    file: usize,
    scope: SyntaxScope,
    epoch: u64,
    appearance: Appearance,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SyntaxKey {
    origin: Origin,
    file: usize,
    scope: SyntaxScope,
}

impl SyntaxIdentity {
    fn key(self) -> SyntaxKey {
        SyntaxKey {
            origin: self.origin,
            file: self.file,
            scope: self.scope,
        }
    }
}

struct Continuation {
    identity: SyntaxIdentity,
    next: usize,
    old: Side,
    new: Side,
}

pub struct Request<S = usize> {
    pub epoch: u64,
    pub appearance: Appearance,
    pub ids: Vec<S>,
    pub files: Vec<FileSource>,
}
pub struct Reply {
    pub origin: Origin,
    pub source_error: Option<Arc<str>>,
    #[cfg(feature = "delivery-trace")]
    pub generation: u64,
    pub epoch: u64,
    pub id: LineId,
    pub style: LineStyle,
}

pub struct Worker {
    requests: Arc<Mailbox>,
    replies: mpsc::Receiver<anyhow::Result<Reply>>,
    thread: Option<thread::JoinHandle<()>>,
}

#[derive(Default)]
struct Pending {
    request: Option<Request<LineId>>,
    #[cfg(feature = "owner-census")]
    census: Option<crate::owner_census::Request>,
    generation: u64,
    closed: bool,
}

struct Mailbox {
    pending: Mutex<Pending>,
    ready: Condvar,
    wake: mpsc::SyncSender<()>,
}

impl Worker {
    #[cfg(feature = "owner-census")]
    pub(crate) fn request_census(
        &self,
        request: crate::owner_census::Request,
    ) -> anyhow::Result<()> {
        let mut pending = self.requests.pending.lock().unwrap();
        anyhow::ensure!(
            !pending.closed && pending.census.is_none(),
            "Census worker unavailable"
        );
        pending.census = Some(request);
        self.requests.ready.notify_one();
        Ok(())
    }

    pub fn try_reply(&self) -> Result<anyhow::Result<Reply>, mpsc::TryRecvError> {
        // Pair capacity release with the publisher's atomic unlock-and-wait.
        let _pending = self.requests.pending.lock().unwrap();
        let reply = self.replies.try_recv()?;
        self.requests.ready.notify_one();
        Ok(reply)
    }

    pub fn request<S: Into<LineId>>(&self, request: Request<S>) {
        let mut pending = self.requests.pending.lock().unwrap();
        if pending.closed {
            return;
        }
        pending.request = Some(Request {
            epoch: request.epoch,
            appearance: request.appearance,
            ids: request.ids.into_iter().map(Into::into).collect(),
            files: request.files,
        });
        pending.generation = pending.generation.wrapping_add(1);
        crate::delivery_event!(
            "request",
            pending.generation,
            pending.request.as_ref().unwrap().epoch
        );
        self.requests.ready.notify_one();
    }

    pub fn cancel(&self) {
        let mut pending = self
            .requests
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        pending.request = None;
        pending.generation = pending.generation.wrapping_add(1);
        crate::delivery_event!("cancel", pending.generation, 0);
        self.requests.ready.notify_one();
    }

    /// Cancel outstanding work and wait for the syntax worker to release its owners.
    pub fn close(&mut self) -> anyhow::Result<()> {
        {
            let mut pending = self
                .requests
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            pending.closed = true;
            pending.request = None;
            #[cfg(feature = "owner-census")]
            {
                pending.census = None;
            }
            self.requests.ready.notify_one();
        }
        let result = self.thread.take().map_or(Ok(()), |thread| {
            thread
                .join()
                .map_err(|_| anyhow::anyhow!("The syntax worker panicked"))
        });
        while self.replies.try_recv().is_ok() {}
        result
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

impl Mailbox {
    fn take(
        &self,
        #[cfg(feature = "owner-census")] syntaxes: &syntect::parsing::SyntaxSet,
        #[cfg(feature = "owner-census")] inspect: impl Fn() -> serde_json::Value,
    ) -> Option<(u64, Request<LineId>)> {
        let mut pending = self.pending.lock().unwrap();
        loop {
            if pending.closed {
                return None;
            }
            if let Some(request) = pending.request.take() {
                return Some((pending.generation, request));
            }
            #[cfg(feature = "owner-census")]
            if let Some(census) = pending.census.take() {
                drop(pending);
                census.capture(syntaxes, &inspect);
                let _ = self.wake.try_send(());
                pending = self.pending.lock().unwrap();
                continue;
            }
            pending = self.ready.wait(pending).unwrap();
        }
    }

    fn current(&self, generation: u64) -> bool {
        let pending = self.pending.lock().unwrap();
        !pending.closed && pending.generation == generation
    }

    fn publish(
        &self,
        generation: u64,
        mut reply: anyhow::Result<Reply>,
        output: &mpsc::SyncSender<anyhow::Result<Reply>>,
    ) -> bool {
        #[cfg(feature = "delivery-trace")]
        crate::delivery_trace::source_event("ready", generation, reply.as_ref().ok().map(|r| r.id));
        let mut pending = self.pending.lock().unwrap();
        loop {
            if pending.closed || pending.generation != generation {
                return false;
            }
            #[cfg(feature = "delivery-trace")]
            crate::delivery_trace::source_event(
                "offer",
                generation,
                reply.as_ref().ok().map(|r| r.id),
            );
            match output.try_send(reply) {
                Ok(()) => {
                    crate::delivery_event!("published", generation, 0);
                    drop(pending);
                    let _ = self.wake.try_send(());
                    return true;
                }
                Err(mpsc::TrySendError::Disconnected(_)) => return false,
                Err(mpsc::TrySendError::Full(value)) => reply = value,
            }
            crate::delivery_event!("full", generation, 0);
            pending = self.ready.wait(pending).unwrap();
        }
    }
}

pub fn start(wake: mpsc::SyncSender<()>) -> Worker {
    let requests = Arc::new(Mailbox {
        pending: Mutex::default(),
        ready: Condvar::default(),
        wake,
    });
    let input = requests.clone();
    let (output, replies) = mpsc::sync_channel(16);
    let thread = thread::spawn(move || {
        // Drop the reply sender before notifying, including during unwinding.
        let _notify_exit = crate::wake::NotifyOnExit(input.wake.clone());
        let output = output;
        #[cfg(feature = "delivery-trace")]
        let syntax_start = Instant::now();
        let syntaxes = two_face::syntax::extra_newlines();
        #[cfg(feature = "regex-census")]
        let _census = crate::regex_census::Capture::new(&syntaxes);
        #[cfg(feature = "delivery-trace")]
        crate::delivery_event!("syntax_set_startup", syntax_start.elapsed().as_nanos(), 0);
        let mut continuation: Option<Continuation> = None;
        let mut full = full_source::Replay::default();
        let mut checkpoints = Checkpoints::new();
        'requests: while let Some((generation, request)) = input.take(
            #[cfg(feature = "owner-census")]
            &syntaxes,
            #[cfg(feature = "owner-census")]
            || census::worker(&continuation, &full, &checkpoints),
        ) {
            #[cfg(feature = "delivery-trace")]
            let mut work = RequestWork::new(generation, request.epoch, &request.ids);
            #[cfg(feature = "delivery-trace")]
            let theme_start = Instant::now();
            let Ok((_, theme)) = theme::load(request.appearance) else {
                #[cfg(feature = "delivery-trace")]
                {
                    work.summary.theme.record(theme_start);
                    work.summary.outcome = Outcome::ThemeError;
                }
                break;
            };
            let highlighter = Highlighter::new(&theme);
            #[cfg(feature = "delivery-trace")]
            work.summary.theme.record(theme_start);
            let mut ids = request.ids.clone();
            ids.sort_unstable();
            ids.dedup();
            #[cfg(feature = "delivery-trace")]
            {
                work.summary.requested_unique = Some(ids.len());
            }
            for source in &request.files {
                let mut checkpoint_scope = checkpoints.scope();
                let checkpoints = checkpoint_scope.cache();
                let file = source.file();
                let origin = source.origin();
                let selected: Vec<_> = ids
                    .iter()
                    .copied()
                    .filter(|id| id.belongs_to(file))
                    .collect();
                if !matches!(source, FileSource::Patch(_)) {
                    continuation = None;
                    let completed = full.run(full_source::Job {
                        input: &input,
                        output: &output,
                        generation,
                        request: &request,
                        source,
                        wanted: &selected,
                        syntaxes: &syntaxes,
                        highlighter: &highlighter,
                        checkpoints,
                        #[cfg(feature = "delivery-trace")]
                        work: &mut work,
                    });
                    match completed {
                        Ok(true) => continue,
                        Ok(false) => continue 'requests,
                        Err(error) => {
                            input.publish(generation, Err(error), &output);
                            return;
                        }
                    }
                }
                let patch_ids: Vec<_> = selected.iter().filter_map(|id| id.patch()).collect();
                let mut wanted = patch_ids.as_slice();
                full = full_source::Replay::default();
                let ext = Path::new(file.path.as_ref())
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                let syntax = syntaxes
                    .find_syntax_by_extension(ext)
                    .unwrap_or_else(|| syntaxes.find_syntax_plain_text());
                while let Some(&first) = wanted.first() {
                    let hunk_index = file.hunks.partition_point(|h| h.lines.end <= first);
                    let hunk = &file.hunks[hunk_index];
                    let count = wanted.partition_point(|id| *id < hunk.lines.end);
                    let (current, remaining) = wanted.split_at(count);
                    wanted = remaining;
                    let last = current[current.len() - 1];
                    let identity = SyntaxIdentity {
                        origin,
                        file: file.id,
                        scope: SyntaxScope::PatchHunk(hunk_index),
                        epoch: request.epoch,
                        appearance: request.appearance,
                    };
                    #[cfg(feature = "delivery-trace")]
                    let mut reason = match continuation.as_ref() {
                        None => ReplayReason::Missing,
                        Some(c) if c.identity.file != file.id => ReplayReason::File,
                        Some(c) if c.identity.scope != identity.scope => ReplayReason::Hunk,
                        Some(c)
                            if c.identity.epoch != request.epoch
                                || c.identity.appearance != request.appearance =>
                        {
                            ReplayReason::Epoch
                        }
                        Some(c) if c.next > first => ReplayReason::Backward,
                        Some(_) => ReplayReason::Resume,
                    };
                    let active = continuation
                        .as_ref()
                        .filter(|c| c.identity == identity && c.next <= first)
                        .map(|c| c.next);
                    if let Some(checkpoint) = checkpoints.nearest(identity, first)
                        && active.is_none_or(|next| checkpoint.next > next)
                    {
                        if !input.current(generation) {
                            #[cfg(feature = "delivery-trace")]
                            {
                                work.summary.outcome = Outcome::SupersededOrClosed;
                            }
                            continue 'requests;
                        }
                        let Some(restored) = checkpoint.restore(identity, &highlighter) else {
                            continue 'requests;
                        };
                        if !input.current(generation) {
                            #[cfg(feature = "delivery-trace")]
                            {
                                work.summary.outcome = Outcome::SupersededOrClosed;
                            }
                            continue 'requests;
                        }
                        continuation = Some(restored);
                        #[cfg(feature = "delivery-trace")]
                        {
                            reason = ReplayReason::Checkpoint;
                        }
                    } else if active.is_none() {
                        continuation = Some(Continuation {
                            identity,
                            next: hunk.lines.start,
                            old: Side::new(syntax, &highlighter),
                            new: Side::new(syntax, &highlighter),
                        });
                    }
                    let state = continuation.as_mut().unwrap();
                    #[cfg(feature = "delivery-trace")]
                    work.group(file.id, hunk_index, state.next, first..=last, reason);
                    for id in state.next..=last {
                        #[cfg(feature = "delivery-trace")]
                        if id % 32 == 0 {
                            work.summary.replay_cancellation_checks += 1;
                        }
                        if id % 32 == 0 && !input.current(generation) {
                            #[cfg(feature = "delivery-trace")]
                            {
                                work.summary.outcome = Outcome::SupersededOrClosed;
                            }
                            continue 'requests;
                        }
                        let source = file.info(id);
                        let visible = current.binary_search(&id).is_ok();
                        let bytes = match file.bytes(id) {
                            Ok(bytes) => bytes,
                            Err(error) => {
                                #[cfg(feature = "delivery-trace")]
                                let publish_start = Instant::now();
                                input.publish(generation, Err(error), &output);
                                #[cfg(feature = "delivery-trace")]
                                {
                                    work.summary.publication.record(publish_start);
                                    work.summary.outcome = Outcome::SourceError;
                                }
                                return;
                            }
                        };
                        let text = display_text(&bytes);
                        let text = format!("{text}\n");
                        let too_long = text.len() > 40_004 || text.chars().count() > 10_001;
                        let mut style = LineStyle {
                            fallback: too_long,
                            ..LineStyle::default()
                        };
                        if too_long {
                            if source.old.is_some() {
                                state.old.fallback = Some(Fallback::LongLine);
                            }
                            if source.new.is_some() {
                                state.new.fallback = Some(Fallback::LongLine);
                            }
                        }
                        if !too_long {
                            let old_tokens = if source.old.is_some() {
                                state.old.advance(
                                    &text,
                                    &syntaxes,
                                    &highlighter,
                                    visible,
                                    #[cfg(feature = "delivery-trace")]
                                    &mut work,
                                    #[cfg(feature = "delivery-trace")]
                                    (LineId::Patch(id), true),
                                )
                            } else {
                                None
                            };
                            let new_tokens = if source.new.is_some() {
                                state.new.advance(
                                    &text,
                                    &syntaxes,
                                    &highlighter,
                                    visible,
                                    #[cfg(feature = "delivery-trace")]
                                    &mut work,
                                    #[cfg(feature = "delivery-trace")]
                                    (LineId::Patch(id), false),
                                )
                            } else {
                                None
                            };
                            if visible {
                                style.fallback |= (source.old.is_some() && old_tokens.is_none())
                                    || (source.new.is_some() && new_tokens.is_none());
                                if source.kind == Kind::Deletion {
                                    style.tokens = old_tokens.unwrap_or_default();
                                } else {
                                    style.tokens = new_tokens.unwrap_or_default();
                                    if source.kind == Kind::Context {
                                        let old = old_tokens.unwrap_or_default();
                                        if old != style.tokens {
                                            style.old_tokens = Some(old);
                                        }
                                    }
                                }
                            }
                        }
                        state.next = id + 1;
                        #[cfg(feature = "delivery-trace")]
                        {
                            work.summary.processed_records += 1;
                            if visible {
                                work.summary.requested_records += 1;
                                work.summary.fallback_requested += usize::from(style.fallback);
                            } else if id < first {
                                work.summary.predecessor_records += 1;
                            } else {
                                work.summary.other_replay_records += 1;
                            }
                        }
                        if !checkpoints
                            .capture(state, hunk.lines.start, || input.current(generation))
                        {
                            #[cfg(feature = "delivery-trace")]
                            {
                                work.summary.outcome = Outcome::SupersededOrClosed;
                            }
                            continue 'requests;
                        }
                        if !visible {
                            continue;
                        }
                        if !too_long && let Some(partner) = file.partner(id) {
                            let bytes = match file.bytes(partner) {
                                Ok(bytes) => bytes,
                                Err(error) => {
                                    #[cfg(feature = "delivery-trace")]
                                    let publish_start = Instant::now();
                                    input.publish(generation, Err(error), &output);
                                    #[cfg(feature = "delivery-trace")]
                                    {
                                        work.summary.publication.record(publish_start);
                                        work.summary.outcome = Outcome::SourceError;
                                    }
                                    return;
                                }
                            };
                            let other = display_text(&bytes);
                            if other.len() <= 40_000 && other.chars().count() <= 10_000 {
                                #[cfg(feature = "delivery-trace")]
                                let comparison_start = Instant::now();
                                let text = text.trim_end_matches('\n');
                                style.changed = if source.kind == Kind::Deletion {
                                    changed_spans(text, &other, ChangeSide::Old)
                                } else {
                                    changed_spans(&other, text, ChangeSide::New)
                                };
                                #[cfg(feature = "delivery-trace")]
                                work.summary.word_comparison.record(comparison_start);
                            }
                        }
                        #[cfg(feature = "delivery-trace")]
                        let publish_start = Instant::now();
                        let published = input.publish(
                            generation,
                            Ok(Reply {
                                origin,
                                source_error: None,
                                #[cfg(feature = "delivery-trace")]
                                generation,
                                epoch: request.epoch,
                                id: LineId::Patch(id),
                                style,
                            }),
                            &output,
                        );
                        #[cfg(feature = "delivery-trace")]
                        {
                            work.summary.publication.record(publish_start);
                            work.summary.style_replies += usize::from(published);
                        }
                        if !published {
                            #[cfg(feature = "delivery-trace")]
                            {
                                work.summary.outcome = Outcome::PublicationStopped;
                            }
                            continue 'requests;
                        }
                    }
                }
            }
            #[cfg(feature = "delivery-trace")]
            {
                work.summary.outcome = Outcome::Complete;
            }
        }
    });
    Worker {
        requests,
        replies,
        thread: Some(thread),
    }
}

#[derive(Default)]
pub struct Cache {
    lines: hashbrown::HashMap<LineId, LineStyle, RandomState>,
    order: VecDeque<LineId>,
    visible: Vec<LineId>,
    bytes: usize,
    #[cfg(feature = "owner-census")]
    activity: cache_activity::Activity,
}

impl Cache {
    pub(crate) fn protect_visible(&mut self, mut ids: Vec<LineId>) {
        ids.sort_unstable();
        ids.dedup();
        if self.visible == ids {
            return;
        }
        #[cfg(feature = "owner-census")]
        let before = cache_activity::Point::observe(self);
        self.visible = ids;
        self.trim();
        #[cfg(feature = "owner-census")]
        self.activity
            .release(before, cache_activity::Point::observe(self));
    }

    fn trim(&mut self) {
        while self.bytes > 32 * 1024 * 1024 {
            let Some(index) = self
                .order
                .iter()
                .position(|id| self.visible.binary_search(id).is_err())
            else {
                break;
            };
            let id = self.order.remove(index).expect("Eviction index is missing");
            let style = self.lines.remove(&id).expect("Queued style is missing");
            self.bytes -= style.cache_weight();
        }
    }

    pub(crate) fn invalidate(&mut self, file: &FileIndex) {
        #[cfg(feature = "owner-census")]
        let before = cache_activity::Point::observe(self);
        self.order.retain(|id| !id.belongs_to(file));
        self.lines.retain(|id, style| {
            if !id.belongs_to(file) {
                return true;
            }
            self.bytes -= style.cache_weight();
            false
        });
        #[cfg(feature = "owner-census")]
        {
            let after = cache_activity::Point::observe(self);
            self.activity.invalidate(before, after);
        }
    }
    pub fn get(&self, id: impl Into<LineId>) -> Option<&LineStyle> {
        self.lines.get(&id.into())
    }
    pub fn insert(&mut self, id: impl Into<LineId>, style: LineStyle) {
        let id = id.into();
        if self.lines.contains_key(&id) {
            #[cfg(feature = "owner-census")]
            {
                self.activity.duplicates += 1;
            }
            return;
        }
        #[cfg(feature = "owner-census")]
        let before = cache_activity::Point::observe(self);
        self.bytes += style.cache_weight();
        self.lines.insert(id, style);
        self.order.push_back(id);
        #[cfg(feature = "owner-census")]
        let inserted = cache_activity::Point::observe(self);
        self.trim();
        #[cfg(feature = "owner-census")]
        {
            let settled = cache_activity::Point::observe(self);
            self.activity.admit(before, inserted, settled);
        }
    }
}

#[cfg(test)]
#[path = "highlight/cache_tests.rs"]
mod cache_tests;
