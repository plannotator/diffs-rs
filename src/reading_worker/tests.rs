use super::*;
use native_diff::engine::{Engine, Input, Options, Side};
use std::{
    fs,
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

pub(super) struct Checkpoint(Mutex<Option<(SyncSender<()>, Receiver<()>)>>);

impl Checkpoint {
    pub(super) fn new() -> (Self, Receiver<()>, SyncSender<()>) {
        let (entered, ready) = mpsc::sync_channel(1);
        let (resume, released) = mpsc::sync_channel(1);
        (Self(Mutex::new(Some((entered, released)))), ready, resume)
    }

    pub(super) fn wait(&self) -> Result<()> {
        let pause = self.0.lock().unwrap().take();
        if let Some((entered, resume)) = pause {
            entered.send(())?;
            resume.recv_timeout(Duration::from_secs(10))?;
        }
        Ok(())
    }
}

fn review() -> (Engine, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("review.patch");
    fs::write(
        &path,
        b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-first needle\n+second needle\n",
    )
    .unwrap();
    let (notify, ready) = mpsc::sync_channel(1);
    let mut engine = Engine::open(Input::Patch(path), Options::default(), notify).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while engine.status().loading {
        engine.poll().unwrap();
        assert!(Instant::now() < deadline);
        if engine.status().loading {
            let _ = ready.recv_timeout(Duration::from_millis(20));
        }
    }
    assert!(engine.status().error.is_none());
    (engine, dir)
}

fn reply(worker: &Worker, ready: &Receiver<()>) -> Reply {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(reply) = worker.poll().unwrap() {
            return reply;
        }
        assert!(Instant::now() < deadline, "search did not finish");
        ready.recv_timeout(Duration::from_millis(20)).ok();
    }
}

fn search(reply: Reply) -> SearchMatch {
    match reply.result.unwrap() {
        Output::Search(found) => found.unwrap(),
        _ => panic!("Expected search output"),
    }
}

#[test]
fn latest_request_replaces_queued_replies_and_owns_worker_cleanup() {
    let (mut engine, _dir) = review();
    let (notify, ready) = mpsc::sync_channel(1);
    let mut worker = Worker::start(notify).unwrap();
    let first = Arc::new(engine.search_read("first".into()).unwrap());
    let first_owner = Arc::downgrade(&first);
    worker
        .submit(first, None, SearchDirection::Forward)
        .unwrap();
    ready.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(worker.state.lock().unwrap().reply.is_some());
    let latest = Arc::new(engine.search_read("needle".into()).unwrap());
    let mut wanted = 0;
    for _ in 0..8 {
        wanted = worker
            .submit(latest.clone(), None, SearchDirection::Forward)
            .unwrap();
    }
    let found = reply(&worker, &ready);
    assert_eq!(found.id, wanted);
    let hit = search(found);
    assert_eq!(hit.location().side, Side::Old);
    assert!(first_owner.upgrade().is_none());
    let next = worker
        .submit(latest.clone(), Some(hit), SearchDirection::Forward)
        .unwrap();
    let found = reply(&worker, &ready);
    assert_eq!(found.id, next);
    assert_eq!(search(found).location().side, Side::New);
    worker
        .submit(latest, None, SearchDirection::Backward)
        .unwrap();
    worker.cancel().unwrap();
    assert!(worker.poll().unwrap().is_none());
    worker.close().unwrap();
    worker.close().unwrap();
    assert!(worker.join.is_none());
    assert!(worker.poll().unwrap().is_none());
    engine.close().unwrap();
}

#[test]
fn invalidated_review_returns_an_error_and_the_same_worker_can_search_a_new_review() {
    let (mut engine, _dir) = review();
    let stale = Arc::new(engine.search_read("needle".into()).unwrap());
    engine.close().unwrap();
    let (notify, ready) = mpsc::sync_channel(1);
    let mut worker = Worker::start(notify).unwrap();
    let wanted = worker
        .submit(stale, None, SearchDirection::Forward)
        .unwrap();
    let failed = reply(&worker, &ready);
    assert_eq!(failed.id, wanted);
    assert!(
        failed
            .result
            .err()
            .unwrap()
            .to_string()
            .contains("closed review")
    );
    let (mut next, _next_dir) = review();
    let read = Arc::new(next.search_read("second".into()).unwrap());
    worker.submit(read, None, SearchDirection::Forward).unwrap();
    let found = search(reply(&worker, &ready));
    assert_eq!(found.location().review, next.review_id());
    assert_eq!(found.location().side, Side::New);
    worker.close().unwrap();
    next.close().unwrap();
}

#[test]
fn in_flight_search_is_superseded_canceled_and_joined_without_a_late_reply() {
    enum Action {
        Supersede,
        Cancel,
        Close,
    }

    for action in [Action::Supersede, Action::Cancel, Action::Close] {
        let (mut engine, _dir) = review();
        let (notify, ready) = mpsc::sync_channel(1);
        let mut worker = Worker::start(notify).unwrap();
        let (checkpoint, entered, resume) = Checkpoint::new();
        worker.state.lock().unwrap().checkpoint = Some(checkpoint);
        let read = Arc::new(engine.search_read("needle".into()).unwrap());
        let owner = Arc::downgrade(&read);
        worker.submit(read, None, SearchDirection::Forward).unwrap();
        entered.recv_timeout(Duration::from_secs(10)).unwrap();
        {
            let state = worker.state.lock().unwrap();
            assert!(state.pending.is_none());
            assert!(state.reply.is_none());
        }
        assert!(!worker.join.as_ref().unwrap().is_finished());
        assert!(owner.upgrade().is_some());

        match action {
            Action::Supersede => {
                let next = Arc::new(engine.search_read("second".into()).unwrap());
                let wanted = worker.submit(next, None, SearchDirection::Forward).unwrap();
                resume.send(()).unwrap();
                let found = reply(&worker, &ready);
                assert_eq!(found.id, wanted);
                assert_eq!(search(found).location().side, Side::New);
            }
            Action::Cancel => {
                worker.cancel().unwrap();
                assert!(worker.poll().unwrap().is_none());
                resume.send(()).unwrap();
                let deadline = Instant::now() + Duration::from_secs(10);
                while owner.strong_count() != 0 {
                    assert!(Instant::now() < deadline, "canceled search did not finish");
                    thread::sleep(Duration::from_millis(1));
                }
                assert!(worker.poll().unwrap().is_none());
                assert!(!worker.join.as_ref().unwrap().is_finished());
            }
            Action::Close => {
                let stopped = worker.stop.clone();
                thread::scope(|scope| {
                    let closing = scope.spawn(|| worker.close());
                    let deadline = Instant::now() + Duration::from_secs(10);
                    while stopped.check().is_ok() {
                        assert!(Instant::now() < deadline, "close did not begin");
                        thread::yield_now();
                    }
                    resume.send(()).unwrap();
                    closing.join().unwrap().unwrap();
                });
            }
        }
        worker.close().unwrap();
        assert!(owner.upgrade().is_none());
        assert!(worker.join.is_none());
        assert!(worker.poll().unwrap().is_none());
        engine.close().unwrap();
    }
}

#[test]
fn completed_saves_are_private_and_discarded_by_cancel_supersession_and_close() {
    enum Action {
        Cancel,
        Supersede,
        Close,
    }
    for action in [Action::Cancel, Action::Supersede, Action::Close] {
        let (mut engine, dir) = review();
        let (notify, ready) = mpsc::sync_channel(1);
        let mut worker = Worker::start(notify).unwrap();
        let read = Arc::new(
            engine
                .source_read(native_diff::reading::SourceRange {
                    review: engine.review_id(),
                    file: 0,
                    side: Side::Old,
                    first: 1,
                    last: 1,
                })
                .unwrap(),
        );
        let owner = Arc::downgrade(&read);
        let destination = dir.path().join("saved");
        let wanted = worker.save(read, destination.clone()).unwrap();
        ready.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(
            worker.state.lock().unwrap().reply.as_ref().unwrap().id,
            wanted
        );
        assert!(owner.upgrade().is_some());
        assert!(!destination.exists());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
        match action {
            Action::Cancel => worker.cancel().unwrap(),
            Action::Close => worker.close().unwrap(),
            Action::Supersede => {
                let read = Arc::new(engine.search_read("second".into()).unwrap());
                worker.submit(read, None, SearchDirection::Forward).unwrap();
                assert_eq!(search(reply(&worker, &ready)).location().side, Side::New);
            }
        }
        assert!(owner.upgrade().is_none());
        assert!(worker.poll().unwrap().is_none());
        assert!(!destination.exists());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        worker.close().unwrap();
        assert!(worker.join.is_none());
        engine.close().unwrap();
    }
}

#[test]
fn an_in_flight_save_can_be_canceled_and_followed_by_a_successful_save() {
    let (mut engine, dir) = review();
    let (notify, ready) = mpsc::sync_channel(1);
    let mut worker = Worker::start(notify).unwrap();
    let read = Arc::new(
        engine
            .source_read(native_diff::reading::SourceRange {
                review: engine.review_id(),
                file: 0,
                side: Side::New,
                first: 1,
                last: 1,
            })
            .unwrap(),
    );
    let (entered, resume) = worker.pause_next();
    let canceled = dir.path().join("canceled");
    worker.save(read.clone(), canceled.clone()).unwrap();
    entered.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(worker.state.lock().unwrap().pending.is_none());
    worker.cancel().unwrap();
    let saved = dir.path().join("saved");
    let wanted = worker.save(read, saved.clone()).unwrap();
    resume.send(()).unwrap();
    let prepared = reply(&worker, &ready);
    assert_eq!(prepared.id, wanted);
    assert!(!saved.exists());
    match prepared.result.unwrap() {
        Output::Save(prepared) => prepared
            .publish(engine.review_id())
            .unwrap()
            .cleanup
            .unwrap(),
        _ => panic!("Expected save output"),
    }
    assert_eq!(fs::read(&saved).unwrap(), b"second needle\n");
    assert!(!canceled.exists());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    worker.close().unwrap();
    engine.close().unwrap();
}

#[test]
fn accepted_copy_children_are_reaped_before_reuse_and_close() {
    enum Action {
        Cancel,
        Supersede,
        Close,
    }
    for action in [Action::Cancel, Action::Supersede, Action::Close] {
        let (mut engine, dir) = review();
        let (notify, ready) = mpsc::sync_channel(1);
        let mut worker = Worker::start(notify).unwrap();
        let read = Arc::new(
            engine
                .source_read(native_diff::reading::SourceRange {
                    review: engine.review_id(),
                    file: 0,
                    side: Side::Old,
                    first: 1,
                    last: 1,
                })
                .unwrap(),
        );
        let owner = Arc::downgrade(&read);
        let id = worker.copy(read).unwrap();
        let prepared = match reply(&worker, &ready).result.unwrap() {
            Output::Copy(prepared) => prepared,
            _ => panic!("Expected copy preparation"),
        };
        let pid_path = dir.path().join("helper.pid");
        let mut command = std::process::Command::new("/bin/sh");
        command
            .args([
                "-c",
                "printf '%s' \"$$\" > \"$1\"; exec /bin/sleep 30",
                "copy-check",
            ])
            .arg(&pid_path);
        let (entered, resume) = worker.pause_next();
        worker
            .deliver(id, prepared, engine.review_id(), command)
            .unwrap();
        entered.recv_timeout(Duration::from_secs(10)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let pid: u32 = loop {
            if let Ok(contents) = fs::read_to_string(&pid_path)
                && let Ok(pid) = contents.parse()
            {
                break pid;
            }
            assert!(Instant::now() < deadline, "helper did not start");
            thread::sleep(Duration::from_millis(1));
        };
        assert!(owner.upgrade().is_some());
        assert!(worker.state.lock().unwrap().pending.is_none());
        match action {
            Action::Cancel => {
                worker.cancel().unwrap();
                resume.send(()).unwrap();
                let next = Arc::new(engine.search_read("second".into()).unwrap());
                worker.submit(next, None, SearchDirection::Forward).unwrap();
                assert_eq!(search(reply(&worker, &ready)).location().side, Side::New);
            }
            Action::Supersede => {
                let read = Arc::new(
                    engine
                        .source_read(native_diff::reading::SourceRange {
                            review: engine.review_id(),
                            file: 0,
                            side: Side::New,
                            first: 1,
                            last: 1,
                        })
                        .unwrap(),
                );
                let next = worker.copy(read).unwrap();
                resume.send(()).unwrap();
                let prepared = reply(&worker, &ready);
                assert_eq!(prepared.id, next);
                let Output::Copy(prepared) = prepared.result.unwrap() else {
                    panic!("Expected next preparation")
                };
                let output = std::process::Command::new("/bin/ps")
                    .args(["-p", &pid.to_string(), "-o", "pid="])
                    .output()
                    .unwrap();
                assert!(
                    output.stdout.is_empty(),
                    "old helper overlaps the next copy"
                );
                let destination = dir.path().join("copy-output");
                let mut command = std::process::Command::new("/bin/sh");
                command
                    .args(["-c", "exec /bin/cat > \"$1\"", "copy-check"])
                    .arg(&destination);
                worker
                    .deliver(next, prepared, engine.review_id(), command)
                    .unwrap();
                let copied = reply(&worker, &ready);
                assert_eq!(copied.id, next);
                let Output::Copied(receipt) = copied.result.unwrap() else {
                    panic!("Expected delivery receipt")
                };
                receipt.check(engine.review_id()).unwrap();
                assert_eq!(fs::read(destination).unwrap(), b"second needle\n");
            }
            Action::Close => {
                let stopped = worker.stop.clone();
                thread::scope(|scope| {
                    let closing = scope.spawn(|| worker.close());
                    let deadline = Instant::now() + Duration::from_secs(10);
                    while stopped.check().is_ok() {
                        assert!(Instant::now() < deadline);
                        thread::yield_now();
                    }
                    resume.send(()).unwrap();
                    closing.join().unwrap().unwrap();
                });
            }
        }
        worker.close().unwrap();
        assert!(owner.upgrade().is_none());
        assert!(worker.poll().unwrap().is_none());
        let output = std::process::Command::new("/bin/ps")
            .args(["-p", &pid.to_string(), "-o", "pid="])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        engine.close().unwrap();
    }
}
