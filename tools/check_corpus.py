"""Compare the real executable's complete source stream with pinned expectations."""

import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def exported_digest(executable, patch):
    with tempfile.TemporaryFile() as errors:
        child = subprocess.Popen(
            [str(executable), "inspect", str(patch), "--canonical"],
            stdout=subprocess.PIPE, stderr=errors, start_new_session=True,
        )
        reader = ThreadPoolExecutor(max_workers=1)
        try:
            future = reader.submit(hashlib.file_digest, child.stdout, "sha256")
            try:
                code = child.wait(timeout=30)
            except subprocess.TimeoutExpired as error:
                raise RuntimeError("Source export exceeded the 30-second watchdog") from error
            actual = future.result(timeout=1).hexdigest()
            errors.seek(0)
            if code != 0:
                raise RuntimeError(errors.read().decode("utf-8", errors="backslashreplace"))
            return actual
        finally:
            if child.poll() is None:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
            reader.shutdown(wait=True)
            child.stdout.close()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise FileExistsError(args.output)
    manifest = json.loads(Path("fixtures/diffshub/corpus.json").read_text())
    reference_path = Path("fixtures/diffshub/reference.json")
    references = {row["id"]: row for row in json.loads(reference_path.read_text())["fixtures"]}
    executable = args.executable.resolve()
    results = []
    for fixture in manifest["fixtures"]:
        expected = references[fixture["id"]]
        patch = Path(fixture["local_path"])
        if digest(patch) != fixture["sha256"] or fixture["sha256"] != expected["sha256"]:
            raise ValueError(f"Fixture identity changed: {fixture['id']}")
        actual = exported_digest(executable, patch)
        result = {
            "fixture": fixture["id"], "fixture_sha256": fixture["sha256"],
            "source_records": expected["source_records"], "source_sha256": actual,
            "pass": actual == expected["source_sha256"],
        }
        results.append(result)
        print(json.dumps(result), flush=True)
    report = {
        "schema_version": 1,
        "scope": "Complete supplied source, signs, side line numbers, file order, hunk starts, and newline markers. Not terminal-grid, normalized metadata, styling, or timing acceptance.",
        "executable_sha256": digest(executable),
        "reference_sha256": digest(reference_path),
        "checker_sha256": digest(Path(__file__)),
        "results": results,
    }
    with args.output.open("x") as output:
        json.dump(report, output, indent=2)
        output.write("\n")
    if not all(row["pass"] for row in results):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
