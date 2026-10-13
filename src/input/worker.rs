use super::*;
use std::{
    sync::{
        atomic::Ordering,
        mpsc::{self, Receiver, SyncSender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub enum Message {
    File(Arc<FileIndex>),
    Complete { files: usize },
    Source { file: usize, state: SourceState },
}

pub struct Worker {
    cancel: Cancellation,
    destination: Arc<AtomicUsize>,
    wake: SyncSender<()>,
    replies: Receiver<Result<Message>>,
    join: Option<JoinHandle<Result<()>>>,
}

enum Input {
    Local(LocalInput),
    Patch(PatchInput),
}

impl Worker {
    pub fn start(input: LocalInput, notify: SyncSender<()>) -> Self {
        Self::spawn(Input::Local(input), notify)
    }

    pub fn start_patch(input: PatchInput, notify: SyncSender<()>) -> Self {
        Self::spawn(Input::Patch(input), notify)
    }

    fn spawn(input: Input, notify: SyncSender<()>) -> Self {
        let cancel = Cancellation::default();
        let destination = Arc::new(AtomicUsize::new(usize::MAX));
        let (wake, requests) = mpsc::sync_channel(1);
        let (send, replies) = mpsc::sync_channel(2);
        let stop = cancel.clone();
        let wanted = destination.clone();
        let join = thread::spawn(move || {
            let _notify_exit = crate::wake::NotifyOnExit(notify.clone());
            let send = send;
            let deliver = |mut value| -> Result<()> {
                loop {
                    stop.check()?;
                    match send.try_send(value) {
                        Ok(()) => {
                            let _ = notify.try_send(());
                            return Ok(());
                        }
                        Err(mpsc::TrySendError::Disconnected(_)) => return Err(Cancelled.into()),
                        Err(mpsc::TrySendError::Full(v)) => value = v,
                    }
                    thread::sleep(Duration::from_millis(5));
                }
            };
            let result = (|| -> Result<()> {
                let input = match input {
                    Input::Local(input) => input,
                    Input::Patch(input) => {
                        let files =
                            patch::read(input, &stop, |file| deliver(Ok(Message::File(file))))?;
                        return deliver(Ok(Message::Complete { files }));
                    }
                };
                let mut review =
                    Review::open(input, &stop, |file| deliver(Ok(Message::File(file))))?;
                deliver(Ok(Message::Complete {
                    files: review.doc.files.len(),
                }))?;
                loop {
                    stop.check()?;
                    match requests.recv_timeout(Duration::from_millis(50)) {
                        Ok(()) => {}
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
                    }
                    let id = wanted.load(Ordering::Acquire);
                    if id >= review.doc.files.len() {
                        continue;
                    }
                    let request = Cancellation {
                        stopped: stop.stopped.clone(),
                        destination: Some((wanted.clone(), id)),
                    };
                    match review.prepare(id, &request) {
                        Ok(state) => deliver(Ok(Message::Source { file: id, state }))?,
                        Err(error) if error.is::<Cancelled>() => {}
                        Err(error) => return Err(error),
                    }
                }
            })();
            if let Err(error) = &result
                && !error.is::<Cancelled>()
            {
                let _ = deliver(Err(anyhow::anyhow!("{error:#}")));
            }
            match result {
                Err(error) if error.is::<patch::RestoreFailure>() => Err(error),
                _ => Ok(()),
            }
        });
        Self {
            cancel,
            destination,
            wake,
            replies,
            join: Some(join),
        }
    }

    pub fn request(&self, file: usize) {
        if self.destination.swap(file, Ordering::AcqRel) != file {
            let _ = self.wake.try_send(());
        }
    }
    pub fn try_reply(&self) -> std::result::Result<Result<Message>, mpsc::TryRecvError> {
        self.replies.try_recv()
    }

    /// Cancel and join. Input errors arrive through replies; this reports cleanup failure or panic.
    pub fn close(&mut self) -> Result<()> {
        self.cancel.stop();
        let _ = self.wake.try_send(());
        let result = self.join.take().map_or(Ok(()), |join| {
            join.join()
                .map_err(|_| anyhow::anyhow!("The source worker panicked"))
                .and_then(|result| match result {
                    Err(error) if error.is::<Cancelled>() => Ok(()),
                    result => result,
                })
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
