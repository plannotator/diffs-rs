"""Pre-freeze checks using a Git-generated patch and real viewer processes."""

import copy
import gzip
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

from freeze_trace import BASELINE, ROOT, chrome
from patch_reference import decode
from terminal_baseline import Frames, Session, check_freeze, mismatch
from trace_reference import Document, View, digest


def main():
    config = {"watchdog_ms": 2000, "cadence_ms": 50, "sampled_rss_stop_bytes": 6 * 1024**3}
    with tempfile.TemporaryDirectory(prefix="native-diff-measurement-") as temporary:
        directory = Path(temporary)
        old = "\tconst café = '日本語';\r\n" + "".join(f"old line {n}\n" for n in range(180)) + "last"
        new = "\tconst cafe\u0301 = '日本語';\r\n" + "".join(f"new line {n}\n" for n in range(180)) + "last!"
        (directory / "old.rs").write_bytes(old.encode())
        (directory / "new.rs").write_bytes(new.encode())
        patch = subprocess.run(["/usr/bin/git", "diff", "--no-index", "--no-ext-diff", "--no-textconv",
                                "--no-color", "--unified=100000", "old.rs", "new.rs"],
                               cwd=directory, capture_output=True, check=False)
        assert patch.returncode == 1, patch.stderr
        patch_path = directory / "input.patch"
        patch_path.write_bytes(patch.stdout)
        reference = decode(patch_path)
        assert reference["source_records"] == 364
        doc = Document(patch_path, reference)
        view = View(doc)
        fixture = {"path": str(patch_path)}
        expected = lambda: chrome(view.grid(), str(patch_path), 1)
        session = Session(ROOT / BASELINE, fixture, config)
        try:
            result = session.await_grid(session.started_ns, expected())
            assert result["status"] == "correct", result.get("mismatch")
            initial = session.raw_tail
            expected_opening = expected()
            # Deliberately wrong content must not complete, even after a real paint.
            step = {"input_hex": b"\x1b[<65;90;10M".hex()}
            view.scroll(3)
            wrong = copy.deepcopy(expected())
            wrong["body"][4] = "WRONG SOURCE".ljust(160)
            session.trace = dict(config, watchdog_ms=150)
            start, _ = session.inject(step)
            result = session.await_grid(start, wrong)
            assert result["status"] == "timeout", result
            assert mismatch(session.frames.screen.display, expected()) is None
            session.trace = config
            for _ in range(5):
                start, _ = session.inject(step)
                view.scroll(3)
                result = session.await_grid(start, expected())
                assert result["status"] == "correct", result.get("mismatch")
            for split, size, step in [(False, (160, 50), {"input_hex": "75"}),
                                       (False, (100, 30), {"resize": [100, 30]}),
                                       (True, (100, 30), {"input_hex": "75"}),
                                       (True, (160, 50), {"resize": [160, 50]})]:
                view.reflow(split, *size)
                start, _ = session.inject(step)
                result = session.await_grid(start, expected())
                assert result["status"] == "correct", result.get("mismatch")
            view.jump(True, prototype=True)
            start, _ = session.inject({"input_hex": "47"})
            result = session.await_grid(start, expected())
            assert result["status"] == "correct", result.get("mismatch")
            quit_result = session.quit()
            assert quit_result["exit_code"] == 0 and quit_result["termios_restored"], quit_result
            assert all(quit_result["cleanup_sequences_received"].values()), quit_result
        finally:
            session.close()
        # Replay actual output through every byte boundary; an unfinished frame never completes.
        frames = Frames()
        last_end = initial.rfind(b"\x1b[?2026l")
        prefix = initial[:last_end + len(b"\x1b[?2026l")]
        displays = []
        for byte in prefix[:-1]:
            displays.extend(frames.feed(bytes([byte])))
        assert not displays or mismatch(displays[-1], expected_opening) is not None
        displays.extend(frames.feed(prefix[-1:]))
        assert mismatch(displays[-1], expected_opening) is None
        print("PASS: real Git patch, Unicode/tabs/CRLF, wrong-grid rejection, scroll, layout, resize, frame replay, clean exit")

        fifo = directory / "blocked.patch"
        os.mkfifo(fifo)
        session = Session(ROOT / BASELINE, {"path": str(fifo)}, dict(config, watchdog_ms=150))
        try:
            result = session.await_grid(session.started_ns, expected_opening)
            assert result["status"] == "timeout"
            quit_result = session.quit()
            assert quit_result["exit_code"] == 0 and quit_result["termios_restored"], quit_result
        finally:
            session.close()
        try:
            os.kill(session.pid, 0)
        except ProcessLookupError:
            pass
        else:
            raise AssertionError("Blocked viewer survived cleanup")
        print("PASS: blocked real FIFO timeout, cancellation, terminal restoration, no surviving viewer")

        session = Session(ROOT / BASELINE, {"path": str(fifo)}, dict(config, watchdog_ms=150))
        result = session.await_grid(session.started_ns, expected_opening)
        assert result["status"] == "timeout"
        session.close()
        assert session.returncode is not None and session.restored is True
        print("PASS: forced cleanup restores the terminal and reaps the owned blocked process")

        payload = gzip.compress(b'{"fixture":"unchanged"}', mtime=0)
        (directory / "trace.json.gz").write_bytes(payload)
        freeze = {"sha256": {}, "trace_sha256": hashlib.sha256(payload).hexdigest(),
                  "runtime": {"python_sha256": digest(sys.executable), "python_version": sys.version,
                              "dependencies": {"pyte": importlib.metadata.version("pyte")}}}
        (directory / "freeze.json").write_text(json.dumps(freeze))
        assert check_freeze(directory)["fixture"] == "unchanged"
        (directory / "trace.json.gz").write_bytes(gzip.compress(b'{"fixture":"altered"}', mtime=0))
        try:
            check_freeze(directory)
        except ValueError as error:
            assert "trace bytes" in str(error)
        else:
            raise AssertionError("Changed trace was accepted")
        print("PASS: altered supplied trace rejected before any viewer spawn")


if __name__ == "__main__":
    main()
