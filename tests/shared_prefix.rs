use std::{
    fs,
    process::Command,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

use native_diff::{
    highlight::{self, Request, Token},
    storage::{self, FileIndex},
    theme::{self, Appearance},
};
use ratatui::style::Color;
use syntect::{easy::HighlightLines, parsing::SyntaxSet};

fn patch(old: &str, new: &str, extension: &str) -> Arc<FileIndex> {
    let dir = tempfile::tempdir().unwrap();
    let old_path = format!("old.{extension}");
    let new_path = format!("new.{extension}");
    fs::write(dir.path().join(&old_path), old).unwrap();
    fs::write(dir.path().join(&new_path), new).unwrap();
    let output = Command::new("git")
        .args([
            "diff",
            "--no-index",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--unified=100000",
            &old_path,
            &new_path,
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let mut files = Vec::new();
    storage::read(output.stdout.as_slice(), |file| {
        files.push(file);
        Ok(())
    })
    .unwrap();
    assert_eq!(files.len(), 1);
    files.pop().unwrap()
}

fn sequential(text: &str, syntaxes: &SyntaxSet, appearance: Appearance) -> Vec<Vec<Token>> {
    let (_, theme) = theme::load(appearance).unwrap();
    let syntax = syntaxes.find_syntax_by_extension("rb").unwrap();
    let mut reference = HighlightLines::new(syntax, &theme);
    text.split_inclusive('\n')
        .map(|line| {
            let mut offset = 0;
            reference
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

fn replies(worker: &highlight::Worker, count: usize) -> Vec<highlight::Reply> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut result = Vec::new();
    while result.len() < count {
        assert!(Instant::now() < deadline, "syntax worker did not finish");
        match worker.try_reply() {
            Ok(reply) => result.push(reply.unwrap()),
            Err(mpsc::TryRecvError::Empty) => std::thread::sleep(Duration::from_millis(1)),
            Err(error) => panic!("syntax worker disconnected: {error}"),
        }
    }
    result
}

#[test]
fn shared_captured_prefix_revisits_preserve_both_histories() {
    let syntaxes = two_face::syntax::extra_newlines();
    for prefix_len in [255, 256, 257, 520] {
        let prefix = format!(
            "value = <<STOP\n{}",
            (1..prefix_len)
                .map(|n| format!("return 1 # common_{n}\n"))
                .collect::<String>()
        );
        let tail = (0..320)
            .map(|n| format!("return 2 # tail_{n}\n"))
            .collect::<String>();
        for close_old in [true, false] {
            let closed = format!("{prefix}STOP\n{tail}STOP\nreturn 3\n");
            let open = format!("{prefix}not_closed\n{tail}STOP\nreturn 3\n");
            let (old, new) = if close_old {
                (&closed, &open)
            } else {
                (&open, &closed)
            };
            let file = patch(old, new, "rb");
            let (wake, _notified) = mpsc::sync_channel(1);
            let worker = highlight::start(wake);
            for (epoch, appearance) in [(0, Appearance::Dark), (1, Appearance::Light)] {
                let old_expected = sequential(old, &syntaxes, appearance);
                let new_expected = sequential(new, &syntaxes, appearance);
                for ids in [
                    vec![file.source_end() - 2, file.source_end() - 1],
                    vec![prefix_len - 2, prefix_len - 1],
                    (prefix_len..prefix_len + 5).collect(),
                    vec![file.source_end() - 1],
                ] {
                    worker.request(Request {
                        epoch,
                        appearance,
                        ids: ids.clone(),
                        files: vec![file.clone().into()],
                    });
                    let actual = replies(&worker, ids.len());
                    for (id, reply) in ids.into_iter().zip(actual) {
                        assert_eq!((reply.epoch, reply.id), (epoch, id.into()));
                        assert!(!reply.style.fallback);
                        let source = file.info(id);
                        for (old_side, line, expected) in [
                            (true, source.old, &old_expected),
                            (false, source.new, &new_expected),
                        ] {
                            if let Some(line) = line {
                                assert_eq!(
                                    reply.style.tokens_for(old_side),
                                    &expected[(line - 1) as usize],
                                    "prefix={prefix_len}, close_old={close_old}, source={id}, old={old_side}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn canceled_coloring_revisits_match_sequential_old_and_new_histories() {
    let syntaxes = two_face::syntax::extra_newlines();
    let prefix = format!(
        "value = <<STOP\n{}",
        (1..257)
            .map(|n| format!("return 1 # common_{n}\n"))
            .collect::<String>()
    );
    let tail = (0..320)
        .map(|n| format!("return 2 # tail_{n}\n"))
        .collect::<String>();
    let closed = format!("{prefix}STOP\n{tail}STOP\nreturn 3\n");
    let open = format!("{prefix}not_closed\n{tail}STOP\nreturn 3\n");
    for close_old in [true, false] {
        let (old, new) = if close_old {
            (&closed, &open)
        } else {
            (&open, &closed)
        };
        let file = patch(old, new, "rb");
        for appearance in [Appearance::Dark, Appearance::Light] {
            let old_expected = sequential(old, &syntaxes, appearance);
            let new_expected = sequential(new, &syntaxes, appearance);
            let (wake, notified) = mpsc::sync_channel(64);
            let worker = highlight::start(wake);
            worker.request(Request {
                epoch: 0,
                appearance,
                ids: (260..324).collect(),
                files: vec![file.clone().into()],
            });
            for _ in 0..16 {
                notified.recv_timeout(Duration::from_secs(5)).unwrap();
            }
            worker.cancel();
            // First consume the replies published before cancellation.
            for (index, ids) in [
                (260..276).collect::<Vec<_>>(),
                vec![file.source_end() - 2, file.source_end() - 1],
                vec![254, 255, 256],
                (257..280).collect(),
                vec![file.source_end() - 1],
            ]
            .into_iter()
            .enumerate()
            {
                if index != 0 {
                    worker.request(Request {
                        epoch: 0,
                        appearance,
                        ids: ids.clone(),
                        files: vec![file.clone().into()],
                    });
                }
                for (id, reply) in ids.iter().zip(replies(&worker, ids.len())) {
                    assert_eq!((reply.epoch, reply.id), (0, (*id).into()));
                    assert!(!reply.style.fallback);
                    let info = file.info(*id);
                    for (old_side, line, expected) in [
                        (true, info.old, &old_expected),
                        (false, info.new, &new_expected),
                    ] {
                        if let Some(line) = line {
                            assert_eq!(
                                reply.style.tokens_for(old_side),
                                &expected[(line - 1) as usize],
                                "close_old={close_old}, source={id}, old={old_side}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn first_side_only_long_line_does_not_invalidate_the_other_side() {
    let prefix = (0..260)
        .map(|n| format!("const value_{n} = true;\n"))
        .collect::<String>();
    let short = format!("{prefix}return 2;\n");
    let long = format!("{prefix}{}\nreturn 2;\n", "x".repeat(10_002));
    for long_old in [true, false] {
        let (old, new) = if long_old {
            (&long, &short)
        } else {
            (&short, &long)
        };
        let file = patch(old, new, "ts");
        let (wake, _notified) = mpsc::sync_channel(1);
        let worker = highlight::start(wake);
        for id in [file.source_end() - 1, 258, file.source_end() - 1] {
            worker.request(Request {
                epoch: 0,
                appearance: Appearance::Dark,
                ids: vec![id],
                files: vec![file.clone().into()],
            });
            let reply = replies(&worker, 1).pop().unwrap();
            assert_eq!(reply.id, id.into());
            if id == 258 {
                assert!(!reply.style.fallback);
                continue;
            }
            assert!(reply.style.fallback);
            assert!(reply.style.tokens_for(long_old).is_empty());
            assert!(
                reply
                    .style
                    .tokens_for(!long_old)
                    .iter()
                    .any(|token| token.range.start == 0
                        && token.range.end == 6
                        && token.color == Color::Rgb(255, 103, 141))
            );
        }
    }
}
