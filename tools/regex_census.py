"""Checked, unscored PTY ownership census; never infer engine bytes from counts."""

import argparse
from collections import defaultdict
import gzip
import hashlib
import json
import os
from pathlib import Path
import time

from corpus_acceptance import LIMITS, MeasuredSession, await_state, travel
from corpus_measure import bindings, compare_baseline
from corpus_reference import ROOT, digest


def write_json(path, value):
    with path.open("x") as stream:
        json.dump(value, stream, indent=2)
        stream.write("\n")


def summarize(census):
    owners = {}
    groups = defaultdict(set)
    grammar_owners = defaultdict(set)
    references = cold = 0
    syntax_ids = set()
    for syntax in census["loaded_syntaxes"]:
        identity = syntax["id"]
        assert identity not in syntax_ids and 0 <= identity < census["syntax_count"]
        syntax_ids.add(identity)
        coordinates = set()
        local_patterns = defaultdict(set)
        for rule in syntax["patterns"]:
            location = (rule["context"], rule["rule"])
            assert location not in coordinates and 0 <= location[0] < syntax["contexts"]
            coordinates.add(location)
            pattern, owner = rule["pattern"], rule["compiled_owner"]
            assert isinstance(pattern, str)
            if owner is None:
                cold += 1
                continue
            assert type(owner) is int and owner > 0
            references += 1
            assert owners.setdefault(owner, pattern) == pattern, "One owner names two patterns"
            groups[pattern].add(owner)
            grammar_owners[owner].add(identity)
            local_patterns[pattern].add(owner)
        assert all(len(values) == 1 for values in local_patterns.values()), "Per-grammar sharing differs"
    duplicates = []
    for pattern, values in groups.items():
        if len(values) > 1:
            grammars = sorted(set().union(*(grammar_owners[owner] for owner in values)))
            assert len(grammars) > 1
            duplicates.append({"pattern_sha256": hashlib.sha256(pattern.encode()).hexdigest(),
                               "owners": sorted(values), "syntax_ids": grammars})
    duplicates.sort(key=lambda row: (-len(row["owners"]), row["pattern_sha256"]))
    first = census["first_line_patterns"]
    assert census["first_line_cache_initialized"] or not first
    return {"loaded_syntaxes": len(syntax_ids), "syntax_count": census["syntax_count"],
            "compiled_rule_references": references, "uncompiled_rule_references": cold,
            "distinct_compiled_owners": len(owners), "distinct_compiled_patterns": len(groups),
            "patterns_with_duplicate_owners": len(duplicates),
            "redundant_compiled_roots": sum(len(row["owners"]) - 1 for row in duplicates),
            "first_line_cache_initialized": census["first_line_cache_initialized"],
            "first_line_compiled_references": sum(row["compiled_owner"] is not None for row in first),
            "duplicates": duplicates,
            "scope": "Static live roots only; not duplicated bytes, RSS, or temporary substitutions"}


def session_for(binary, fixture, path):
    previous = os.environ.get("NATIVE_DIFF_REGEX_CENSUS")
    os.environ["NATIVE_DIFF_REGEX_CENSUS"] = str(path)
    try:
        return MeasuredSession(binary, {"path": str(fixture)}, LIMITS)
    finally:
        if previous is None:
            os.environ.pop("NATIVE_DIFF_REGEX_CENSUS", None)
        else:
            os.environ["NATIVE_DIFF_REGEX_CENSUS"] = previous


def replay(binary, trace, out, tours, census_path, expect_write_error=False):
    session = session_for(binary, ROOT / trace["fixture"]["local_path"], census_path)
    report = {"status": "running", "pid": session.pid, "checked_views": [], "tours": tours}
    try:
        await_state(session, None, trace["opening"]["state"], session.started_ns,
                    expected=trace["opening"]["expected"], timeout=120)
        report["checked_views"].append("opening")
        for tour in range(tours):
            width, height = 160, 50
            for step in trace["steps"]:
                action = step["action"]
                if "travel" in action:
                    started, _ = travel(session, bytes.fromhex(action["travel"]), width, height)
                else:
                    if "resize" in action:
                        width, height = action["resize"]
                    started, _ = session.inject(action)
                await_state(session, None, step["state"], started, expected=step["expected"], timeout=120)
                report["checked_views"].append({"tour": tour + 1, "label": step["label"]})
            print(f"Tour {tour + 1}: all {len(trace['steps'])} views checked", flush=True)
        report["quit"] = session.quit()
        assert report["quit"]["exit_code"] == 0 and report["quit"]["termios_restored"]
        assert all(report["quit"]["cleanup_sequences_received"].values())
        assert not session.rss_error, session.rss_error
        report["status"] = "complete"
    except BaseException as error:
        report.update(status="failed", error=str(error))
        raise
    finally:
        try:
            session.close()
            report["sampler_stopped"] = not session.sampler.is_alive()
            if session.rss_error:
                raise RuntimeError(session.rss_error)
            assert b"Regex census worker failed" not in session.raw_tail
            write_error = b"Regex census failed:" in session.raw_tail
            assert write_error == expect_write_error, "Unexpected census write outcome"
            report["expected_write_error"] = expect_write_error
        except BaseException as error:
            report.update(status="failed", cleanup_error=str(error))
            raise
        finally:
            report.update(output_sha256=session.raw_hash.hexdigest(), output_bytes=session.raw_bytes,
                          rss_error=session.rss_error, exit_code=session.returncode,
                          termios_restored=session.restored)
            with (out / "tail.ansi").open("xb") as stream:
                stream.write(session.raw_tail)
            write_json(out / "process.json", report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("trace", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--baseline-trace", type=Path, required=True)
    parser.add_argument("--check-control", type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    trace = json.loads(gzip.decompress(args.trace.read_bytes()))
    assert trace["kind"] == "memory" and not trace.get("error")
    assert digest(args.reference) == trace["style_check"]["reference_sha256"]
    fixture = ROOT / trace["fixture"]["local_path"]
    assert digest(fixture) == trace["fixture"]["sha256"]
    hashes = bindings(args.binary, args.reference)
    for path in [args.trace, fixture, Path(__file__), ROOT / "REGEX_CENSUS_PLAN.md"]:
        hashes[str(path.resolve())] = digest(path)
    if args.check_control:
        hashes[str(args.check_control.resolve())] = digest(args.check_control)
    declaration = {"scope": "Ownership diagnostic only; no accepted latency or RSS series",
                   "started_ns": time.time_ns(), "sha256": hashes,
                   "baseline": compare_baseline(trace, args.baseline_trace),
                   "mode": "process-controls" if args.check_control else "three-tour-census"}
    write_json(out / "declaration.json", declaration)
    report = {"status": "running"}
    try:
        instrumentation = {name: os.environ.get(name) for name in (
            "MallocStackLogging", "MallocStackLoggingNoCompact", "MallocScribble",
            "MallocGuardEdges", "MallocNanoZone", "DYLD_INSERT_LIBRARIES",
            "NATIVE_DIFF_DELIVERY_TRACE")}
        report["instrumentation_environment"] = instrumentation
        if any(value is not None for name, value in instrumentation.items() if name != "MallocNanoZone"):
            raise ValueError("Unset inherited allocator/trace instrumentation for this diagnostic")
        census_path = out / "census.json"
        report["process"] = replay(args.binary.resolve(), trace, out,
                                    0 if args.check_control else 3, census_path)
        census = json.loads(census_path.read_text())
        report["census"] = summarize(census)
        assert report["census"]["distinct_compiled_owners"] > 0
        assert not census["first_line_cache_initialized"]
        if args.check_control:
            original = digest(census_path)
            failure = out / "existing-file-control"
            failure.mkdir()
            replay(args.binary.resolve(), trace, failure, 0, census_path, expect_write_error=True)
            assert digest(census_path) == original, "Diagnostic overwrote existing evidence"
            assert b"Regex census failed:" in (failure / "tail.ansi").read_bytes()
            ordinary = out / "ordinary-control"
            ordinary.mkdir()
            replay(args.check_control.resolve(), trace, ordinary, 0, ordinary / "absent.json")
            assert not (ordinary / "absent.json").exists(), "Ordinary binary emitted diagnostic data"
            report["controls"] = "write complete, existing file preserved and error detected, ordinary feature off"
        for path, wanted in hashes.items():
            assert digest(Path(path)) == wanted, f"Bound input changed: {path}"
        assert compare_baseline(trace, args.baseline_trace) == declaration["baseline"]
        report.update(status="complete", census_sha256=digest(census_path))
    except BaseException as error:
        report.update(status="failed", error=str(error))
        raise
    finally:
        write_json(out / "report.json", report)


if __name__ == "__main__":
    main()
