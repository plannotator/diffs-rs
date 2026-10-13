use std::{
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};

use super::{Request, start};
use crate::{storage, theme::Appearance};

#[test]
fn host_calls_survive_a_poisoned_mailbox() {
    let patch =
        "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n-let a = 1;\n+let a = 2;\n";
    let mut files = Vec::new();
    storage::read(patch.as_bytes(), |file| {
        files.push(file);
        Ok(())
    })
    .unwrap();
    let (wake, notified) = mpsc::sync_channel(4);
    let mut worker = start(wake);
    // A panic while holding the lock poisons it for every later caller.
    let requests = Arc::clone(&worker.requests);
    let poisoned = thread::spawn(move || {
        let _pending = requests.pending.lock().unwrap();
        panic!("poison the mailbox");
    })
    .join();
    assert!(poisoned.is_err());
    assert!(worker.requests.pending.is_poisoned());

    // Requests, replies, cancellation, and close still work without panicking the host.
    worker.request(Request {
        epoch: 0,
        appearance: Appearance::Dark,
        ids: vec![0, 1],
        files: vec![files[0].clone().into()],
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut replies = 0;
    while replies < 2 {
        assert!(Instant::now() < deadline, "worker stopped replying");
        let _ = notified.recv_timeout(Duration::from_millis(50));
        while let Ok(reply) = worker.try_reply() {
            assert!(reply.is_ok());
            replies += 1;
        }
    }
    worker.cancel();
    worker.close().unwrap();
}
