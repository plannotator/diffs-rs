"""Real-PTY challenge checks with independent source decoding and retained cell comparisons."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

from corpus_acceptance import LIMITS, MeasuredSession, await_state
from corpus_reference import Corpus, ROOT, digest


class Challenge(Corpus):
    def __init__(self, case):
        self.path = ROOT / case["path"]
        self.ref_path = ROOT / case["reference"]
        self.ref = json.loads(self.ref_path.read_text())
        self.count = len(self.ref["files"])
        if digest(self.path) != case["sha256"] or digest(self.ref_path) != case["reference_sha256"]:
            raise ValueError("Challenge identity changed")


def run(binary, case, expected=None):
    corpus = Challenge(case)
    env = dict(os.environ)
    env.pop("NATIVE_DIFF_DELIVERY_TRACE", None)
    canonical = subprocess.run([binary, "inspect", corpus.path, "--canonical"],
                               capture_output=True, check=True, env=env).stdout
    if hashlib.sha256(canonical).hexdigest() != corpus.ref["source_sha256"]:
        raise AssertionError("Independent full-source digest differs")
    endpoint_files = {0, corpus.ref["first"]["file"], corpus.ref["last"]["file"]}
    if any("\u200d" in line.text for index in endpoint_files for line in corpus.file(index).lines):
        return {"id": case["id"], "source": "pass", "status": "unsupported_oracle",
                "reason": "pyte does not preserve joined emoji. No visual pass or timing from this probe; use Rust position contracts."}
    session = MeasuredSession(binary, {"path": str(corpus.path)}, LIMITS)
    result = {"id": case["id"], "steps": [], "source": "pass"}
    state = {"expected_file": 0, "split": True, "wrapped": False, "horizontal": 0}
    actions = [("opening", None), ("end", {"input_hex": "47"}),
               ("start", {"input_hex": "67"}), ("light", {"input_hex": "74"}),
               ("narrow", {"resize": [100, 50]}), ("wide", {"resize": [160, 50]}),
               ("dark", {"input_hex": "74"})]
    try:
        for index, (label, action) in enumerate(actions):
            if label in ["end", "start"]:
                endpoint = corpus.ref["last" if label == "end" else "first"]
                state.update(expected_file=endpoint["file"], anchor=endpoint["source_ordinal"],
                             require_anchor=True, anchor_old=endpoint["new"] is None)
            started = session.started_ns if action is None else session.inject(action)[0]
            wanted = expected["steps"][index]["screen"] if expected else None
            observed = await_state(session, corpus, state, started, expected=wanted)
            # Frozen comparison does not replace the independent content check.
            from corpus_reference import check_screen
            check_screen(session.frames.screen, corpus, **state)
            result["steps"].append({"label": label, **observed})
        result["quit"] = session.quit()
        if result["quit"]["exit_code"] != 0 or not result["quit"]["termios_restored"]:
            raise AssertionError("Terminal cleanup failed")
        result["cpu_seconds"] = session.rusage.ru_utime + session.rusage.ru_stime
        result["peak_rss_bytes"] = session.rusage.ru_maxrss
        return result
    finally:
        session.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--trace", action="store_true")
    args = parser.parse_args()
    manifest = ROOT / "evidence/delivery-v1/cases/manifest.json"
    cases = json.loads(manifest.read_text())["cases"]
    expected = json.loads(args.expected.read_text())["cases"] if args.expected else None
    report = {"binary_sha256": digest(args.binary), "manifest_sha256": digest(manifest),
              "scope": "Development challenge correctness and observations, not corpus acceptance",
              "instrumented": args.trace, "cases": [], "status": "running"}
    args.output.mkdir()
    try:
        for index, case in enumerate(cases):
            print(case["id"], flush=True)
            if args.trace:
                os.environ["NATIVE_DIFF_DELIVERY_TRACE"] = str((args.output / f"{case['id']}.json").resolve())
            report["cases"].append(run(str(args.binary.resolve()), case, expected[index] if expected else None))
        report["status"] = "complete"
    except Exception as error:
        report.update(status="failed", error=str(error))
        raise
    finally:
        (args.output / "report.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
