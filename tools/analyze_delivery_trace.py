"""Summarize bounded delivery attribution, never terminal paint or accepted latency."""

import argparse
from bisect import bisect_left
from collections import Counter
import json
import math
from pathlib import Path
import statistics

from trace_reference import digest


U64_MAX = (1 << 64) - 1
INITIALIZERS = ("grammar", "outer_regex")


def counter(value, label):
    if type(value) is not int or not 0 <= value < U64_MAX:
        raise ValueError(f"Invalid {label}: counter is out of range or saturated")
    return value


def validate_timing(timing, label, initializer=False):
    calls, total, maximum = (counter(timing[field], f"{label}.{field}")
                             for field in ("calls", "total_ns", "max_ns"))
    if maximum > total or total > calls * maximum or (calls == 0 and maximum != 0):
        raise ValueError(f"Invalid {label} timing")
    if initializer:
        failed = counter(timing["failed"], f"{label}.failed")
        if failed:
            raise ValueError(f"Failed initialization in {label}")


def validate_counts(counts, label):
    if counts["invalid"] is not False:
        raise ValueError(f"Invalid initialization counters in {label}")
    counter(counts["source_patterns"], f"{label}.source_patterns")
    for phase in INITIALIZERS:
        validate_timing(counts[phase], f"{label}.{phase}", initializer=True)


def timing_remainder(parent, children, label):
    calls = parent["calls"] - sum(child["calls"] for child in children)
    total = parent["total_ns"] - sum(child["total_ns"] for child in children)
    maximum = max((child["max_ns"] for child in children), default=0)
    if (calls < 0 or total < 0 or maximum > parent["max_ns"]
            or total > calls * parent["max_ns"]
            or parent["max_ns"] > max(maximum, total)):
        raise ValueError(f"Nested timing subset differs in {label}")
    return {"calls": calls, "total_ns": total, "max_ns": None}


def counts_remainder(parent, children, label):
    patterns = parent["source_patterns"] - sum(child["source_patterns"] for child in children)
    if patterns < 0:
        raise ValueError(f"Nested source-pattern subset differs in {label}")
    return {**{phase: timing_remainder(parent[phase], [child[phase] for child in children],
                                       f"{label}.{phase}") for phase in INITIALIZERS},
            "source_patterns": patterns}


def initialization_ns(counts):
    return counter(sum(counts[phase]["total_ns"] for phase in INITIALIZERS),
                   "initialization duration")


def initialization_work(work, phase_total, schema):
    if work["initialization_inconsistent"] is not False:
        raise ValueError("Inconsistent initialization accounting")
    for name in ("initialization", "initialization_requested", "initialization_replay"):
        validate_counts(work[name], name)
    outside = counts_remainder(work["initialization"], [work["initialization_requested"],
                               work["initialization_replay"]], "request initialization")
    for side in ("requested", "replay"):
        parser = work[f"parse_{side}"]
        remainder = work[f"parse_{side}_remainder"]
        counts = work[f"initialization_{side}"]
        validate_timing(remainder, f"parse_{side}_remainder")
        initial_ns = initialization_ns(counts)
        if (remainder["calls"] != parser["calls"]
                or remainder["total_ns"] + initial_ns != parser["total_ns"]
                or remainder["max_ns"] > parser["max_ns"]
                or parser["max_ns"] > remainder["max_ns"] + initial_ns
                or any(counts[phase]["max_ns"] > parser["max_ns"] for phase in INITIALIZERS)
                or (parser["calls"] == 0 and (counts["source_patterns"]
                    or any(counts[phase]["calls"] for phase in INITIALIZERS)))):
            raise ValueError(f"Parser remainder accounting differs in {side}")
    longest = work["longest_parser_call"]
    longest_remainder = None
    if longest is None:
        if work["parse_requested"]["calls"] or work["parse_replay"]["calls"]:
            raise ValueError("Longest parser call is missing")
    else:
        if (type(longest["requested"]) is not bool
                or (schema != 4 and type(longest["old"]) is not bool)
                or (schema == 4 and longest["side"] not in ("old", "new", "both"))):
            raise ValueError("Longest parser call has an invalid side")
        counter(longest["source"], "longest parser source")
        counter(longest["ns"], "longest parser duration")
        validate_counts(longest["initialization"], "longest parser initialization")
        side = "requested" if longest["requested"] else "replay"
        parser = work[f"parse_{side}"]
        counts_remainder(work[f"initialization_{side}"], [longest["initialization"]],
                         "longest parser initialization")
        longest_remainder = longest["ns"] - initialization_ns(longest["initialization"])
        if longest_remainder < 0 or longest["ns"] != parser["max_ns"]:
            raise ValueError("Longest parser initialization exceeds its call or side")
        for parent, ns in ((parser, longest["ns"]),
                           (work[f"parse_{side}_remainder"], longest_remainder)):
            timing_remainder(parent, [{"calls": 1, "total_ns": ns, "max_ns": ns}],
                             "longest parser call")
    request_ns = initialization_ns(work["initialization"])
    outside_ns = initialization_ns(outside)
    disjoint_total = counter(phase_total + outside_ns, "disjoint phase duration")
    if request_ns > work["elapsed_ns"] or disjoint_total > work["elapsed_ns"]:
        raise ValueError("Initialization-inclusive phases exceed request duration")
    return {"request_total_ms": request_ns / 1e6,
            "inside_requested_parser_ms": initialization_ns(work["initialization_requested"]) / 1e6,
            "inside_replay_parser_ms": initialization_ns(work["initialization_replay"]) / 1e6,
            "outside_parser": outside, "outside_parser_total_ms": outside_ns / 1e6,
            "longest_parser_call_remainder_ns": longest_remainder,
            "phase_total_with_outside_initialization_ms": disjoint_total / 1e6,
            "unattributed_after_initialization_ms": (work["elapsed_ns"] - disjoint_total) / 1e6}


def distribution(values):
    ordered = sorted(values)
    if not ordered:
        return {"count": 0, "median_ms": None, "p95_ms": None, "max_ms": None}
    return {"count": len(values), "median_ms": statistics.median(ordered),
            "p95_ms": ordered[math.ceil(len(ordered) * .95) - 1], "max_ms": max(ordered)}


def request_work(trace):
    starts = {event["a"]: event for event in trace["events"]
              if event["kind"] == "worker_start"}
    submitted = {event["a"] for event in trace["events"] if event["kind"] == "request"}
    published = Counter(event["a"] for event in trace["events"] if event["kind"] == "published")
    full = Counter(event["a"] for event in trace["events"] if event["kind"] == "full")
    phases = ["theme", "parse_requested", "parse_replay", "visible_colors",
              "replay_scopes", "word_comparison", "publication"]
    outcomes = {"incomplete", "complete", "superseded_or_closed", "publication_stopped",
                "source_error", "theme_error"}
    seen, summaries = set(), []
    for work in trace["requests"]:
        if trace["schema"] >= 3:
            counter(work["elapsed_ns"], "request elapsed duration")
        generation = work["generation"]
        if generation in seen or generation not in starts:
            raise ValueError("Request summary has no unique worker start")
        seen.add(generation)
        if (work["epoch"] != starts[generation]["b"]
                or work["finished_ns"] < starts[generation]["ns"]
                or work["outcome"] not in outcomes):
            raise ValueError("Request identity or outcome is inconsistent")
        total = 0
        for phase in phases:
            timing = work[phase]
            if trace["schema"] >= 3:
                validate_timing(timing, phase)
            if (min(timing.values()) < 0 or timing["max_ns"] > timing["total_ns"]
                    or (timing["calls"] == 0 and timing["total_ns"] != 0)):
                raise ValueError(f"Invalid {phase} timing")
            total += timing["total_ns"]
        if total > work["elapsed_ns"]:
            raise ValueError("Disjoint phase durations exceed request duration")
        if work["processed_records"] != sum(work[field] for field in [
                "requested_records", "predecessor_records", "other_replay_records"]):
            raise ValueError("Processed-source accounting differs")
        if (work["fallback_requested"] > work["requested_records"]
                or work["style_replies"] > work["requested_records"]):
            raise ValueError("Requested-style accounting differs")
        if work["requested_unique"] is not None and (
                work["requested_records"] > work["requested_unique"]
                or work["requested_unique"] > work["requested_raw"]):
            raise ValueError("Requested-source accounting differs")
        if work["outcome"] == "complete" and (
                work["requested_records"] != work["requested_unique"]
                or work["style_replies"] != work["requested_unique"]
                or work["style_replies"] != published[generation]):
            raise ValueError("Completed request has missing source or replies")
        longest = work["longest_parser_call"]
        maximum = max(work["parse_requested"]["max_ns"], work["parse_replay"]["max_ns"])
        if (longest is None and maximum != 0) or (longest is not None and longest["ns"] != maximum):
            raise ValueError("Longest parser call differs from aggregate")
        if trace["schema"] == 4:
            if longest is not None and longest["side"] not in ("old", "new", "both"):
                raise ValueError("Longest parser call has an invalid side")
            shared = counter(work["shared_parser_calls"], "shared parser calls")
            calls = work["parse_requested"]["calls"] + work["parse_replay"]["calls"]
            if (shared > min(calls, work["processed_records"])
                    or calls + shared > 2 * work["processed_records"]
                    or (longest is not None and longest["side"] == "both" and shared == 0)
                    or (longest is not None and longest["side"] != "both" and shared == calls)):
                raise ValueError("Shared parser accounting differs")
        prefix = work["largest_prefix"]
        if prefix is not None and (prefix["start"] > prefix["first_requested"]
                or prefix["records"] != prefix["first_requested"] - prefix["start"]
                or prefix["first_requested"] > prefix["last_requested"]):
            raise ValueError("Replay-prefix accounting differs")
        summary = {**work, "phase_total_ms": total / 1e6,
                   "unattributed_ms": (work["elapsed_ns"] - total) / 1e6,
                   "backpressure_full_events": full[generation]}
        if trace["schema"] >= 3:
            summary["initialization_attribution"] = initialization_work(work, total, trace["schema"])
        summaries.append(summary)
    cache_views = [{"ns": event["ns"], "cached_occurrences": event["a"],
                    "missing_occurrences": event["b"]}
                   for event in trace["events"] if event["kind"] == "viewport_cache_occurrences"]
    unfinished = sorted(starts.keys() - seen)
    incomplete = sorted(work["generation"] for work in summaries if work["outcome"] == "incomplete")
    startup = [{"finished_ns": event["ns"], "elapsed_ms": event["a"] / 1e6}
               for event in trace["events"] if event["kind"] == "syntax_set_startup"]
    result = {"observation_status": "partial" if unfinished or incomplete else "complete",
            "summaries": summaries, "started_without_summary": unfinished,
            "incomplete_summaries": incomplete, "syntax_set_startup_deserialization": startup,
            "submitted_not_started_before_trace_end": sorted(submitted - starts.keys()),
            "viewport_cache_occurrences": cache_views,
            "scope": "Phase durations are disjoint instrumented intervals. Syntax-set startup precedes requests; lazy grammar/regex initialization can still occur inside parser calls. Publication includes backpressure. The remainder includes source reads, sanitizing, bookkeeping, and instrumentation, without separate attribution. Largest prefix is planned distance, not necessarily completed work. Replay cause is the first matching file/hunk/epoch/backward incompatibility, or missing/resume. Source occurrences are not distinct IDs: split context and wrapped rows can repeat them. Missing worker summaries at shutdown are incomplete observations, not zero work or completed cancellation. Observation completeness does not imply completed styling or performance acceptance."}
    if trace["schema"] == 4:
        result["shared_parser_scope"] = (
            "Parser counts are actual calls. A both-side call processes one identical leading "
            "source history for old and new. Shared calls are a subset, not additional work. "
            "No shared-state reconvergence occurs after a side-only record.")
    if trace["schema"] >= 3:
        result["initialization_scope"] = (
            "Request initialization contains the disjoint requested/replay parser subsets; "
            "these subsets are already inside the original parser phases, not additive work. "
            "Only outside-parser initialization is added to the original disjoint phase total. "
            "Outside-parser maxima are unavailable: cumulative maxima cannot be subtracted. "
            "Parser remainder maxima are recorded per call, not differences of maxima. "
            "Parser remainders include dispatch, matching, allocation, and bookkeeping, "
            "not pure matching. Source-pattern counts do not identify initialization durations "
            "attributable to source-derived patterns. Original phase counters lack an overflow "
            "flag; saturation-boundary values are rejected, not interpreted as durations.")
    return result


def analyze(path):
    trace = json.loads(path.read_text())
    schema = trace.get("schema", 1)
    if schema not in [1, 2, 3, 4, 6]:
        raise ValueError("Unsupported delivery trace schema")
    if schema == 6:
        for work in trace.get("requests", []):
            counter(work["shared_source_lines"], "shared source lines")
    if any(work.get("complete_source_lines", 0) or work.get("plain_records", 0)
           or work.get("shared_source_lines", 0)
           for work in trace.get("requests", [])) or any(
               isinstance(event.get("b"), dict) for event in trace["events"]):
        raise ValueError("This analyzer supports patch-only traces; full-source replay needs source-aware accounting")
    if trace["dropped"]:
        raise ValueError("Incomplete event trace")
    if schema >= 2 and trace["requests_dropped"]:
        raise ValueError("Incomplete request summary trace")
    ready, offered, queued = {}, {}, {}
    blocked, delivery, emission = [], [], []
    frames = [event["ns"] for event in trace["events"] if event["kind"] == "frame"]
    for event in trace["events"]:
        key = (event["a"], event["b"])
        if event["kind"] == "ready":
            ready[key] = event["ns"]
        if event["kind"] == "offer":
            offered[event["a"]] = (key, event["ns"])
        if event["kind"] == "published":
            key, ns = offered[event["a"]]
            queued[key] = ns
            blocked.append((ns - ready[key]) / 1e6)
    for event in trace["events"]:
        if event["kind"] == "drain":
            delivery.append((event["ns"] - queued[(event["a"], event["b"])] ) / 1e6)
            index = bisect_left(frames, event["ns"])
            if index < len(frames):
                emission.append((frames[index] - event["ns"]) / 1e6)
    result = {"path": str(path), "sha256": digest(path), "frames": len(frames),
            "ready_to_successful_offer": distribution(blocked),
            "successful_offer_to_drain": distribution(delivery),
            "drain_to_next_frame": distribution(emission),
            "scope": "Offer precedes try_send: delivery is an instrumented upper bound. Next frame may not display offscreen or stale replies."}
    if schema >= 2:
        result["request_work"] = request_work(trace)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("traces", type=Path, nargs="+")
    args = parser.parse_args()
    with args.output.open("x") as output:
        json.dump({"analyzer_sha256": digest(Path(__file__)),
                   "traces": [analyze(path) for path in args.traces]}, output, indent=2)


if __name__ == "__main__":
    main()
