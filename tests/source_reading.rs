use std::{
    fs::{self, File},
    io::{self, BufWriter, Read, Seek, Write},
    sync::mpsc,
    time::{Duration, Instant},
};

use diffs_rs::{
    engine::{Engine, Input, Options, Side, SidebarPreference},
    input::{Cancellation, Cancelled, LocalInput},
    reading::{SourceRange, SourceRead},
};
use ratatui::{Terminal, backend::TestBackend};

struct Review {
    engine: Engine,
    ready: mpsc::Receiver<()>,
    directory: tempfile::TempDir,
}

impl Review {
    fn patch(bytes: &[u8]) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("review.patch");
        fs::write(&path, bytes).unwrap();
        Self::open(Input::Patch(path), directory)
    }

    fn open(input: Input, directory: tempfile::TempDir) -> Self {
        let (wake, ready) = mpsc::sync_channel(1);
        let engine = Engine::open(
            input,
            Options {
                sidebar: SidebarPreference::Hide,
                ..Options::default()
            },
            wake,
        )
        .unwrap();
        let mut review = Self {
            engine,
            ready,
            directory,
        };
        review.settle(false);
        review
    }

    fn settle(&mut self, full_source: bool) {
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        loop {
            let _ = self.ready.try_recv();
            self.engine.poll().unwrap();
            if full_source {
                let mut result = Ok(());
                terminal
                    .draw(|frame| {
                        let area = frame.area();
                        result = self.engine.draw(frame, area);
                    })
                    .unwrap();
                result.unwrap();
                self.engine.after_draw();
            }
            let status = self.engine.status();
            assert!(status.error.is_none(), "{:?}", status.error);
            if !status.loading && (!full_source || (!status.preparing_source && !status.coloring)) {
                return;
            }
            assert!(Instant::now() < deadline, "source worker did not settle");
            let _ = self.ready.recv_timeout(Duration::from_millis(20));
        }
    }

    fn range(&self, file: usize, side: Side, first: u32, last: u32) -> SourceRange {
        SourceRange {
            review: self.engine.review_id(),
            file,
            side,
            first,
            last,
        }
    }

    fn read(&mut self, file: usize, side: Side, first: u32, last: u32) -> SourceRead {
        let range = self.range(file, side, first, last);
        self.engine.source_read(range).unwrap()
    }
}

fn bytes(read: &SourceRead) -> Vec<u8> {
    let mut output = tempfile::tempfile().unwrap();
    let length = read.write_to(&mut output, &|| Ok(())).unwrap();
    assert_eq!(output.metadata().unwrap().len(), length);
    output.rewind().unwrap();
    let mut actual = Vec::new();
    output.read_to_end(&mut actual).unwrap();
    assert_eq!(actual.len() as u64, length);
    actual
}

#[test]
fn interleaved_patch_ranges_preserve_original_side_bytes_and_newline_state() {
    let mut patch = "diff --git a/review b/review\n--- a/review\n+++ b/review\n@@ -1,3 +1,4 @@\n shared\t日本語\r\n-old\r\n+new\r\n+inserted\r\n-".as_bytes().to_vec();
    patch.extend_from_slice(
        b"\xfftail\n\\ No newline at end of file\n+tail!\n\\ No newline at end of file\n",
    );
    patch.extend_from_slice(b"diff --git a/review b/review\n--- a/review\n+++ b/review\n@@ -1 +1 @@\n-other old\n+other new\n");
    let mut review = Review::patch(&patch);
    assert_eq!(review.engine.status().files, 2);
    let mut old = "shared\t日本語\r\nold\r\n".as_bytes().to_vec();
    old.extend_from_slice(b"\xfftail");
    assert_eq!(bytes(&review.read(0, Side::Old, 1, 3)), old);
    assert_eq!(
        bytes(&review.read(0, Side::New, 1, 4)),
        "shared\t日本語\r\nnew\r\ninserted\r\ntail!".as_bytes()
    );
    assert_eq!(bytes(&review.read(0, Side::Old, 3, 3)), b"\xfftail");
    assert_eq!(
        bytes(&review.read(0, Side::New, 2, 3)),
        b"new\r\ninserted\r\n"
    );
    assert_eq!(bytes(&review.read(1, Side::Old, 1, 1)), b"other old\n");
    assert_eq!(bytes(&review.read(1, Side::New, 1, 1)), b"other new\n");
}

#[test]
fn missing_lines_absent_sides_and_metadata_never_publish_partial_ranges() {
    let mut review = Review::patch(b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n@@ -100 +100 @@\n-end old\n+end new\ndiff --git a/script b/script\nold mode 100644\nnew mode 100755\ndiff --git a/added b/added\nnew file mode 100644\n--- /dev/null\n+++ b/added\n@@ -0,0 +1 @@\n+added\n");
    assert_eq!(review.engine.status().files, 3);
    for (file, side, first, last) in [
        (0, Side::Old, 1, 100),
        (0, Side::New, 2, 2),
        (0, Side::New, 100, 101),
        (1, Side::Old, 1, 1),
        (2, Side::Old, 1, 1),
    ] {
        let read = review.read(file, side, first, last);
        let mut output = tempfile::tempfile().unwrap();
        assert!(read.write_to(&mut output, &|| Ok(())).is_err());
        assert_eq!(output.metadata().unwrap().len(), 0);
    }
    for range in [
        review.range(0, Side::Old, 0, 1),
        review.range(0, Side::Old, 2, 1),
        review.range(3, Side::New, 1, 1),
    ] {
        assert!(review.engine.source_read(range).is_err());
    }
    assert_eq!(bytes(&review.read(0, Side::Old, 100, 100)), b"end old\n");
    assert_eq!(bytes(&review.read(2, Side::New, 1, 1)), b"added\n");
}

#[test]
fn adjacent_hunks_and_large_source_numbers_keep_their_exact_coordinates() {
    let mut review = Review::patch(b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old 1\n+new 1\n@@ -2 +2 @@\n-old 2\n+new 2\ndiff --git a/b b/b\n--- a/b\n+++ b/b\n@@ -2147483646 +2147483646 @@\n-old last\n+new last\n");
    assert_eq!(bytes(&review.read(0, Side::Old, 1, 2)), b"old 1\nold 2\n");
    assert_eq!(bytes(&review.read(0, Side::New, 1, 2)), b"new 1\nnew 2\n");
    assert_eq!(
        bytes(&review.read(1, Side::New, 2147483646, 2147483646)),
        b"new last\n"
    );
    let mut output = tempfile::tempfile().unwrap();
    assert!(
        review
            .read(1, Side::New, u32::MAX, u32::MAX)
            .write_to(&mut output, &|| Ok(()))
            .is_err()
    );
    assert_eq!(output.metadata().unwrap().len(), 0);
}

#[test]
fn replace_close_and_drop_revoke_prepared_reads_and_stale_ranges() {
    let mut review =
        Review::patch(b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n");
    let range = review.range(0, Side::New, 1, 1);
    let first = review.engine.source_read(range).unwrap();
    let path = review.directory.path().join("next.patch");
    fs::write(
        &path,
        b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-new\n+later\n",
    )
    .unwrap();
    review.engine.replace(Input::Patch(path)).unwrap();
    review.settle(false);
    assert!(review.engine.source_read(range).is_err());
    let second = review.read(0, Side::New, 1, 1);
    assert_eq!(bytes(&second), b"later\n");
    review.engine.close().unwrap();
    let closed_range = review.range(0, Side::New, 1, 1);
    assert!(review.engine.source_read(closed_range).is_err());
    for read in [first, second] {
        let mut output = tempfile::tempfile().unwrap();
        assert!(read.write_to(&mut output, &|| Ok(())).is_err());
        assert_eq!(output.metadata().unwrap().len(), 0);
    }
    let dropped = {
        let mut review =
            Review::patch(b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -0,0 +1 @@\n+value\n");
        review.read(0, Side::New, 1, 1)
    };
    let mut output = tempfile::tempfile().unwrap();
    assert!(dropped.write_to(&mut output, &|| Ok(())).is_err());
    assert_eq!(output.metadata().unwrap().len(), 0);
}

#[test]
fn exact_copy_preserves_caller_buffering_and_propagates_real_output_failure() {
    let mut review =
        Review::patch(b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-before\n+after\n");
    let read = review.read(0, Side::New, 1, 1);
    let mut output = BufWriter::new(tempfile::tempfile().unwrap());
    assert_eq!(read.write_to(&mut output, &|| Ok(())).unwrap(), 6);
    assert_eq!(output.get_ref().metadata().unwrap().len(), 0);
    assert_eq!(output.buffer(), b"after\n");
    output.flush().unwrap();
    let mut output = output.into_inner().unwrap();
    output.rewind().unwrap();
    let mut actual = Vec::new();
    output.read_to_end(&mut actual).unwrap();
    assert_eq!(actual, b"after\n");

    let file = tempfile::NamedTempFile::new().unwrap();
    let mut read_only = File::open(file.path()).unwrap();
    let error = read.write_to(&mut read_only, &|| Ok(())).unwrap_err();
    assert!(error.downcast_ref::<io::Error>().is_some());
    assert_eq!(fs::metadata(file.path()).unwrap().len(), 0);
}

#[test]
fn long_lines_stream_exactly_and_can_cancel_before_or_during_output() {
    let content = "\t日本語 e\u{301} 👩‍💻 ".repeat(20_000);
    let mut expected = content.as_bytes().to_vec();
    expected.extend_from_slice(b"\r\n");
    let mut patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -0,0 +1 @@\n+".to_vec();
    patch.extend_from_slice(&expected);
    let mut review = Review::patch(&patch);
    let read = review.read(0, Side::New, 1, 1);
    assert_eq!(bytes(&read), expected);

    let cancel = Cancellation::default();
    cancel.stop();
    let mut output = tempfile::tempfile().unwrap();
    assert!(
        read.write_to(&mut output, &|| cancel.check())
            .unwrap_err()
            .is::<Cancelled>()
    );
    assert_eq!(output.metadata().unwrap().len(), 0);

    let observed = output.try_clone().unwrap();
    let error = read
        .write_to(&mut output, &|| {
            if observed.metadata()?.len() != 0 {
                return Err(Cancelled.into());
            }
            Ok(())
        })
        .unwrap_err();
    assert!(error.is::<Cancelled>());
    let count = output.metadata().unwrap().len();
    assert!(count > 0 && count <= 64 * 1024, "{count}");
    output.rewind().unwrap();
    let mut partial = Vec::new();
    output.read_to_end(&mut partial).unwrap();
    assert_eq!(partial, expected[..count as usize]);
}

#[test]
fn closing_a_review_revokes_a_range_running_on_an_owned_thread() {
    let mut patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -0,0 +1 @@\n+".to_vec();
    patch.extend(std::iter::repeat_n(b'x', 256 * 1024));
    patch.push(b'\n');
    let mut review = Review::patch(&patch);
    let read = review.read(0, Side::New, 1, 1);
    let (written, observed) = mpsc::sync_channel(1);
    let (resume, resumed) = mpsc::sync_channel(1);
    std::thread::scope(|scope| {
        let worker = scope.spawn(move || {
            let mut output = tempfile::tempfile().unwrap();
            let file = output.try_clone().unwrap();
            let waited = std::cell::Cell::new(false);
            let result = read.write_to(&mut output, &|| {
                if file.metadata()?.len() != 0 && !waited.replace(true) {
                    written.send(())?;
                    resumed.recv_timeout(Duration::from_secs(5))?;
                }
                Ok(())
            });
            (result, output.metadata().unwrap().len())
        });
        observed.recv_timeout(Duration::from_secs(5)).unwrap();
        review.engine.close().unwrap();
        resume.send(()).unwrap();
        let (result, length) = worker.join().unwrap();
        assert!(result.unwrap_err().to_string().contains("closed review"));
        assert!(length > 0 && length <= 2 * 64 * 1024, "{length}");
    });
}

#[test]
fn checked_sources_supply_hidden_context_from_immutable_captured_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let old = directory.path().join("before.txt");
    let new = directory.path().join("after.txt");
    let original = (1..=80)
        .map(|n| format!("line {n:03}\t日本語\r\n"))
        .collect::<String>()
        + "last without newline";
    let changed = original.replace("line 040", "changed 040");
    fs::write(&old, original.as_bytes()).unwrap();
    fs::write(&new, changed.as_bytes()).unwrap();
    let input = Input::Local(LocalInput::Files {
        old: old.clone(),
        new: new.clone(),
    });
    let mut review = Review::open(input, directory);
    review.settle(true);
    assert!(!review.engine.status().source_failed);
    let old_read = review.read(0, Side::Old, 1, 81);
    let new_read = review.read(0, Side::New, 1, 81);
    fs::write(&old, b"unrelated later edit\n").unwrap();
    fs::write(&new, b"another later edit\n").unwrap();
    assert_eq!(bytes(&old_read), original.as_bytes());
    assert_eq!(bytes(&new_read), changed.as_bytes());
    assert_eq!(
        bytes(&review.read(0, Side::New, 81, 81)),
        b"last without newline"
    );
    let invalid = review.range(0, Side::New, 1, 82);
    assert!(review.engine.source_read(invalid).is_err());
}
