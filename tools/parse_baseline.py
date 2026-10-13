"""Bounded, serial diagnostic baseline; this does not measure terminal rendering."""

import argparse
import hashlib
import json
import os
import signal
import subprocess
import time
from pathlib import Path


def stop(child):
    if child.poll() is not None:
        return
    try:
        os.killpg(child.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        child.wait(timeout=1)
    except subprocess.TimeoutExpired:
        os.killpg(child.pid, signal.SIGKILL)
        child.wait()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.mkdir(exist_ok=False)
    manifest = json.loads(Path("fixtures/diffshub/corpus.json").read_text())
    executable = args.executable.resolve()
    results = []
    for fixture in manifest["fixtures"]:
        with Path(fixture["local_path"]).open("rb") as source:
            if hashlib.file_digest(source, "sha256").hexdigest() != fixture["sha256"]:
                raise ValueError(f"Fixture identity changed: {fixture['id']}")
        usage = (args.output / (fixture["id"] + ".usage.txt")).resolve()
        stdout = args.output / (fixture["id"] + ".stdout.json")
        stderr = args.output / (fixture["id"] + ".stderr.txt")
        with stdout.open("xb") as out, stderr.open("xb") as err:
            start = time.monotonic()
            child = subprocess.Popen([
                "/usr/bin/time", "-l", "-o", str(usage), str(executable),
                "inspect", fixture["local_path"], "--summary",
            ], stdout=out, stderr=err, start_new_session=True)
            failure = None
            maximum_kib = 0
            try:
                while child.poll() is None:
                    sample = subprocess.run(
                        ["/bin/ps", "-axo", "pgid=,rss="],
                        capture_output=True, text=True, check=True, timeout=1,
                    )
                    resident = sum(int(row.split()[1]) for row in sample.stdout.splitlines()
                                   if row.split()[0] == str(child.pid))
                    maximum_kib = max(maximum_kib, resident)
                    if resident > 6 * 1024 * 1024:
                        failure = "safety_stop_above_6_GiB_process_group_RSS"
                    elif time.monotonic() - start > 30:
                        failure = "30_second_watchdog"
                    if failure:
                        break
                    time.sleep(0.05)
            finally:
                stop(child)
            elapsed = time.monotonic() - start
        summary = json.loads(stdout.read_text()) if child.returncode == 0 else None
        high_water = None
        if usage.exists():
            for row in usage.read_text().splitlines():
                if "maximum resident set size" in row:
                    high_water = int(row.split()[0])
        result = {
            "fixture": fixture["id"], "fixture_sha256": fixture["sha256"],
            "elapsed_seconds_including_wrapper_and_polling": elapsed,
            "exit_code": child.returncode, "failure": failure,
            "sampled_process_group_peak_bytes": maximum_kib * 1024,
            "os_viewer_high_water_bytes": high_water,
            "summary": summary,
            "git_totals_match": summary is not None and all([
                summary["files"] == fixture["git_numstat"]["file_records"],
                summary["additions"] == fixture["git_numstat"]["added_source_lines"],
                summary["deletions"] == fixture["git_numstat"]["deleted_source_lines"],
            ]),
        }
        results.append(result)
        print(json.dumps(result), flush=True)
    with executable.open("rb") as source:
        executable_hash = hashlib.file_digest(source, "sha256").hexdigest()
    report = {
        "scope": "One diagnostic parse/index run per fixture. Not PTY, display, five-run acceptance, or full-content correctness evidence.",
        "executable_sha256": executable_hash,
        "rss_units": "macOS time -l bytes; ps RSS KiB converted to bytes",
        "results": results,
    }
    with (args.output / "results.json").open("x") as output:
        json.dump(report, output, indent=2)
        output.write("\n")


if __name__ == "__main__":
    main()
