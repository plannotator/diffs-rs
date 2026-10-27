use super::*;
use crossterm::event::KeyEvent;
use diffs_rs::engine::{Input, Options, SidebarPreference};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect, style::Modifier};
use std::{
    fs,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

fn key(reading: &mut Reading, engine: &mut Engine, code: KeyCode) {
    assert!(
        reading
            .handle(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)), engine)
            .unwrap()
    );
}

fn emphasized(buffer: &Buffer) -> Vec<(usize, String)> {
    buffer
        .content()
        .iter()
        .enumerate()
        .filter(|(_, cell)| cell.modifier.contains(Modifier::UNDERLINED))
        .map(|(index, cell)| (index, cell.symbol().to_owned()))
        .collect()
}

#[test]
fn escape_cancels_an_established_scan_without_moving_or_clearing_the_previous_match() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("review.patch");
    fs::write(
        &path,
        "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-first needle\n+second needle\n",
    )
    .unwrap();
    let (wake, ready) = mpsc::sync_channel(1);
    let mut engine = Engine::open(
        Input::Patch(path),
        Options {
            sidebar: SidebarPreference::Hide,
            ..Options::default()
        },
        wake.clone(),
    )
    .unwrap();
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    let pane = Rect::new(5, 3, 110, 24);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        engine.poll().unwrap();
        terminal
            .draw(|frame| engine.draw(frame, pane).unwrap())
            .unwrap();
        engine.after_draw();
        assert!(engine.status().error.is_none());
        if !engine.status().loading && !engine.status().coloring {
            break;
        }
        assert!(Instant::now() < deadline);
        let _ = ready.recv_timeout(Duration::from_millis(20));
    }
    let mut reading = Reading::new(wake);
    key(&mut reading, &mut engine, KeyCode::Char('/'));
    reading
        .handle(&Event::Paste("needle".into()), &mut engine)
        .unwrap();
    key(&mut reading, &mut engine, KeyCode::Enter);
    while reading.pending.is_some() {
        reading.poll(&mut engine).unwrap();
        assert!(Instant::now() < deadline);
        let _ = ready.recv_timeout(Duration::from_millis(20));
    }
    assert_eq!(reading.current.as_ref().unwrap().location().side, Side::Old);
    let position = engine.selected_source().unwrap();
    let before = terminal
        .draw(|frame| engine.draw(frame, pane).unwrap())
        .unwrap()
        .buffer
        .clone();
    let marked = emphasized(&before);
    assert_eq!(
        marked
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<String>(),
        "needle"
    );
    for (index, _) in &marked {
        let x = *index as u16 % 120;
        let y = *index as u16 / 120;
        assert!(x >= pane.x && x < pane.right() && y >= pane.y && y < pane.bottom());
    }
    let (entered, resume) = reading.worker.as_mut().unwrap().pause_next();
    key(&mut reading, &mut engine, KeyCode::F(3));
    entered.recv_timeout(Duration::from_secs(10)).unwrap();
    let pending = reading.pending.unwrap();
    for _ in 0..8 {
        key(&mut reading, &mut engine, KeyCode::F(3));
    }
    assert_eq!(reading.pending, Some(pending));
    assert!(Arc::strong_count(reading.read.as_ref().unwrap()) > 1);
    key(&mut reading, &mut engine, KeyCode::Esc);
    assert!(reading.pending.is_none());
    assert_eq!(reading.controls(120).status, Some("Search canceled"));
    resume.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Arc::strong_count(reading.read.as_ref().unwrap()) != 1 {
        assert!(
            Instant::now() < deadline,
            "canceled request retained its source owner"
        );
        thread::sleep(Duration::from_millis(1));
    }
    reading.poll(&mut engine).unwrap();
    assert_eq!(engine.selected_source().unwrap(), position);
    assert_eq!(reading.current.as_ref().unwrap().location().side, Side::Old);
    let after = terminal
        .draw(|frame| engine.draw(frame, pane).unwrap())
        .unwrap()
        .buffer
        .clone();
    assert_eq!(emphasized(&after), marked);
    key(&mut reading, &mut engine, KeyCode::F(3));
    while reading.pending.is_some() {
        reading.poll(&mut engine).unwrap();
        assert!(Instant::now() < deadline);
        let _ = ready.recv_timeout(Duration::from_millis(20));
    }
    assert_eq!(reading.current.as_ref().unwrap().location().side, Side::New);
    reading.close().unwrap();
    engine.close().unwrap();
}

#[test]
fn save_acceptance_rejects_completed_output_after_escape_or_review_close() {
    let dir = tempfile::tempdir().unwrap();
    let patch = dir.path().join("review.patch");
    fs::write(
        &patch,
        b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n",
    )
    .unwrap();
    let (wake, ready) = mpsc::sync_channel(1);
    let mut engine = Engine::open(Input::Patch(patch), Options::default(), wake).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while engine.status().loading {
        engine.poll().unwrap();
        assert!(Instant::now() < deadline);
        if engine.status().loading {
            let _ = ready.recv_timeout(Duration::from_millis(20));
        }
    }
    assert!(engine.status().error.is_none());
    let (wake, ready) = mpsc::sync_channel(1);
    let mut reading = Reading::new(wake);
    for close_review in [false, true] {
        let read = Arc::new(
            engine
                .source_read(SourceRange {
                    review: engine.review_id(),
                    file: 0,
                    side: Side::Old,
                    first: 1,
                    last: 1,
                })
                .unwrap(),
        );
        let owner = Arc::downgrade(&read);
        let destination = dir
            .path()
            .join(if close_review { "closed" } else { "canceled" });
        reading
            .save(read, destination.to_str().unwrap().into())
            .unwrap();
        ready.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(!destination.exists());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
        assert!(owner.upgrade().is_some());
        if close_review {
            engine.close().unwrap();
        } else {
            key(&mut reading, &mut engine, KeyCode::Esc);
            assert_eq!(reading.controls(160).status, Some("Save canceled"));
        }
        reading.poll(&mut engine).unwrap();
        if close_review {
            assert!(
                reading
                    .controls(160)
                    .status
                    .unwrap()
                    .contains("Save failed:")
            );
        }
        assert!(owner.upgrade().is_none());
        assert!(reading.pending.is_none());
        assert!(!destination.exists());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    reading.close().unwrap();
}

#[test]
fn copy_acceptance_discards_prepared_text_after_escape_or_review_close() {
    for close_review in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let patch = dir.path().join("review.patch");
        fs::write(
            &patch,
            b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n",
        )
        .unwrap();
        let (wake, ready) = mpsc::sync_channel(1);
        let mut engine = Engine::open(Input::Patch(patch), Options::default(), wake).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while engine.status().loading {
            engine.poll().unwrap();
            assert!(Instant::now() < deadline);
            if engine.status().loading {
                let _ = ready.recv_timeout(Duration::from_millis(10));
            }
        }
        let read = Arc::new(
            engine
                .source_read(SourceRange {
                    review: engine.review_id(),
                    file: 0,
                    side: Side::Old,
                    first: 1,
                    last: 1,
                })
                .unwrap(),
        );
        let owner = Arc::downgrade(&read);
        let (wake, ready) = mpsc::sync_channel(1);
        let mut reading = Reading::new(wake.clone());
        let mut worker = Worker::start(wake).unwrap();
        let id = worker.copy(read).unwrap();
        reading.pending = Some(Pending::Copy(id));
        reading.worker = Some(worker);
        ready.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(owner.upgrade().is_some());
        if close_review {
            engine.close().unwrap();
        } else {
            key(&mut reading, &mut engine, KeyCode::Esc);
        }
        reading.poll(&mut engine).unwrap();
        assert!(reading.pending.is_none());
        assert!(owner.upgrade().is_none());
        assert!(reading.notice.as_ref().unwrap().contains(if close_review {
            "Copy failed"
        } else {
            "Copy canceled"
        }));
        reading.close().unwrap();
        engine.close().unwrap();
    }
}

#[test]
fn search_and_save_prompts_report_an_interrupted_delivery() {
    for save in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let patch = dir.path().join("review.patch");
        fs::write(
            &patch,
            b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n",
        )
        .unwrap();
        let (wake, ready) = mpsc::sync_channel(1);
        let mut engine = Engine::open(Input::Patch(patch), Options::default(), wake).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while engine.status().loading {
            engine.poll().unwrap();
            assert!(Instant::now() < deadline);
            if engine.status().loading {
                let _ = ready.recv_timeout(Duration::from_millis(10));
            }
        }
        terminal
            .draw(|frame| engine.draw(frame, frame.area()).unwrap())
            .unwrap();
        engine.execute(Command::FirstSource).unwrap();
        let read = Arc::new(Reading::source(&mut engine).unwrap());
        let (wake, ready) = mpsc::sync_channel(1);
        let mut reading = Reading::new(wake.clone());
        let mut worker = Worker::start(wake).unwrap();
        let id = worker.copy(read).unwrap();
        ready.recv_timeout(Duration::from_secs(10)).unwrap();
        let Output::Copy(prepared) = worker.poll().unwrap().unwrap().result.unwrap() else {
            panic!("Expected copy preparation")
        };
        let mut command = std::process::Command::new("/bin/sleep");
        command.arg("30");
        worker
            .deliver(id, prepared, engine.review_id(), command)
            .unwrap();
        reading.worker = Some(worker);
        reading.pending = Some(Pending::Deliver(id));
        let event = if save {
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)
        } else {
            KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE)
        };
        assert!(reading.handle(&Event::Key(event), &mut engine).unwrap());
        assert!(reading.pending.is_none());
        assert_eq!(reading.notice.as_deref(), Some(COPY_INTERRUPTED));
        terminal.draw(|frame| reading.draw_prompt(frame)).unwrap();
        let contents: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(contents.contains(COPY_INTERRUPTED));
        assert!(contents.contains(if save { "/save:" } else { "/diff:" }));
        reading.close().unwrap();
        engine.close().unwrap();
    }
}
