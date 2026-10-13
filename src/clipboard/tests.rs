use super::*;
use native_diff::{
    engine::{Engine, Input, Options, Side},
    reading::SourceRange,
};
use std::{
    fs,
    io::SeekFrom,
    os::unix::fs::{MetadataExt, PermissionsExt},
    sync::mpsc,
};

fn review(bytes: &[u8]) -> (Engine, tempfile::TempDir) {
    let directory = tempfile::tempdir().unwrap();
    let patch = directory.path().join("review.patch");
    fs::write(&patch, bytes).unwrap();
    let (notify, ready) = mpsc::sync_channel(1);
    let mut engine = Engine::open(Input::Patch(patch), Options::default(), notify).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while engine.status().loading {
        engine.poll().unwrap();
        assert!(Instant::now() < deadline);
        if engine.status().loading {
            let _ = ready.recv_timeout(Duration::from_millis(10));
        }
    }
    assert!(engine.status().error.is_none());
    (engine, directory)
}

fn source(engine: &mut Engine, first: u32, last: u32) -> Arc<SourceRead> {
    Arc::new(
        engine
            .source_read(SourceRange {
                review: engine.review_id(),
                file: 0,
                side: Side::Old,
                first,
                last,
            })
            .unwrap(),
    )
}

fn prepared(engine: &mut Engine, first: u32, last: u32) -> Prepared {
    Prepared::prepare(source(engine, first, last), &|| Ok(())).unwrap()
}

fn absent(pid: u32) {
    let output = Command::new("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", "pid="])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "helper {pid} still exists");
}

const PATCH: &[u8] = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1,3 +1 @@\n-{\\rtf1 plain source}\r\n-%!PS-Adobe-3.0 EPSF-3.0\n-\t\xe6\x97\xa5 e\xcc\x81\n\\ No newline at end of file\n+new\n";

#[test]
fn source_bytes_reach_a_real_child_without_format_detection_or_terminal_io() {
    let (mut engine, directory) = review(PATCH);
    let mut input = prepared(&mut engine, 1, 3);
    let expected = b"{\\rtf1 plain source}\r\n%!PS-Adobe-3.0 EPSF-3.0\n\t\xe6\x97\xa5 e\xcc\x81";
    let metadata = input.input.metadata().unwrap();
    assert_eq!(metadata.nlink(), 0);
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    assert_eq!(input.bytes(), expected.len() as u64);
    assert_eq!(
        text(&mut input.input, input.bytes).unwrap().as_bytes(),
        expected
    );
    input.input.rewind().unwrap();
    let saved = directory.path().join("received");
    let mut command = Command::new("/bin/sh");
    command
        .args(["-c", "exec /bin/cat > \"$1\"", "clipboard-check"])
        .arg(&saved);
    let delivery = input.start(engine.review_id(), command).unwrap();
    let pid = delivery.child.id();
    let receipt = delivery.finish(&|| Ok(())).unwrap();
    receipt.check(engine.review_id()).unwrap();
    assert_eq!(fs::read(saved).unwrap(), expected);
    assert_eq!(receipt.bytes, expected.len() as u64);
    absent(pid);
    engine.close().unwrap();
    assert!(receipt.check(engine.review_id()).is_err());
}

#[test]
fn utf8_is_checked_across_buffer_boundaries_without_changing_original_bytes() {
    for prefix in 65532..65540 {
        let mut original = vec![b'x'; prefix];
        original.extend_from_slice("日👩\u{200d}💻e\u{301}\t\r\n".as_bytes());
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(&original).unwrap();
        validate(&mut file, &|| Ok(())).unwrap();
        assert_eq!(file.stream_position().unwrap(), 0);
        assert_eq!(
            text(file, original.len() as u64).unwrap().as_bytes(),
            original
        );
    }
    for invalid in [
        b"a\xff".as_slice(),
        b"a\xe6\x97",
        b"\xf0\x80\x80\x80",
        b"\xed\xa0\x80",
    ] {
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(invalid).unwrap();
        assert!(
            validate(&mut file, &|| Ok(()))
                .unwrap_err()
                .to_string()
                .contains("UTF-8")
        );
        file.rewind().unwrap();
        assert!(text(file, invalid.len() as u64).is_err());
    }
    let (mut engine, _directory) =
        review(b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\xff\n+new\n");
    let error = Prepared::prepare(source(&mut engine, 1, 1), &|| Ok(()))
        .err()
        .unwrap();
    assert!(format!("{error:#}").contains("UTF-8"));
    engine.close().unwrap();
}

#[test]
fn copy_limits_and_helper_lengths_reject_instead_of_truncating() {
    let mut file = tempfile::tempfile().unwrap();
    let mut output = Limited {
        output: &mut file,
        bytes: 0,
    };
    let block = [b'x'; 64 * 1024];
    for _ in 0..MAX_BYTES / block.len() as u64 {
        output.write_all(&block).unwrap();
    }
    assert_eq!(output.bytes, MAX_BYTES);
    assert!(
        output
            .write_all(b"!")
            .unwrap_err()
            .to_string()
            .contains("16 MiB")
    );
    file.rewind().unwrap();
    validate(&mut file, &|| Ok(())).unwrap();
    assert_eq!(text(&mut file, MAX_BYTES).unwrap().len() as u64, MAX_BYTES);
    file.seek(SeekFrom::End(0)).unwrap();
    file.write_all(b"!").unwrap();
    file.rewind().unwrap();
    assert!(
        text(file, MAX_BYTES)
            .unwrap_err()
            .to_string()
            .contains("16 MiB")
    );
    assert!(
        text(b"short".as_slice(), 6)
            .unwrap_err()
            .to_string()
            .contains("length changed")
    );
    assert!(text(b"long".as_slice(), 3).is_err());
}

#[test]
fn failed_launch_exit_timeout_cancel_and_source_revocation_release_the_child() {
    let (mut engine, directory) = review(PATCH);
    let error = prepared(&mut engine, 1, 3)
        .start(
            engine.review_id(),
            Command::new(directory.path().join("missing")),
        )
        .err()
        .unwrap();
    assert!(error.to_string().contains("clipboard was not changed"));
    let mut failing = Command::new("/bin/sh");
    failing.args(["-c", "printf 'controlled failure' >&2; exit 7"]);
    let failed = prepared(&mut engine, 1, 3)
        .start(engine.review_id(), failing)
        .unwrap();
    let pid = failed.child.id();
    let error = failed.finish(&|| Ok(())).err().unwrap();
    assert!(format!("{error:#}").contains("controlled failure"));
    assert!(format!("{error:#}").contains("may have changed"));
    absent(pid);
    for cancel in [false, true] {
        let mut command = Command::new("/bin/sleep");
        command.arg("30");
        let child = prepared(&mut engine, 1, 3)
            .start(engine.review_id(), command)
            .unwrap();
        let pid = child.child.id();
        let error = child
            .wait(
                &|| {
                    if cancel {
                        anyhow::bail!("Canceled")
                    } else {
                        Ok(())
                    }
                },
                Duration::from_millis(30),
            )
            .err()
            .unwrap();
        assert!(format!("{error:#}").contains(if cancel { "Canceled" } else { "timed out" }));
        absent(pid);
    }
    let not_started = prepared(&mut engine, 1, 3);
    let mut command = Command::new("/bin/sleep");
    command.arg("30");
    let running = prepared(&mut engine, 1, 3)
        .start(engine.review_id(), command)
        .unwrap();
    let pid = running.child.id();
    engine.close().unwrap();
    assert!(
        not_started
            .start(engine.review_id(), Command::new("/usr/bin/false"))
            .is_err()
    );
    assert!(format!("{:#}", running.finish(&|| Ok(())).err().unwrap()).contains("closed review"));
    absent(pid);
}

#[test]
fn noisy_helper_diagnostics_are_drained_with_a_fixed_tail_and_reaped() {
    let (mut engine, _directory) = review(PATCH);
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "i=0; while [ \"$i\" -lt 256 ]; do printf '%1024s' x >&2; i=$((i+1)); done; printf FINAL_DIAGNOSTIC >&2; exit 7"]);
    let delivery = prepared(&mut engine, 1, 3)
        .start(engine.review_id(), command)
        .unwrap();
    let pid = delivery.child.id();
    assert_eq!(delivery.diagnostics.tail.capacity(), DIAGNOSTIC_BYTES);
    let error = format!("{:#}", delivery.finish(&|| Ok(())).err().unwrap());
    assert!(error.contains("FINAL_DIAGNOSTIC"), "{error}");
    assert!(error.len() < DIAGNOSTIC_BYTES + 300);
    absent(pid);
    engine.close().unwrap();
}
