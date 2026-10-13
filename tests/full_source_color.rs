use std::{
    fs,
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};

use native_diff::{
    app::{App, Layout},
    highlight::{self, Fallback, FileSource, Request, Token},
    input::{Cancellation, LocalInput, Review, SourceState},
    source::CheckedPair,
    storage,
    text::display_text,
    theme::{self, Appearance},
};
use ratatui::style::Color;
use syntect::{easy::HighlightLines, parsing::SyntaxSet};

fn capture(old: &[u8], new: &[u8], old_ext: &str, new_ext: &str) -> Arc<CheckedPair> {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join(format!("old.{old_ext}"));
    let b = dir.path().join(format!("new.{new_ext}"));
    fs::write(&a, old).unwrap();
    fs::write(&b, new).unwrap();
    let mut review = Review::open(
        LocalInput::Files { old: a, new: b },
        &Cancellation::default(),
        |_| Ok(()),
    )
    .unwrap();
    let SourceState::Checked(pair) = review.prepare(0, &Cancellation::default()).unwrap() else {
        panic!("source not checked");
    };
    pair
}

fn fixture(ext: &str, count: usize) -> (String, String) {
    let (opening, closing, body) = match ext {
        "py" => ("text = '''", "'''", "return 1 # body"),
        "rb" => ("text = <<STOP", "STOP", "return 1 # body"),
        "html" => ("<script>/*", "*/", "return 1; // body"),
        _ => ("/*", "*/", "return 1; // body"),
    };
    let mut a = vec![body.to_owned(); count];
    a[0] = opening.into();
    a[300] = closing.into();
    a[900] = closing.into();
    let mut b = a.clone();
    b[300] = "not closed".into();
    b[530] = body.replace('1', "2");
    b[count - 4] = body.replace('1', "3");
    (a.join("\n") + "\n", b.join("\n") + "\n")
}

fn sequential(
    bytes: &[u8],
    ext: &str,
    appearance: Appearance,
    syntaxes: &SyntaxSet,
) -> Vec<Vec<Token>> {
    let (_, theme) = theme::load(appearance).unwrap();
    let syntax = syntaxes.find_syntax_by_extension(ext).unwrap();
    let mut parser = HighlightLines::new(syntax, &theme);
    bytes
        .split_inclusive(|b| *b == b'\n')
        .map(|line| {
            let text = format!("{}\n", display_text(line));
            let mut offset = 0;
            parser
                .highlight_line(&text, syntaxes)
                .unwrap()
                .into_iter()
                .map(|(style, part)| {
                    let start = offset;
                    offset += part.len();
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

fn receive(worker: &highlight::Worker) -> highlight::Reply {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        assert!(
            Instant::now() < deadline,
            "source syntax worker did not finish"
        );
        match worker.try_reply() {
            Ok(reply) => return reply.unwrap(),
            Err(mpsc::TryRecvError::Empty) => thread::sleep(Duration::from_millis(1)),
            Err(error) => panic!("{error}"),
        }
    }
}

fn request(
    worker: &highlight::Worker,
    source: FileSource,
    ids: Vec<usize>,
    epoch: u64,
    appearance: Appearance,
) {
    worker.request(Request {
        epoch,
        appearance,
        ids,
        files: vec![source],
    });
}

#[test]
fn hidden_runs_preserve_converging_and_divergent_histories_with_shifted_coordinates() {
    use native_diff::view::LineId;
    #[cfg(feature = "delivery-trace")]
    let trace = native_diff::delivery_trace::init().unwrap();
    let syntaxes = two_face::syntax::extra_newlines();
    for (old_ext, new_ext) in [
        ("ts", "ts"),
        ("py", "py"),
        ("rb", "rb"),
        ("html", "html"),
        ("ts", "rs"),
    ] {
        let (open, close) = match old_ext {
            "py" => ("text = '''", "'''"),
            "rb" => ("text = <<STOP", "STOP"),
            "html" => ("<script>/*", "*/"),
            _ => ("/*", "*/"),
        };
        let mut old: Vec<_> = (1..=2700)
            .map(|n| format!("return {n}; // source-{n}\t日本語 café"))
            .collect();
        old[0] = open.into();
        old[99] = close.into();
        old[299] = close.into();
        old[1300].push_str(" \u{1b}");
        let mut new = old.clone();
        new[99] = "not closed".into();
        new[1900] = "return -1901;".into();
        new[2696] = "return -2697;".into();
        new.insert(1000, "inserted source".into());
        new.remove(1501);
        let old = old.join("\r\n");
        let new = new.join("\r\n");
        let pair = capture(old.as_bytes(), new.as_bytes(), old_ext, new_ext);
        let ids: Vec<_> = [64, 256, 768, 1024, 1280, 1792, 2400]
            .into_iter()
            .map(|old| {
                LineId::context(0, old, old + u32::from((1001..=1500).contains(&old))).unwrap()
            })
            .collect();
        let (wake, _) = mpsc::sync_channel(1);
        let worker = highlight::start(wake);
        for (epoch, appearance) in [
            (0, Appearance::Dark),
            (1, Appearance::Light),
            (2, Appearance::Dark),
        ] {
            let a = sequential(old.as_bytes(), old_ext, appearance, &syntaxes);
            let b = sequential(new.as_bytes(), new_ext, appearance, &syntaxes);
            for wanted in [
                ids.clone(),
                vec![ids[1]],
                ids.iter().rev().copied().collect(),
            ] {
                worker.request(Request {
                    epoch,
                    appearance,
                    ids: wanted.clone(),
                    files: vec![FileSource::Complete(pair.clone())],
                });
                let mut received = std::collections::HashSet::new();
                for _ in &wanted {
                    let reply = receive(&worker);
                    assert!(wanted.contains(&reply.id) && received.insert(reply.id));
                    assert_eq!(reply.epoch, epoch);
                    assert_eq!(reply.origin, FileSource::Complete(pair.clone()).origin());
                    assert!(reply.source_error.is_none() && !reply.style.fallback);
                    let LineId::Context { old, new, .. } = reply.id else {
                        unreachable!()
                    };
                    assert_eq!(reply.style.tokens_for(true), a[old.get() as usize - 1]);
                    assert_eq!(reply.style.tokens_for(false), b[new.get() as usize - 1]);
                    if old.get() == 256 && old_ext == new_ext {
                        assert_ne!(reply.style.tokens_for(true), reply.style.tokens_for(false));
                    }
                }
            }
        }
        worker.request(Request {
            epoch: 2,
            appearance: Appearance::Dark,
            ids: vec![ids[0]],
            files: vec![FileSource::Complete(pair)],
        });
        assert_eq!(receive(&worker).id, ids[0]);
    }
    #[cfg(feature = "delivery-trace")]
    drop(trace);
}

#[cfg(feature = "delivery-trace")]
#[test]
fn shared_hidden_work_counts_real_calls_and_rejects_incompatible_histories() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("trace.json");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "hidden_runs_preserve_converging_and_divergent_histories_with_shifted_coordinates",
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
    assert_eq!(trace["schema"], 6);
    assert_eq!(trace["dropped"], 0);
    assert_eq!(trace["requests_dropped"], 0);
    let requests = trace["requests"].as_array().unwrap();
    let initial: Vec<_> = requests.iter().filter(|r| r["generation"] == 1).collect();
    assert_eq!(initial.len(), 5);
    assert!(initial[0]["shared_source_lines"].as_u64().unwrap() > 99);
    assert!(initial[1]["shared_source_lines"].as_u64().unwrap() <= 99);
    assert_eq!(initial[4]["shared_source_lines"], 0);
    for request in requests {
        assert_eq!(request["outcome"], "complete");
        assert_eq!(request["fallback_requested"], 0);
        let calls = request["parse_requested"]["calls"].as_u64().unwrap()
            + request["parse_replay"]["calls"].as_u64().unwrap();
        let shared = request["shared_source_lines"].as_u64().unwrap();
        assert_eq!(request["complete_source_lines"], calls + shared);
    }
}

#[test]
fn sparse_checkpoint_boundaries_match_sequential_colors_on_revisit_and_retheme() {
    use native_diff::view::LineId;
    #[cfg(feature = "delivery-trace")]
    let trace = native_diff::delivery_trace::init().unwrap();
    let (old, new) = fixture("ts", 6000);
    let pair = capture(old.as_bytes(), new.as_bytes(), "ts", "ts");
    let syntaxes = two_face::syntax::extra_newlines();
    let (wake, _) = mpsc::sync_channel(1);
    let worker = highlight::start(wake);
    let ids: Vec<_> = [
        255, 256, 257, 1535, 1536, 1537, 3583, 3584, 3585, 5631, 5632, 5633,
    ]
    .into_iter()
    .map(|number| LineId::context(0, number, number).unwrap())
    .collect();
    for (epoch, appearance) in [
        (0, Appearance::Dark),
        (0, Appearance::Dark),
        (1, Appearance::Light),
        (2, Appearance::Dark),
    ] {
        let a = sequential(old.as_bytes(), "ts", appearance, &syntaxes);
        let b = sequential(new.as_bytes(), "ts", appearance, &syntaxes);
        worker.request(Request {
            epoch,
            appearance,
            ids: ids.clone(),
            files: vec![FileSource::Complete(pair.clone())],
        });
        for &id in &ids {
            let reply = receive(&worker);
            assert_eq!(reply.id, id);
            assert_eq!(reply.epoch, epoch);
            assert!(reply.source_error.is_none() && !reply.style.fallback);
            let LineId::Context { old, new, .. } = id else {
                unreachable!()
            };
            assert_eq!(reply.style.tokens_for(true), a[old.get() as usize - 1]);
            assert_eq!(reply.style.tokens_for(false), b[new.get() as usize - 1]);
        }
    }
    let a = sequential(old.as_bytes(), "ts", Appearance::Dark, &syntaxes);
    let b = sequential(new.as_bytes(), "ts", Appearance::Dark, &syntaxes);
    for offset in [0, 1] {
        let ids: Vec<_> = [256, 1536, 3584, 5632]
            .into_iter()
            .map(|n| LineId::context(0, n + offset, n + offset).unwrap())
            .collect();
        worker.request(Request {
            epoch: 2,
            appearance: Appearance::Dark,
            ids: ids.clone(),
            files: vec![FileSource::Complete(pair.clone())],
        });
        for id in ids {
            let reply = receive(&worker);
            assert_eq!(reply.id, id);
            assert!(reply.source_error.is_none() && !reply.style.fallback);
            let LineId::Context { old, new, .. } = id else {
                unreachable!()
            };
            assert_eq!(reply.style.tokens_for(true), a[old.get() as usize - 1]);
            assert_eq!(reply.style.tokens_for(false), b[new.get() as usize - 1]);
        }
    }
    // A subsequent reply proves the preceding request has left its work scope.
    worker.request(Request {
        epoch: 2,
        appearance: Appearance::Dark,
        ids: vec![ids[0]],
        files: vec![FileSource::Complete(pair)],
    });
    assert_eq!(receive(&worker).id, ids[0]);
    drop(worker);
    #[cfg(feature = "delivery-trace")]
    drop(trace);
}

#[cfg(feature = "delivery-trace")]
#[test]
fn sparse_revisits_and_theme_changes_reuse_saved_hidden_source_work() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("trace.json");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "sparse_checkpoint_boundaries_match_sequential_colors_on_revisit_and_retheme",
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
    let trace: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(trace["dropped"], 0);
    assert_eq!(trace["requests_dropped"], 0);
    let requests = trace["requests"].as_array().unwrap();
    assert!(requests.len() >= 4);
    for request in &requests[..4] {
        assert_eq!(request["outcome"], "complete");
        assert_eq!(request["style_replies"], 12);
        assert_eq!(request["fallback_requested"], 0);
    }
    assert_eq!(requests[0]["complete_source_lines"], 2 * 5633);
    for request in &requests[1..4] {
        let lines = request["complete_source_lines"].as_u64().unwrap();
        assert!(
            lines <= 2 * 4 * 257,
            "Compatible sparse request replayed {lines} lines"
        );
    }
}

#[test]
fn expanded_context_uses_exact_old_and_new_prefixes_with_mixed_patch_requests() {
    use native_diff::view::LineId;
    let mut a: Vec<_> = (0..1400)
        .map(|n| format!("return {n}; // unique line {n}"))
        .collect();
    a[0] = "/*".into();
    a[300] = "*/".into();
    a[900] = "*/".into();
    let mut b = a.clone();
    b[300] = "not closed".into();
    b[530] = "return -530;".into();
    b[1396] = "return -1396;".into();
    let old = a.join("\n") + "\n";
    let new = b.join("\n") + "\n";
    let pair = capture(old.as_bytes(), new.as_bytes(), "ts", "ts");
    let syntaxes = two_face::syntax::extra_newlines();
    let (wake, _) = mpsc::sync_channel(1);
    let worker = highlight::start(wake);
    let context = |number| LineId::context(0, number, number).unwrap();
    let patch = pair.file().hunks[1].lines.start;
    let ids = vec![context(1050), patch.into(), context(350), context(750)];
    for (epoch, appearance) in [
        (0, Appearance::Dark),
        (1, Appearance::Light),
        (2, Appearance::Dark),
    ] {
        let a = sequential(old.as_bytes(), "ts", appearance, &syntaxes);
        let b = sequential(new.as_bytes(), "ts", appearance, &syntaxes);
        for wanted in [ids.clone(), vec![context(350)], ids.clone()] {
            worker.request(Request {
                epoch,
                appearance,
                ids: wanted.clone(),
                files: vec![FileSource::Complete(pair.clone())],
            });
            let mut received = std::collections::HashSet::new();
            for _ in &wanted {
                let reply = receive(&worker);
                assert!(
                    reply.source_error.is_none() && !reply.style.fallback,
                    "{:?}: {:?} {:?}",
                    reply.id,
                    reply.source_error,
                    reply.style.fallback_reason
                );
                assert!(wanted.contains(&reply.id) && received.insert(reply.id));
                let (old_number, new_number) = match reply.id {
                    LineId::Patch(id) => {
                        let info = pair.file().info(id);
                        (info.old, info.new)
                    }
                    LineId::Context { old, new, .. } => (Some(old.get()), Some(new.get())),
                };
                for (side, number, expected) in [(true, old_number, &a), (false, new_number, &b)] {
                    if let Some(number) = number {
                        assert_eq!(reply.style.tokens_for(side), expected[number as usize - 1]);
                    }
                }
                if reply.id == context(350) {
                    assert_ne!(reply.style.tokens_for(true), reply.style.tokens_for(false));
                }
                if matches!(reply.id, LineId::Context { .. }) {
                    assert!(reply.style.changed.is_empty());
                }
            }
        }
    }
    worker.request(Request {
        epoch: 3,
        appearance: Appearance::Dark,
        ids: vec![LineId::context(0, 750, 751).unwrap()],
        files: vec![FileSource::Complete(pair)],
    });
    let reply = receive(&worker);
    assert!(
        reply
            .source_error
            .as_deref()
            .unwrap()
            .contains("does not match")
    );
    assert!(reply.style.tokens.is_empty());
}

#[test]
fn replacing_hidden_source_cannot_reuse_checkpoints_for_the_same_patch_entry() {
    use native_diff::source::{SourceStore, check_pair};

    let old = format!("/*\n{}return 1;\n", "return 1;\n".repeat(1200));
    let new = format!("/*\n{}return 2;\n", "return 1;\n".repeat(1200));
    let original = capture(old.as_bytes(), new.as_bytes(), "ts", "ts");
    let replaced_old = old.replacen("/*", "//", 1);
    let replaced_new = new.replacen("/*", "//", 1);
    let file = original.file();
    let mut store = SourceStore::sharing(&file.spool).unwrap();
    let replaced = Arc::new(
        check_pair(
            file.clone(),
            Some(store.capture(replaced_old.as_bytes(), &|| Ok(())).unwrap()),
            Some(store.capture(replaced_new.as_bytes(), &|| Ok(())).unwrap()),
            &|| Ok(()),
        )
        .unwrap(),
    );
    assert!(Arc::ptr_eq(original.file(), replaced.file()));
    assert_ne!(
        FileSource::Complete(original.clone()).origin(),
        FileSource::Complete(replaced.clone()).origin()
    );
    let syntaxes = two_face::syntax::extra_newlines();
    let (wake, _) = mpsc::sync_channel(1);
    let worker = highlight::start(wake);
    let id = file.hunks[0].lines.start;
    for (epoch, appearance, pair, old, new) in [
        (0, Appearance::Dark, &original, &old, &new),
        (0, Appearance::Dark, &replaced, &replaced_old, &replaced_new),
        (1, Appearance::Light, &original, &old, &new),
        (
            1,
            Appearance::Light,
            &replaced,
            &replaced_old,
            &replaced_new,
        ),
    ] {
        request(
            &worker,
            FileSource::Complete(pair.clone()),
            vec![id],
            epoch,
            appearance,
        );
        let reply = receive(&worker);
        assert_eq!(reply.id, id.into());
        assert!(reply.source_error.is_none() && !reply.style.fallback);
        for (side, text, number) in [
            (true, old, file.info(id).old),
            (false, new, file.info(id).new),
        ] {
            let expected = sequential(text.as_bytes(), "ts", appearance, &syntaxes);
            assert_eq!(
                reply.style.tokens_for(side),
                expected[number.unwrap() as usize - 1]
            );
        }
    }
}

#[test]
fn hidden_syntax_boundaries_match_full_files_across_checkpoints_sides_and_themes() {
    let syntaxes = two_face::syntax::extra_newlines();
    for ext in ["ts", "py", "rb", "html"] {
        let (old, new) = fixture(ext, 1400);
        let pair = capture(old.as_bytes(), new.as_bytes(), ext, ext);
        let file = pair.file();
        assert!(file.hunks.len() >= 3);
        assert!(file.hunks[0].old_start > 1, "opening syntax must be hidden");
        let (wake, _) = mpsc::sync_channel(1);
        let worker = highlight::start(wake);
        let mut explicit_roles = 0;
        for (epoch, appearance) in [
            (0, Appearance::Dark),
            (1, Appearance::Light),
            (2, Appearance::Dark),
        ] {
            let a = sequential(old.as_bytes(), ext, appearance, &syntaxes);
            let b = sequential(new.as_bytes(), ext, appearance, &syntaxes);
            for h in [2, 1, 0, 1, 2] {
                let ids: Vec<_> = file.hunks[h].lines.clone().collect();
                request(
                    &worker,
                    FileSource::Complete(pair.clone()),
                    ids.clone(),
                    epoch,
                    appearance,
                );
                for id in ids {
                    let reply = receive(&worker);
                    assert_eq!(reply.id, id.into());
                    assert!(
                        !reply.style.fallback,
                        "{ext}: {:?}",
                        reply.style.fallback_reason
                    );
                    let info = file.info(id);
                    for (old_side, number, expected) in
                        [(true, info.old, &a), (false, info.new, &b)]
                    {
                        if let Some(number) = number {
                            assert_eq!(
                                reply.style.tokens_for(old_side),
                                expected[number as usize - 1],
                                "{ext} {number} old={old_side}"
                            );
                        }
                    }
                    if info.old.is_some_and(|n| (527..=535).contains(&n))
                        && info.new.is_some_and(|n| (527..=535).contains(&n))
                    {
                        explicit_roles += 1;
                        let old_color = reply
                            .style
                            .tokens_for(true)
                            .iter()
                            .find(|t| t.range.contains(&0))
                            .unwrap()
                            .color;
                        let new_color = reply
                            .style
                            .tokens_for(false)
                            .iter()
                            .find(|t| t.range.contains(&0))
                            .unwrap()
                            .color;
                        assert_eq!(
                            old_color,
                            if appearance == Appearance::Dark {
                                Color::Rgb(255, 103, 141)
                            } else {
                                Color::Rgb(211, 42, 97)
                            }
                        );
                        let expected = if matches!(ext, "ts" | "html") {
                            Color::Rgb(115, 115, 115)
                        } else if appearance == Appearance::Dark {
                            Color::Rgb(94, 204, 113)
                        } else {
                            Color::Rgb(25, 159, 67)
                        };
                        assert_eq!(new_color, expected, "independent {ext} literal role");
                    }
                }
            }
        }
        assert!(
            explicit_roles > 0,
            "{ext}: no explicit syntax-role assertion ran"
        );
        let id = file.hunks[1]
            .lines
            .clone()
            .find(|id| file.info(*id).new.is_some_and(|n| (527..=535).contains(&n)))
            .unwrap();
        request(
            &worker,
            FileSource::Patch(file.clone()),
            vec![id],
            3,
            Appearance::Dark,
        );
        let incomplete = receive(&worker);
        let expected = sequential(new.as_bytes(), ext, Appearance::Dark, &syntaxes);
        assert_ne!(
            incomplete.style.tokens_for(false),
            expected[file.info(id).new.unwrap() as usize - 1],
            "{ext}: patch-only negative control did not differ"
        );
    }
}

#[test]
fn complete_source_revisits_survive_checkpoint_eviction() {
    let (old, new) = fixture("ts", 20_000);
    let pair = capture(old.as_bytes(), new.as_bytes(), "ts", "ts");
    let file = pair.file();
    let syntaxes = two_face::syntax::extra_newlines();
    let expected = sequential(new.as_bytes(), "ts", Appearance::Dark, &syntaxes);
    let (wake, _) = mpsc::sync_channel(1);
    let worker = highlight::start(wake);
    for h in [2, 0, 1, 2] {
        let ids: Vec<_> = file.hunks[h].lines.clone().collect();
        request(
            &worker,
            FileSource::Complete(pair.clone()),
            ids.clone(),
            0,
            Appearance::Dark,
        );
        for id in ids {
            let reply = receive(&worker);
            if let Some(n) = file.info(id).new {
                assert_eq!(reply.style.tokens, expected[n as usize - 1]);
            }
        }
    }
}

#[test]
fn complete_source_uses_each_extension_and_preserves_display_conversion() {
    let syntaxes = two_face::syntax::extra_newlines();
    for (a, b, old_ext, new_ext) in [
        (
            b"# comment\nreturn True\n".as_slice(),
            b"# comment\nreturn False\n".as_slice(),
            "py",
            "rs",
        ),
        (
            b"/*\r\nold\r\n\xff\t\x1b\n*/\nreturn 1;".as_slice(),
            b"/*\r\nnew\r\n\xff\t\x1b\n*/\nreturn 2;\n".as_slice(),
            "ts",
            "ts",
        ),
        (b"".as_slice(), b"return 1;\n".as_slice(), "ts", "ts"),
        (b"return 1;\n".as_slice(), b"".as_slice(), "ts", "ts"),
    ] {
        let pair = capture(a, b, old_ext, new_ext);
        let old = sequential(a, old_ext, Appearance::Dark, &syntaxes);
        let new = sequential(b, new_ext, Appearance::Dark, &syntaxes);
        let file = pair.file();
        let (wake, _) = mpsc::sync_channel(1);
        let worker = highlight::start(wake);
        let ids: Vec<_> = (file.source_base..file.source_end()).collect();
        request(
            &worker,
            FileSource::Complete(pair.clone()),
            ids.clone(),
            0,
            Appearance::Dark,
        );
        for id in ids {
            let reply = receive(&worker);
            for (side, number, expected) in [
                (true, file.info(id).old, &old),
                (false, file.info(id).new, &new),
            ] {
                if let Some(n) = number {
                    assert_eq!(reply.style.tokens_for(side), expected[n as usize - 1]);
                }
            }
        }
    }
}

#[test]
fn oversized_hidden_line_invalidates_only_its_side_through_later_sections() {
    let prefix = "return 0;\n".repeat(20);
    let suffix = "return 1;\n".repeat(600);
    let long = format!(
        "{prefix}{}\n{suffix}return 2;\n",
        "x".repeat(8 * 1024 * 1024)
    );
    let short = format!("{prefix}short\n{suffix}return 3;\n");
    for long_old in [true, false] {
        let (old, new) = if long_old {
            (&long, &short)
        } else {
            (&short, &long)
        };
        let pair = capture(old.as_bytes(), new.as_bytes(), "ts", "ts");
        let file = pair.file();
        assert_eq!(file.hunks.len(), 2);
        let (wake, _) = mpsc::sync_channel(1);
        let worker = highlight::start(wake);
        for h in [1, 0, 1] {
            let id = if h == 0 {
                file.hunks[0].lines.start
            } else {
                file.hunks[1].lines.start
            };
            request(
                &worker,
                FileSource::Complete(pair.clone()),
                vec![id],
                0,
                Appearance::Dark,
            );
            let reply = receive(&worker);
            assert_eq!(reply.style.fallback, h == 1);
            if h == 1 {
                assert_eq!(reply.style.fallback_reason, Some(Fallback::LongLine));
                assert!(reply.style.tokens_for(long_old).is_empty());
                assert_eq!(
                    reply.style.tokens_for(!long_old)[0].color,
                    Color::Rgb(255, 103, 141)
                );
            }
        }
    }
    let pair = capture(b"old\n", b"return 1;\n", "unknown-language", "ts");
    let (wake, _) = mpsc::sync_channel(1);
    let worker = highlight::start(wake);
    request(
        &worker,
        FileSource::Complete(pair.clone()),
        (0..pair.file().source_end()).collect(),
        0,
        Appearance::Dark,
    );
    let reply = receive(&worker);
    assert_eq!(
        reply.style.fallback_reason,
        Some(Fallback::UnsupportedLanguage)
    );
    assert!(reply.style.tokens.is_empty());
    assert!(!receive(&worker).style.fallback);
}

#[test]
fn identical_oversized_hidden_source_preserves_both_fallbacks() {
    for (length, insertion) in [(11_000, ""), (80_000, "const offset = 1;\n")] {
        let prefix = format!("{}{}\n", "return 1;\n".repeat(3_000), "x".repeat(length));
        let old = format!("{prefix}{}return 2;\n", "return 1;\n".repeat(600));
        let new = format!(
            "{insertion}{prefix}{}return 3;\n",
            "return 1;\n".repeat(600)
        );
        let pair = capture(old.as_bytes(), new.as_bytes(), "ts", "ts");
        let file = pair.file();
        let (wake, _) = mpsc::sync_channel(1);
        let worker = highlight::start(wake);
        let ids: Vec<_> = file.hunks.last().unwrap().lines.clone().collect();
        request(
            &worker,
            FileSource::Complete(pair.clone()),
            ids.clone(),
            0,
            Appearance::Dark,
        );
        for id in ids {
            let reply = receive(&worker);
            assert_eq!(reply.id, id.into());
            assert!(reply.source_error.is_none());
            assert_eq!(reply.style.fallback_reason, Some(Fallback::LongLine));
            assert!(reply.style.tokens_for(true).is_empty());
            assert!(reply.style.tokens_for(false).is_empty());
        }
    }
}

#[test]
fn real_queued_styles_cannot_cross_source_readiness_review_or_theme_boundaries() {
    let old = "return 1;\n".repeat(80);
    let new = "return 2;\n".repeat(80);
    let pair = capture(old.as_bytes(), new.as_bytes(), "ts", "ts");
    let file = pair.file().clone();
    let (wake, notified) = mpsc::sync_channel(64);
    let mut app = App::new(
        storage::Document {
            files: vec![file.clone()],
        },
        "origins".into(),
        Layout::Split,
        Appearance::Dark,
        wake,
    )
    .unwrap();
    app.sources = Some(vec![SourceState::Pending]);
    request(
        &app.worker,
        FileSource::Plain(file.clone()),
        (0..file.source_end()).collect(),
        0,
        Appearance::Dark,
    );
    for _ in 0..16 {
        notified.recv_timeout(Duration::from_secs(10)).unwrap();
    }
    assert_eq!(
        notified.recv_timeout(Duration::from_millis(20)),
        Err(mpsc::RecvTimeoutError::Timeout)
    );
    let plain = receive(&app.worker);
    assert!(plain.style.tokens.is_empty());
    assert!(!plain.style.changed.is_empty());
    assert!(app.accept_highlight(plain).unwrap());
    app.set_source(0, SourceState::Checked(pair.clone()))
        .unwrap();
    assert!(app.cache.get(0).is_none());
    while let Ok(reply) = app.worker.try_reply() {
        assert!(!app.accept_highlight(reply.unwrap()).unwrap());
    }
    let id = file.source_end() - 1;
    request(
        &app.worker,
        FileSource::Complete(pair.clone()),
        vec![id],
        0,
        Appearance::Dark,
    );
    let reply = receive(&app.worker);
    assert!(app.accept_highlight(reply).unwrap());
    assert!(!app.cache.get(id).unwrap().tokens.is_empty());
    let foreign = capture(old.as_bytes(), new.as_bytes(), "ts", "ts");
    assert!(
        app.set_source(0, SourceState::Checked(foreign.clone()))
            .is_err()
    );
    request(
        &app.worker,
        FileSource::Complete(foreign),
        vec![id],
        0,
        Appearance::Dark,
    );
    assert!(!app.accept_highlight(receive(&app.worker)).unwrap());
    request(
        &app.worker,
        FileSource::Complete(pair.clone()),
        vec![id],
        0,
        Appearance::Dark,
    );
    let reply = receive(&app.worker);
    app.toggle_theme().unwrap();
    assert!(!app.accept_highlight(reply).unwrap());
    while notified.try_recv().is_ok() {}
    request(
        &app.worker,
        FileSource::Complete(pair.clone()),
        (0..file.source_end()).collect(),
        1,
        Appearance::Light,
    );
    // Cancel with a full real reply queue. The worker must release request-owned snapshots.
    for _ in 0..16 {
        notified.recv_timeout(Duration::from_secs(10)).unwrap();
    }
    assert_eq!(
        notified.recv_timeout(Duration::from_millis(20)),
        Err(mpsc::RecvTimeoutError::Timeout)
    );
    app.worker.cancel();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Arc::strong_count(&pair) != 2 {
        assert!(
            Instant::now() < deadline,
            "source retained after cancellation"
        );
        thread::sleep(Duration::from_millis(1));
    }
}
