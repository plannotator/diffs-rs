//! Fixed-size, thread-local attribution. No source or compiled owners are retained.

use std::{cell::Cell, marker::PhantomData, rc::Rc, time::Instant};

#[derive(Clone, Copy, Debug, Default, serde_derive::Serialize)]
pub struct Timing {
    pub calls: u64,
    pub total_ns: u64,
    pub max_ns: u64,
    pub failed: u64,
}

#[derive(Clone, Copy, Debug, Default, serde_derive::Serialize)]
pub struct Counts {
    pub grammar: Timing,
    pub outer_regex: Timing,
    pub source_patterns: u64,
    pub invalid: bool,
}

fn add(left: u64, right: u64, invalid: &mut bool) -> u64 {
    left.checked_add(right).unwrap_or_else(|| {
        *invalid = true;
        u64::MAX
    })
}

impl Timing {
    fn merge(&mut self, other: Self, invalid: &mut bool) {
        self.calls = add(self.calls, other.calls, invalid);
        self.total_ns = add(self.total_ns, other.total_ns, invalid);
        self.max_ns = self.max_ns.max(other.max_ns);
        self.failed = add(self.failed, other.failed, invalid);
    }
}

impl Counts {
    pub fn merge(&mut self, other: Self) {
        self.invalid |= other.invalid;
        self.grammar.merge(other.grammar, &mut self.invalid);
        self.outer_regex.merge(other.outer_regex, &mut self.invalid);
        self.source_patterns = add(
            self.source_patterns,
            other.source_patterns,
            &mut self.invalid,
        );
    }
}

#[derive(Clone, Copy, Default)]
struct Recorder {
    windows: [Option<Counts>; 2],
    initializer_depth: usize,
}

thread_local! {
    static RECORDER: Cell<Recorder> = Cell::new(Recorder::default());
}

fn update(f: impl FnOnce(&mut Recorder)) {
    RECORDER.with(|cell| {
        let mut recorder = cell.get();
        f(&mut recorder);
        cell.set(recorder);
    });
}

/// An interval on the calling thread. Request and parser intervals can overlap.
pub struct Window {
    slot: usize,
    active: bool,
    thread: PhantomData<Rc<()>>,
}

impl Window {
    pub fn request() -> Self {
        Self::start(0)
    }

    pub fn parser_call() -> Self {
        Self::start(1)
    }

    fn start(slot: usize) -> Self {
        let mut active = false;
        update(|recorder| {
            if let Some(counts) = &mut recorder.windows[slot] {
                counts.invalid = true;
            } else {
                recorder.windows[slot] = Some(Counts {
                    invalid: recorder.initializer_depth != 0,
                    ..Counts::default()
                });
                active = true;
            }
        });
        Self {
            slot,
            active,
            thread: PhantomData,
        }
    }

    pub fn finish(mut self) -> Counts {
        let mut counts = Counts {
            invalid: true,
            ..Counts::default()
        };
        if self.active {
            update(|recorder| {
                if let Some(value) = recorder.windows[self.slot].take() {
                    counts = value;
                    counts.invalid |= recorder.initializer_depth != 0
                        || (self.slot == 0 && recorder.windows[1].is_some());
                }
            });
            self.active = false;
        }
        counts
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        if self.active {
            update(|recorder| {
                recorder.windows[self.slot] = None;
                for counts in recorder.windows.iter_mut().flatten() {
                    counts.invalid = true;
                }
            });
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Phase {
    Grammar,
    OuterRegex,
}

pub(crate) struct Initializer {
    start: Instant,
    phase: Phase,
    thread: PhantomData<Rc<()>>,
}

impl Initializer {
    pub(crate) fn new(phase: Phase) -> Self {
        update(|recorder| {
            if recorder.initializer_depth != 0 {
                for counts in recorder.windows.iter_mut().flatten() {
                    counts.invalid = true;
                }
            }
            recorder.initializer_depth += 1;
        });
        Self {
            start: Instant::now(),
            phase,
            thread: PhantomData,
        }
    }
}

impl Drop for Initializer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed().as_nanos();
        let ns = elapsed.min(u64::MAX as u128) as u64;
        let failed = u64::from(std::thread::panicking());
        update(|recorder| {
            recorder.initializer_depth -= 1;
            for counts in recorder.windows.iter_mut().flatten() {
                counts.invalid |= elapsed > u64::MAX as u128;
                let timing = match self.phase {
                    Phase::Grammar => &mut counts.grammar,
                    Phase::OuterRegex => &mut counts.outer_regex,
                };
                timing.merge(
                    Timing {
                        calls: 1,
                        total_ns: ns,
                        max_ns: ns,
                        failed,
                    },
                    &mut counts.invalid,
                );
            }
        });
    }
}

pub(crate) fn source_pattern() {
    update(|recorder| {
        for counts in recorder.windows.iter_mut().flatten() {
            counts.source_patterns = add(counts.source_patterns, 1, &mut counts.invalid);
        }
    });
}
