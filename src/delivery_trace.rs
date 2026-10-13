//! Bounded attribution only. This module is absent from normal builds.

use std::{
    fs::File,
    sync::{Mutex, OnceLock},
    time::Instant,
};

use serde::Serialize;
use syntect::parsing::syntax_timing::{Counts, Window};

const CAPACITY: usize = 16_384;
const REQUEST_CAPACITY: usize = 512;
static TRACE: OnceLock<Mutex<Trace>> = OnceLock::new();

#[derive(Serialize)]
struct Event {
    ns: u64,
    kind: &'static str,
    a: u64,
    b: EventValue,
}

#[derive(Serialize)]
#[serde(untagged)]
enum EventValue {
    Number(u64),
    Source(crate::view::LineId),
}

struct Trace {
    start: Instant,
    events: Vec<Event>,
    dropped: usize,
    requests: Vec<RequestSummary>,
    requests_dropped: usize,
}

#[derive(Default, Serialize)]
pub(crate) struct Timing {
    calls: u64,
    total_ns: u64,
    max_ns: u64,
}

impl Timing {
    pub(crate) fn record(&mut self, start: Instant) -> u64 {
        let ns = nanos(start.elapsed());
        self.record_ns(ns);
        ns
    }

    fn record_ns(&mut self, ns: u64) {
        self.calls += 1;
        self.total_ns = self.total_ns.saturating_add(ns);
        self.max_ns = self.max_ns.max(ns);
    }
}

fn nanos(duration: std::time::Duration) -> u64 {
    duration.as_nanos().min(u64::MAX as u128) as u64
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReplayReason {
    Missing,
    File,
    Hunk,
    Epoch,
    Backward,
    Resume,
    Checkpoint,
}

#[derive(Default, Serialize)]
struct ReplayCounts {
    missing: usize,
    file: usize,
    hunk: usize,
    epoch: usize,
    backward: usize,
    resume: usize,
    checkpoint: usize,
}

#[derive(Serialize)]
struct Prefix {
    file: usize,
    hunk: usize,
    start: usize,
    first_requested: usize,
    last_requested: usize,
    records: usize,
    reason: ReplayReason,
}

#[derive(Serialize)]
struct ParserCall {
    source: crate::view::LineId,
    old: bool,
    requested: bool,
    ns: u64,
    initialization: Counts,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Outcome {
    #[default]
    Incomplete,
    Complete,
    SupersededOrClosed,
    PublicationStopped,
    SourceError,
    ThemeError,
}

#[derive(Default, Serialize)]
pub(crate) struct RequestSummary {
    generation: u64,
    epoch: u64,
    finished_ns: u64,
    elapsed_ns: u64,
    pub(crate) outcome: Outcome,
    requested_raw: usize,
    pub(crate) requested_unique: Option<usize>,
    first_requested: Option<crate::view::LineId>,
    last_requested: Option<crate::view::LineId>,
    groups: ReplayCounts,
    largest_prefix: Option<Prefix>,
    pub(crate) processed_records: usize,
    pub(crate) complete_source_lines: usize,
    pub(crate) shared_source_lines: usize,
    pub(crate) plain_records: usize,
    pub(crate) requested_records: usize,
    pub(crate) predecessor_records: usize,
    pub(crate) other_replay_records: usize,
    pub(crate) fallback_requested: usize,
    pub(crate) replay_cancellation_checks: usize,
    pub(crate) style_replies: usize,
    pub(crate) theme: Timing,
    parse_requested: Timing,
    parse_replay: Timing,
    initialization: Counts,
    initialization_requested: Counts,
    initialization_replay: Counts,
    parse_requested_remainder: Timing,
    parse_replay_remainder: Timing,
    initialization_inconsistent: bool,
    longest_parser_call: Option<ParserCall>,
    pub(crate) visible_colors: Timing,
    pub(crate) replay_scopes: Timing,
    pub(crate) word_comparison: Timing,
    pub(crate) publication: Timing,
}

pub(crate) struct RequestWork {
    pub(crate) summary: RequestSummary,
    start: Instant,
    initialization: Option<Window>,
}

impl RequestWork {
    pub(crate) fn new(generation: u64, epoch: u64, ids: &[crate::view::LineId]) -> Self {
        let start = Instant::now();
        record("worker_start", generation, epoch);
        Self {
            summary: RequestSummary {
                generation,
                epoch,
                requested_raw: ids.len(),
                first_requested: ids.iter().copied().min(),
                last_requested: ids.iter().copied().max(),
                ..RequestSummary::default()
            },
            start,
            initialization: Some(Window::request()),
        }
    }

    pub(crate) fn group(
        &mut self,
        file: usize,
        hunk: usize,
        start: usize,
        requested: std::ops::RangeInclusive<usize>,
        reason: ReplayReason,
    ) {
        let counts = &mut self.summary.groups;
        match reason {
            ReplayReason::Missing => counts.missing += 1,
            ReplayReason::File => counts.file += 1,
            ReplayReason::Hunk => counts.hunk += 1,
            ReplayReason::Epoch => counts.epoch += 1,
            ReplayReason::Backward => counts.backward += 1,
            ReplayReason::Resume => counts.resume += 1,
            ReplayReason::Checkpoint => counts.checkpoint += 1,
        }
        let records = requested.start().saturating_sub(start);
        if self
            .summary
            .largest_prefix
            .as_ref()
            .is_none_or(|prefix| records > prefix.records)
        {
            self.summary.largest_prefix = Some(Prefix {
                file,
                hunk,
                start,
                first_requested: *requested.start(),
                last_requested: *requested.end(),
                records,
                reason,
            });
        }
    }

    pub(crate) fn parsed(
        &mut self,
        elapsed: std::time::Duration,
        requested: bool,
        source: crate::view::LineId,
        old: bool,
        initialization: Counts,
    ) {
        let timing = if requested {
            &mut self.summary.parse_requested
        } else {
            &mut self.summary.parse_replay
        };
        let ns = nanos(elapsed);
        timing.record_ns(ns);
        let (counts, remainder) = if requested {
            (
                &mut self.summary.initialization_requested,
                &mut self.summary.parse_requested_remainder,
            )
        } else {
            (
                &mut self.summary.initialization_replay,
                &mut self.summary.parse_replay_remainder,
            )
        };
        counts.merge(initialization);
        match initialization
            .grammar
            .total_ns
            .checked_add(initialization.outer_regex.total_ns)
            .and_then(|init| ns.checked_sub(init))
        {
            Some(ns) => remainder.record_ns(ns),
            None => self.summary.initialization_inconsistent = true,
        }
        if self
            .summary
            .longest_parser_call
            .as_ref()
            .is_none_or(|call| ns > call.ns)
        {
            self.summary.longest_parser_call = Some(ParserCall {
                source,
                old,
                requested,
                ns,
                initialization,
            });
        }
    }
}

impl Drop for RequestWork {
    fn drop(&mut self) {
        self.summary.initialization = self.initialization.take().unwrap().finish();
        let Some(trace) = TRACE.get() else {
            return;
        };
        self.summary.elapsed_ns = nanos(self.start.elapsed());
        let mut trace = trace.lock().unwrap();
        if trace.requests.len() == REQUEST_CAPACITY {
            trace.requests_dropped += 1;
            return;
        }
        self.summary.finished_ns = nanos(trace.start.elapsed());
        trace.requests.push(std::mem::take(&mut self.summary));
    }
}

pub struct Guard(Option<File>);

pub fn init() -> anyhow::Result<Guard> {
    let Some(path) = std::env::var_os("NATIVE_DIFF_DELIVERY_TRACE") else {
        return Ok(Guard(None));
    };
    let file = File::create_new(path)?;
    TRACE
        .set(Mutex::new(Trace {
            start: Instant::now(),
            events: Vec::with_capacity(CAPACITY),
            dropped: 0,
            requests: Vec::with_capacity(REQUEST_CAPACITY),
            requests_dropped: 0,
        }))
        .map_err(|_| anyhow::anyhow!("Delivery trace already initialized"))?;
    Ok(Guard(Some(file)))
}

pub fn record(kind: &'static str, a: u64, b: u64) {
    record_value(kind, a, EventValue::Number(b));
}

pub fn source_event(kind: &'static str, generation: u64, source: Option<crate::view::LineId>) {
    record_value(
        kind,
        generation,
        source.map_or(EventValue::Number(u64::MAX), EventValue::Source),
    );
}

fn record_value(kind: &'static str, a: u64, b: EventValue) {
    let Some(trace) = TRACE.get() else {
        return;
    };
    let mut trace = trace.lock().unwrap();
    if trace.events.len() == CAPACITY {
        trace.dropped += 1;
        return;
    }
    let ns = trace.start.elapsed().as_nanos() as u64;
    trace.events.push(Event { ns, kind, a, b });
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(file) = &mut self.0 {
            let trace = TRACE.get().unwrap().lock().unwrap();
            if let Err(error) = serde_json::to_writer(
                file,
                &serde_json::json!({
                    "schema": 6,
                    "scope": "Instrumented attribution, not accepted timing or terminal paint",
                    "capacity": CAPACITY, "dropped": trace.dropped, "events": trace.events,
                    "request_capacity": REQUEST_CAPACITY,
                    "requests_dropped": trace.requests_dropped, "requests": trace.requests,
                }),
            ) {
                eprintln!("Cannot write delivery trace: {error}");
            }
        }
    }
}
