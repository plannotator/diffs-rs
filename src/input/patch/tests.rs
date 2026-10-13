use super::*;
use nix::{
    sys::stat::Mode,
    unistd::{mkfifo, pipe},
};
use std::{
    io::{BufRead, BufReader, Write},
    sync::mpsc,
    thread,
    time::Duration,
};

fn fifo() -> (tempfile::TempDir, std::path::PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("input.patch");
    mkfifo(&path, Mode::S_IRUSR | Mode::S_IWUSR).unwrap();
    (directory, path)
}

fn reader(path: &std::path::Path, cancel: Cancellation) -> Reader {
    Reader::new(
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)
            .unwrap(),
        cancel,
    )
    .unwrap()
}

fn flags(file: &File) -> OFlag {
    OFlag::from_bits_retain(fcntl(file.as_raw_fd(), FcntlArg::F_GETFL).unwrap())
}

#[test]
fn fifo_waits_without_a_writer_and_finishes_an_empty_producer() {
    let (_directory, path) = fifo();
    let mut input = reader(&path, Cancellation::default());
    let (send, done) = mpsc::sync_channel(1);
    let join = thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = input.read_to_end(&mut bytes);
        send.send((result, bytes)).unwrap();
    });
    assert!(matches!(
        done.recv_timeout(Duration::from_millis(100)),
        Err(mpsc::RecvTimeoutError::Timeout)
    ));
    drop(OpenOptions::new().write(true).open(&path).unwrap());
    let (result, bytes) = done.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(result.unwrap(), 0);
    assert!(bytes.is_empty());
    join.join().unwrap();

    // The empty writer can finish before the reader's first readiness wait.
    let mut input = reader(&path, Cancellation::default());
    drop(OpenOptions::new().write(true).open(&path).unwrap());
    assert_eq!(input.read(&mut []).unwrap(), 0);
    assert_eq!(input.read(&mut [0; 1]).unwrap(), 0);
    assert_eq!(input.read(&mut [0; 1]).unwrap(), 0);
}

#[test]
fn fifo_cancellation_covers_unconnected_idle_and_partial_line_after_unlink() {
    for state in ["unconnected", "idle", "partial"] {
        let (_directory, path) = fifo();
        let cancel = Cancellation::default();
        let input = reader(&path, cancel.clone());
        let mut writer =
            (state != "unconnected").then(|| OpenOptions::new().write(true).open(&path).unwrap());
        if state == "partial" {
            writer
                .as_mut()
                .unwrap()
                .write_all(b"diff --git a/")
                .unwrap();
        }
        std::fs::remove_file(&path).unwrap();
        let (send, done) = mpsc::sync_channel(1);
        let join = thread::spawn(move || {
            let result = BufReader::new(input).read_until(b'\n', &mut Vec::new());
            send.send(result).unwrap();
        });
        assert!(matches!(
            done.recv_timeout(Duration::from_millis(100)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        cancel.stop();
        let error = done
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap_err();
        assert_ne!(error.kind(), io::ErrorKind::Interrupted);
        assert!(error.to_string().contains("canceled"));
        join.join().unwrap();
        drop(writer);
    }
}

#[test]
fn owned_pipe_preserves_eof_bytes_and_original_flags_on_completion_and_cancellation() {
    for nonblocking in [false, true] {
        for state in ["empty", "bytes", "partial"] {
            let (read_end, write_end) = pipe().unwrap();
            let observed = File::from(read_end);
            if nonblocking {
                fcntl(
                    observed.as_raw_fd(),
                    FcntlArg::F_SETFL(flags(&observed) | OFlag::O_NONBLOCK),
                )
                .unwrap();
            }
            let before = flags(&observed);
            let cancel = Cancellation::default();
            let mut input = Reader::new(observed.try_clone().unwrap(), cancel.clone()).unwrap();
            assert!(flags(&observed).contains(OFlag::O_NONBLOCK));
            let mut writer = File::from(write_end);
            if state != "empty" {
                writer.write_all(b"exact\t\xe6\x97\xa5\npartial").unwrap();
            }
            let writer = (state == "partial").then_some(writer);
            let (send, done) = mpsc::sync_channel(1);
            let join = thread::spawn(move || {
                let mut bytes = Vec::new();
                let result = input.read_to_end(&mut bytes);
                input.restore().unwrap();
                send.send((result, bytes)).unwrap();
            });
            if state == "partial" {
                assert!(matches!(
                    done.recv_timeout(Duration::from_millis(100)),
                    Err(mpsc::RecvTimeoutError::Timeout)
                ));
                cancel.stop();
            }
            let (result, bytes) = done.recv_timeout(Duration::from_secs(5)).unwrap();
            assert_eq!(result.is_err(), state == "partial");
            assert_eq!(
                bytes,
                if state == "empty" {
                    b"".as_slice()
                } else {
                    b"exact\t\xe6\x97\xa5\npartial"
                }
            );
            join.join().unwrap();
            assert_eq!(flags(&observed), before);
            drop(writer);
        }
    }
    assert_eq!(
        read(
            PatchInput::Path("/dev/null".into()),
            &Cancellation::default(),
            |_| Ok(())
        )
        .unwrap(),
        0
    );
}

#[test]
fn parser_failure_publication_failure_and_unwind_restore_stream_flags() {
    let patch = b"diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-old\n+new\n";
    for mode in ["valid", "malformed", "publication", "unwind"] {
        let (read_end, write_end) = pipe().unwrap();
        let observed = File::from(read_end);
        let before = flags(&observed);
        let mut writer = File::from(write_end);
        writer
            .write_all(if mode == "malformed" {
                &patch[..patch.len() - 4]
            } else {
                patch
            })
            .unwrap();
        drop(writer);
        let input = PatchInput::Stream(observed.try_clone().unwrap());
        let mut files = Vec::new();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            read(input, &Cancellation::default(), |file| {
                if mode == "unwind" {
                    panic!("consumer stopped during publication");
                }
                if mode == "publication" {
                    anyhow::bail!("consumer rejected publication");
                }
                files.push(file);
                Ok(())
            })
        }));
        assert_eq!(flags(&observed), before);
        if mode == "unwind" {
            assert!(result.is_err());
        } else if mode == "valid" {
            assert_eq!(result.unwrap().unwrap(), 1);
            assert_eq!(files[0].bytes(0).unwrap(), b"old\n");
            assert_eq!(files[0].bytes(1).unwrap(), b"new\n");
        } else {
            assert!(result.unwrap().is_err());
            assert!(files.is_empty());
        }
    }
}
