"""Check reporting corrections against retained real-process evidence, without running the viewer."""

import argparse
from copy import deepcopy
import gzip
import json
from pathlib import Path
import shutil
import subprocess
import sys

from aggregate_five_patch_report import FIXTURES
from corpus_measure import CATEGORIES, TARGETS, summarize, verify
from corpus_reference import ROOT, digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    baseline = args.baseline.resolve()
    args.output.mkdir(parents=True, exist_ok=False)
    identities = {}
    checks = []

    def read(path, compressed=False):
        identities[str(path)] = digest(path)
        data = path.read_bytes()
        return json.loads(gzip.decompress(data) if compressed else data)

    def series(name):
        return [read(path, True) for path in sorted((baseline / name).glob("repeat-*.json.gz"))]

    report = {"scope": "Reporting controls on stored observations, not new performance measurements",
              "status": "running", "checks": checks, "inputs": identities,
              "targets": TARGETS}
    try:
        for name, kind in [("linux-v6-v7-latency2", "latency"), ("pierre-memory-results", "memory")]:
            records = series(name)
            expected = summarize(records, kind)
            expected.pop("fixture")
            for label in ["linux-label-only", "unfamiliar-project"]:
                renamed = [{**row, "fixture": label} for row in records]
                actual = summarize(renamed, kind)
                actual.pop("fixture")
                assert actual == expected, f"Fixture name changed {kind} outcomes"
            checks.append(f"{kind}: unchanged observations have identical outcomes under both labels")

        records = series("pierre-latency2")
        assert not summarize(records, "latency")["navigation_target_pass"]
        partial = deepcopy(records[:1])
        partial[0].update(status="failed", failed_step="audit corruption control", error="deliberately removed remainder")
        partial[0]["samples"] = partial[0]["samples"][:1]
        incomplete = summarize(partial, "latency")
        assert not incomplete["complete"] and not incomplete["navigation_target_pass"]
        assert len(incomplete["not_run_repetitions"]) == 4 and incomplete["failures"]
        for category in CATEGORIES:
            assert all(d["p95_ms"] is None and d["max_ms"] is None for d in incomplete["categories"][category].values())
        checks.append("Real seek outlier remains failed; deliberately incomplete series gets no percentile")

        legacy = read(baseline / "pierre-latency2/freeze.json")
        for manifest in [legacy, {**legacy, "targets": {**TARGETS, "peak_rss_bytes": TARGETS["peak_rss_bytes"] + 1}}]:
            try:
                verify(manifest)
            except ValueError as error:
                assert "Measurement policy differs" in str(error)
            else:
                raise AssertionError("Accepted a historical or changed policy")
        checks.append("Historical and altered policies rejected before replay")

        copied = args.output / "supplied-evidence"
        copied.mkdir()
        for fixture, rehearsal, latency, _memory_rehearsal, memory in FIXTURES:
            oracle = "pierre-oracle.json" if fixture == "pierre-0800fb" else f"{fixture}-oracle.json"
            for relative in [f"{rehearsal}.log", oracle, f"{latency}/summary.json", f"{memory}/summary.json"]:
                source, destination = baseline / relative, copied / relative
                identities[str(source)] = digest(source)
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source, destination)

        def aggregate(name):
            path = args.output / name
            subprocess.run([sys.executable, str(ROOT / "tools/aggregate_five_patch_report.py"),
                            str(copied), str(path)], check=True, capture_output=True, text=True, timeout=30)
            return {entry["id"]: entry for entry in json.loads(path.read_text())["items"]}

        items = aggregate("retained-evidence-report.json")
        assert items["latency-pierre-0800fb"]["status"] == "fail"
        for fixture in ["pierre-0800fb", "node-59805", "bun-30412", "linux-v6-v7"]:
            assert items[f"memory-{fixture}"]["status"] == "fail"
        for key in ["source-digests", "engineering-checks"]:
            assert items[key]["status"] == "skipped" and "Unverified here" in items[key]["summary"]
        checks.append("Actual report CLI retains performance failures and makes no unsupported source/test passes")

        memory_path = copied / "pierre-memory-results/summary.json"
        memory = json.loads(memory_path.read_text())
        memory.update(complete=False, memory_target_pass=False, steady_rss_bytes=None)
        memory_path.write_text(json.dumps(memory))
        incomplete["opening"] = [None]
        (copied / "pierre-latency2/summary.json").write_text(json.dumps(incomplete))
        items = aggregate("incomplete-control-report.json")
        for key in ["latency-pierre-0800fb", "memory-pierre-0800fb"]:
            assert items[key]["status"] == "fail" and "complete=False" in items[key]["summary"]
        checks.append("Actual report CLI handles a missing opening and does not call incomplete work complete")

        oracle_path = copied / "pierre-oracle.json"
        oracle = json.loads(oracle_path.read_text())
        oracle["cases"][0]["status"] = "accepted"
        oracle_path.write_text(json.dumps(oracle))
        items = aggregate("oracle-control-report.json")
        assert items["interior-pierre-0800fb"]["status"] == "fail"
        assert "all rejected=False" in items["interior-pierre-0800fb"]["summary"]
        checks.append("Failed corruption control is not described as rejected")

        oracle["cases"] = []
        oracle_path.write_text(json.dumps(oracle))
        items = aggregate("empty-oracle-control-report.json")
        assert items["interior-pierre-0800fb"]["status"] == "fail"
        checks.append("Missing corruption controls cannot vacuously pass")

        for path, expected in identities.items():
            assert digest(Path(path)) == expected, f"Historical input changed: {path}"
        report["status"] = "pass"
    except Exception as error:
        report.update(status="failed", error=str(error))
        raise
    finally:
        report["tools"] = {name: digest(ROOT / "tools" / name) for name in
                           ["check_reporting.py", "corpus_measure.py", "aggregate_five_patch_report.py"]}
        (args.output / "checks.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps({"status": report["status"], "checks": checks}), flush=True)


if __name__ == "__main__":
    main()
