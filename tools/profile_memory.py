"""Unscored native allocation snapshots around an existing real-PTY memory tour."""

import argparse
import gzip
import json
from pathlib import Path
import subprocess
import time

from corpus_acceptance import LIMITS, MeasuredSession, await_state, travel
from corpus_reference import ROOT, digest


def capture(pid, output, label):
    results = {}
    commands = {
        "vmmap": ["/usr/bin/vmmap", "-summary", str(pid)],
        "heap": ["/usr/bin/heap", "-s", "--showSizes", str(pid)],
        "allocations": ["/usr/bin/malloc_history", str(pid), "-allBySize"],
    }
    for name, command in commands.items():
        observation = {"command": command, "started_ns": time.monotonic_ns(),
                       "started_unix_ns": time.time_ns()}
        with (output / f"{label}-{name}.txt").open("x") as stream:
            try:
                result = subprocess.run(command, stdout=stream, stderr=subprocess.STDOUT, timeout=60)
                observation["exit_code"] = result.returncode
            except subprocess.TimeoutExpired:
                observation["error"] = "60-second timeout"
            finally:
                observation["finished_ns"] = time.monotonic_ns()
                observation["finished_unix_ns"] = time.time_ns()
        results[name] = observation
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("trace", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--tours", type=int, default=3, choices=[0, 1, 3])
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    trace = json.loads(gzip.decompress(args.trace.read_bytes()))
    if trace["kind"] != "memory" or trace.get("error"):
        raise ValueError("Requires a successful memory trace")
    fixture = ROOT / trace["fixture"]["local_path"]
    if digest(fixture) != trace["fixture"]["sha256"]:
        raise ValueError("Fixture does not match the checked trace")
    report = {"scope": "Allocation attribution only; instrumented timings are not accepted measurements",
              "binary_sha256": digest(args.binary), "trace_sha256": digest(args.trace),
              "profiler_sha256": digest(Path(__file__)),
              "snapshot_scope": "Sequential command intervals, not simultaneous memory observations",
              "snapshots": {}, "checked_views": [], "status": "running"}
    session = MeasuredSession(args.binary.resolve(), {"path": str(fixture)}, LIMITS,
                              child_env={"MallocStackLogging": "1"})
    report["viewer_pid"] = session.pid
    report["child_environment_override"] = {"MallocStackLogging": "1"}
    try:
        await_state(session, None, trace["opening"]["state"], session.started_ns,
                    expected=trace["opening"]["expected"], timeout=120)
        report["checked_views"].append("opening")
        report["snapshots"]["opening"] = capture(session.pid, args.output, "opening")
        print("Opening allocation snapshot complete", flush=True)
        for tour in range(args.tours):
            width, height = 160, 50
            for step in trace["steps"]:
                action = step["action"]
                if "travel" in action:
                    started, _ = travel(session, bytes.fromhex(action["travel"]), width, height)
                else:
                    if "resize" in action:
                        width, height = action["resize"]
                    started, _ = session.inject(action)
                await_state(session, None, step["state"], started,
                            expected=step["expected"], timeout=120)
                report["checked_views"].append({"tour": tour + 1, "label": step["label"]})
            label = f"tour-{tour + 1}"
            report["snapshots"][label] = capture(session.pid, args.output, label)
            print(f"{label} allocation snapshot complete", flush=True)
        report["quit"] = session.quit()
        if report["quit"]["exit_code"] != 0 or not report["quit"]["termios_restored"]:
            raise AssertionError("Viewer exit/terminal restoration failed")
        if any(observation.get("exit_code") != 0
               for snapshot in report["snapshots"].values() for observation in snapshot.values()):
            raise RuntimeError("One or more allocation snapshots failed; inspect command results")
        if digest(args.binary) != report["binary_sha256"] or digest(args.trace) != report["trace_sha256"]:
            raise ValueError("Executable or trace changed during profiling")
        if digest(fixture) != trace["fixture"]["sha256"]:
            raise ValueError("Fixture changed during profiling")
        report["status"] = "complete"
    except BaseException as error:
        report.update(status="failed", error=str(error))
        raise
    finally:
        try:
            session.close()
            if session.rss_error:
                raise RuntimeError(session.rss_error)
        except BaseException as error:
            report.update(status="failed", cleanup_error=str(error))
            raise
        finally:
            report["rss_timeline"] = session.rss_timeline
            report["rss_queries"] = session.rss_queries
            report["exit_code"] = session.returncode
            report["termios_restored"] = session.restored
            report["rss_error"] = session.rss_error
            report["sampler_stopped"] = not session.sampler.is_alive()
            report["finished_ns"] = time.monotonic_ns()
            with (args.output / "report.json").open("x") as output:
                json.dump(report, output, indent=2)
                output.write("\n")


if __name__ == "__main__":
    main()
