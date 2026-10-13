"""Check real source-transition censuses and reject corrupted accounting records."""

import argparse
from copy import deepcopy
import json
from pathlib import Path

from owner_census import check_style_storage
from regex_census import write_json


def check(cache):
    check_style_storage(cache)
    assert cache["payload_counter_bytes"] == cache["recomputed_payload_bytes"] == cache["fifo_accounted_bytes"]
    assert cache["entries"] == cache["fifo_length"] <= cache["fifo_capacity"]
    assert cache["entries"] <= cache["map_capacity"]


def rejected(cache, path, value):
    record = deepcopy(cache)
    target = record
    for key in path[:-1]:
        target = target[key]
    if value is None:
        del target[path[-1]]
    else:
        target[path[-1]] = value
    try:
        check(record)
    except (AssertionError, KeyError):
        return
    raise AssertionError(f"Accepted corrupted accounting: {path} = {value}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("transitions", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--historical", type=Path, nargs="+", default=[])
    args = parser.parse_args()
    if not __debug__:
        raise RuntimeError("Accounting checks require assertions; do not use -O or PYTHONOPTIMIZE")
    records = json.loads(args.transitions.read_text())
    labels = ["empty", "plain", "checked", "replacement-invalidated", "replacement-colored",
              "revoked", "empty-revocation", "recovered", "reset", "new-instance"]
    assert [record["label"] for record in records] == labels
    for record in records:
        check(record["cache"])
        assert record["cache"]["activity"]["schema"] == 3
    cache = records[5]["cache"]
    counts = cache["activity"]["observations"]
    assert counts["invalidations"] > 0 and counts["invalidated_entries"] > 0
    corruptions = 0
    for field in ("admitted_payload_bytes", "evicted_payload_bytes", "invalidated_entries",
                  "invalidated_payload_bytes", "invalidation_capacity_gained", "invalidation_capacity_lost"):
        rejected(cache, ["activity", "observations", field], counts[field] + 1)
        corruptions += 1
    for field in ("admitted_payload_bytes", "evicted_payload_bytes", "invalidations",
                  "invalidated_entries", "invalidated_payload_bytes",
                  "invalidation_capacity_gained", "invalidation_capacity_lost"):
        for value in (None, -1, float(counts[field]), bool(counts[field])):
            rejected(cache, ["activity", "observations", field], value)
            corruptions += 1
    for schema in (0, 1, 2, 4, True, 3.0):
        rejected(cache, ["activity", "schema"], schema)
        corruptions += 1
    rejected(cache, ["activity", "observations", "invalidations"], 0)
    corruptions += 1
    historical = []
    for path in args.historical:
        old = json.loads(path.read_text())["cache"]
        check(old)
        schema = old.get("activity", {}).get("schema")
        assert schema in (None, 1, 2)
        historical.append({"path": str(path.resolve()), "activity_schema": schema,
                           "invalidation_qualified": False})
    result = {"status": "pass", "source_transitions": labels,
              "corruptions_rejected": corruptions, "historical": historical}
    write_json(args.output, result)
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
