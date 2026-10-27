"""Run the existing five-patch checks serially, preserving baseline semantics."""

import argparse
import json
from pathlib import Path
import subprocess
import sys

from aggregate_five_patch_report import FIXTURES
from corpus_reference import ROOT, digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--reuse-captures", type=Path)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/diffs")
    parser.add_argument("--reference", type=Path, default=ROOT / "target/release/examples/sequential_style_reference")
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    reference = args.reference.resolve()
    report = {"binary_sha256": digest(binary), "reference_sha256": digest(reference),
              "driver_sha256": digest(Path(__file__)), "commands": [], "results": []}

    def run(label, script, *arguments):
        command = [sys.executable, str(ROOT / "tools" / script), *map(str, arguments)]
        print(label, flush=True)
        with (out / f"{label}.log").open("x") as log:
            result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)
        report["commands"].append({"argv": command, "exit_code": result.returncode,
                                   "log": str(out / f"{label}.log")})
        if result.returncode:
            raise RuntimeError(f"{label} exited {result.returncode}")

    for fixture, latency_capture, latency_run, memory_capture, memory_run in FIXTURES:
        for kind, capture, results in [("latency", latency_capture, latency_run), ("memory", memory_capture, memory_run)]:
            entry = {"fixture": fixture, "kind": kind, "status": "running"}
            try:
                trace = out / capture / "trace.json.gz"
                reused = args.reuse_captures.resolve() / capture / "trace.json.gz" if args.reuse_captures else None
                if reused and reused.exists():
                    trace = reused
                    entry["reused_capture"] = str(trace)
                else:
                    run(capture, "corpus_acceptance.py", binary, reference, out / capture,
                        "--fixture", fixture, *(["--memory"] if kind == "memory" else []))
                prepare = out / f"{fixture}-{kind}-prepare"
                run(prepare.name, "corpus_measure.py", trace, prepare, "--prepare-only",
                    "--binary", binary, "--reference", reference,
                    "--baseline-trace", args.baseline.resolve() / capture / "trace.json.gz")
                run(results, "corpus_measure.py", trace, out / results, "--freeze", prepare / "freeze.json")
                entry.update(status="complete", summary=json.loads((out / results / "summary.json").read_text()))
            except RuntimeError as error:
                entry.update(status="failed", error=str(error))
            report["results"].append(entry)
            report["status"] = "running"
            (out / "checkpoint.json").write_text(json.dumps(report, indent=2) + "\n")
            print(f"{fixture} {kind}: {entry['status']}", flush=True)
    report["status"] = "complete" if all(r["status"] == "complete" for r in report["results"]) else "failed"
    (out / "checkpoint.json").write_text(json.dumps(report, indent=2) + "\n")
    if report["status"] != "complete":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
