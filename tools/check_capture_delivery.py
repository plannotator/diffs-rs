"""Verify capture with actual viewer sessions and corrupted copies of their output."""

from copy import deepcopy
from itertools import cycle
from pathlib import Path
import base64
import gzip
import hashlib
import json

from capture_delivery import (BEGIN, END, read_packets, replay_recording, run,
                              wanted_case, write_json)
from corpus_reference import digest


def copy_capture(out, report, packets):
    out.mkdir()
    sha = hashlib.sha256()
    with gzip.open(out / "packets.jsonl.gz", "xb") as stream:
        for data, received in packets:
            sha.update(data)
            stream.write((json.dumps({"received_ns": received,
                          "base64": base64.b64encode(data).decode("ascii")}) + "\n").encode())
    report.update(captured_bytes=sum(len(data) for data, _ in packets),
                  captured_sha256=sha.hexdigest(), packets=len(packets))
    write_json(out / "report.json", report)


def verify_protocol(recording, out):
    """Check the explicit protocol reader using copies of an actual completed capture."""
    from first_visit_highlighting import protocol

    out.mkdir(parents=True, exist_ok=False)
    checks = {"status": "running", "scope": "Offline copied-artifact controls; no viewer or timing trial",
              "checks": []}
    identities = {name: digest(recording / name) for name in ("report.json", "packets.jsonl.gz")}
    try:
        baseline = replay_recording(recording)
        original = json.loads((recording / "report.json").read_text())
        raw = read_packets(recording / "packets.jsonl.gz", original)
        expected = wanted_case(Path(original["expected_path"]), original["case"]["id"])
        expected = deepcopy(expected)
        expected["protocol"] = protocol(original["case"])
        expected_path = out / "expected.json"
        write_json(expected_path, {"cases": [expected]})
        declared = deepcopy(original)
        declared.update(schema=4, protocol=expected["protocol"], expected_path=str(expected_path.resolve()),
                        expected_sha256=digest(expected_path))

        def control(label, report, packets, rejection=None):
            directory = out / label
            copy_capture(directory, report, packets)
            try:
                actual = replay_recording(directory)
            except (AssertionError, ValueError) as error:
                if rejection is None or rejection not in str(error):
                    raise
                checks["checks"].append({"id": label, "status": "rejected", "error": str(error)})
                return
            if rejection is not None:
                raise AssertionError(f"{label}: changed evidence was accepted")
            if actual != baseline:
                raise AssertionError("Explicit protocol changed retained observation results")
            checks["checks"].append({"id": label, "status": "pass"})

        control("legacy", deepcopy(original), raw)
        control("explicit", deepcopy(declared), raw)
        changed = deepcopy(declared)
        changed["protocol"]["window_ns"] = 5_000_000_000
        control("changed-protocol", changed, raw, "Frozen protocol differs")
        changed = deepcopy(declared)
        changed["steps"][0]["deadline_ns"] += 1
        control("changed-window", changed, raw, "Invalid capture window")
        changed = deepcopy(declared)
        changed["steps"][1]["action"] = {"input_hex": "67"}
        control("changed-action", changed, raw, "Recorded action differs")
        changed_raw = raw.copy()
        at = declared["steps"][1]["packet_start"]
        changed_raw[at] = (raw[at][0], declared["steps"][1]["started_ns"] - 1)
        control("unassigned-output", deepcopy(declared), changed_raw, "Receipt falls outside")
        if any(digest(recording / name) != sha for name, sha in identities.items()):
            raise AssertionError("Original recording changed")
        checks["status"] = "pass"
    except BaseException as error:
        checks.update(status="failed", error=str(error))
        raise
    finally:
        write_json(out / "checks.json", checks)
    return checks


def verify(binary, cases, expected_path, out):
    out.mkdir(parents=True, exist_ok=False)
    report = {"status": "running", "checks": [], "source_only": [],
              "scope": "Real-process capture and copied-artifact fault controls, not performance trials"}
    try:
        for case in cases:
            directory = out / case["id"]
            actual = run(binary, case, expected_path, directory)
            if actual["status"] == "unsupported_oracle":
                report["source_only"].append(case["id"])
                continue
            replayed = replay_recording(directory)
            if replayed["steps"] != [step["observation"] for step in actual["steps"]]:
                raise AssertionError("Persisted replay differs from original capture validation")
            report["checks"].append({"case": case["id"], "result": "exact replay and source pass"})
        if len(report["checks"]) != 4 or len(report["source_only"]) != 1:
            raise AssertionError("Delivery field accounting differs")

        original = out / "continuous"
        original_hashes = {name: digest(original / name) for name in ["report.json", "packets.jsonl.gz"]}
        original_report = json.loads((original / "report.json").read_text())
        packets = read_packets(original / "packets.jsonl.gz", original_report)

        def control(label, copied_report, copied_packets, error):
            directory = out / label
            copy_capture(directory, copied_report, copied_packets)
            try:
                result = replay_recording(directory)
            except (AssertionError, ValueError) as failure:
                if error is None or error not in str(failure):
                    raise AssertionError(f"{label}: wrong rejection: {failure}") from failure
                report["checks"].append({"case": label, "result": "rejected", "error": str(failure)})
                return None
            if error is not None:
                raise AssertionError(f"{label}: damaged evidence was accepted")
            report["checks"].append({"case": label, "result": "pass"})
            return result

        fragments, offsets = [], [0]
        for data, received in packets:
            cursor = 0
            for size in cycle((1, 2, 7, 61, 257)):
                if cursor == len(data):
                    break
                end = min(len(data), cursor + size)
                fragments.append((data[cursor:end], received))
                cursor = end
            offsets.append(len(fragments))
        fragmented = deepcopy(original_report)
        for window in fragmented["steps"]:
            window["packet_start"] = offsets[window["packet_start"]]
            window["packet_end"] = offsets[window["packet_end"]]
        fragmented["quit_packet_start"] = offsets[fragmented["quit_packet_start"]]
        result = control("fragmented", fragmented, fragments, None)
        if result["steps"] != [step["observation"] for step in original_report["steps"]]:
            raise AssertionError("Fragmentation changed cells or receipt milestones")

        final_index = original_report["steps"][-1]["packet_end"] - 1
        data, received = packets[final_index]
        screen = wanted_case(expected_path, "continuous")["steps"][-1]["screen"]
        y, x, character = next((y, x, c) for y, row in enumerate(screen["rows"][3:-1], 3)
                               for x, c in enumerate(row) if c.isascii() and c.isalpha())
        position = f"\x1b[{y + 1};{x + 1}H".encode()
        for label, update in [("correct-then-wrong-text", position + b"?"),
                              ("correct-then-wrong-color", position + b"\x1b[38;2;1;2;3m" + character.encode())]:
            damaged = packets.copy()
            damaged[final_index] = (data + BEGIN + update + END, received)
            control(label, deepcopy(original_report), damaged, "Final captured screen differs")

        def truncated(label, recording, raw):
            damaged, copied = raw.copy(), deepcopy(recording)
            last = copied["steps"][-1]["packet_end"] - 1
            chunk, stamp = damaged[last]
            if len(chunk) > 1:
                damaged[last] = (chunk[:-1], stamp)
            else:
                damaged.pop(last)
                copied["steps"][-1]["packet_end"] -= 1
                copied["quit_packet_start"] -= 1
            control(label, copied, damaged, "Unbalanced synchronized updates")

        truncated("truncated-update", original_report, packets)
        split_tail, split_report = packets.copy(), deepcopy(original_report)
        if len(data) > 1:
            split_tail[final_index:final_index + 1] = [(data[:-1], received), (data[-1:], received)]
            split_report["steps"][-1]["packet_end"] += 1
            split_report["quit_packet_start"] += 1
        truncated("truncated-one-byte-tail", split_report, split_tail)
        damaged = packets.copy()
        damaged[1] = (damaged[1][0], damaged[0][1] - 1)
        control("reversed-receipt", deepcopy(original_report), damaged, "Invalid or reversed packet receipt")
        malformed = deepcopy(original_report)
        malformed["steps"][1]["packet_start"] += 1
        control("skipped-packet", malformed, packets, "Noncontiguous packet windows")

        for label, field, value in [("late-sampler-error", "rss_error", "RSS sampler failed"),
                                    ("sampler-still-running", "sampler_stopped", False)]:
            incomplete = deepcopy(original_report)
            incomplete[field] = value
            control(label, incomplete, packets, "Cannot accept an incomplete capture")

        for label, value in [("join-after-quit", original_report["quit"]["input_ns"] + 1),
                              ("missing-join-boundary", None)]:
            incomplete = deepcopy(original_report)
            if value is None:
                incomplete.pop("rss_shutdown")
            else:
                incomplete["rss_shutdown"]["finished_ns"] = value
            control(label, incomplete, packets, "RSS shutdown boundary")
        negative_quit = deepcopy(original_report)
        negative_quit["quit"]["exit_observation_ms"] = -1
        control("negative-quit-time", negative_quit, packets, "Invalid quit elapsed time")

        case = next(case for case in cases if case["id"] == "continuous")
        try:
            run(binary, case, expected_path, out / "capture-limit", max_bytes=128)
        except RuntimeError as error:
            if "Capture byte or packet limit exceeded" not in str(error):
                raise
        else:
            raise AssertionError("Capture limit did not stop the real viewer session")
        limited = json.loads((out / "capture-limit/report.json").read_text())
        if (limited["status"] != "failed" or limited["captured_bytes"] > 128
                or limited["cleanup_exit_code"] is None or limited.get("cleanup_error")
                or limited.get("rss_error") or not limited["sampler_stopped"]):
            raise AssertionError("Bounded capture failure did not clean up its real child")
        report["checks"].append({"case": "capture-limit", "result": "failed as required; child reaped",
                                 "exit_code": limited["cleanup_exit_code"],
                                 "termios_restored": limited["cleanup_termios_restored"]})
        if any(digest(original / name) != sha for name, sha in original_hashes.items()):
            raise AssertionError("Original capture changed during controls")
        report["status"] = "pass"
    except BaseException as error:
        report.update(status="failed", error=str(error))
        raise
    finally:
        write_json(out / "checks.json", report)
    return report
