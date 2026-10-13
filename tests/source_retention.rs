use std::{
    fs,
    sync::{Arc, mpsc},
    time::Duration,
};

use native_diff::{
    highlight::{self, FileSource, Request, Token},
    input::{Cancellation, LocalInput, Review, SourceState},
    source::CheckedPair,
    theme::{self, Appearance},
    view::LineId,
};
use ratatui::style::Color;
use syntect::{easy::HighlightLines, parsing::SyntaxSet};

fn sources() -> (String, String, Arc<CheckedPair>) {
    let mut old: Vec<_> = (0..12_000)
        .map(|n| format!("return {n}; // source-{n}"))
        .collect();
    let mut new = old.clone();
    for (open, old_close, new_close) in [(0, 300, 900), (6144, 6656, 7168), (10240, 10496, 11776)] {
        old[open] = "/*".into();
        new[open] = "/*".into();
        old[old_close] = "*/".into();
        new[new_close] = "*/".into();
    }
    let old = old.join("\n") + "\n";
    let new = new.join("\n") + "\n";
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("old.ts");
    let b = dir.path().join("new.ts");
    fs::write(&a, &old).unwrap();
    fs::write(&b, &new).unwrap();
    let mut review = Review::open(
        LocalInput::Files { old: a, new: b },
        &Cancellation::default(),
        |_| Ok(()),
    )
    .unwrap();
    let SourceState::Checked(pair) = review.prepare(0, &Cancellation::default()).unwrap() else {
        panic!("unchecked source")
    };
    (old, new, pair)
}

fn sequential(text: &str, appearance: Appearance, syntaxes: &SyntaxSet) -> Vec<Vec<Token>> {
    let (_, theme) = theme::load(appearance).unwrap();
    let mut parser = HighlightLines::new(syntaxes.find_syntax_by_extension("ts").unwrap(), &theme);
    text.split_inclusive('\n')
        .map(|line| {
            let mut offset = 0;
            parser
                .highlight_line(line, syntaxes)
                .unwrap()
                .into_iter()
                .map(|(style, text)| {
                    let start = offset;
                    offset += text.len();
                    Token {
                        range: start..offset,
                        color: Color::Rgb(
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                        ),
                    }
                })
                .collect()
        })
        .collect()
}

fn context(number: u32) -> LineId {
    LineId::context(0, number, number).unwrap()
}

fn send(
    worker: &highlight::Worker,
    pair: &Arc<CheckedPair>,
    ids: Vec<LineId>,
    epoch: u64,
    appearance: Appearance,
) {
    worker.request(Request {
        epoch,
        appearance,
        ids,
        files: vec![FileSource::Complete(pair.clone())],
    });
}

fn receive(worker: &highlight::Worker, wake: &mpsc::Receiver<()>) -> highlight::Reply {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        match worker.try_reply() {
            Ok(reply) => return reply.unwrap(),
            Err(mpsc::TryRecvError::Disconnected) => panic!("worker stopped"),
            Err(mpsc::TryRecvError::Empty) => {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                assert!(!remaining.is_zero(), "worker deadline expired");
                wake.recv_timeout(remaining).unwrap();
            }
        }
    }
}

fn check(
    reply: &highlight::Reply,
    pair: &Arc<CheckedPair>,
    number: u32,
    epoch: u64,
    a: &[Vec<Token>],
    b: &[Vec<Token>],
) {
    assert_eq!(reply.id, context(number));
    assert_eq!(reply.origin, FileSource::Complete(pair.clone()).origin());
    assert_eq!(reply.epoch, epoch);
    assert!(reply.source_error.is_none() && !reply.style.fallback);
    assert_eq!(reply.style.tokens_for(true), a[number as usize - 1]);
    assert_eq!(reply.style.tokens_for(false), b[number as usize - 1]);
    if [769, 6913, 11009].contains(&number) {
        assert_ne!(reply.style.tokens_for(true), reply.style.tokens_for(false));
    }
}

#[test]
fn sparse_pressure_keeps_exact_side_states_across_revisits_and_themes() {
    #[cfg(feature = "delivery-trace")]
    let trace = native_diff::delivery_trace::init().unwrap();
    let (old, new, pair) = sources();
    let syntaxes = two_face::syntax::extra_newlines();
    let (wake, notified) = mpsc::sync_channel(1);
    let worker = highlight::start(wake);
    let numbers = [257, 769, 1537, 3585, 5633, 6913, 8193, 11009, 11905];
    for (epoch, appearance) in [
        (0, Appearance::Dark),
        (0, Appearance::Dark),
        (1, Appearance::Light),
        (2, Appearance::Dark),
    ] {
        let a = sequential(&old, appearance, &syntaxes);
        let b = sequential(&new, appearance, &syntaxes);
        send(
            &worker,
            &pair,
            numbers.into_iter().map(context).collect(),
            epoch,
            appearance,
        );
        for number in numbers {
            check(&receive(&worker, &notified), &pair, number, epoch, &a, &b);
        }
    }
    send(&worker, &pair, vec![context(257)], 2, Appearance::Dark);
    assert_eq!(receive(&worker, &notified).id, context(257));
    drop(worker);
    #[cfg(feature = "delivery-trace")]
    drop(trace);
}

#[test]
fn protected_source_job_cancels_with_a_full_reply_queue_and_releases_its_source() {
    let (_, _, pair) = sources();
    let (old, new, replacement) = sources();
    let syntaxes = two_face::syntax::extra_newlines();
    let a = sequential(&old, Appearance::Dark, &syntaxes);
    let b = sequential(&new, Appearance::Dark, &syntaxes);
    let (wake, notified) = mpsc::sync_channel(64);
    let worker = highlight::start(wake);
    send(&worker, &pair, vec![context(11905)], 0, Appearance::Dark);
    notified.recv_timeout(Duration::from_secs(20)).unwrap();
    check(&receive(&worker, &notified), &pair, 11905, 0, &a, &b);
    while notified.try_recv().is_ok() {}
    send(
        &worker,
        &pair,
        (11880..11920).map(context).collect(),
        1,
        Appearance::Light,
    );
    for _ in 0..16 {
        notified.recv_timeout(Duration::from_secs(20)).unwrap();
    }
    assert_eq!(
        notified.recv_timeout(Duration::from_millis(20)),
        Err(mpsc::RecvTimeoutError::Timeout)
    );
    let weak = Arc::downgrade(&pair);
    worker.cancel();
    drop(pair);
    send(
        &worker,
        &replacement,
        vec![context(11009)],
        2,
        Appearance::Dark,
    );
    loop {
        let reply = receive(&worker, &notified);
        if reply.epoch == 1 {
            continue;
        }
        check(&reply, &replacement, 11009, 2, &a, &b);
        break;
    }
    assert!(
        weak.upgrade().is_none(),
        "canceled job retained its complete source"
    );
}

#[cfg(feature = "delivery-trace")]
#[test]
fn sparse_pressure_reuses_parser_work_without_losing_replies() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("trace.json");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "sparse_pressure_keeps_exact_side_states_across_revisits_and_themes",
        ])
        .env("NATIVE_DIFF_DELIVERY_TRACE", &path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let trace: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(trace["dropped"], 0);
    assert_eq!(trace["requests_dropped"], 0);
    let requests = trace["requests"].as_array().unwrap();
    assert!(requests.len() >= 4);
    for (index, request) in requests[..4].iter().enumerate() {
        assert_eq!(request["generation"], index + 1);
        assert_eq!(request["outcome"], "complete");
        assert_eq!(request["style_replies"], 9);
        assert_eq!(request["fallback_requested"], 0);
    }
    let opening = requests[0]["complete_source_lines"].as_u64().unwrap();
    for request in &requests[1..4] {
        assert!(
            request["complete_source_lines"].as_u64().unwrap() < opening,
            "revisit repeated the entire source prefix"
        );
    }
}
