"""Disjoint retained-index payload counts for the pinned five-patch field."""

import argparse
import json
from pathlib import Path
import subprocess

from corpus_reference import ROOT, digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    results = []
    with args.output.open("x") as output:
        for fixture in json.loads((ROOT / "fixtures/diffshub/corpus.json").read_text())["fixtures"]:
            path = ROOT / fixture["local_path"]
            if digest(path) != fixture["sha256"]:
                raise ValueError(f"Fixture hash changed: {path}")
            result = subprocess.run([str(args.binary.resolve()), "inspect", str(path), "--summary"],
                                    capture_output=True, text=True, check=True, timeout=60)
            summary = json.loads(result.stdout)
            storage = summary["storage"]
            components = ["source_run_bytes", "shared_block_bytes", "side_index_bytes",
                          "marker_index_bytes", "boundary_index_bytes"] if "shared_block_bytes" in storage else [
                              "source_run_bytes", "row_index_bytes", "location_index_bytes"]
            if storage["projection_bytes"] != sum(storage[key] for key in components):
                raise AssertionError("Projection accounting components overlap or are missing")
            results.append({"fixture": fixture["id"], "fixture_sha256": fixture["sha256"], **summary})
            print(f"{fixture['id']}: {storage}", flush=True)
        json.dump({"binary_sha256": digest(args.binary), "results": results,
                   "scope": "Payload/capacity accounting, not total heap or RSS. Excludes allocator rounding, Arc headers, spool/cache containers, library allocations and stacks."}, output, indent=2)
        output.write("\n")


if __name__ == "__main__":
    main()
