"""Serial, real-PTY measurements of the preserved executable and frozen content trace."""

import argparse
from datetime import datetime, timezone
import errno
import fcntl
import gzip
import hashlib
import importlib.metadata
import json
import math
import os
from pathlib import Path
import platform
import select
import signal
import statistics
import struct
import subprocess
import sys
import termios
import threading
import time

import pyte

from trace_reference import digest
from rss_sampling import PROTOCOL, query_group

ROOT = Path(__file__).resolve().parents[1]
END = b"\x1b[?2026l"
CATEGORIES = ["forward", "reverse", "distant_seek", "layout", "resize"]


class Frames:
    def __init__(self, width=160, height=50):
        self.screen = pyte.Screen(width, height)
        self.stream = pyte.ByteStream(self.screen)
        self.pending = b""
        self.count = 0

    def feed(self, data):
        self.pending += data
        while END in self.pending:
            end = self.pending.index(END) + len(END)
            frame, self.pending = self.pending[:end], self.pending[end:]
            self.stream.feed(frame)
            self.count += 1
            yield self.screen.display
        if len(self.pending) > 4 * 1024**2:
            raise RuntimeError("More than 4 MiB without a complete synchronized frame")


def mismatch(display, grid):
    expected = [grid["title"], *grid["body"]]
    if len(display) != grid["height"]:
        return {"reason": "height", "actual": len(display), "expected": grid["height"]}
    for y, (actual, wanted) in enumerate(zip(display, expected)):
        if actual != wanted:
            return {"reason": "content", "row": y, "actual": actual, "expected": wanted}
    if display[-1] not in grid["allowed_footers"]:
        return {"reason": "footer", "actual": display[-1], "allowed": grid["allowed_footers"]}
    return None


class Session:
    def __init__(self, executable, fixture, trace, *, child_env=None, background_rss=True):
        self.trace = trace
        self.frames = Frames()
        self.raw_tail = b""
        self.raw_hash = hashlib.sha256()
        self.raw_bytes = 0
        self.receipts = 0
        self.returncode = None
        self.exit_observed_ns = None
        self.rusage = None
        self.sample_peak = 0
        self.rss_samples = 0
        self.rss_sampler_ns = 0
        self.rss_error = None
        self.rss_timeline = []
        self.rss_queries = []
        self.rss_holds = []
        self.rss_stop = threading.Event()
        self.rss_shutdown = None
        self.last_input = 0
        self.restored = None
        self.cleanup_bytes = {name: False for name in ["leave_alternate_screen", "show_cursor", "disable_mouse"]}
        master, slave = os.openpty()
        self.master = master
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 50, 160, 0, 0))
        self.initial_termios = termios.tcgetattr(slave)
        env = dict(os.environ, TERM="xterm-256color", COLORTERM="truecolor")
        env.pop("NO_COLOR", None)
        if child_env is not None:
            env.update(child_env)
        self.argv = [str(executable), "--layout", "split", "--theme", "dark", "patch", fixture["path"]]
        self.started_ns = time.monotonic_ns()
        self.pid = os.fork()
        if self.pid == 0:
            try:
                os.close(master)
                os.setsid()
                fcntl.ioctl(slave, termios.TIOCSCTTY, 0)
                for fd in range(3):
                    os.dup2(slave, fd)
                if slave > 2:
                    os.close(slave)
                os.chdir(ROOT)
                os.execve(executable, self.argv, env)
            except BaseException:
                os._exit(127)
        os.close(slave)
        os.set_blocking(master, False)
        self.sampler = threading.Thread(target=self.sample_rss, daemon=True)
        if background_rss:
            self.sampler.start()

    def reap(self):
        if self.returncode is None:
            pid, status, usage = os.wait4(self.pid, os.WNOHANG)
            if pid:
                self.exit_observed_ns = time.monotonic_ns()
                self.returncode = os.waitstatus_to_exitcode(status)
                self.rusage = usage
                try:
                    self.restored = termios.tcgetattr(self.master) == self.initial_termios
                except OSError:
                    self.restored = None

    def sample_rss(self):
        # Start after fork. Queries cannot block the PTY reader.
        while not self.rss_stop.is_set():
            self.sample_rss_once()
            if self.rss_error or self.rss_stop.wait(PROTOCOL["post_query_wait_seconds"]):
                break

    def sample_rss_once(self):
        query = query_group(self.pid)
        self.rss_queries.append(query)
        self.rss_sampler_ns += query["end_ns"] - query["start_ns"]
        if query["status"] == "error":
            self.rss_error = f"RSS sampler failed: {query['error']}"
        elif query["status"] == "ok":
            total = query["rss_bytes"]
            self.rss_timeline.append([query["end_ns"], total])
            self.sample_peak = max(self.sample_peak, total)
            self.rss_samples += 1
            if total > self.trace["sampled_rss_stop_bytes"]:
                self.rss_error = "Sampled process-group RSS exceeded the safety stop"
        return query

    def read(self, timeout=0.005):
        ready, _, _ = select.select([self.master], [], [], timeout)
        if not ready:
            self.reap()
            return None
        try:
            data = os.read(self.master, 262144)
            received = time.monotonic_ns()
        except OSError as error:
            if error.errno not in [errno.EIO, errno.EAGAIN]:
                raise
            data, received = b"", time.monotonic_ns()
        self.reap()
        if not data:
            return None
        self.receipts += 1
        self.raw_bytes += len(data)
        self.raw_hash.update(data)
        self.raw_tail = (self.raw_tail + data)[-262144:]
        for name, sequence in [("leave_alternate_screen", b"\x1b[?1049l"),
                               ("show_cursor", b"\x1b[?25h"), ("disable_mouse", b"\x1b[?1006l")]:
            self.cleanup_bytes[name] |= sequence in self.raw_tail
        return data, received

    def await_grid(self, started, expected, known=None):
        validation_ns = 0
        last_mismatch = None
        initial_frames = self.frames.count
        deadline = started + self.trace["watchdog_ms"] * 1_000_000
        while time.monotonic_ns() < deadline:
            packet = self.read()
            if packet:
                data, received = packet
                before = time.monotonic_ns()
                for display in self.frames.feed(data):
                    last_mismatch = mismatch(display, expected)
                    conforming = last_mismatch is None
                    diagnostic = known is not None and mismatch(display, known) is None
                    error_screen = display[0].rstrip() == "Cannot open patch"
                    if conforming or diagnostic or error_screen:
                        if self.rss_error:
                            raise RuntimeError(self.rss_error)
                        validation_ns += time.monotonic_ns() - before
                        return {"status": "correct" if conforming else "known_wrong_grid" if diagnostic else "open_error",
                                "latency_ms": (received-started)/1e6 if conforming else None,
                                "observed_ms": (received-started)/1e6, "validation_ms": validation_ns/1e6,
                                "frames": self.frames.count-initial_frames,
                                "mismatch": None if conforming else last_mismatch}
                validation_ns += time.monotonic_ns() - before
            if self.returncode is not None:
                return {"status": "process_exit", "latency_ms": None, "exit_code": self.returncode,
                        "validation_ms": validation_ns/1e6, "mismatch": last_mismatch}
            if self.rss_error:
                raise RuntimeError(self.rss_error)
        return {"status": "timeout", "latency_ms": None, "validation_ms": validation_ns/1e6,
                "frames": self.frames.count-initial_frames, "mismatch": last_mismatch}

    def inject(self, step):
        if self.rss_error:
            raise RuntimeError(self.rss_error)
        earliest = self.last_input + self.trace["cadence_ms"] * 1_000_000
        while time.monotonic_ns() < earliest:
            # Drain late style frames before injection; they cannot complete this sample.
            packet = self.read(max(0, min(0.005, (earliest-time.monotonic_ns())/1e9)))
            if packet:
                list(self.frames.feed(packet[0]))
        self.reap()
        if self.returncode is not None:
            raise RuntimeError("Viewer exited before input")
        if self.rss_error:
            raise RuntimeError(self.rss_error)
        started = time.monotonic_ns()
        if "resize" in step:
            width, height = step["resize"]
            self.frames.screen.resize(height, width)
            fcntl.ioctl(self.master, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
        else:
            payload = bytes.fromhex(step["input_hex"])
            if os.write(self.master, payload) != len(payload):
                raise RuntimeError("Partial public-command injection")
        previous, self.last_input = self.last_input, started
        return started, (started-previous)/1e6 if previous else None

    def stop_rss(self, reason):
        if self.rss_shutdown is not None:
            if self.rss_shutdown["status"] != "complete":
                raise RuntimeError(self.rss_shutdown["error"])
            return
        record = {"clock": "monotonic_ns", "requested_ns": time.monotonic_ns(),
                  "reason": reason, "status": "running"}
        self.rss_shutdown = record
        self.rss_stop.set()
        try:
            if self.sampler.ident is not None:
                self.sampler.join(timeout=PROTOCOL["join_timeout_seconds"])
            if self.sampler.is_alive():
                raise RuntimeError("RSS sampler failed to stop")
            record["status"] = "complete"
        except BaseException as error:
            record.update(status="failed", error=str(error))
            raise
        finally:
            record.update(finished_ns=time.monotonic_ns(), query_count=len(self.rss_queries),
                          last_query_end_ns=self.rss_queries[-1]["end_ns"] if self.rss_queries else None)

    def quit(self):
        self.stop_rss("quit")
        if self.rss_error:
            raise RuntimeError(self.rss_error)
        self.reap()
        if self.returncode is not None:
            raise RuntimeError("Viewer exited before quit input")
        started, _ = self.inject({"input_hex": "71"})
        while self.returncode is None and time.monotonic_ns()-started < 2_000_000_000:
            self.read(0.001)
        # Drain restoration bytes even if process exit was observed first.
        while self.read(0) is not None:
            pass
        if self.exit_observed_ns is not None and self.exit_observed_ns < started:
            raise RuntimeError("Viewer exited before quit input")
        return {"exit_observation_ms": (self.exit_observed_ns-started)/1e6 if self.exit_observed_ns else None,
                "input_ns": started, "rss_shutdown": dict(self.rss_shutdown),
                "exit_code": self.returncode, "termios_restored": self.restored,
                "cleanup_sequences_received": self.cleanup_bytes.copy()}

    def close(self):
        try:
            self.stop_rss("cleanup")
        finally:
            try:
                if self.returncode is None:
                    for sig, allowance in [(signal.SIGTERM, 1), (signal.SIGKILL, 1)]:
                        try:
                            os.killpg(self.pid, sig)
                        except ProcessLookupError:
                            # The child can fail before setsid.
                            try:
                                os.kill(self.pid, sig)
                            except ProcessLookupError:
                                pass
                        until = time.monotonic() + allowance
                        while self.returncode is None and time.monotonic() < until:
                            self.read(0.005)
                        if self.returncode is not None:
                            break
                    if self.returncode is None:
                        raise RuntimeError("Owned viewer did not exit after SIGKILL")
            finally:
                os.close(self.master)


def trial(fixture, trace, output, number):
    session = Session(ROOT / trace["baseline"], fixture, trace)
    result = {"fixture": fixture["id"], "trial": number, "argv": session.argv, "samples": []}
    blocked = None
    try:
        result["opening"] = session.await_grid(session.started_ns, fixture["grids"][fixture["opening"]])
        result["index_ready_upper_bound_ms"] = result["opening"].get("latency_ms")
        if result["opening"]["status"] != "correct":
            blocked = "Opening did not produce the expected source grid."
        for step in fixture["steps"]:
            category = step["category"]
            if step.get("baseline_unsupported"):
                blocked = step["baseline_unsupported"] if not blocked else blocked
            if blocked:
                if category in CATEGORIES:
                    result["samples"].append({"category": category, "sample": step["sample"],
                                              "status": "not_run", "reason": blocked, "latency_ms": None})
                continue
            started, interval = session.inject(step)
            sample = session.await_grid(started, fixture["grids"][step["expected"]],
                                        fixture["grids"].get(step.get("known_nonconforming")))
            sample.update(category=category, sample=step.get("sample"), actual_input_interval_ms=interval)
            result["samples"].append(sample)
            if sample["status"] not in ["correct", "known_wrong_grid"]:
                blocked = f"Earlier {category} operation {sample['status']}; subsequent state is not trustworthy."
        if session.returncode is None:
            result["quit"] = session.quit()
    except Exception as error:
        result["harness_error"] = str(error)
    finally:
        session.close()
    if session.rss_error:
        result.setdefault("harness_error", session.rss_error)
    recorded = {(s["category"], s["sample"]) for s in result["samples"]}
    for category in CATEGORIES:
        for index in range(40):
            if (category, index) not in recorded:
                result["samples"].append({"category": category, "sample": index, "status": "not_run",
                                          "latency_ms": None, "reason": result.get("harness_error", "Trial aborted")})
    result.update(exit_code=session.returncode, termios_restored=session.restored,
                  viewer_os_peak_rss_bytes=session.rusage.ru_maxrss if session.rusage else None,
                  sampled_group_peak_rss_bytes=session.sample_peak, rss_samples=session.rss_samples,
                  rss_sampler_error=session.rss_error,
                  rss_shutdown=session.rss_shutdown, rss_protocol=PROTOCOL,
                  rss_sampler_ms=session.rss_sampler_ns/1e6, terminal_rss_bytes=None, steady_rss_bytes=None,
                  output_bytes=session.raw_bytes, output_sha256=session.raw_hash.hexdigest())
    (output / f"{fixture['id']}-{number}.tail.ansi").write_bytes(session.raw_tail)
    (output / f"{fixture['id']}-{number}.screen.txt").write_text("\n".join(session.frames.screen.display)+"\n")
    (output / f"{fixture['id']}-{number}.json").write_text(json.dumps(result, indent=2)+"\n")
    return result


def summarize(results):
    summary = []
    for fixture in dict.fromkeys(r["fixture"] for r in results):
        trials = [r for r in results if r["fixture"] == fixture]
        row = {"fixture": fixture, "trials": len(trials), "opening_correct": sum(r.get("opening", {}).get("status") == "correct" for r in trials),
               "harness_errors": [r["harness_error"] for r in trials if "harness_error" in r], "categories": {}}
        valid = len(trials) == 5 and not row["harness_errors"]
        opens = [r["opening"]["latency_ms"] for r in trials if r.get("opening", {}).get("status") == "correct"]
        row["opening_max_ms"] = max(opens) if len(opens) == 5 and valid else None
        row["opening_median_ms"] = statistics.median(opens) if len(opens) == 5 and valid else None
        quits = [r["quit"]["exit_observation_ms"] for r in trials
                 if r.get("quit", {}).get("exit_observation_ms") is not None]
        row["quit_max_ms"] = max(quits) if len(quits) == 5 and valid else None
        row["terminal_restored_trials"] = sum(r["termios_restored"] is True for r in trials)
        peaks = [r["viewer_os_peak_rss_bytes"] for r in trials if r["viewer_os_peak_rss_bytes"] is not None]
        row["viewer_os_peak_rss_bytes"] = max(peaks) if peaks else None
        for category in CATEGORIES:
            samples = [s for r in trials for s in r["samples"] if s["category"] == category]
            timings = sorted(s["latency_ms"] for s in samples if s["status"] == "correct")
            complete = len(samples) == len(timings) == 200 and valid
            row["categories"][category] = {
                "expected_samples": 200, "recorded_samples": len(samples), "correct": len(timings),
                "statuses": {s: sum(t["status"] == s for t in samples) for s in sorted({s["status"] for s in samples})},
                "median_ms": statistics.median(timings) if complete else None,
                "p95_ms": timings[math.ceil(.95*len(timings))-1] if complete else None,
                "maximum_ms": max(timings) if complete else None}
        summary.append(row)
    return summary


def check_freeze(directory):
    freeze = json.loads((directory / "freeze.json").read_text())
    for relative, expected in freeze["sha256"].items():
        if digest(ROOT / relative) != expected:
            raise ValueError(f"Frozen hash mismatch: {relative}")
    payload = (directory / "trace.json.gz").read_bytes()
    if hashlib.sha256(payload).hexdigest() != freeze["trace_sha256"]:
        raise ValueError("Supplied trace bytes do not match the frozen trace")
    runtime = freeze["runtime"]
    if digest(sys.executable) != runtime["python_sha256"] or sys.version != runtime["python_version"]:
        raise ValueError("Python runtime differs from the freeze")
    for name, version in runtime["dependencies"].items():
        if importlib.metadata.version(name) != version:
            raise ValueError(f"Frozen dependency version mismatch: {name}")
    return json.loads(gzip.decompress(payload))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("freeze", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    if sys.platform != "darwin":
        raise SystemExit("This protocol reports macOS wait4 ru_maxrss in bytes; this runner requires macOS.")
    trace = check_freeze(args.freeze)
    args.output.mkdir(parents=True, exist_ok=False)
    environment = {"started_utc": datetime.now(timezone.utc).isoformat(), "platform": platform.platform(),
                   "machine": platform.machine(), "python": sys.version, "python_sha256": digest(sys.executable),
                   "dependencies": {name: importlib.metadata.version(name) for name in ["pyte", "wcwidth", "regex"]},
                   "freeze_sha256": digest(args.freeze / "freeze.json"), "condition": "fresh-process, warm-filesystem",
                   "terminal": "PTY + pyte; no GUI terminal", "TERM": "xterm-256color", "COLORTERM": "truecolor",
                   "clock": time.get_clock_info("monotonic").__dict__,
                   "host": subprocess.check_output(["/usr/sbin/sysctl", "-n", "machdep.cpu.brand_string", "hw.memsize"], text=True).splitlines()}
    (args.output / "environment.json").write_text(json.dumps(environment, indent=2)+"\n")
    results = []
    for fixture in trace["fixtures"]:
        # One full read warms the filesystem. It is not a viewer trial.
        with (ROOT / fixture["path"]).open("rb") as source:
            while source.read(1024**2):
                pass
        for number in range(1, trace["trials"]+1):
            print(f"{fixture['id']} trial {number}/{trace['trials']}", flush=True)
            result = trial(fixture, trace, args.output, number)
            results.append(result)
            print(f"  opening={result.get('opening', {}).get('status')} peak={result['viewer_os_peak_rss_bytes']} bytes", flush=True)
    report = {"scope": trace["scope"], "unmeasured": trace["unmeasured"], "summary": summarize(results), "trials": results}
    (args.output / "results.json").write_text(json.dumps(report, indent=2)+"\n")


if __name__ == "__main__":
    main()
