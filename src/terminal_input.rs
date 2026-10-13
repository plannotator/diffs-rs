use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI32, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::Result;
use crossterm::event::{self, Event};
use signal_hook::iterator::{Handle, Signals};

pub struct TerminalInput {
    keys: Receiver<io::Result<Event>>,
    signal: Arc<AtomicI32>,
    stop: Arc<AtomicBool>,
    signals: Handle,
    key_thread: Option<JoinHandle<()>>,
    signal_thread: Option<JoinHandle<()>>,
}

impl TerminalInput {
    pub fn start(wake: SyncSender<()>) -> Result<Self> {
        let mut events = Signals::new([
            signal_hook::consts::SIGINT,
            signal_hook::consts::SIGTERM,
            signal_hook::consts::SIGHUP,
        ])?;
        let signals = events.handle();
        let signal = Arc::new(AtomicI32::new(0));
        let target = signal.clone();
        let notify = wake.clone();
        let signal_thread = thread::spawn(move || {
            for event in events.forever() {
                target.store(event, Ordering::Relaxed);
                let _ = notify.try_send(());
            }
        });
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let (send, keys) = mpsc::sync_channel(1);
        let key_thread = thread::spawn(move || {
            while !stopped.load(Ordering::Acquire) {
                let mut event = match event::poll(Duration::from_millis(25)) {
                    Ok(false) => continue,
                    Ok(true) => event::read(),
                    Err(error) => Err(error),
                };
                let failed = event.is_err();
                loop {
                    if stopped.load(Ordering::Acquire) {
                        return;
                    }
                    match send.try_send(event) {
                        Ok(()) => break,
                        Err(mpsc::TrySendError::Disconnected(_)) => return,
                        Err(mpsc::TrySendError::Full(pending)) => event = pending,
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                let _ = wake.try_send(());
                if failed {
                    return;
                }
            }
        });
        Ok(Self {
            keys,
            signal,
            stop,
            signals,
            key_thread: Some(key_thread),
            signal_thread: Some(signal_thread),
        })
    }

    pub fn signal(&self) -> i32 {
        self.signal.load(Ordering::Relaxed)
    }

    pub fn try_event(&self) -> Result<io::Result<Event>, TryRecvError> {
        self.keys.try_recv()
    }

    pub fn close(&mut self) -> Result<()> {
        self.stop.store(true, Ordering::Release);
        self.signals.close();
        let keys = self.key_thread.take().map(|thread| thread.join());
        let signals = self.signal_thread.take().map(|thread| thread.join());
        let keys = keys
            .transpose()
            .map(|_| ())
            .map_err(|_| anyhow::anyhow!("The terminal input worker panicked"));
        let signals = signals
            .transpose()
            .map(|_| ())
            .map_err(|_| anyhow::anyhow!("The signal worker panicked"));
        crate::cleanup::finish(keys, signals)
    }
}

impl Drop for TerminalInput {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
