use std::io::{BufRead, BufReader};

use anyhow::{Context, Result};

use super::*;
use crate::{
    source::{CheckedPair, Snapshot},
    storage::StoreReader,
};

#[cfg(test)]
mod tests;

#[derive(Debug)]
struct Stopped;

impl std::fmt::Display for Stopped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Coloring request superseded or closed")
    }
}
impl std::error::Error for Stopped {}

pub(super) struct State {
    pub(super) identity: SyntaxIdentity,
    pub(super) next: usize,
    pub(super) side: Side,
}

#[derive(Default)]
pub(super) struct Replay {
    old: Option<State>,
    new: Option<State>,
}

#[cfg(feature = "owner-census")]
impl Replay {
    pub(super) fn census(&self) -> serde_json::Value {
        let observe = |state: &State| {
            serde_json::json!({
                "next": state.next, "side": census::side(&state.side),
            })
        };
        serde_json::json!({
            "inline_size_bytes": size_of::<Self>(),
            "old": self.old.as_ref().map(observe), "new": self.new.as_ref().map(observe),
        })
    }
}

pub(super) struct Job<'a, 'theme> {
    pub(super) input: &'a Mailbox,
    pub(super) output: &'a mpsc::SyncSender<Result<Reply>>,
    pub(super) generation: u64,
    pub(super) request: &'a Request<LineId>,
    pub(super) source: &'a FileSource,
    pub(super) wanted: &'a [LineId],
    pub(super) syntaxes: &'a SyntaxSet,
    pub(super) highlighter: &'a Highlighter<'theme>,
    pub(super) checkpoints: &'a mut Checkpoints,
    #[cfg(feature = "delivery-trace")]
    pub(super) work: &'a mut RequestWork,
}

impl Job<'_, '_> {
    fn identity(&self, old: bool) -> SyntaxIdentity {
        SyntaxIdentity {
            origin: self.source.origin(),
            file: self.source.file().id,
            scope: SyntaxScope::SourceSide { old },
            epoch: self.request.epoch,
            appearance: self.request.appearance,
        }
    }

    fn current(&self) -> Result<()> {
        if self.input.current(self.generation) {
            Ok(())
        } else {
            Err(Stopped.into())
        }
    }

    fn publish(
        &mut self,
        id: LineId,
        style: LineStyle,
        source_error: Option<Arc<str>>,
    ) -> Result<()> {
        #[cfg(feature = "delivery-trace")]
        let started = Instant::now();
        #[cfg(feature = "delivery-trace")]
        {
            self.work.summary.requested_records += 1;
            self.work.summary.fallback_requested += usize::from(style.fallback);
        }
        let published = self.input.publish(
            self.generation,
            Ok(Reply {
                origin: self.source.origin(),
                source_error,
                #[cfg(feature = "delivery-trace")]
                generation: self.generation,
                epoch: self.request.epoch,
                id,
                style,
            }),
            self.output,
        );
        #[cfg(feature = "delivery-trace")]
        {
            self.work.summary.publication.record(started);
            self.work.summary.style_replies += usize::from(published);
        }
        if published {
            Ok(())
        } else {
            Err(Stopped.into())
        }
    }
}

impl Replay {
    pub(super) fn run(&mut self, mut job: Job<'_, '_>) -> Result<bool> {
        match self.run_inner(&mut job) {
            Ok(()) => Ok(true),
            Err(error) if error.is::<Stopped>() => {
                #[cfg(feature = "delivery-trace")]
                {
                    job.work.summary.outcome = Outcome::SupersededOrClosed;
                }
                Ok(false)
            }
            Err(error) if error.is::<crate::source_spans::ReadFailure>() => Err(error),
            Err(error) if matches!(job.source, FileSource::Complete(_)) => {
                if let Some(&id) = job.wanted.first() {
                    let reason = crate::text::safe_text(format!("{error:#}").as_bytes());
                    // A stale failure is discarded by the same mailbox and origin checks as a style.
                    let _ = job.publish(id, LineStyle::default(), Some(reason.into()));
                }
                #[cfg(feature = "delivery-trace")]
                {
                    job.work.summary.outcome = Outcome::SourceError;
                }
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    fn run_inner(&mut self, job: &mut Job<'_, '_>) -> Result<()> {
        let file = job.source.file();
        let FileSource::Complete(pair) = job.source else {
            *self = Self::default();
            for &id in job.wanted {
                job.current()?;
                #[cfg(feature = "delivery-trace")]
                {
                    job.work.summary.plain_records += 1;
                }
                job.publish(
                    id,
                    LineStyle {
                        changed: id
                            .patch()
                            .map(|id| emphasis(file, id))
                            .transpose()?
                            .unwrap_or_default(),
                        ..LineStyle::default()
                    },
                    None,
                )?;
            }
            return Ok(());
        };
        let mut wanted = job
            .wanted
            .iter()
            .map(|&id| Ok((id, job.source.info(id)?)))
            .collect::<Result<Vec<_>>>()?;
        wanted.sort_unstable_by(|(a, left), (b, right)| match (a.patch(), b.patch()) {
            (Some(a), Some(b)) => a.cmp(&b),
            _ => left
                .old
                .zip(right.old)
                .or_else(|| left.new.zip(right.new))
                .map_or(std::cmp::Ordering::Equal, |(a, b)| a.cmp(&b)),
        });
        for (_, info) in &wanted {
            job.current()?;
            for (old, number) in [(true, info.old), (false, info.new)] {
                if let Some(number) = number {
                    let identity = job.identity(old);
                    job.checkpoints.protect(identity, number as usize - 1);
                }
            }
        }
        let mut old = Reader::prepare(
            &mut self.old,
            pair.old(),
            true,
            wanted.iter().filter_map(|(_, info)| info.old).min(),
            job,
        )?;
        let mut new = Reader::prepare(
            &mut self.new,
            pair.new_source(),
            false,
            wanted.iter().filter_map(|(_, info)| info.new).min(),
            job,
        )?;
        let same_syntax = syntax_for_path(&file.old_path, job.syntaxes)
            .zip(syntax_for_path(&file.path, job.syntaxes))
            .is_some_and(|(old, new)| std::ptr::eq(old, new));
        for (id, info) in wanted {
            job.current()?;
            if same_syntax
                && let (Some(old), Some(new), Some(a), Some(b)) =
                    (&mut old, &mut new, info.old, info.new)
            {
                advance_pair(old, new, pair, (a as usize - 1, b as usize - 1), id, job)?;
            }
            let mut style = LineStyle::default();
            let mut side_tokens = |reader: &mut Option<Reader<'_>>,
                                   number: Option<u32>,
                                   old_side|
             -> Result<Vec<Token>> {
                let Some(number) = number else {
                    return Ok(Vec::new());
                };
                let reader = reader.as_mut().context("Checked source side is missing")?;
                let result = reader.advance(number, id, old_side, job)?;
                if let Some(reason) = reader.state.side.fallback {
                    style.fallback = true;
                    style.fallback_reason.get_or_insert(reason);
                }
                Ok(result)
            };
            let old_tokens = side_tokens(&mut old, info.old, true)?;
            let new_tokens = side_tokens(&mut new, info.new, false)?;
            if info.kind == Kind::Deletion {
                style.tokens = old_tokens;
            } else {
                style.tokens = new_tokens;
                if info.kind == Kind::Context && old_tokens != style.tokens {
                    style.old_tokens = Some(old_tokens);
                }
            }
            style.changed = id
                .patch()
                .map(|id| emphasis(file, id))
                .transpose()?
                .unwrap_or_default();
            job.publish(id, style, None)?;
        }
        Ok(())
    }
}

struct Reader<'a> {
    state: &'a mut State,
    source: &'a Snapshot,
    input: BufReader<StoreReader>,
    bytes: Vec<u8>,
}

fn advance_pair(
    old: &mut Reader<'_>,
    new: &mut Reader<'_>,
    pair: &CheckedPair,
    ends: (usize, usize),
    id: LineId,
    job: &mut Job<'_, '_>,
) -> Result<()> {
    old.restore_before(ends.0, job)?;
    new.restore_before(ends.1, job)?;
    while old.state.next < ends.0
        && new.state.next < ends.1
        && old.state.side.fallback.is_none()
        && new.state.side.fallback.is_none()
    {
        job.current()?;
        let Some((a, b)) = pair.shared_gap_after(old.state.next, new.state.next) else {
            break;
        };
        let len = a
            .len()
            .min(ends.0.saturating_sub(a.start))
            .min(ends.1.saturating_sub(b.start));
        if len == 0 {
            break;
        }
        old.advance_until(a.start, false, id, true, job)?;
        new.advance_until(b.start, false, id, false, job)?;
        let len = len
            .min(checkpoint::INTERVAL - a.start % checkpoint::INTERVAL)
            .min(checkpoint::INTERVAL - b.start % checkpoint::INTERVAL);
        if old.state.side.fallback.is_none()
            && new.state.side.fallback.is_none()
            && old.state.side.parse == new.state.side.parse
            && old.state.side.highlight.path == new.state.side.highlight.path
        {
            // Commit the new-side continuation only after the chunk succeeds.
            // On error the readers are dropped, leaving both saved histories valid.
            let mut processed = 0;
            for _ in 0..len {
                let a_large = old.read_line(job)?;
                let b_large = new.read_line(job)?;
                anyhow::ensure!(
                    a_large == b_large && (a_large || old.bytes == new.bytes),
                    "Checked source bytes differ during syntax replay"
                );
                old.process_line(a_large, false, id, true, job)?;
                processed += 1;
                if old.state.side.fallback.is_some() {
                    break;
                }
            }
            new.state.side.parse.clone_from(&old.state.side.parse);
            new.state
                .side
                .highlight
                .path
                .clone_from(&old.state.side.highlight.path);
            new.state.side.needs_colors = true;
            new.state.side.fallback = old.state.side.fallback;
            new.state.next += processed;
            #[cfg(feature = "delivery-trace")]
            {
                job.work.summary.complete_source_lines += processed;
                job.work.summary.shared_source_lines += processed;
            }
            if !job
                .checkpoints
                .capture_source(new.state, || job.input.current(job.generation))
            {
                return Err(Stopped.into());
            }
        } else {
            old.advance_until(a.start + len, false, id, true, job)?;
            new.advance_until(b.start + len, false, id, false, job)?;
        }
    }
    Ok(())
}

impl<'a> Reader<'a> {
    fn prepare(
        slot: &'a mut Option<State>,
        source: Option<&'a Arc<Snapshot>>,
        old: bool,
        first: Option<u32>,
        job: &Job<'_, '_>,
    ) -> Result<Option<Self>> {
        let file = job.source.file();
        let Some(first) = first else {
            return Ok(None);
        };
        job.current()?;
        let source = source.context("Checked source side is missing")?;
        let first = first as usize - 1;
        let identity = job.identity(old);
        let active = slot
            .as_ref()
            .filter(|s| s.identity == identity && s.next <= first)
            .map(|s| s.next);
        if let Some(saved) = job.checkpoints.nearest(identity, first)
            && active.is_none_or(|next| saved.next > next)
        {
            *slot = saved.restore_source(identity, job.highlighter);
        } else if active.is_none() {
            *slot = None;
        }
        let state = slot.get_or_insert_with(|| {
            let path = if old { &file.old_path } else { &file.path };
            let known = syntax_for_path(path, job.syntaxes);
            let syntax = known.unwrap_or_else(|| job.syntaxes.find_syntax_plain_text());
            let mut side = Side::new(syntax, job.highlighter);
            if known.is_none() {
                side.fallback = Some(Fallback::UnsupportedLanguage);
            }
            State {
                identity,
                next: 0,
                side,
            }
        });
        job.current()?;
        let input = source.uncached_range(u32::try_from(state.next)?..source.lines())?;
        Ok(Some(Self {
            state,
            source,
            input: BufReader::with_capacity(64 * 1024, input),
            bytes: Vec::new(),
        }))
    }

    fn advance(
        &mut self,
        number: u32,
        _id: LineId,
        _old: bool,
        job: &mut Job<'_, '_>,
    ) -> Result<Vec<Token>> {
        let target = number as usize;
        anyhow::ensure!(
            self.state.next < target,
            "Source coloring request moved backward within a side"
        );
        self.advance_until(target, true, _id, _old, job)
    }

    fn restore_before(&mut self, target: usize, job: &Job<'_, '_>) -> Result<()> {
        if self.state.next < target {
            job.current()?;
            if let Some(saved) = job.checkpoints.nearest(self.state.identity, target)
                && saved.next > self.state.next
            {
                let restored = saved
                    .restore_source(self.state.identity, job.highlighter)
                    .context("Incompatible source checkpoint")?;
                job.current()?;
                let input = self
                    .source
                    .uncached_range(u32::try_from(restored.next)?..self.source.lines())?;
                job.current()?;
                *self.state = restored;
                self.input = BufReader::with_capacity(64 * 1024, input);
                self.bytes.clear();
            }
        }
        Ok(())
    }

    fn advance_until(
        &mut self,
        target: usize,
        visible: bool,
        id: LineId,
        old: bool,
        job: &mut Job<'_, '_>,
    ) -> Result<Vec<Token>> {
        self.restore_before(target.saturating_sub(usize::from(visible)), job)?;
        while self.state.next < target {
            job.current()?;
            if self.state.side.fallback.is_some() {
                self.state.next = target;
                return Ok(Vec::new());
            }
            let oversized = self.read_line(job)?;
            let visible = visible && self.state.next + 1 == target;
            let colored = self.process_line(oversized, visible, id, old, job)?;
            if visible {
                return Ok(colored);
            }
        }
        Ok(Vec::new())
    }

    fn process_line(
        &mut self,
        oversized: bool,
        visible: bool,
        _id: LineId,
        _old: bool,
        job: &mut Job<'_, '_>,
    ) -> Result<Vec<Token>> {
        let mut text = if oversized {
            String::new()
        } else {
            display_text(&self.bytes)
        };
        text.push('\n');
        let colored = if oversized || text.len() > 40_004 || text.chars().count() > 10_001 {
            self.state.side.fallback = Some(Fallback::LongLine);
            None
        } else {
            self.state.side.advance(
                &text,
                job.syntaxes,
                job.highlighter,
                visible,
                #[cfg(feature = "delivery-trace")]
                job.work,
                #[cfg(feature = "delivery-trace")]
                (_id, _old),
            )
        };
        self.state.next += 1;
        #[cfg(feature = "delivery-trace")]
        {
            job.work.summary.complete_source_lines += 1;
        }
        if !job
            .checkpoints
            .capture_source(self.state, || job.input.current(job.generation))
        {
            return Err(Stopped.into());
        }
        Ok(colored.unwrap_or_default())
    }

    fn read_line(&mut self, job: &Job<'_, '_>) -> Result<bool> {
        self.bytes.clear();
        let mut oversized = false;
        loop {
            job.current()?;
            let chunk = self.input.fill_buf()?;
            if chunk.is_empty() {
                return Ok(oversized);
            }
            let newline = chunk.iter().position(|byte| *byte == b'\n');
            let count = newline.map_or(chunk.len(), |n| n + 1);
            if self.bytes.len() + count > 40_005 {
                oversized = true;
            }
            if !oversized {
                self.bytes.extend_from_slice(&chunk[..count]);
            }
            self.input.consume(count);
            if newline.is_some() {
                return Ok(oversized);
            }
        }
    }
}

fn emphasis(file: &FileIndex, id: usize) -> Result<Vec<Range<usize>>> {
    let Some(partner) = file.partner(id) else {
        return Ok(Vec::new());
    };
    let text = display_text(&file.bytes(id)?);
    let other = display_text(&file.bytes(partner)?);
    if text.len() > 40_000
        || other.len() > 40_000
        || text.chars().count() > 10_000
        || other.chars().count() > 10_000
    {
        return Ok(Vec::new());
    }
    Ok(if file.info(id).kind == Kind::Deletion {
        changed_spans(&text, &other, ChangeSide::Old)
    } else {
        changed_spans(&other, &text, ChangeSide::New)
    })
}
