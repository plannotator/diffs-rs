"""Challenge migration checks with mutations of an actual retained capture."""

import argparse
from copy import deepcopy
import gzip
import json
from pathlib import Path

from corpus_measure import compare_baseline


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("trace", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--candidate", type=Path)
    args = parser.parse_args()
    trace = json.loads(gzip.decompress(args.trace.read_bytes()))
    baseline = compare_baseline(trace, args.trace)
    candidate = deepcopy(trace)
    candidate["binary_sha256"] = "new candidate identity"
    candidate["quit"]["exit_observation_ms"] = 0
    for step in candidate["steps"]:
        if "setup_ms" in step["action"]:
            step["action"]["setup_ms"] += 1000
    assert compare_baseline(candidate, args.trace) == baseline
    if args.candidate:
        assert compare_baseline(json.loads(gzip.decompress(args.candidate.read_bytes())), args.trace) == baseline
    anchor_step = next(n for n, step in enumerate(trace["steps"]) if "anchor" in step["state"])
    cases = {
        "content": lambda t: t["opening"]["expected"]["rows"].__setitem__(1, "wrong source"),
        "style": lambda t: t["opening"]["expected"]["styles"][1][0].__setitem__(2, "ff00ff"),
        "input": lambda t: t["steps"][0].__setitem__("action", {"keys": "71"}),
        "anchor": lambda t: t["steps"][anchor_step]["state"].__setitem__("anchor", -1),
        "category": lambda t: t["steps"][0].__setitem__("category", "omitted"),
        "missing-step": lambda t: t["steps"].pop(),
        "unknown-field": lambda t: t.__setitem__("unreviewed", True),
    }
    rejected = []
    for name, mutate in cases.items():
        candidate = deepcopy(trace)
        mutate(candidate)
        try:
            compare_baseline(candidate, args.trace)
        except ValueError:
            rejected.append(name)
        else:
            raise AssertionError(f"Migration accepted altered {name}")
    with args.output.open("x") as output:
        json.dump({"baseline": baseline, "status": "pass", "rejected": rejected,
                   "provenance_changes_accepted": True}, output, indent=2)


if __name__ == "__main__":
    main()
