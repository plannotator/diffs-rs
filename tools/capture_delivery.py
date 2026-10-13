"""Bounded PTY capture followed by exact offline delivery checks."""

import argparse
import base64
import fcntl
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import termios
import time

from check_delivery import Challenge
from corpus_acceptance import LIMITS, MeasuredSession, settled
from corpus_reference import ROOT, check_screen, digest, snapshot
from terminal_baseline import Frames
from rss_sampling import PROTOCOL, verify_quit_boundary

WINDOW_NS = 500_000_000
MAX_BYTES = 16 * 1024**2
MAX_PACKETS = 100_000
BEGIN = b"\x1b[?2026h"
END = b"\x1b[?2026l"
ACTIONS = [("opening", None), ("end", {"input_hex": "47"}),
           ("start", {"input_hex": "67"}), ("light", {"input_hex": "74"}),
           ("narrow", {"resize": [100, 50]}), ("wide", {"resize": [160, 50]}),
           ("dark", {"input_hex": "74"})]


def check_protocol(protocol):
    if (not isinstance(protocol, dict) or set(protocol) != {"window_ns", "steps"}
            or type(protocol["window_ns"]) is not int
            or not 0 < protocol["window_ns"] <= 5_000_000_000
            or not isinstance(protocol["steps"], list) or not 1 <= len(protocol["steps"]) <= 32):
        raise ValueError("Invalid declared capture protocol")
    labels = set()
    for index, step in enumerate(protocol["steps"]):
        if (not isinstance(step, dict) or set(step) != {"label", "action", "state"} or not isinstance(step["label"], str)
                or not step["label"] or step["label"] in labels):
            raise ValueError("Invalid declared capture step")
        labels.add(step["label"])
        action = step["action"]
        if index == 0:
            if action is not None:
                raise ValueError("First capture step must be opening")
        elif not isinstance(action, dict) or set(action) not in ({"input_hex"}, {"resize"}):
            raise ValueError("Invalid declared public action")
        elif "input_hex" in action:
            if not isinstance(action["input_hex"], str):
                raise ValueError("Invalid declared input encoding")
            payload = bytes.fromhex(action["input_hex"])
            if not 0 < len(payload) <= 128:
                raise ValueError("Invalid declared input length")
        elif (not isinstance(action["resize"], list) or len(action["resize"]) != 2
              or any(type(n) is not int or not 1 <= n <= 512 for n in action["resize"])):
            raise ValueError("Invalid declared terminal geometry")
        state = step["state"]
        required = {"expected_file", "split", "wrapped", "horizontal"}
        optional = {"anchor", "require_anchor", "require_file", "anchor_old", "anchor_byte"}
        if not isinstance(state, dict) or not required <= state.keys() <= required | optional:
            raise ValueError("Invalid declared source state")
        for name in ("expected_file", "horizontal", "anchor", "anchor_byte"):
            if name in state and (type(state[name]) is not int or state[name] < 0):
                raise ValueError("Invalid declared source position")
        for name in ("split", "wrapped", "require_anchor", "require_file", "anchor_old"):
            if name in state and type(state[name]) is not bool:
                raise ValueError("Invalid declared source mode")
        if state.get("require_anchor") and "anchor" not in state:
            raise ValueError("Required anchor is absent")
    return protocol


def write_json(path, value):
    with path.open("x") as stream:
        json.dump(value, stream, indent=2)
        stream.write("\n")


def wanted_case(path, case_id):
    return next(case for case in json.loads(path.read_text())["cases"] if case["id"] == case_id)


def source_check(binary, corpus):
    env = dict(os.environ)
    env.pop("NATIVE_DIFF_DELIVERY_TRACE", None)
    canonical = subprocess.run([binary, "inspect", corpus.path, "--canonical"],
                               capture_output=True, check=True, timeout=30, env=env).stdout
    if hashlib.sha256(canonical).hexdigest() != corpus.ref["source_sha256"]:
        raise AssertionError("Independent full-source digest differs")


class Capture(MeasuredSession):
    def __init__(self, *args, max_bytes=MAX_BYTES, **kwargs):
        self.packets = []
        self.captured_bytes = 0
        self.max_bytes = max_bytes
        self.capture_error = None
        super().__init__(*args, **kwargs)

    def read(self, timeout=.005):
        packet = super().read(timeout)
        if packet and self.capture_error is None:
            data, received = packet
            if (self.captured_bytes + len(data) > self.max_bytes
                    or len(self.packets) >= MAX_PACKETS):
                # Continue draining during cleanup after recording has failed.
                self.capture_error = "Capture byte or packet limit exceeded; recording incomplete"
            else:
                self.packets.append((data, received - self.started_ns))
                self.captured_bytes += len(data)
        return packet

    def inject(self, action):
        if self.rss_error or self.capture_error:
            raise RuntimeError(self.rss_error or self.capture_error)
        started = time.monotonic_ns()
        if "resize" in action:
            width, height = action["resize"]
            fcntl.ioctl(self.master, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
        else:
            payload = bytes.fromhex(action["input_hex"])
            if os.write(self.master, payload) != len(payload):
                raise RuntimeError("Partial public-command injection")
        self.last_injection = (started, time.monotonic_ns())
        return self.last_injection

    def boundary(self):
        if self.read(0) is not None:
            raise RuntimeError("Output arrived outside the capture window during validation")
        if self.returncode is not None or self.rss_error or self.capture_error:
            raise RuntimeError(self.rss_error or self.capture_error or "Viewer exited between actions")

    def window(self, started, window_ns=WINDOW_NS):
        deadline = started + window_ns
        while time.monotonic_ns() < deadline:
            self.read(min(.005, max(0, (deadline - time.monotonic_ns()) / 1e9)))
            if self.returncode is not None or self.rss_error or self.capture_error:
                raise RuntimeError(self.rss_error or self.capture_error or "Viewer exited during capture")
        return time.monotonic_ns()


class Replay:
    def __init__(self, corpus, protocol=None):
        self.corpus = corpus
        self.protocol = check_protocol(protocol) if protocol is not None else None
        self.window_ns = protocol["window_ns"] if protocol is not None else WINDOW_NS
        self.frames = Frames()
        self.state = {"expected_file": 0, "split": True, "wrapped": False, "horizontal": 0}

    def step(self, index, window, packets, expected):
        if self.protocol is None:
            label, action = ACTIONS[index]
        else:
            step = self.protocol["steps"][index]
            label, action = step["label"], step["action"]
            self.state = step["state"].copy()
        if window["label"] != label or window["action"] != action:
            raise ValueError("Recorded action differs from the declared sequence")
        if self.frames.pending:
            raise ValueError("Partial synchronized frame at input boundary")
        if action and "resize" in action:
            width, height = action["resize"]
            self.frames.screen.resize(height, width)
        if self.protocol is None and label in ("end", "start"):
            endpoint = self.corpus.ref["last" if label == "end" else "first"]
            self.state.update(expected_file=endpoint["file"], anchor=endpoint["source_ordinal"],
                              require_anchor=True, anchor_old=endpoint["new"] is None)
        started = window["started_ns"]
        if window["deadline_ns"] != started + self.window_ns or window["finished_ns"] < window["deadline_ns"]:
            raise ValueError("Invalid capture window")
        first_text = first_style = first_exact = indexed = None
        observations = []
        markers = re.findall(rb"\x1b\[\?2026[hl]", b"".join(data for data, _ in packets))
        if not markers or markers != [BEGIN, END] * (len(markers) // 2):
            raise ValueError("Unbalanced synchronized updates")
        for data, received in packets:
            if not started <= received <= window["deadline_ns"]:
                raise ValueError("Receipt falls outside its action window")
            for _ in self.frames.feed(data):
                actual = snapshot(self.frames.screen)
                content = actual["rows"][1:-1] == expected["rows"][1:-1]
                styles = content and actual["styles"][1:-1] == expected["styles"][1:-1]
                exact = actual == expected and settled(self.frames.screen)
                elapsed = (received - started) / 1e6
                if content:
                    if first_text is None:
                        first_text = elapsed
                    if indexed is None and "Loading" not in actual["rows"][-1]:
                        indexed = elapsed
                if styles and first_style is None:
                    first_style = elapsed
                if exact:
                    check_screen(self.frames.screen, self.corpus, **self.state)
                    if first_exact is None:
                        first_exact = elapsed
                observations.append({"frame": self.frames.count, "received_ns": received,
                                     "content": content, "styles": styles, "exact": exact})
        if self.frames.pending:
            raise ValueError("Partial synchronized frame at capture end")
        if first_exact is None:
            raise AssertionError("No exact settled screen in capture window")
        if snapshot(self.frames.screen) != expected:
            raise AssertionError("Final captured screen differs from frozen cells")
        records = check_screen(self.frames.screen, self.corpus, **self.state)
        return {"content_ms": first_text, "styled_ms": first_style, "ready_ms": first_exact,
                "index_ready_upper_bound_ms": indexed, "frames": observations,
                "independently_checked_records": len(records)}


def run(binary, case, expected_path, out, max_bytes=MAX_BYTES, *, protocol=None, child_env=None):
    out.mkdir(parents=True, exist_ok=False)
    report = {"schema": 3, "status": "running", "case": case, "steps": [], "rss_protocol": PROTOCOL,
              "expected_path": str(expected_path.resolve()), "window_ns": WINDOW_NS,
              "max_bytes": max_bytes, "max_packets": MAX_PACKETS}
    session = None
    try:
        report.update(binary_sha256=digest(binary), expected_sha256=digest(expected_path))
        corpus = Challenge(case)
        source_check(binary, corpus)
        report["source"] = "pass"
        endpoints = {0, corpus.ref["first"]["file"], corpus.ref["last"]["file"]}
        if any("\u200d" in line.text for index in endpoints for line in corpus.file(index).lines):
            report.update(status="unsupported_oracle", reason="Joined emoji is source-only: pyte cannot preserve it.")
            return report
        expected = wanted_case(expected_path, case["id"])
        if protocol is not None:
            check_protocol(protocol)
            if expected.get("protocol") != protocol:
                raise ValueError("Frozen protocol differs")
            report.update(schema=4, protocol=protocol, window_ns=protocol["window_ns"])
        elif "protocol" in expected:
            raise ValueError("Explicit capture protocol required")
        actions = ([(step["label"], step["action"]) for step in protocol["steps"]]
                   if protocol is not None else ACTIONS)
        window_ns = report["window_ns"]
        if [step["label"] for step in expected["steps"]] != [label for label, _ in actions]:
            raise ValueError("Frozen action labels differ")
        replay = Replay(corpus, protocol)
        session = Capture(str(binary.resolve()), {"path": str(corpus.path)}, LIMITS,
                          max_bytes=max_bytes, child_env=child_env)
        report["session_started_ns"] = session.started_ns
        for index, (label, action) in enumerate(actions):
            if action is not None:
                session.boundary()
            begin = len(session.packets)
            started, injected = (session.started_ns, session.started_ns) if action is None else session.inject(action)
            window = {"label": label, "action": action, "packet_start": begin,
                      "started_ns": started - session.started_ns,
                      "injected_ns": injected - session.started_ns,
                      "deadline_ns": started - session.started_ns + window_ns}
            report["steps"].append(window)
            finished = session.window(started, window_ns)
            window.update(packet_end=len(session.packets), finished_ns=finished - session.started_ns)
            before = time.monotonic_ns()
            window["observation"] = replay.step(index, window, session.packets[begin:], expected["steps"][index]["screen"])
            window["validation_ms"] = (time.monotonic_ns() - before) / 1e6
        session.boundary()
        report["quit_packet_start"] = len(session.packets)
        report["quit"] = session.quit()
        report["quit_started_ns"] = session.last_injection[0] - session.started_ns
        verify_quit_boundary(session.rss_shutdown, session.started_ns + report["steps"][-1]["finished_ns"],
                             report["quit"]["input_ns"])
        quit_result = report["quit"]
        if (quit_result["exit_code"] != 0 or not quit_result["termios_restored"]
                or not all(quit_result["cleanup_sequences_received"].values())):
            raise AssertionError("Terminal cleanup failed")
        tail = session.packets[report["steps"][-1]["packet_end"]:]
        if BEGIN in b"".join(data for data, _ in tail):
            raise AssertionError("Unassigned frame after final capture window")
        report["status"] = "complete"
    except BaseException as error:
        report.update(status="failed", error=str(error))
        raise
    finally:
        try:
            if session is not None:
                session.close()
                if session.rss_error:
                    raise RuntimeError(session.rss_error)
                if session.capture_error and report["status"] == "complete":
                    raise RuntimeError(session.capture_error)
        except BaseException as error:
            report.update(status="failed", cleanup_error=str(error))
            raise
        finally:
            if session is not None:
                sha = hashlib.sha256()
                with gzip.open(out / "packets.jsonl.gz", "xb") as stream:
                    for data, received in session.packets:
                        sha.update(data)
                        stream.write((json.dumps({"received_ns": received,
                                      "base64": base64.b64encode(data).decode("ascii")}) + "\n").encode())
                report.update(captured_bytes=session.captured_bytes, captured_sha256=sha.hexdigest(),
                              packets=len(session.packets), capture_error=session.capture_error,
                              total_received_bytes=session.raw_bytes,
                              cleanup_exit_code=session.returncode,
                              cleanup_termios_restored=session.restored,
                              rss_error=session.rss_error,
                              rss_shutdown=session.rss_shutdown,
                              sampler_stopped=not session.sampler.is_alive())
            write_json(out / "report.json", report)
    return report


def read_packets(path, report):
    packets, size, previous = [], 0, -1
    sha = hashlib.sha256()
    with gzip.open(path, "rt") as stream:
        while line := stream.readline(1024 * 1024):
            if not line.endswith("\n"):
                raise ValueError("Oversized or incomplete packet record")
            record = json.loads(line)
            data = base64.b64decode(record["base64"], validate=True)
            received = record["received_ns"]
            if not data or not isinstance(received, int) or received < previous:
                raise ValueError("Invalid or reversed packet receipt")
            size += len(data)
            if size > MAX_BYTES or len(packets) >= MAX_PACKETS:
                raise ValueError("Stored capture exceeds bounds")
            previous = received
            sha.update(data)
            packets.append((data, received))
    if (size != report["captured_bytes"] or sha.hexdigest() != report["captured_sha256"]
            or len(packets) != report["packets"]):
        raise ValueError("Raw capture identity differs")
    return packets


def replay_recording(directory):
    report = json.loads((directory / "report.json").read_text())
    if (report["schema"] not in (3, 4) or report.get("rss_protocol") != PROTOCOL
            or report["status"] != "complete" or report["capture_error"] is not None
            or "rss_error" not in report or report["rss_error"] is not None
            or report.get("sampler_stopped") is not True):
        raise ValueError("Cannot accept an incomplete capture")
    expected_path = Path(report["expected_path"])
    if digest(expected_path) != report["expected_sha256"]:
        raise ValueError("Frozen expectation identity differs")
    packets = read_packets(directory / "packets.jsonl.gz", report)
    corpus = Challenge(report["case"])
    expected = wanted_case(expected_path, report["case"]["id"])
    if report["schema"] == 4:
        protocol = check_protocol(expected.get("protocol"))
        if report.get("protocol") != protocol or report["window_ns"] != protocol["window_ns"]:
            raise ValueError("Frozen protocol differs")
        count = len(protocol["steps"])
    else:
        protocol, count = None, len(ACTIONS)
        if report["window_ns"] != WINDOW_NS or "protocol" in report or "protocol" in expected:
            raise ValueError("Legacy capture protocol differs")
    if len(report["steps"]) != count or len(expected["steps"]) != count:
        raise ValueError("Incomplete action sequence")
    origin = report["session_started_ns"]
    if report["quit"]["input_ns"] != origin + report["quit_started_ns"]:
        raise ValueError("Quit clock origin differs")
    verify_quit_boundary(report.get("rss_shutdown"), origin + report["steps"][-1]["finished_ns"],
                         report["quit"]["input_ns"])
    if report["rss_shutdown"] != report["quit"]["rss_shutdown"]:
        raise ValueError("RSS shutdown boundary differs after cleanup")
    replay, results, cursor, previous_deadline = Replay(corpus, protocol), [], 0, -1
    for index, window in enumerate(report["steps"]):
        end = window["packet_end"]
        if not window["packet_start"] == cursor < end <= len(packets):
            raise ValueError("Noncontiguous packet windows")
        if not previous_deadline <= window["started_ns"] <= window["injected_ns"] <= window["deadline_ns"]:
            raise ValueError("Invalid input ordering")
        results.append(replay.step(index, window, packets[cursor:end], expected["steps"][index]["screen"]))
        cursor, previous_deadline = end, window["deadline_ns"]
    tail = b"".join(data for data, _ in packets[cursor:])
    if (report["quit_packet_start"] != cursor or report["quit_started_ns"] < previous_deadline
            or any(received < report["quit_started_ns"] for _, received in packets[cursor:])):
        raise ValueError("Invalid cleanup boundary")
    if BEGIN in tail:
        raise ValueError("Unassigned frame after final capture window")
    if not all(sequence in tail for sequence in [END, b"\x1b[?1049l", b"\x1b[?25h", b"\x1b[?1006l"]):
        raise ValueError("Raw cleanup sequences missing")
    if report["source"] != "pass" or report["cleanup_exit_code"] != 0 or not report["cleanup_termios_restored"]:
        raise ValueError("Source or terminal cleanup did not pass")
    if not all(report["quit"]["cleanup_sequences_received"].values()):
        raise ValueError("Missing terminal cleanup sequences")
    if report["quit"]["exit_observation_ms"] is None or report["quit"]["exit_observation_ms"] < 0:
        raise ValueError("Invalid quit elapsed time")
    return {"status": "pass", "steps": results}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("recording", type=Path, help="Replay a completed recording; never launches the viewer")
    args = parser.parse_args()
    print(json.dumps(replay_recording(args.recording), indent=2))


if __name__ == "__main__":
    main()
