"""Real-PTY syntax replay diagnostics; no historical trace is changed."""

import argparse
import hashlib
import json
from pathlib import Path
import platform

from reading_diagnostic import fixtures, observe
from terminal_baseline import Session


def grid(screen, first, active, light=False, initial=False, styled=False):
    display = screen.display
    keyword = "d32a61" if light else "ff678d"
    start = 3 if initial else 2
    if "deep.ts" not in display[1] or "SPLIT" not in display[0]:
        return False
    for y in range(start, 49):
        n = first + y - start
        final = n == 12001
        text = "const finished = true;" if final else f"const value_{n:05d} = true;"
        for old, x, width in [(True, 0, 79), (False, 80, 80)]:
            cursor = "▎" if not old and n == active else " "
            expected = (" " * width if final and old else
                        (f"{cursor}{n:5d}{'+' if final else ' '} " + text).ljust(width))
            if display[y][x:x + width] != expected:
                return False
            if styled and not (old and final):
                if any(screen.buffer[y][x + 8 + offset].fg != keyword for offset in range(5)):
                    return False
    return True


def trial(binary, path):
    session = Session(binary, {"path": str(path)}, {
        "sampled_rss_stop_bytes": 6 * 1024**3, "cadence_ms": 50,
    })
    result = {"samples": []}
    try:
        result["samples"].append(observe(session, session.started_ns,
            lambda s: grid(s, 1, None, initial=True),
            lambda s: grid(s, 1, None, initial=True, styled=True), "open"))
        for keys, first, active, light, label in [
            (b"G", 11955, 12001, False, "cold-end"),
            (b"\x1b[5~\x1b[5~", 11909, 11909, False, "reverse"),
            (b"t", 11909, 11909, True, "theme-change"),
            (b"\x1b[6~\x1b[6~", 11955, 12001, True, "uncached-forward"),
        ]:
            started, _ = session.inject({"input_hex": keys.hex()})
            result["samples"].append(observe(session, started,
                lambda s: grid(s, first, active, light),
                lambda s: grid(s, first, active, light, styled=True), label))
        result["quit"] = session.quit()
        if (result["quit"]["exit_code"] != 0 or not result["quit"]["termios_restored"]
                or not all(result["quit"]["cleanup_sequences_received"].values())):
            raise RuntimeError(f"Terminal cleanup failed: {result['quit']}")
        result["sampled_peak_rss_bytes"] = session.sample_peak
        result["wait4_peak_rss_bytes"] = session.rusage.ru_maxrss
        result["pty_sha256"] = session.raw_hash.hexdigest()
        return result
    finally:
        session.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("fixture_dir", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--trials", type=int, choices=range(1, 6), default=3)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("The output path already exists")
    if platform.system() != "Darwin":
        parser.error("This diagnostic reports macOS wait4 RSS units")
    paths, _ = fixtures(args.fixture_dir)
    path = paths["deep"]
    result = {
        "schema": 1, "diagnostic_only": True, "host": platform.platform(),
        "binary": str(args.binary.resolve()),
        "binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
        "fixture_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "helpers": {name: hashlib.sha256(Path(__file__).with_name(name).read_bytes()).hexdigest()
                    for name in ["reading_diagnostic.py", "terminal_baseline.py", "trace_reference.py"]},
        "notes": ["Actual PTY receipt, not Ghostty display latency.",
                  "Fresh process, warm files, one pending measured command.",
                  "Theme change clears styles before forward exposure of lines 11956–12001.",
                  "Process peak RSS, not three-tour sustained RSS or a total-memory guarantee."],
        "trials": [],
    }
    try:
        for _ in range(args.trials):
            result["trials"].append(trial(args.binary.resolve(), path))
    except BaseException as error:
        result["error"] = str(error)
        raise
    finally:
        args.output.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
