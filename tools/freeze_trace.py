"""Resolve content-latency inputs and grids before running any measured trial."""

import argparse
import copy
import gzip
import hashlib
import importlib.metadata
import json
from pathlib import Path
import sys

from trace_reference import Document, View, digest, put

ROOT = Path(__file__).resolve().parents[1]
BASELINE = "evidence/large-diff-baseline/native-diff-before"
BASELINE_HASH = "52646a55a3635d2df506ab6e3e458b4fca2386f861ead2c0e62c57739308ed78"
TOOLS = ["tools/trace_reference.py", "tools/patch_reference.py", "tools/freeze_trace.py",
         "tools/terminal_baseline.py", "tools/check_measurement.py", "tools/measurement-requirements.txt",
         "tools/rss_sampling.py", "/bin/ps"]


def chrome(grid, label, count):
    width = grid["width"]
    title = [" "] * width
    right = f"{count} files  ·  {'SPLIT' if grid['split'] else 'UNIFIED'}  ·  dark "
    put(title, 0, f" diffs  /  {label}", width - len(right))
    put(title, width - len(right), right, len(right))
    hint = " u layout  w wrap  t theme  ? help  q quit "
    footers = []
    for status in ["Highlighting visible code…", "Plain styling fallback",
                   "Patch view · syntax context may be incomplete"]:
        footer = [" "] * width
        put(footer, 1, status, width - len(hint) - 1)
        put(footer, width - len(hint), hint, len(hint))
        footers.append("".join(footer))
    return dict(grid, title="".join(title), allowed_footers=footers)


def resolve(fixture, ref):
    doc = Document(ROOT / fixture["local_path"], ref)
    view = View(doc)
    grids = {}

    def grid(v):
        result = chrome(v.grid(), fixture["local_path"], doc.count)
        key = hashlib.sha256(json.dumps(result, sort_keys=True).encode()).hexdigest()
        grids[key] = result
        return key

    opening = grid(view)
    first = doc.source_pos(ref["first"]["source_ordinal"], True)
    y = next(y for p, y in view.visible() if p == first)
    view.cursor = first
    view.old = ref["first"]["new"] is None
    x = 10 if view.old else 90
    steps = [{"category": "setup", "input_hex": f"\x1b[<0;{x};{y+1}M".encode().hex(),
              "expected": grid(view)}]
    for category, amount, button in [("forward", 3, 65), ("reverse", -3, 64)]:
        for i in range(40):
            view.scroll(amount)
            steps.append({"category": category, "sample": i, "input_hex": f"\x1b[<{button};90;10M".encode().hex(),
                          "expected": grid(view)})
    prototype = copy.copy(view)
    for i in range(40):
        end = i % 2 == 0
        view.jump(end)
        prototype.jump(end, prototype=True)
        expected, wrong = grid(view), grid(prototype)
        steps.append({"category": "distant_seek", "sample": i, "input_hex": (b"G" if end else b"g").hex(),
                      "expected": expected, "known_nonconforming": wrong if wrong != expected else None})

    hunk = ref["largest_hunk"]
    ordinal = hunk["source_start"] + hunk["source_records"] // 2
    anchored = View(doc)
    anchored.locate(ordinal)
    f, r = anchored.cursor
    # File movement lands at the header. Counted row movement then reveals the target.
    first_row = first[1]
    commands = [b"g"]
    if f > first[0]:
        commands.append(f"{f-first[0]}}}".encode())
        first_row = 0
    if r > first_row:
        commands.append(f"{r-first_row}j".encode())
    elif r < first_row:
        commands.append(f"{first_row-r}k".encode())
    # A public click chooses the side without moving the viewport.
    y = next(y for p, y in anchored.visible() if p == anchored.cursor)
    commands.append(f"\x1b[<0;{10 if anchored.old else 90};{y+1}M".encode())
    setup = {"category": "structural_setup", "inputs_hex": [c.hex() for c in commands],
             "expected": grid(anchored), "baseline_unsupported": "Counted navigation is absent from the original executable."}
    steps.append(setup)
    for i in range(40):
        anchored.reflow(not anchored.split, 160, 50)
        steps.append({"category": "layout", "sample": i, "input_hex": b"u".hex(), "expected": grid(anchored)})
    for i in range(40):
        width, height = (100, 30) if i % 2 == 0 else (160, 50)
        anchored.reflow(True, width, height)
        steps.append({"category": "resize", "sample": i, "resize": [width, height], "expected": grid(anchored)})
    anchors = {key: ref[key] for key in ["first", "last", "largest_hunk", "largest_file", "longest_line", "file_quantiles"]}
    anchors["layout_resize_source_ordinal"] = ordinal
    return {"id": fixture["id"], "path": fixture["local_path"], "sha256": fixture["sha256"],
            "source_sha256": ref["source_sha256"], "anchors": anchors, "opening": opening,
            "steps": steps, "grids": grids}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit("Refusing to overwrite an existing trace directory")
    if digest(ROOT / BASELINE) != BASELINE_HASH:
        raise SystemExit("Original executable hash mismatch")
    for pin in (ROOT / "tools/measurement-requirements.txt").read_text().splitlines():
        name, version = pin.split("==")
        if importlib.metadata.version(name) != version:
            raise SystemExit(f"Pinned measurement dependency mismatch: {name}")
    corpus = json.loads((ROOT / "fixtures/diffshub/corpus.json").read_text())
    hashes = {p: digest(ROOT / p) for p in TOOLS + [BASELINE, "fixtures/diffshub/corpus.json",
                                                   "fixtures/diffshub/scenario.json", "fixtures/diffshub/reference.json",
                                                   "MEASUREMENT.md", "SPEC.md", "LARGE_DIFFS.md"]}
    hashes.update({str(p.relative_to(ROOT)): digest(p) for p in sorted((ROOT / "src").glob("*.rs"))})
    fixtures = []
    for fixture in corpus["fixtures"]:
        ref_path = f"evidence/large-diff-baseline/{fixture['id'].split('-')[0]}.reference.json"
        ref = json.loads((ROOT / ref_path).read_text())
        if digest(ROOT / fixture["local_path"]) != fixture["sha256"] or ref["sha256"] != fixture["sha256"]:
            raise SystemExit(f"Fixture identity mismatch: {fixture['id']}")
        if ref["source_sha256"] != fixture["expected_segment_digests"]["source_sha256"]:
            raise SystemExit("Independent source digest mismatch")
        hashes[ref_path] = digest(ROOT / ref_path)
        hashes[fixture["local_path"]] = fixture["sha256"]
        fixtures.append(resolve(fixture, ref))
        print(f"Resolved {fixture['id']}", flush=True)
    trace = {"schema": 1, "scope": "Content-grid latency baseline; not full large-diff acceptance",
             "baseline": BASELINE, "baseline_sha256": BASELINE_HASH, "trials": 5,
             "cadence_ms": 50, "watchdog_ms": 30000, "sampled_rss_stop_bytes": 6 * 1024**3,
             "unmeasured": ["Sequential style oracle and style latency", "Three-tour steady RSS",
                            "Long-line wrap/horizontal tour", "Actual terminal pixels and terminal-process RSS"],
             "fixtures": fixtures}
    args.output.mkdir(parents=True)
    payload = gzip.compress(json.dumps(trace, ensure_ascii=False, separators=(",", ":")).encode(), mtime=0)
    (args.output / "trace.json.gz").write_bytes(payload)
    hashes[str((args.output / "trace.json.gz").resolve().relative_to(ROOT))] = hashlib.sha256(payload).hexdigest()
    runtime = {"python_sha256": digest(sys.executable), "python_version": sys.version,
               "dependencies": {name: importlib.metadata.version(name) for name in ["pyte", "wcwidth", "regex"]}}
    (args.output / "freeze.json").write_text(json.dumps({"schema": 1, "sha256": hashes,
                                                        "trace_sha256": hashlib.sha256(payload).hexdigest(),
                                                        "runtime": runtime}, indent=2) + "\n")


if __name__ == "__main__":
    main()
