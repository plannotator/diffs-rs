"""Hash-bound replay of independently checked real-PTY corpus captures."""

import argparse
from datetime import datetime, timezone
import gzip
import hashlib
import importlib.metadata
import json
import math
from pathlib import Path
import platform
import subprocess
import sys
import time

from corpus_acceptance import LIMITS, PRESENTATION, MeasuredSession, await_state, travel
from corpus_reference import Corpus, ROOT, digest, restore, snapshot
from check_corpus_oracle import check
from rss_sampling import PROTOCOL, verify_quit_boundary
from sidebar_reference import diff_geometry, diff_region

CATEGORIES = ["forward", "reverse", "distant_seek", "layout", "resize"]
TARGETS = {"policy": "uniform-development-v1", "first_text_ms": 500, "first_style_ms": 1000,
           "index_ready_ms": 2000, "navigation_p95_ms": 50, "navigation_max_ms": 200,
           "quit_ms": 250, "steady_rss_bytes": 128 * 1024**2, "peak_rss_bytes": 192 * 1024**2}
MILESTONES = {
    "content_ms": "Receipt of correct final-layout diff text; upper bound, not first progressive text",
    "styled_ms": "Receipt of correct final-layout diff colors; excludes sidebar publication",
    "ready_ms": "Receipt of the complete settled screen, including sidebar",
    "index_ready_upper_bound_ms": "Loading indicator absent after correct final-layout diff text",
    "capture": "Live PTY parsing and frozen-cell comparison; semantic and sequential checks run in preparation",
}
REQUIRED_CONTROLS = {"source-glyph", "source-number", "source-sign", "blank-body",
                     "wrong-syntax-foreground", "wrong-source-background", "sidebar-heading-count",
                     "sidebar-divider", "sidebar-missing-current", "sidebar-duplicate-current",
                     "sidebar-wrong-current", "sidebar-unexpected-focus", "sidebar-foreground", "sidebar-background"}


def bindings(binary, reference):
    paths = [binary, reference, Path(sys.executable), Path(PROTOCOL["command"]), ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
    paths += [ROOT / name for name in ["FIVE_PATCH_ACCEPTANCE.md", "LARGE_DIFFS.md", "MEASUREMENT_POLICY.md",
                                      "SIDEBAR_SPEC.md", "SIDEBAR_CORPUS_PLAN.md",
                                      "fixtures/diffshub/scenario.json", "fixtures/diffshub/corpus.json"]]
    paths += sorted((ROOT / "src").rglob("*.rs"))
    paths += sorted((ROOT / "assets").rglob("*.json"))
    for manifest in sorted((ROOT / "crates").glob("*/Cargo.toml")):
        paths += [manifest]
        paths += sorted((manifest.parent / "src").rglob("*.rs"))
        for data in ("assets", "generated"):
            paths += sorted(path for path in (manifest.parent / data).rglob("*") if path.is_file())
    paths += [ROOT / "examples/sequential_style_reference.rs"]
    paths += [ROOT / "tools" / name for name in ["corpus_acceptance.py", "corpus_reference.py",
              "corpus_measure.py", "patch_reference.py", "trace_reference.py", "terminal_baseline.py",
              "check_corpus_oracle.py", "sidebar_reference.py", "rss_sampling.py"]]
    return {str(path.resolve()): digest(path) for path in paths}


def semantic_trace(trace):
    allowed = {"fixture", "kind", "opening", "steps", "binary_sha256", "style_check", "quit", "presentation"}
    if set(trace) != allowed:
        raise ValueError(f"Unexpected trace fields: {set(trace) ^ allowed}")
    steps = [{**step, "action": {key: value for key, value in step["action"].items()
                                if key != "setup_ms"}} for step in trace["steps"]]
    return {**{name: trace[name] for name in ["fixture", "kind", "opening", "presentation"]}, "steps": steps,
            "style_check": {key: value for key, value in trace["style_check"].items()
                            if key != "reference_sha256"}}


def compare_baseline(trace, baseline_path):
    baseline = json.loads(gzip.decompress(baseline_path.read_bytes()))
    actual, expected = semantic_trace(trace), semantic_trace(baseline)
    for field in expected:
        if actual[field] != expected[field]:
            raise ValueError(f"Candidate changed baseline semantics: {field}")
    return {"trace": str(baseline_path.resolve()), "trace_sha256": digest(baseline_path),
            "semantic_sha256": hashlib.sha256(json.dumps(expected, sort_keys=True).encode()).hexdigest()}


def freeze(trace_path, binary, reference, baseline_path=None):
    trace = json.loads(gzip.decompress(trace_path.read_bytes()))
    if trace.get("error") or not trace.get("style_check"):
        raise ValueError("A failed or unchecked rehearsal cannot be frozen")
    if trace.get("presentation") != PRESENTATION or trace["kind"] not in ("latency", "memory"):
        raise ValueError("A current sidebar-qualified performance trace is required")
    if trace.get("binary_sha256") != digest(binary) or trace["style_check"]["reference_sha256"] != digest(reference):
        raise ValueError("Rehearsal executable identities differ")
    baseline = compare_baseline(trace, baseline_path) if baseline_path else None
    oracle = check(trace_path)
    checked = {case["name"] for case in oracle["cases"] if case["status"] == "rejected"}
    required = REQUIRED_CONTROLS.copy()
    if oracle["sidebar_coverage"]["visible-file-only"]:
        required.add("sidebar-current-outside-visible-files")
    if oracle["bounded_horizontal_views"]:
        required.add("bounded-horizontal-source-glyph")
    if oracle["status"] != "pass" or not required <= checked:
        raise ValueError("Actual-capture oracle controls did not pass")
    corpus = Corpus(trace["fixture"])
    identities = bindings(binary, reference)
    identities[str(corpus.path)] = digest(corpus.path)
    identities[str(corpus.ref_path)] = digest(corpus.ref_path)
    style_path = trace_path.parent / "sequential-reference.json.gz"
    identities[str(style_path)] = digest(style_path)
    return {"created_at": datetime.now(timezone.utc).isoformat(), "trace": str(trace_path),
            "trace_sha256": digest(trace_path), "binary": str(binary), "sha256": identities,
            "oracle_check": oracle, "rss_protocol": PROTOCOL,
            "presentation": dict(PRESENTATION), "milestones": dict(MILESTONES),
            "viewports": sorted({(s["expected"]["width"], s["expected"]["height"],
                                   *diff_geometry(s["expected"]["width"], s["expected"]["height"], s["state"].get("sidebar", False)))
                                  for s in [trace["opening"], *trace["steps"]]}),
            "baseline_comparison": baseline,
            "limits": LIMITS, "targets": TARGETS, "repetitions": 5, "memory_tours": 3, "hold_seconds": 1,
            "python": sys.version, "dependencies": {name: importlib.metadata.version(name)
             for name in ["pyte", "regex", "wcwidth"]}, "platform": platform.platform(),
            "cpu": subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip(),
            "rustc": subprocess.check_output(["rustc", "-Vv"], text=True).strip(),
            "physical_memory": int(subprocess.check_output(["sysctl", "-n", "hw.memsize"], text=True))}


def verify(frozen):
    if frozen.get("presentation") != PRESENTATION or frozen.get("milestones") != MILESTONES:
        raise ValueError("Presentation or milestone contract differs. Prepare a new freeze.")
    if frozen.get("rss_protocol") != PROTOCOL:
        raise ValueError("RSS protocol differs. Use the archived runner or prepare a new cohort.")
    if (frozen.get("hold_seconds"), frozen.get("memory_tours"), frozen.get("repetitions")) != (1, 3, 5):
        raise ValueError("Frozen hold or repetition policy differs")
    if frozen.get("targets") != TARGETS:
        raise ValueError("Measurement policy differs. Historical freezes require their archived runner; prepare a new freeze for current runs.")
    trace_path = Path(frozen["trace"])
    if digest(trace_path) != frozen["trace_sha256"]:
        raise ValueError("Frozen trace changed")
    trace = json.loads(gzip.decompress(trace_path.read_bytes()))
    if baseline := frozen.get("baseline_comparison"):
        if compare_baseline(trace, Path(baseline["trace"])) != baseline:
            raise ValueError("Baseline trace or semantic comparison changed")
    for path, wanted in frozen["sha256"].items():
        if digest(Path(path)) != wanted:
            raise ValueError(f"Frozen dependency changed: {path}")
    if sys.version != frozen["python"] or LIMITS != frozen["limits"]:
        raise ValueError("Measurement runtime differs")
    for name, wanted in frozen["dependencies"].items():
        if importlib.metadata.version(name) != wanted:
            raise ValueError(f"Dependency version changed: {name}")
    return trace


def hold(session, expected, seconds, *, foreground_rss=False):
    if foreground_rss and (session.sampler.ident is not None or session.rss_stop.is_set()):
        raise ValueError("Foreground RSS holds require an open session without a background sampler")
    begin = time.monotonic_ns()
    until = begin + int(seconds * 1e9)
    record = {"start_ns": begin, "deadline_ns": until, "status": "incomplete", "query_indices": []}
    if foreground_rss:
        record["sampling_scope"] = "settled-hold-only-v1"
    session.rss_holds.append(record)
    try:
        next_query = begin
        while time.monotonic_ns() < until:
            if foreground_rss and time.monotonic_ns() >= next_query:
                query = session.sample_rss_once()
                next_query = query["end_ns"] + int(PROTOCOL["post_query_wait_seconds"] * 1e9)
                if query["status"] != "ok" or session.rss_error:
                    raise RuntimeError(session.rss_error or f"Unavailable settled RSS: {query['status']}")
            packet = session.read(.01)
            if packet:
                for _ in session.frames.feed(packet[0]):
                    if snapshot(session.frames.screen) != expected:
                        raise AssertionError("Settled hold changed after completed styling")
            if session.rss_error or session.returncode is not None:
                raise RuntimeError(session.rss_error or "Viewer exited during hold")
    except Exception as error:
        record["error"] = str(error)
        raise
    finally:
        record["end_ns"] = time.monotonic_ns()
        boundary = min(until, record["end_ns"])
        indices = [index for index, query in enumerate(session.rss_queries)
                   if query["status"] == "ok" and begin <= query["start_ns"] <= query["end_ns"] <= boundary]
        samples = [session.rss_queries[index]["rss_bytes"] for index in indices]
        record.update(query_indices=indices, samples=len(samples))
    if len(samples) < PROTOCOL["minimum_hold_samples"]:
        record["error"] = f"Too few settled RSS samples: {len(samples)}"
        raise RuntimeError(record["error"])
    record.update(status="complete", rss_max_bytes=max(samples), rss_min_bytes=min(samples))
    return dict(record)


def expected_region(step):
    return diff_region(restore(step["expected"]), step["state"].get("sidebar", False))


def observe(session, trace, step, start, region):
    result = await_state(session, None, step["state"], start, expected=step["expected"], wanted_region=region)
    result.pop("screen")
    result.pop("records")
    return result


def replay(frozen, trace, out, number):
    opening_region = expected_region(trace["opening"])
    session = MeasuredSession(Path(frozen["binary"]), {"path": str(ROOT / trace["fixture"]["local_path"])}, LIMITS)
    result = {"fixture": trace["fixture"]["id"], "kind": trace["kind"], "repeat": number,
              "argv": session.argv, "samples": [], "status": "running"}
    try:
        result["opening"] = observe(session, trace, trace["opening"], session.started_ns, opening_region)
        for tour in range(frozen["memory_tours"] if trace["kind"] == "memory" else 1):
            width, height = 160, 50
            for step in trace["steps"]:
                region = expected_region(step)
                action = step["action"]
                if "travel" in action:
                    started, setup_ms = travel(session, bytes.fromhex(action["travel"]), width, height)
                    interval = None
                else:
                    if "resize" in action:
                        width, height = action["resize"]
                    started, interval = session.inject(action)
                    setup_ms = None
                sample = {"label": step["label"], "category": step["category"], "tour": tour + 1,
                          "actual_input_interval_ms": interval, "setup_ms": setup_ms}
                sample.update(observe(session, trace, step, started, region))
                if step.get("hold"):
                    sample["hold"] = hold(session, step["expected"], frozen["hold_seconds"])
                result["samples"].append(sample)
                if step["category"] == "setup" or len(result["samples"]) % 40 == 0:
                    print(f"{result['fixture']} {trace['kind']} repeat {number} tour {tour + 1}: {step['label']}", flush=True)
            if trace["kind"] == "memory":
                holds = [s["hold"]["rss_max_bytes"] for s in result["samples"] if s["tour"] == tour + 1 and "hold" in s]
                print(f"tour {tour + 1} settled peak {max(holds) / 1024**2:.1f} MiB", flush=True)
        result["viewing_finished_ns"] = time.monotonic_ns()
        result["quit"] = session.quit()
        verify_quit_boundary(session.rss_shutdown, result["viewing_finished_ns"], result["quit"]["input_ns"])
        if result["quit"]["exit_code"] != 0 or not result["quit"]["termios_restored"] or not all(result["quit"]["cleanup_sequences_received"].values()):
            raise AssertionError("Viewer exit or terminal cleanup failed")
        result["status"] = "complete"
    except Exception as error:
        result["status"] = "failed"
        result["error"] = str(error)
        result["failed_step"] = step["label"] if "step" in locals() else "opening"
    finally:
        try:
            session.close()
        except Exception as error:
            result.update(status="failed", cleanup_error=str(error))
            result.setdefault("error", f"Cleanup failed: {error}")
        if session.rss_error:
            result["status"] = "failed"
            result.setdefault("error", session.rss_error)
        result["sampled_peak_rss_bytes"] = session.sample_peak
        result["os_peak_rss_bytes"] = session.rusage.ru_maxrss if session.rusage else None
        result["rss_timeline"] = session.rss_timeline
        result["rss_queries"] = session.rss_queries
        result["rss_holds"] = session.rss_holds
        result["rss_protocol"] = PROTOCOL
        result["rss_error"] = session.rss_error
        result["rss_shutdown"] = session.rss_shutdown
        result["sampler_stopped"] = not session.sampler.is_alive()
        result["rss_sampler_wall_ms"] = session.rss_sampler_ns / 1e6
        result["raw_pty_sha256"] = session.raw_hash.hexdigest()
        result["raw_pty_bytes"] = session.raw_bytes
        result["terminal_renderer_rss_bytes"] = None
        result["cleanup_after_failure"] = {"exit_code": session.returncode, "termios_restored": session.restored}
        (out / f"repeat-{number}.json.gz").write_bytes(gzip.compress(json.dumps(result).encode(), mtime=0))
        if result["status"] != "complete":
            (out / f"repeat-{number}-last-screen.txt").write_text("\n".join(session.frames.screen.display))
            (out / f"repeat-{number}-last-screen.json.gz").write_bytes(gzip.compress(json.dumps(snapshot(session.frames.screen)).encode(), mtime=0))
            (out / f"repeat-{number}-tail.ansi").write_bytes(session.raw_tail)
    return result


def distribution(values, expected):
    complete = len(values) == expected
    return {"count": len(values), "expected": expected,
            "p95_ms": sorted(values)[math.ceil(.95 * len(values)) - 1] if complete else None,
            "max_ms": max(values) if complete else None,
            "complete": complete}


def summarize(results, kind):
    count_ok = len(results) == (5 if kind == "latency" else 1)
    if kind == "latency":
        count_ok &= all(sum(s["category"] == category for s in r["samples"]) == 40
                        for r in results for category in CATEGORIES)
    else:
        counts = [sum(s["tour"] == tour for s in results[0]["samples"]) for tour in [1, 2, 3]]
        count_ok &= min(counts) > 0 and len(set(counts)) == 1
    summary = {"fixture": results[0]["fixture"], "kind": kind, "targets": dict(TARGETS),
               "milestones": dict(MILESTONES), "presentation": dict(PRESENTATION),
               "opening_target_interpretation": "Passing upper bounds establish the targets; a failed upper bound does not prove earlier progressive text or colors missed them",
               "complete": count_ok and all(r["status"] == "complete" for r in results),
               "repetitions_expected": 5 if kind == "latency" else 1,
               "repetitions_attempted": len(results),
               "failures": [{"repeat": r["repeat"], "step": r.get("failed_step"), "error": r.get("error")}
                            for r in results if r["status"] != "complete"],
               "opening": [r.get("opening") for r in results],
               "quit": [r.get("quit") for r in results],
               "os_peak_rss_bytes": max(r["os_peak_rss_bytes"] for r in results)
               if all(r["os_peak_rss_bytes"] is not None for r in results) else None}
    summary["not_run_repetitions"] = list(range(len(results) + 1, summary["repetitions_expected"] + 1))
    if kind == "latency":
        summary["categories"] = {category: {milestone: distribution([
            s[milestone] for r in results for s in r["samples"] if s["category"] == category], 200)
            for milestone in ["content_ms", "styled_ms"]} for category in CATEGORIES}
        summary["navigation_target_pass"] = all(d["complete"] and d["p95_ms"] <= TARGETS["navigation_p95_ms"] and d["max_ms"] <= TARGETS["navigation_max_ms"]
                                                  for c in summary["categories"].values() for d in c.values())
        summary["opening_target_pass"] = summary["complete"] and all(
            r["opening"]["content_ms"] <= TARGETS["first_text_ms"] and r["opening"]["styled_ms"] <= TARGETS["first_style_ms"]
            and r["opening"]["index_ready_upper_bound_ms"] <= TARGETS["index_ready_ms"]
            for r in results)
    else:
        summary["tour_settled_rss_max_bytes"] = [max((s["hold"]["rss_max_bytes"] for s in results[0]["samples"]
                                                    if s["tour"] == tour and "hold" in s), default=None)
                                                for tour in [1, 2, 3]]
        summary["steady_rss_bytes"] = summary["tour_settled_rss_max_bytes"][2] if summary["complete"] else None
        summary["memory_target_pass"] = summary["complete"] and summary["steady_rss_bytes"] <= TARGETS["steady_rss_bytes"] and summary["os_peak_rss_bytes"] is not None and summary["os_peak_rss_bytes"] <= TARGETS["peak_rss_bytes"]
    summary["quit_target_pass"] = summary["complete"] and all(0 <= r["quit"]["exit_observation_ms"] <= TARGETS["quit_ms"] for r in results)
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("trace", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/diffs")
    parser.add_argument("--reference", type=Path, default=ROOT / "target/release/examples/sequential_style_reference")
    parser.add_argument("--baseline-trace", type=Path)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--prepare-only", action="store_true")
    mode.add_argument("--freeze", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    if args.freeze and args.baseline_trace:
        raise ValueError("Baseline relationship must be established during preparation")
    frozen = json.loads(args.freeze.read_text()) if args.freeze else freeze(args.trace.resolve(), args.binary.resolve(), args.reference.resolve(), args.baseline_trace)
    if Path(frozen["trace"]).resolve() != args.trace.resolve():
        raise ValueError("Supplied trace differs from frozen trace")
    (args.output / "freeze.json").write_text(json.dumps(frozen, indent=2) + "\n")
    trace = verify(frozen)
    if args.prepare_only:
        print("Freeze verified; no viewer started.", flush=True)
        return
    results = []
    for repeat in range(1, (1 if trace["kind"] == "memory" else frozen["repetitions"]) + 1):
        verify(frozen)
        results.append(replay(frozen, trace, args.output, repeat))
        try:
            verify(frozen)
        except ValueError as error:
            results[-1].update(status="failed", error=str(error), failed_step="post-run identity check")
            (args.output / f"repeat-{repeat}-identity-failure.json").write_text(json.dumps(results[-1], indent=2))
        if results[-1]["status"] != "complete":
            break
    summary = summarize(results, trace["kind"])
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary), flush=True)
    if not summary["complete"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
