use std::{
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};

use native_diff::{
    app::{App, Layout},
    highlight::{self, Request},
    model::RowKind,
    render,
    storage::{self, FileIndex},
    theme::Appearance,
};
use ratatui::{Terminal, backend::TestBackend, style::Color};

fn source() -> Arc<FileIndex> {
    let mut patch = String::from(
        "diff --git a/code.ts b/code.ts\nnew file mode 100644\n--- /dev/null\n+++ b/code.ts\n@@ -0,0 +1,64 @@\n",
    );
    for n in 0..64 {
        patch.push_str(&format!("+return 2; // item {n}\n"));
    }
    let mut files = Vec::new();
    storage::read(patch.as_bytes(), |file| {
        files.push(file);
        Ok(())
    })
    .unwrap();
    assert_eq!(files.len(), 1);
    let file = files.pop().unwrap();
    assert_eq!(file.source_end(), 64);
    file
}

fn wait_for_source_owners(file: &Arc<FileIndex>, expected: usize) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Arc::strong_count(file) != expected {
        assert!(
            Instant::now() < deadline,
            "worker retained source after cancellation: {} owners, expected {expected}",
            Arc::strong_count(file)
        );
        thread::sleep(Duration::from_millis(1));
    }
}

fn draw(app: &mut App, width: u16, height: u16) {
    app.reflow(Layout::Split, width, height).unwrap();
    Terminal::new(TestBackend::new(width, height))
        .unwrap()
        .draw(|frame| render::draw(frame, app, None).unwrap())
        .unwrap();
}

#[test]
fn cached_and_empty_resized_views_cancel_obsolete_coloring() {
    for empty in [false, true] {
        let file = source();
        let (wake, notified) = mpsc::sync_channel(64);
        let mut app = App::new(
            storage::Document {
                files: vec![file.clone()],
            },
            "cancellation".into(),
            Layout::Split,
            Appearance::Dark,
            wake,
        )
        .unwrap();
        app.loading = false;
        draw(&mut app, 160, 12);
        let first_ids: Vec<_> = app
            .visible
            .iter()
            .filter_map(|(p, _)| match app.row(p.row)?.kind {
                RowKind::Code { right, .. } => right,
                _ => None,
            })
            .collect();
        assert!(!first_ids.is_empty() && first_ids.len() < 16);
        app.request_highlights();
        for &id in &first_ids {
            notified.recv_timeout(Duration::from_secs(5)).unwrap();
            let reply = app.worker.try_reply().unwrap().unwrap();
            assert_eq!((reply.epoch, reply.id), (0, id));
            app.cache.insert(id, reply.style);
        }

        draw(&mut app, 160, 50);
        app.request_highlights();
        for _ in 0..16 {
            notified.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        draw(
            &mut app,
            if empty { 30 } else { 160 },
            if empty { 8 } else { 12 },
        );
        assert_eq!(app.visible.is_empty(), empty);
        app.request_highlights();

        // The application and this test retain the only two source owners.
        wait_for_source_owners(&file, 2);
        for id in first_ids.len()..first_ids.len() + 16 {
            let reply = app.worker.try_reply().unwrap().unwrap();
            assert_eq!((reply.epoch, reply.id), (0, id.into()));
            assert!(!reply.style.fallback);
            app.cache.insert(id, reply.style);
        }
        assert!(matches!(
            app.worker.try_reply(),
            Err(mpsc::TryRecvError::Empty)
        ));
        for id in first_ids {
            assert!(!app.cache.get(id).unwrap().fallback);
        }
    }
}

#[test]
fn full_reply_queue_resumes_replaces_and_releases_real_worker_sources() {
    for action in [
        "resume",
        "navigate",
        "replace",
        "replace-twice",
        "cancel",
        "cancel-pending",
        "drop",
    ] {
        let file = source();
        let released = Arc::downgrade(&file);
        let (wake, notified) = mpsc::sync_channel(64);
        let worker = highlight::start(wake);
        worker.request(Request {
            epoch: 0,
            appearance: Appearance::Dark,
            ids: (0..64).collect(),
            files: vec![file.clone().into()],
        });
        for _ in 0..16 {
            notified.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        assert_eq!(
            notified.recv_timeout(Duration::from_millis(20)),
            Err(mpsc::RecvTimeoutError::Timeout),
            "publication exceeded the full reply queue: {action}"
        );
        let replacement = match action {
            "navigate" => Some((0, Appearance::Dark)),
            "replace" => Some((1, Appearance::Light)),
            "replace-twice" => Some((2, Appearance::Dark)),
            _ => None,
        };
        if action == "replace-twice" {
            worker.request(Request {
                epoch: 1,
                appearance: Appearance::Light,
                ids: (16..32).collect(),
                files: vec![file.clone().into()],
            });
        }
        if let Some((epoch, appearance)) = replacement {
            worker.request(Request {
                epoch,
                appearance,
                ids: (32..64).collect(),
                files: vec![file.clone().into()],
            });
        }
        let canceled = matches!(action, "cancel" | "cancel-pending");
        if action == "cancel-pending" {
            worker.request(Request {
                epoch: 1,
                appearance: Appearance::Light,
                ids: (32..64).collect(),
                files: vec![file.clone().into()],
            });
        }
        if canceled {
            worker.cancel();
            worker.cancel();
            wait_for_source_owners(&file, 1);
        }
        if action != "drop" {
            let deadline = Instant::now() + Duration::from_secs(5);
            let expected: Vec<_> = if action == "resume" {
                (0..64).map(|id| (0, id)).collect()
            } else if canceled {
                (0..16).map(|id| (0, id)).collect()
            } else {
                (0..16)
                    .map(|id| (0, id))
                    .chain((32..64).map(|id| (replacement.unwrap().0, id)))
                    .collect()
            };
            for (epoch, id) in expected {
                let reply = loop {
                    assert!(Instant::now() < deadline, "worker stopped: {action}, {id}");
                    match worker.try_reply() {
                        Ok(reply) => break reply.unwrap(),
                        Err(mpsc::TryRecvError::Empty) => {
                            let _ = notified.recv_timeout(Duration::from_millis(10));
                        }
                        Err(error) => panic!("worker disconnected: {error}"),
                    }
                };
                assert_eq!((reply.epoch, reply.id), (epoch, id.into()), "{action}");
                assert!(!reply.style.fallback);
                assert!(reply.style.tokens.iter().any(|token| {
                    token.range.start == 0
                        && token.range.end >= 6
                        && token.color
                            == if epoch != 1 {
                                Color::Rgb(255, 103, 141)
                            } else {
                                Color::Rgb(211, 42, 97)
                            }
                }));
            }
            if canceled {
                assert!(matches!(worker.try_reply(), Err(mpsc::TryRecvError::Empty)));
                worker.request(Request {
                    epoch: 1,
                    appearance: Appearance::Light,
                    ids: vec![63],
                    files: vec![file.clone().into()],
                });
                let reply = loop {
                    assert!(Instant::now() < deadline, "worker did not resume: {action}");
                    match worker.try_reply() {
                        Ok(reply) => break reply.unwrap(),
                        Err(mpsc::TryRecvError::Empty) => {
                            let _ = notified.recv_timeout(Duration::from_millis(10));
                        }
                        Err(error) => panic!("worker disconnected: {error}"),
                    }
                };
                assert_eq!((reply.epoch, reply.id), (1, 63.into()));
                assert!(!reply.style.fallback);
                assert!(reply.style.tokens.iter().any(|token| {
                    token.range.start == 0
                        && token.range.end >= 6
                        && token.color == Color::Rgb(211, 42, 97)
                }));
            }
        }
        drop(file);
        drop(worker);
        assert!(
            released.upgrade().is_none(),
            "worker retained source after closing: {action}"
        );
    }
}
