"""Count retained metadata extents for the five pinned patches, without rendering."""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tarfile

from preserve_release import ROOT, command_output, digest, sources


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    manifest = ROOT / "fixtures/diffshub/corpus.json"

    def inputs():
        return sources() | {
            name: digest(ROOT / name) for name in (
                "examples/metadata_accounting.rs", "tools/metadata_accounting.py",
                "tools/preserve_release.py", "fixtures/diffshub/corpus.json")}

    build_inputs = inputs()
    command = ["cargo", "build", "--release", "--locked", "--offline",
               "--example", "metadata_accounting"]
    report = {"status": "running", "source_sha256": build_inputs,
              "rustc": command_output(["rustc", "-Vv"]),
              "cargo": command_output(["cargo", "-V"]),
              "build_command": command, "results": [],
              "scope": "Retained extents, not process-memory or performance measurements."}
    try:
        with tarfile.open(output / "source.tar.gz", "x:gz") as archive:
            for name in build_inputs:
                archive.add(ROOT / name, arcname=name, recursive=False)
        with (output / "build.log").open("x") as log:
            subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
        binary = output / "metadata_accounting"
        with (ROOT / "target/release/examples/metadata_accounting").open("rb") as src, binary.open("xb") as dst:
            shutil.copyfileobj(src, dst)
        binary.chmod(0o555)
        report["binary_sha256"] = digest(binary)
        for fixture in json.loads(manifest.read_text())["fixtures"]:
            path = ROOT / fixture["local_path"]
            if digest(path) != fixture["sha256"]:
                raise ValueError(f"Fixture changed: {path}")
            command = [str(binary), str(path)]
            with (output / f"{fixture['id']}.json").open("x") as stdout, \
                    (output / f"{fixture['id']}.stderr").open("x") as stderr:
                subprocess.run(command, cwd=ROOT, stdout=stdout, stderr=stderr,
                               check=True, timeout=60)
            counts = json.loads((output / f"{fixture['id']}.json").read_text())
            expected = fixture["expected_segment_digests"]["source_records"]
            if (counts["source_lines"] != expected
                    or counts["files"] != fixture["git_numstat"]["file_records"]
                    or counts["hunks"]["len"] != fixture["physical_shape"]["hunk_headers"]
                    or counts["spool_bytes"] != fixture["bytes"]
                    or counts["cached_source_bytes"] != 0
                    or not counts["reconciled"]):
                raise ValueError(f"Counting scope disagrees with corpus: {fixture['id']}")
            report["results"].append({"fixture": fixture["id"], "command": command,
                                      "fixture_sha256": fixture["sha256"],
                                      "result_sha256": digest(output / f"{fixture['id']}.json")})
            print(f"{fixture['id']}: {counts['metadata_components']}", flush=True)
        if inputs() != build_inputs or command_output(["rustc", "-Vv"]) != report["rustc"]:
            raise ValueError("Inputs or compiler changed during accounting")
        report["status"] = "complete"
    except (Exception, KeyboardInterrupt) as error:
        report.update(status="failed", error=str(error))
        raise
    finally:
        with (output / "identity.json").open("x") as stream:
            json.dump(report, stream, indent=2)
            stream.write("\n")


if __name__ == "__main__":
    main()
