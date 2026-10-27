use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        mpsc::{self, SyncSender},
    },
    thread::{self, JoinHandle},
};

use anyhow::{Context, Result, ensure};
use diffs_rs::{
    engine::ReviewId,
    input::Cancellation,
    reading::{SearchDirection, SearchMatch, SearchRead, SourceRead},
};

use crate::{
    clipboard,
    reading_export::{CleanupError, Prepared},
};

pub fn cleanup_failed(error: &anyhow::Error) -> bool {
    error.is::<CleanupError>() || error.is::<clipboard::CleanupError>()
}

enum Job {
    Search {
        read: Arc<SearchRead>,
        previous: Option<SearchMatch>,
        direction: SearchDirection,
    },
    Save {
        read: Arc<SourceRead>,
        destination: PathBuf,
    },
    Copy(Arc<SourceRead>),
    Deliver(clipboard::Delivery),
}

pub enum Output {
    Search(Option<SearchMatch>),
    Save(Prepared),
    Copy(clipboard::Prepared),
    Copied(clipboard::Receipt),
}

impl Output {
    fn discard(self) -> Result<()> {
        match self {
            Self::Search(_) | Self::Copy(_) | Self::Copied(_) => Ok(()),
            Self::Save(prepared) => prepared.discard(),
        }
    }
}

struct Request {
    id: u64,
    job: Job,
    cancel: Cancellation,
}

pub struct Reply {
    pub id: u64,
    pub result: Result<Output>,
}

impl Reply {
    pub fn discard(self) -> Result<()> {
        match self.result {
            Ok(output) => output.discard(),
            Err(error) if cleanup_failed(&error) => Err(error),
            Err(_) => Ok(()),
        }
    }
}

#[derive(Default)]
struct State {
    generation: u64,
    pending: Option<Request>,
    delivery: Option<Request>,
    reply: Option<Reply>,
    #[cfg(test)]
    checkpoint: Option<tests::Checkpoint>,
}

pub struct Worker {
    state: Arc<Mutex<State>>,
    wake: SyncSender<()>,
    stop: Cancellation,
    current: Option<Cancellation>,
    join: Option<JoinHandle<Result<()>>>,
}

impl Worker {
    #[cfg(test)]
    pub(crate) fn pause_next(&mut self) -> (mpsc::Receiver<()>, SyncSender<()>) {
        let (checkpoint, entered, resume) = tests::Checkpoint::new();
        self.state.lock().unwrap().checkpoint = Some(checkpoint);
        (entered, resume)
    }

    pub fn start(notify: SyncSender<()>) -> Result<Self> {
        let state = Arc::new(Mutex::new(State::default()));
        let stop = Cancellation::default();
        let shared = state.clone();
        let stopped = stop.clone();
        let (wake, requests) = mpsc::sync_channel(1);
        let join = thread::Builder::new()
            .name("diff-reading".into())
            .spawn(move || {
                let result = (|| -> Result<()> {
                    loop {
                        let request = {
                            let mut state = shared.lock().map_err(|_| {
                                anyhow::anyhow!("Reading worker state was poisoned")
                            })?;
                            state.delivery.take().or_else(|| {
                                if stopped.check().is_ok() {
                                    state.pending.take()
                                } else {
                                    None
                                }
                            })
                        };
                        let Some(request) = request else {
                            if stopped.check().is_err() || requests.recv().is_err() {
                                break;
                            }
                            continue;
                        };
                        #[cfg(test)]
                        let checkpoint = shared
                            .lock()
                            .map_err(|_| anyhow::anyhow!("Reading worker state was poisoned"))?
                            .checkpoint
                            .take();
                        let check = || {
                            #[cfg(test)]
                            if let Some(checkpoint) = &checkpoint {
                                checkpoint.wait()?;
                            }
                            stopped.check()?;
                            request.cancel.check()
                        };
                        let result = match request.job {
                            Job::Search {
                                read,
                                previous,
                                direction,
                            } => read
                                .find(previous.as_ref(), direction, &check)
                                .map(Output::Search),
                            Job::Save { read, destination } => {
                                Prepared::prepare(read, destination, &check).map(Output::Save)
                            }
                            Job::Copy(read) => {
                                clipboard::Prepared::prepare(read, &check).map(Output::Copy)
                            }
                            Job::Deliver(delivery) => delivery.finish(&check).map(Output::Copied),
                        };
                        let result = match result {
                            Err(error) if cleanup_failed(&error) => return Err(error),
                            result => result,
                        };
                        let reply = Reply {
                            id: request.id,
                            result,
                        };
                        let (published, displaced) = {
                            let mut state = match shared.lock() {
                                Ok(state) => state,
                                Err(_) => {
                                    return crate::cleanup::finish(
                                        Err(anyhow::anyhow!("Reading worker state was poisoned")),
                                        reply.discard(),
                                    );
                                }
                            };
                            if state.generation == request.id
                                && stopped.check().is_ok()
                                && request.cancel.check().is_ok()
                            {
                                (true, state.reply.replace(reply))
                            } else {
                                (false, Some(reply))
                            }
                        };
                        if let Some(reply) = displaced {
                            reply.discard()?;
                        }
                        if published {
                            let _ = notify.try_send(());
                        }
                    }
                    Ok(())
                })();
                let _ = notify.try_send(());
                result
            })
            .context("Cannot start the reading worker")?;
        Ok(Self {
            state,
            wake,
            stop,
            current: None,
            join: Some(join),
        })
    }

    pub fn submit(
        &mut self,
        read: Arc<SearchRead>,
        previous: Option<SearchMatch>,
        direction: SearchDirection,
    ) -> Result<u64> {
        self.queue(Job::Search {
            read,
            previous,
            direction,
        })
    }

    pub fn save(&mut self, read: Arc<SourceRead>, destination: PathBuf) -> Result<u64> {
        self.queue(Job::Save { read, destination })
    }

    pub fn copy(&mut self, read: Arc<SourceRead>) -> Result<u64> {
        self.queue(Job::Copy(read))
    }

    pub fn deliver(
        &mut self,
        id: u64,
        prepared: clipboard::Prepared,
        review: ReviewId,
        command: std::process::Command,
    ) -> Result<()> {
        ensure!(
            self.join.as_ref().is_some_and(|join| !join.is_finished()),
            "The reading worker is closed"
        );
        self.stop.check()?;
        let cancel = self
            .current
            .as_ref()
            .context("Copy was canceled before delivery")?;
        cancel.check()?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Reading worker state was poisoned"))?;
        ensure!(
            state.generation == id && state.delivery.is_none() && state.pending.is_none(),
            "Copy was superseded before delivery"
        );
        let delivery = prepared.start(review, command)?;
        // Accepted delivery owns a child. Cancellation leaves it for the worker to reap.
        state.delivery = Some(Request {
            id,
            job: Job::Deliver(delivery),
            cancel: cancel.clone(),
        });
        drop(state);
        let _ = self.wake.try_send(());
        Ok(())
    }

    fn queue(&mut self, job: Job) -> Result<u64> {
        ensure!(
            self.join.as_ref().is_some_and(|join| !join.is_finished()),
            "The reading worker is closed"
        );
        let cancel = Cancellation::default();
        let (id, displaced, reply) = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow::anyhow!("Reading worker state was poisoned"))?;
            let id = state
                .generation
                .checked_add(1)
                .context("Reading request identities exhausted")?;
            if let Some(current) = self.current.replace(cancel.clone()) {
                current.stop();
            }
            state.generation = id;
            let displaced = state.pending.replace(Request { id, job, cancel });
            (id, displaced, state.reply.take())
        };
        drop(displaced);
        if let Some(reply) = reply {
            reply.discard()?;
        }
        let _ = self.wake.try_send(());
        Ok(id)
    }

    pub fn cancel(&mut self) -> Result<()> {
        if let Some(cancel) = self.current.take() {
            cancel.stop();
        }
        let discarded = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow::anyhow!("Reading worker state was poisoned"))?;
            (state.pending.take(), state.reply.take())
        };
        drop(discarded.0);
        if let Some(reply) = discarded.1 {
            reply.discard()?;
        }
        let _ = self.wake.try_send(());
        Ok(())
    }

    pub fn poll(&self) -> Result<Option<Reply>> {
        let reply = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Reading worker state was poisoned"))?
            .reply
            .take();
        if reply.is_none() && self.join.as_ref().is_some_and(|join| join.is_finished()) {
            anyhow::bail!("The reading worker stopped unexpectedly");
        }
        Ok(reply)
    }

    pub fn close(&mut self) -> Result<()> {
        self.stop.stop();
        let canceled = self.cancel();
        let _ = self.wake.try_send(());
        let joined = self.join.take().map_or(Ok(()), |join| {
            join.join()
                .map_err(|_| anyhow::anyhow!("The reading worker panicked"))
                .and_then(|result| result)
        });
        crate::cleanup::finish(canceled, joined)
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests;
