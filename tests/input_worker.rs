use std::{
    io::Read,
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};

use diffs_rs::input::{LocalInput, Message, PatchInput, SourceState, Worker};

#[path = "support/readiness.rs"]
mod readiness;

fn receive(worker: &Worker, notified: &mpsc::Receiver<()>) -> Message {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match worker.try_reply() {
            Ok(reply) => return reply.unwrap(),
            Err(mpsc::TryRecvError::Empty) => {
                notified
                    .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                    .expect("input worker stopped returning replies");
            }
            Err(error) => panic!("input worker disconnected: {error}"),
        }
    }
}

fn close(mut worker: Worker) {
    let (done, closed) = mpsc::sync_channel(1);
    let join = thread::spawn(move || {
        worker.close().unwrap();
        drop(worker);
        done.send(()).unwrap();
    });
    closed
        .recv_timeout(Duration::from_secs(10))
        .expect("input worker did not close with a full reply queue");
    join.join().unwrap();
}

fn start(repo: &std::path::Path) -> (Worker, mpsc::Receiver<()>) {
    let (wake, notified) = mpsc::sync_channel(64);
    let worker = Worker::start(
        LocalInput::Git {
            repo: repo.to_owned(),
            base: "HEAD^".into(),
            head: "HEAD".into(),
        },
        wake,
    );
    (worker, notified)
}

#[test]
fn closing_a_full_patch_queue_releases_the_producer_but_keeps_published_source_valid() {
    let repo = readiness::repository(false);
    let (worker, notified) = start(repo.path());
    for _ in 0..2 {
        notified.recv_timeout(Duration::from_secs(10)).unwrap();
    }
    let Message::File(file) = worker.try_reply().unwrap().unwrap() else {
        panic!("expected the first file");
    };
    assert_eq!(file.id, 0);
    notified.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(
        notified.recv_timeout(Duration::from_millis(20)),
        Err(mpsc::RecvTimeoutError::Timeout)
    );
    close(worker);
    assert_eq!(Arc::strong_count(&file), 1);
    assert_eq!(
        file.bytes(file.source_base).unwrap(),
        b"return 97; // source 97\n"
    );
    let released = Arc::downgrade(&file.spool);
    drop(file);
    assert!(released.upgrade().is_none());
}

#[test]
fn closing_a_full_source_queue_releases_review_owners_without_invalidating_retained_snapshots() {
    let repo = readiness::repository(false);
    let (worker, notified) = start(repo.path());
    let mut files = 0;
    loop {
        match receive(&worker, &notified) {
            Message::File(_) => files += 1,
            Message::Complete { files: count } => {
                assert_eq!((files, count), (12, 12));
                break;
            }
            Message::Source { .. } => panic!("source prepared without a request"),
        }
    }
    worker.request(0);
    let Message::Source {
        file: 0,
        state: SourceState::Checked(pair),
    } = receive(&worker, &notified)
    else {
        panic!("expected the requested checked source");
    };
    while notified.try_recv().is_ok() {}
    for file in [1, 2] {
        worker.request(file);
        notified.recv_timeout(Duration::from_secs(10)).unwrap();
    }
    worker.request(3);
    assert_eq!(
        notified.recv_timeout(Duration::from_millis(20)),
        Err(mpsc::RecvTimeoutError::Timeout)
    );
    close(worker);
    assert_eq!(Arc::strong_count(&pair), 1);
    let released = [
        Arc::downgrade(pair.old().unwrap()),
        Arc::downgrade(pair.new_source().unwrap()),
    ];
    let patch_store = Arc::downgrade(&pair.file().spool);
    for (source, changed) in [
        (pair.old().unwrap(), false),
        (pair.new_source().unwrap(), true),
    ] {
        for line in [1, 100, 400, 760, 800] {
            let mut bytes = Vec::new();
            source
                .range(line - 1..line)
                .unwrap()
                .read_to_end(&mut bytes)
                .unwrap();
            let sign = if changed && [100, 400, 760].contains(&line) {
                "-"
            } else {
                ""
            };
            assert_eq!(
                bytes,
                format!("return {sign}{line}; // source {line}\n").as_bytes()
            );
        }
    }
    drop(pair);
    assert!(released.iter().all(|source| source.upgrade().is_none()));
    assert!(patch_store.upgrade().is_none());
}

#[test]
fn closing_patch_readers_releases_blocked_input_and_queues_before_a_new_review() {
    use nix::{
        fcntl::{FcntlArg, OFlag, fcntl},
        unistd::pipe,
    };
    use std::{fs::File, io::Write, os::fd::AsRawFd};
    for mode in ["idle", "partial", "full-queue"] {
        let (read_end, write_end) = pipe().unwrap();
        let observed = File::from(read_end);
        let original = fcntl(observed.as_raw_fd(), FcntlArg::F_GETFL).unwrap();
        let mut producer = File::from(write_end);
        let (wake, notified) = mpsc::sync_channel(64);
        let worker = Worker::start_patch(PatchInput::Stream(observed.try_clone().unwrap()), wake);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !OFlag::from_bits_retain(fcntl(observed.as_raw_fd(), FcntlArg::F_GETFL).unwrap())
            .contains(OFlag::O_NONBLOCK)
        {
            assert!(Instant::now() < deadline, "patch reader did not start");
            thread::sleep(Duration::from_millis(1));
        }
        let retained = if mode == "full-queue" {
            for n in 0..8 {
                writeln!(producer, "diff --git a/{n}.txt b/{n}.txt\n--- a/{n}.txt\n+++ b/{n}.txt\n@@ -1 +1 @@\n-old\n+new").unwrap();
            }
            for _ in 0..2 {
                notified.recv_timeout(Duration::from_secs(5)).unwrap();
            }
            let Message::File(file) = worker.try_reply().unwrap().unwrap() else {
                panic!("expected file");
            };
            notified.recv_timeout(Duration::from_secs(5)).unwrap();
            Some(file)
        } else {
            if mode == "partial" {
                producer.write_all(b"diff --git a/").unwrap();
            }
            assert!(matches!(
                notified.recv_timeout(Duration::from_millis(50)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ));
            None
        };
        close(worker);
        assert_eq!(
            fcntl(observed.as_raw_fd(), FcntlArg::F_GETFL).unwrap(),
            original
        );
        if let Some(file) = retained {
            assert_eq!(Arc::strong_count(&file), 1);
            assert_eq!(file.bytes(file.source_base).unwrap(), b"old\n");
            let store = Arc::downgrade(&file.spool);
            drop(file);
            assert!(store.upgrade().is_none());
        }
        drop(producer);

        let (read_end, write_end) = pipe().unwrap();
        let mut producer = File::from(write_end);
        producer.write_all(b"diff --git a/0.txt b/0.txt\n--- a/0.txt\n+++ b/0.txt\n@@ -1 +1 @@\n-before\n+after\n").unwrap();
        drop(producer);
        let (wake, notified) = mpsc::sync_channel(64);
        let next = Worker::start_patch(PatchInput::Stream(read_end.into()), wake);
        let Message::File(file) = receive(&next, &notified) else {
            panic!("expected file");
        };
        assert_eq!(file.bytes(file.source_base + 1).unwrap(), b"after\n");
        assert!(matches!(
            receive(&next, &notified),
            Message::Complete { files: 1 }
        ));
        close(next);
    }
}

#[test]
fn failed_patch_worker_reports_failure_and_disconnection_without_an_extra_input() {
    let directory = tempfile::tempdir().unwrap();
    let (wake, notified) = mpsc::sync_channel(1);
    let mut worker = Worker::start_patch(
        PatchInput::Path(directory.path().join("absent.patch")),
        wake,
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut failure = false;
    loop {
        match worker.try_reply() {
            Ok(Err(error)) => {
                assert!(error.to_string().contains("Cannot open"));
                failure = true;
            }
            Ok(Ok(_)) => panic!("failed open published source"),
            Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {
                notified
                    .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                    .unwrap();
            }
        }
    }
    assert!(failure);
    worker.close().unwrap();
    worker.close().unwrap();
}
