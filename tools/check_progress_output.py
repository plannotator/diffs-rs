"""Check progress disconnection with real child processes and saved results."""

import argparse
import errno
import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import time

from progress_output import Progress


def write_json(path, value):
    with path.open("x") as stream:
        json.dump(value, stream, indent=2)
        stream.write("\n")


def child(output, mode):
    progress = Progress(enabled=mode != "quiet")
    report = {"status": "running", "pid": os.getpid(), "events": []}
    write_json(output / "started.json", {"pid": os.getpid()})
    # The parent closes the progress reader before releasing the input gate.
    os.write(1, b"ready\n")
    if sys.stdin.buffer.read(1) != b"g":
        raise RuntimeError("Missing parent gate")
    try:
        if mode == "bad-descriptor":
            with Path(__file__).open("rb") as read_only:
                Progress(fd=read_only.fileno())("must fail")
        if mode == "legacy":
            print("legacy buffered print", flush=True)
        else:
            progress("work started: 日本語")
        report["events"].append("after-progress")
        if mode == "work-failure":
            raise ValueError("Deliberate work failure")
        write_json(output / "result.json", {"status": "complete", "pid": os.getpid()})
        report["events"].append("result-saved")
        progress("work complete")
        # This stands in for the postflight work skipped by the original failure.
        report["events"].append("postflight")
        report["status"] = "complete"
    except Exception as error:
        report.update(status="failed", error=type(error).__name__, detail=str(error),
                      errno=getattr(error, "errno", None))
        raise
    finally:
        report["progress_disconnected"] = progress.disconnected
        write_json(output / "report.json", report)


def run_case(output, name, mode, disconnected):
    directory = output / name
    directory.mkdir()
    if mode == "result-collision":
        (directory / "result.json").write_bytes(b"original result\n")
    argv = [sys.executable, "-B", str(Path(__file__).resolve()), str(directory), "--child", mode]
    with subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE) as process:
        try:
            if not select.select([process.stdout], [], [], 10)[0]:
                raise TimeoutError("Child did not reach its gate")
            assert process.stdout.readline() == b"ready\n"
            if disconnected:
                process.stdout.close()
                process.stdout = None
            stdout, stderr = process.communicate(b"g", timeout=10)
        except BaseException:
            process.kill()
            process.communicate()
            raise
    (directory / "stdout.txt").write_bytes(stdout or b"")
    (directory / "stderr.txt").write_bytes(stderr)
    report = json.loads((directory / "report.json").read_text())
    started = json.loads((directory / "started.json").read_text())
    assert report["pid"] == started["pid"] == process.pid
    if mode in ("success", "quiet"):
        assert process.returncode == 0 and not stderr
        assert report["status"] == "complete"
        assert report["events"] == ["after-progress", "result-saved", "postflight"]
        assert report["progress_disconnected"] == (disconnected and mode != "quiet")
        assert json.loads((directory / "result.json").read_text()) == {
            "status": "complete", "pid": process.pid}
        if not disconnected:
            assert stdout == ("work started: 日本語\nwork complete\n".encode() if mode != "quiet" else b"")
    else:
        expected = {"legacy": "BrokenPipeError", "work-failure": "ValueError",
                    "result-collision": "FileExistsError", "bad-descriptor": "OSError"}[mode]
        assert process.returncode != 0 and report["status"] == "failed"
        assert report["error"] == expected and expected.encode() in stderr
        if mode == "result-collision":
            assert (directory / "result.json").read_bytes() == b"original result\n"
        else:
            assert not (directory / "result.json").exists()
        if mode == "bad-descriptor":
            assert report["errno"] == errno.EBADF
        if mode in ("work-failure", "result-collision"):
            assert report["events"] == ["after-progress"]
            assert report["progress_disconnected"]
    return {"case": name, "mode": mode, "reader_closed": disconnected,
            "exit_code": process.returncode, "pid": process.pid, "status": "pass"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--child", choices=["success", "quiet", "work-failure",
                                           "result-collision", "bad-descriptor", "legacy"])
    args = parser.parse_args()
    if args.child:
        child(args.output, args.child)
        return
    if not __debug__:
        raise RuntimeError("Checks require Python assertions")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {"status": "running", "started_ns": time.time_ns(), "python": sys.version,
              "cases": [], "tools_sha256": {
                  name: hashlib.sha256(Path(__file__).with_name(name).read_bytes()).hexdigest()
                  for name in ("progress_output.py", "check_progress_output.py")}}
    progress = Progress()
    try:
        for name, mode, disconnected in [
            ("connected", "success", False),
            ("disconnected", "success", True),
            ("quiet", "quiet", False),
            ("quiet-disconnected", "quiet", True),
            ("work-failure", "work-failure", True),
            ("result-collision", "result-collision", True),
            ("bad-descriptor", "bad-descriptor", False),
            ("legacy-control", "legacy", True),
        ]:
            report["cases"].append(run_case(output, name, mode, disconnected))
            progress(f"PASS: {name}")
        report["status"] = "pass"
    except BaseException as error:
        report.update(status="failed", error=repr(error))
        raise
    finally:
        report["finished_ns"] = time.time_ns()
        write_json(output / "checks.json", report)


if __name__ == "__main__":
    main()
