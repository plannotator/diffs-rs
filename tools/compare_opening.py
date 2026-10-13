"""Paired first-viewport diagnostic using exact retained terminal expectations."""

import argparse
import gzip
import json
from pathlib import Path

from corpus_acceptance import LIMITS, MeasuredSession, await_state
from corpus_reference import ROOT, digest


def opening(binary, trace):
    session = MeasuredSession(binary.resolve(), {"path": str(ROOT / trace["fixture"]["local_path"])}, LIMITS)
    try:
        result = await_state(session, None, trace["opening"]["state"], session.started_ns,
                             expected=trace["opening"]["expected"])
        result.pop("screen")
        result.pop("records")
        result["quit"] = session.quit()
        if result["quit"]["exit_code"] != 0 or not result["quit"]["termios_restored"]:
            raise AssertionError("Terminal cleanup failed")
        return result
    finally:
        session.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("trace", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    trace = json.loads(gzip.decompress(args.trace.read_bytes()))
    paths = [args.reference, args.candidate, args.trace, Path(__file__), ROOT / trace["fixture"]["local_path"]]
    paths += [ROOT / "tools" / name for name in ["corpus_acceptance.py", "corpus_reference.py", "terminal_baseline.py"]]
    identities = {str(p.resolve()): digest(p) for p in paths}
    if identities[str((ROOT / trace["fixture"]["local_path"]).resolve())] != trace["fixture"]["sha256"]:
        raise ValueError("Fixture changed")
    results = []
    report = {"scope": "Paired opening diagnostic, not the five-category acceptance run",
              "identities": identities, "results": results, "status": "running"}
    with args.output.open("x") as output:
        try:
            for pair in range(10):
                order = [("reference", args.reference), ("candidate", args.candidate)]
                if pair % 2:
                    order.reverse()
                for name, binary in order:
                    report["active"] = {"pair": pair, "binary": name}
                    results.append({**opening(binary, trace), **report["active"]})
            for path, expected in identities.items():
                if digest(Path(path)) != expected:
                    raise ValueError(f"Diagnostic input changed: {path}")
            report["status"] = "complete"
        except Exception as error:
            report.update(status="failed", error=str(error))
            raise
        finally:
            json.dump(report, output, indent=2)


if __name__ == "__main__":
    main()
