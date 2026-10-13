"""Exercise the attribution CLI with a real trace and deliberately corrupted copies."""

import argparse
from copy import deepcopy
import json
from pathlib import Path
import subprocess
import sys

from trace_reference import digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("trace", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    original_hash = digest(args.trace)
    original = json.loads(args.trace.read_text())
    if original.get("schema") not in (2, 3, 4, 6) or not original["requests"]:
        raise ValueError("Requires a schema-2, schema-3, schema-4, or schema-6 trace from an actual viewer session")
    analyzer = Path(__file__).with_name("analyze_delivery_trace.py")
    report = {"scope": "CLI integrity controls, not viewer or performance measurements",
              "trace_sha256": original_hash, "analyzer_sha256": digest(analyzer),
              "status": "running", "checks": []}

    def run(label, trace, expected_error=None):
        path = args.output / f"{label}.json"
        path.write_text(json.dumps(trace) + "\n")
        output = args.output / f"{label}-analysis.json"
        result = subprocess.run([sys.executable, str(analyzer), str(output), str(path)],
                                text=True, capture_output=True, timeout=30)
        (args.output / f"{label}.log").write_text(result.stdout + result.stderr)
        if expected_error is not None:
            if result.returncode == 0 or expected_error not in result.stderr:
                raise AssertionError(f"{label}: expected rejection did not occur")
            report["checks"].append({"case": label, "result": "rejected as required"})
            return None
        if result.returncode != 0:
            raise AssertionError(f"{label}: {result.stderr}")
        return json.loads(output.read_text())["traces"][0]

    try:
        run("original", original)
        report["checks"].append({"case": "original", "result": "accepted actual observations"})
        for field, error in [("dropped", "Incomplete event trace"),
                             ("requests_dropped", "Incomplete request summary trace")]:
            damaged = deepcopy(original)
            damaged[field] += 1
            run(field, damaged, error)
        damaged = deepcopy(original)
        damaged["requests"][0]["processed_records"] += 1
        run("inconsistent-records", damaged, "Processed-source accounting differs")
        if original["schema"] >= 3:
            damaged = deepcopy(original)
            damaged["requests"][0]["initialization"]["invalid"] = True
            run("invalid-initialization", damaged, "Invalid initialization counters")
            damaged = deepcopy(original)
            damaged["requests"][0]["initialization_inconsistent"] = True
            run("inconsistent-initialization", damaged, "Inconsistent initialization accounting")
            damaged = deepcopy(original)
            damaged["requests"][0]["initialization"]["grammar"]["failed"] = 1
            run("failed-initialization", damaged, "Failed initialization")
            damaged = deepcopy(original)
            damaged["requests"][0]["initialization"]["source_patterns"] = 1 << 64
            run("overflow-initialization", damaged, "counter is out of range or saturated")
            damaged = deepcopy(original)
            damaged["requests"][0]["theme"]["total_ns"] = (1 << 64) - 1
            run("saturated-original-phase", damaged, "counter is out of range or saturated")
            damaged = deepcopy(original)
            work = damaged["requests"][0]
            work["initialization_requested"]["grammar"]["calls"] = (
                work["initialization"]["grammar"]["calls"] + 1)
            run("nested-initializer-counts", damaged, "Nested timing subset differs")
            damaged = deepcopy(original)
            work = damaged["requests"][0]
            work["initialization_requested"]["source_patterns"] = (
                work["initialization"]["source_patterns"] + 1)
            run("nested-source-pattern-counts", damaged, "Nested source-pattern subset differs")
            damaged = deepcopy(original)
            damaged["requests"][0]["parse_requested_remainder"]["calls"] += 1
            run("parser-remainder", damaged, "Parser remainder accounting differs")
            longest_index = next((i for i, work in enumerate(original["requests"])
                                  if work["longest_parser_call"] is not None), None)
            if longest_index is not None:
                damaged = deepcopy(original)
                work = damaged["requests"][longest_index]
                longest = work["longest_parser_call"]
                side = "requested" if longest["requested"] else "replay"
                longest["initialization"]["grammar"]["calls"] = (
                    work[f"initialization_{side}"]["grammar"]["calls"] + 1)
                run("longest-parser-initialization", damaged, "Nested timing subset differs")
            else:
                report["checks"].append({"case": "longest-parser-initialization",
                                          "result": "unavailable: no observed parser call"})
        if original["schema"] == 6:
            damaged = deepcopy(original)
            damaged["requests"][0]["shared_source_lines"] = 1
            run("unexpected-shared-source", damaged, "supports patch-only traces")
            damaged = deepcopy(original)
            damaged["requests"][0]["shared_source_lines"] = 1 << 64
            run("overflow-shared-source", damaged, "counter is out of range or saturated")
        if original["schema"] == 4:
            damaged = deepcopy(original)
            damaged["requests"][0]["shared_parser_calls"] = 1 << 64
            run("overflow-shared-calls", damaged, "counter is out of range or saturated")
            damaged = deepcopy(original)
            work = damaged["requests"][0]
            work["shared_parser_calls"] = work["processed_records"] + 1
            run("shared-call-count", damaged, "Shared parser accounting differs")
            index = next(i for i, work in enumerate(original["requests"])
                         if work["longest_parser_call"] is not None)
            damaged = deepcopy(original)
            damaged["requests"][index]["longest_parser_call"]["side"] = "unknown"
            run("unknown-parser-side", damaged, "Longest parser call has an invalid side")
            damaged = deepcopy(original)
            work = damaged["requests"][index]
            work["shared_parser_calls"] = 0
            work["longest_parser_call"]["side"] = "both"
            run("missing-shared-call", damaged, "Shared parser accounting differs")
            damaged = deepcopy(original)
            work = damaged["requests"][index]
            work["shared_parser_calls"] = work["parse_requested"]["calls"] + work["parse_replay"]["calls"]
            work["longest_parser_call"]["side"] = "old"
            run("nonshared-longest-with-all-shared", damaged, "Shared parser accounting differs")
        damaged = deepcopy(original)
        removed = damaged["requests"].pop()
        analysis = run("missing-completion", damaged)["request_work"]
        if (analysis["observation_status"] != "partial"
                or removed["generation"] not in analysis["started_without_summary"]):
            raise AssertionError("Missing completion was not reported as partial")
        report["checks"].append({"case": "missing-completion", "result": "explicit partial observation"})
        boundary = next(i for i, event in enumerate(original["events"]) if event["kind"] == "request")
        prefix = deepcopy(original)
        prefix["events"] = prefix["events"][:boundary]
        prefix["requests"] = []
        analysis = run("before-first-request", prefix)
        for field in ["ready_to_successful_offer", "successful_offer_to_drain", "drain_to_next_frame"]:
            if analysis[field] != {"count": 0, "median_ms": None, "p95_ms": None, "max_ms": None}:
                raise AssertionError("No observations were reported as measured values")
        report["checks"].append({"case": "before-first-request", "result": "timings unavailable, not zero"})
        if digest(args.trace) != original_hash:
            raise AssertionError("Original evidence changed")
        if digest(analyzer) != report["analyzer_sha256"]:
            raise AssertionError("Analyzer changed during the controls")
        report["status"] = "pass"
    except Exception as error:
        report.update(status="failed", error=str(error))
        raise
    finally:
        with (args.output / "checks.json").open("x") as stream:
            json.dump(report, stream, indent=2)
            stream.write("\n")


if __name__ == "__main__":
    main()
