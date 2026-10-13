use std::{
    fs::{self, OpenOptions},
    os::unix::fs::OpenOptionsExt,
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::Instant,
};

use super::*;

const DEADLINE: Duration = Duration::from_secs(5);

struct HeldChild {
    directory: tempfile::TempDir,
    cancel: Cancellation,
    result: Receiver<Result<Vec<u8>>>,
    worker: Option<thread::JoinHandle<()>>,
}

impl HeldChild {
    fn start(status: u8, differences: bool, diagnostic: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let gate = directory.path().join("gate");
        assert!(
            Command::new("mkfifo")
                .arg(&gate)
                .status()
                .unwrap()
                .success()
        );
        let executable = directory.path().join("child.sh");
        fs::write(
            &executable,
            format!(
                "#!/bin/sh\nset -eu\nprintf '%s' \"$$\" > child.pid\nprintf 'early\\n'\n\
                 printf '%s' '{diagnostic}' >&2\nexec 2>&-\n: > ready\n\
                 IFS= read -r token < gate\nprintf 'late\\n'\nexit {status}\n"
            ),
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let git = Git {
            executable,
            directory: directory.path().to_path_buf(),
        };
        let cancel = Cancellation::default();
        let worker_cancel = cancel.clone();
        let (tx, result) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            let output = git
                .run(&[], None, differences, &worker_cancel)
                .and_then(|mut file| {
                    let mut bytes = Vec::new();
                    file.read_to_end(&mut bytes)?;
                    Ok(bytes)
                });
            let _ = tx.send(output);
        });
        let child = Self {
            directory,
            cancel,
            result,
            worker: Some(worker),
        };
        let deadline = Instant::now() + DEADLINE;
        while !child.directory.path().join("ready").exists() {
            assert!(Instant::now() < deadline, "child did not reach its gate");
            thread::sleep(Duration::from_millis(5));
        }
        child
    }

    fn release(&self) -> std::io::Result<()> {
        let deadline = Instant::now() + DEADLINE;
        loop {
            match self.try_release() {
                Err(error)
                    if error.raw_os_error() == Some(libc::ENXIO) && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(5));
                }
                result => return result,
            }
        }
    }

    fn try_release(&self) -> std::io::Result<()> {
        let mut gate = OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(self.directory.path().join("gate"))?;
        gate.write_all(b"go\n")
    }

    fn assert_waiting(&self) {
        assert!(matches!(
            self.result.recv_timeout(Duration::from_millis(75)),
            Err(RecvTimeoutError::Timeout)
        ));
    }

    fn finish(&mut self) -> Result<Vec<u8>> {
        let result = self
            .result
            .recv_timeout(DEADLINE)
            .expect("child did not finish");
        self.worker.take().unwrap().join().unwrap();
        let pid = fs::read_to_string(self.directory.path().join("child.pid")).unwrap();
        assert!(
            !Command::new("/bin/kill")
                .args(["-0", pid.trim()])
                .stderr(Stdio::null())
                .status()
                .unwrap()
                .success(),
            "child was not reaped"
        );
        result
    }
}

impl Drop for HeldChild {
    fn drop(&mut self) {
        self.cancel.stop();
        let _ = self.try_release();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[test]
fn early_stderr_eof_preserves_late_stdout_and_allowed_status() {
    for status in [0, 1] {
        let mut child = HeldChild::start(status, status == 1, "");
        child.assert_waiting();
        child.release().unwrap();
        assert_eq!(child.finish().unwrap(), b"early\nlate\n");
    }
}

#[test]
fn early_stderr_eof_preserves_failure_and_bounded_diagnostic() {
    for (status, differences) in [(1, false), (7, true)] {
        let diagnostic = format!("discarded{}tail", "x".repeat(9000));
        let mut child = HeldChild::start(status, differences, &diagnostic);
        child.assert_waiting();
        child.release().unwrap();
        assert_eq!(
            child.finish().unwrap_err().to_string(),
            format!("Git failed: {}tail", "x".repeat(8188))
        );
    }
}

#[test]
fn cancellation_after_stderr_eof_reaps_child() {
    let mut child = HeldChild::start(0, false, "");
    child.assert_waiting();
    child.cancel.stop();
    assert!(child.finish().unwrap_err().is::<crate::input::Cancelled>());
}
