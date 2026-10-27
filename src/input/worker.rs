use super::*;
use std::{
    collections::VecDeque,
    sync::{
        Condvar, Mutex, MutexGuard,
        atomic::Ordering,
        mpsc::{self, SyncSender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub enum Message {
    File(Arc<FileIndex>),
    Complete { files: usize },
    Source { file: usize, state: SourceState },
}

/// Published files waiting for the screen are bounded by count and index bytes. A full
/// queue pauses ingestion until the screen drains it; it never delays keyboard input.
/// The count also bounds the screen's per-turn drain.
const QUEUED_FILES: usize = 1024;
const QUEUED_FILE_BYTES: usize = 8 << 20;
/// Prepared sources retain both complete sides, so only two may wait.
const QUEUED_SOURCES: usize = 2;

#[derive(Default)]
struct Queue {
    replies: VecDeque<Result<Message>>,
    files: usize,
    file_bytes: usize,
    sources: usize,
    /// The worker was closed; a waiting producer must stop.
    closed: bool,
    /// The producer exited; nothing more will arrive.
    finished: bool,
}

#[derive(Clone, Copy)]
enum Cost {
    File(usize),
    Source,
    Final,
}

impl Cost {
    fn of(reply: &Result<Message>) -> Self {
        match reply {
            Ok(Message::File(file)) => Self::File(
                size_of::<FileIndex>()
                    + file.source_index_bytes()
                    + file.projection_bytes()
                    + file.metadata_bytes(),
            ),
            Ok(Message::Source { .. }) => Self::Source,
            Ok(Message::Complete { .. }) | Err(_) => Self::Final,
        }
    }
}

impl Queue {
    fn admits(&self, cost: Cost) -> bool {
        match cost {
            // An oversized file still enters an empty queue; it is accounted for in peak memory.
            Cost::File(bytes) => {
                self.files == 0
                    || (self.files < QUEUED_FILES
                        && self.file_bytes.saturating_add(bytes) <= QUEUED_FILE_BYTES)
            }
            Cost::Source => self.sources < QUEUED_SOURCES,
            // At most one completion and one error are ever published.
            Cost::Final => true,
        }
    }

    fn add(&mut self, cost: Cost) {
        match cost {
            Cost::File(bytes) => {
                self.files += 1;
                self.file_bytes = self.file_bytes.saturating_add(bytes);
            }
            Cost::Source => self.sources += 1,
            Cost::Final => {}
        }
    }

    fn remove(&mut self, cost: Cost) {
        match cost {
            Cost::File(bytes) => {
                self.files = self.files.saturating_sub(1);
                self.file_bytes = self.file_bytes.saturating_sub(bytes);
            }
            Cost::Source => self.sources = self.sources.saturating_sub(1),
            Cost::Final => {}
        }
    }
}

/// Single-producer handoff from the input thread to the screen. The producer blocks on a
/// condition variable when the queue is full; the screen takes everything queued at once.
#[derive(Default)]
struct Mailbox {
    queue: Mutex<Queue>,
    space: Condvar,
}

impl Mailbox {
    fn lock(&self) -> Result<MutexGuard<'_, Queue>> {
        self.queue
            .lock()
            .map_err(|_| anyhow::anyhow!("The input queue lock failed"))
    }

    fn deliver(&self, reply: Result<Message>, stop: &Cancellation) -> Result<()> {
        let cost = Cost::of(&reply);
        let mut queue = self.lock()?;
        loop {
            if queue.closed {
                return Err(Cancelled.into());
            }
            stop.check()?;
            if queue.admits(cost) {
                break;
            }
            queue = self
                .space
                .wait(queue)
                .map_err(|_| anyhow::anyhow!("The input queue lock failed"))?;
        }
        queue.add(cost);
        queue.replies.push_back(reply);
        Ok(())
    }

    fn finish(&self) {
        if let Ok(mut queue) = self.queue.lock() {
            queue.finished = true;
        }
    }

    fn close(&self) {
        let replies = self.queue.lock().map(|mut queue| {
            queue.closed = true;
            queue.files = 0;
            queue.file_bytes = 0;
            queue.sources = 0;
            std::mem::take(&mut queue.replies)
        });
        self.space.notify_all();
        // Release queued files and sources outside the lock.
        drop(replies);
    }

    fn try_reply(&self) -> std::result::Result<Result<Message>, mpsc::TryRecvError> {
        let mut queue = self
            .queue
            .lock()
            .map_err(|_| mpsc::TryRecvError::Disconnected)?;
        match queue.replies.pop_front() {
            Some(reply) => {
                queue.remove(Cost::of(&reply));
                drop(queue);
                self.space.notify_one();
                Ok(reply)
            }
            None if queue.finished => Err(mpsc::TryRecvError::Disconnected),
            None => Err(mpsc::TryRecvError::Empty),
        }
    }

    fn try_replies(&self) -> std::result::Result<VecDeque<Result<Message>>, mpsc::TryRecvError> {
        let mut queue = self
            .queue
            .lock()
            .map_err(|_| mpsc::TryRecvError::Disconnected)?;
        if queue.replies.is_empty() {
            return Err(if queue.finished {
                mpsc::TryRecvError::Disconnected
            } else {
                mpsc::TryRecvError::Empty
            });
        }
        queue.files = 0;
        queue.file_bytes = 0;
        queue.sources = 0;
        let replies = std::mem::take(&mut queue.replies);
        drop(queue);
        self.space.notify_one();
        Ok(replies)
    }
}

/// Marks the producer finished before the exit notification, so the screen observes disconnection.
struct Finish(Arc<Mailbox>);

impl Drop for Finish {
    fn drop(&mut self) {
        self.0.finish();
    }
}

pub struct Worker {
    cancel: Cancellation,
    destination: Arc<AtomicUsize>,
    wake: SyncSender<()>,
    replies: Arc<Mailbox>,
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
        let replies = Arc::new(Mailbox::default());
        let mailbox = replies.clone();
        let stop = cancel.clone();
        let wanted = destination.clone();
        let join = thread::spawn(move || {
            let _notify_exit = crate::wake::NotifyOnExit(notify.clone());
            let _finish = Finish(mailbox.clone());
            let deliver = |value| -> Result<()> {
                mailbox.deliver(value, &stop)?;
                let _ = notify.try_send(());
                Ok(())
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
    /// Take the oldest reply. `Disconnected` means the worker exited and nothing remains.
    pub fn try_reply(&self) -> std::result::Result<Result<Message>, mpsc::TryRecvError> {
        self.replies.try_reply()
    }

    /// Take every queued reply in order, releasing a producer that waits for space.
    /// `Disconnected` means the worker exited and nothing remains.
    pub fn try_replies(
        &self,
    ) -> std::result::Result<VecDeque<Result<Message>>, mpsc::TryRecvError> {
        self.replies.try_replies()
    }

    /// Cancel and join. Input errors arrive through replies; this reports cleanup failure or panic.
    pub fn close(&mut self) -> Result<()> {
        self.cancel.stop();
        self.replies.close();
        let _ = self.wake.try_send(());
        let result = self.join.take().map_or(Ok(()), |join| {
            join.join()
                .map_err(|_| anyhow::anyhow!("The source worker panicked"))
                .and_then(|result| match result {
                    Err(error) if error.is::<Cancelled>() => Ok(()),
                    result => result,
                })
        });
        self.replies.close();
        result
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(file: usize) -> Result<Message> {
        Ok(Message::Source {
            file,
            state: SourceState::Pending,
        })
    }

    #[test]
    fn queued_files_are_bounded_by_count_and_bytes_but_one_always_fits() {
        let mut queue = Queue::default();
        assert!(queue.admits(Cost::File(QUEUED_FILE_BYTES * 2)));
        for _ in 0..QUEUED_FILES {
            assert!(queue.admits(Cost::File(1)));
            queue.add(Cost::File(1));
        }
        assert!(!queue.admits(Cost::File(1)));
        assert!(queue.admits(Cost::Final));
        queue.remove(Cost::File(1));
        assert!(queue.admits(Cost::File(1)));

        let mut queue = Queue::default();
        queue.add(Cost::File(QUEUED_FILE_BYTES - 10));
        assert!(queue.admits(Cost::File(10)));
        assert!(!queue.admits(Cost::File(11)));
        assert!(queue.admits(Cost::Source));
    }

    #[test]
    fn a_full_queue_blocks_the_producer_until_the_screen_drains_everything() {
        let mailbox = Arc::new(Mailbox::default());
        let stop = Cancellation::default();
        for file in 0..QUEUED_SOURCES {
            mailbox.deliver(source(file), &stop).unwrap();
        }
        let (done, delivered) = mpsc::sync_channel(1);
        let producer = {
            let mailbox = mailbox.clone();
            let stop = stop.clone();
            thread::spawn(move || {
                let result = mailbox.deliver(source(QUEUED_SOURCES), &stop);
                done.send(()).unwrap();
                result
            })
        };
        assert_eq!(
            delivered.recv_timeout(Duration::from_millis(50)),
            Err(mpsc::RecvTimeoutError::Timeout)
        );
        let replies = mailbox.try_replies().unwrap();
        assert_eq!(replies.len(), QUEUED_SOURCES);
        delivered.recv_timeout(Duration::from_secs(10)).unwrap();
        producer.join().unwrap().unwrap();
        let replies = mailbox.try_replies().unwrap();
        assert!(matches!(
            replies.front(),
            Some(Ok(Message::Source { file, .. })) if *file == QUEUED_SOURCES
        ));
        assert!(matches!(
            mailbox.try_replies(),
            Err(mpsc::TryRecvError::Empty)
        ));
        mailbox.finish();
        assert!(matches!(
            mailbox.try_reply(),
            Err(mpsc::TryRecvError::Disconnected)
        ));
    }

    #[test]
    fn closing_releases_a_blocked_producer_and_drops_queued_replies() {
        let mailbox = Arc::new(Mailbox::default());
        let stop = Cancellation::default();
        for file in 0..QUEUED_SOURCES {
            mailbox.deliver(source(file), &stop).unwrap();
        }
        let producer = {
            let mailbox = mailbox.clone();
            let stop = stop.clone();
            thread::spawn(move || mailbox.deliver(source(QUEUED_SOURCES), &stop))
        };
        thread::sleep(Duration::from_millis(20));
        stop.stop();
        mailbox.close();
        let error = producer.join().unwrap().unwrap_err();
        assert!(error.is::<Cancelled>());
        assert!(mailbox.lock().unwrap().replies.is_empty());
        assert!(
            mailbox
                .deliver(source(0), &Cancellation::default())
                .unwrap_err()
                .is::<Cancelled>()
        );
    }
}
