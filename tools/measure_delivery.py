"""Alternating, uninstrumented development comparisons; retain every run and failure."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import time

from check_delivery import run
from corpus_acceptance import LIMITS, MeasuredSession, await_state
from corpus_reference import ROOT, digest


def cpu_sample(pid):
    value = subprocess.run(["/bin/ps", "-p", str(pid), "-o", "time="],
                           text=True, capture_output=True, check=True).stdout.strip()
    parts = [float(part) for part in value.split(":")]
    return sum(part * 60**index for index, part in enumerate(reversed(parts)))


def idle(binary, case, expected):
    session = MeasuredSession(binary, {"path": str(ROOT / case["path"])}, LIMITS)
    try:
        await_state(session, None, {}, session.started_ns, expected=expected["steps"][0]["screen"])
        # Drain any unrequested, already-published offscreen replies before the idle interval.
        until = time.monotonic() + .3
        while time.monotonic() < until:
            packet = session.read()
            if packet:
                list(session.frames.feed(packet[0]))
        before = cpu_sample(session.pid)
        frames = session.frames.count
        start = time.monotonic()
        while time.monotonic() - start < 10:
            packet = session.read(.02)
            if packet:
                list(session.frames.feed(packet[0]))
            if session.returncode is not None or session.rss_error:
                raise RuntimeError(session.rss_error or "Viewer exited while idle")
        result = {"wall_seconds": time.monotonic() - start,
                  "cpu_seconds": cpu_sample(session.pid) - before,
                  "cpu_sample_resolution_seconds": .01,
                  "frames": session.frames.count - frames, "quit": session.quit()}
        if result["quit"]["exit_code"] != 0 or not result["quit"]["termios_restored"]:
            raise AssertionError("Idle quit cleanup failed")
        return result
    finally:
        session.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("expected", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--idle", action="store_true")
    args = parser.parse_args()
    if "NATIVE_DIFF_DELIVERY_TRACE" in os.environ:
        raise ValueError("Unset delivery instrumentation before measuring")
    manifest = ROOT / "evidence/delivery-v1/cases/manifest.json"
    cases = json.loads(manifest.read_text())["cases"]
    expected = json.loads(args.expected.read_text())["cases"]
    paths = [args.reference, args.candidate, args.expected, manifest, Path(__file__)]
    paths += [ROOT / "tools" / name for name in ["check_delivery.py", "corpus_acceptance.py",
                                               "corpus_reference.py", "terminal_baseline.py", "rss_sampling.py"]]
    paths += [Path("/bin/ps")]
    paths += [ROOT / case[key] for case in cases for key in ["path", "reference"]]
    identities = {str(path.resolve()): digest(path) for path in paths}
    report = {"scope": "Alternating development comparison, no corpus-wide acceptance claim",
              "identities": identities, "status": "running", "runs": [], "idle": args.idle}
    with args.output.open("x") as output:
        try:
            for pair in range(3 if args.idle else 5):
                order = [("reference", args.reference.resolve()), ("candidate", args.candidate.resolve())]
                if pair % 2:
                    order.reverse()
                for label, binary in order:
                    for case, wanted in zip(cases, expected, strict=True):
                        # The same small complete-file workload is used for both idle binaries.
                        if args.idle and case["id"] != "many-small":
                            continue
                        active = {"pair": pair, "binary": label, "case": case["id"]}
                        report["active"] = active
                        print(active, flush=True)
                        result = idle(str(binary), case, wanted) if args.idle else run(str(binary), case, wanted)
                        for step in result.get("steps", []):
                            step.pop("screen")
                            step.pop("records")
                        report["runs"].append({**active, **result})
            for path, expected_hash in identities.items():
                if digest(Path(path)) != expected_hash:
                    raise ValueError(f"Comparison input changed: {path}")
            report["status"] = "complete"
        except Exception as error:
            report.update(status="failed", error=str(error))
            raise
        finally:
            json.dump(report, output, indent=2)


if __name__ == "__main__":
    main()
