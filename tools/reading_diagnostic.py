"""Bounded local diagnostics, separate from the frozen measurement-v1 trace."""

import argparse
import hashlib
import json
from pathlib import Path
import platform
import time

from terminal_baseline import Session


def fixtures(directory):
    directory.mkdir(parents=True, exist_ok=True)
    old = "".join(f"OLD{i:06d} " for i in range(36000))
    new = "".join(f"NEW{i:06d} " for i in range(36000))
    bodies = {
        "long": ("diff --git a/long.ts b/long.ts\n--- a/long.ts\n+++ b/long.ts\n"
                 "@@ -1 +1 @@\n-" + old + "\n+" + new + "\n"),
        "deep": ("diff --git a/deep.ts b/deep.ts\n--- a/deep.ts\n+++ b/deep.ts\n"
                 "@@ -1,12000 +1,12001 @@\n"
                 + "".join(f" const value_{i:05d} = true;\n" for i in range(1, 12001))
                 + "+const finished = true;\n"),
    }
    paths = {}
    for name, body in bodies.items():
        path = directory / f"{name}.patch"
        data = body.encode()
        if path.exists():
            if path.read_bytes() != data:
                raise RuntimeError(f"Fixture already exists with different bytes: {path}")
        else:
            path.write_bytes(data)
        paths[name] = path
    return paths, (old, new)


def long_grid(screen, lines, wrapped):
    width, height = screen.columns, screen.lines
    display = screen.display
    left = (width - 1) // 2
    for x, pane, text, sign in [(0, left, lines[0], "−"),
                                (left + 1, width - left - 1, lines[1], "+")]:
        available = pane - 6
        for y in range(3, height - 1):
            part = y - 3
            if not wrapped and part:
                expected = " " * pane
            else:
                offset = part * available if wrapped else 0
                gutter = f"   1{sign} " if not part else "    ↪ "
                expected = gutter + text[offset:offset + available].ljust(available)
            if display[y][x:x + pane] != expected:
                return False
    return True


def deep_grid(screen, end, styled, initial=False):
    display = screen.display
    left = (screen.columns - 1) // 2
    start_y, first = (2, 11955) if end else (3 if initial else 2, 1)
    for y in range(start_y, screen.lines - 1):
        number = first + y - start_y
        for old, x, pane in [(True, 0, left), (False, left + 1, screen.columns - left - 1)]:
            final = number == 12001
            text = "const finished = true;" if final else f"const value_{number:05d} = true;"
            expected = (" " * pane if final and old else
                        (f" {number:5d}{'+' if final else ' '} " + text).ljust(pane))
            actual = display[y][x:x + pane]
            if actual.replace("▎", " ") != expected:
                return False
            if styled and not (old and final):
                if any(screen.buffer[y][x + 8 + i].fg != "ff678d" for i in range(5)):
                    return False
    return True


def observe(session, started, content, styled, label, *, timeout=30):
    first = None
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        packet = session.read()
        if packet:
            data, received = packet
            for _ in session.frames.feed(data):
                screen = session.frames.screen
                if content(screen):
                    first = first if first is not None else (received - started) / 1e6
                    footer = screen.display[-1]
                    if "Highlighting" not in footer and "Loading" not in footer and styled(screen):
                        sample = {"step": label, "content_ms": first,
                                  "settled_ms": (received - started) / 1e6}
                        print(json.dumps(sample), flush=True)
                        return sample
        if session.returncode is not None:
            raise RuntimeError(f"Viewer exited during {label}: {session.returncode}")
        if session.rss_error:
            raise RuntimeError(session.rss_error)
    raise RuntimeError(f"Timeout during {label}:\n" + "\n".join(session.frames.screen.display))


def run(binary, path, name, lines):
    session = Session(binary, {"path": str(path)}, {
        "sampled_rss_stop_bytes": 6 * 1024**3, "cadence_ms": 50,
    })
    result = {"fixture": name, "samples": []}
    try:
        if name == "long":
            wanted = lambda s: long_grid(s, lines, False)
            result["samples"].append(observe(session, session.started_ns, wanted, wanted, "open"))
            started, _ = session.inject({"input_hex": "77"})
            wanted = lambda s: long_grid(s, lines, True)
            result["samples"].append(observe(session, started, wanted, wanted, "wrap"))
            for i, width in enumerate([100, 200, 120, 160] * 2):
                started, _ = session.inject({"resize": [width, 50]})
                result["samples"].append(observe(session, started, wanted, wanted, f"resize-{i}-{width}"))
        else:
            result["samples"].append(observe(session, session.started_ns,
                lambda s: deep_grid(s, False, False, True), lambda s: deep_grid(s, False, True, True), "open"))
            for key, end, label in [(b"G", True, "deep-cold"), (b"g", False, "start-revisit"),
                                    (b"G", True, "deep-revisit")]:
                started, _ = session.inject({"input_hex": key.hex()})
                result["samples"].append(observe(session, started,
                    lambda s: deep_grid(s, end, False), lambda s: deep_grid(s, end, True), label))
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
    parser.add_argument("--trials", type=int, default=3)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("The output path already exists")
    paths, lines = fixtures(args.fixture_dir)
    result = {
        "schema": 1, "diagnostic_only": True, "host": platform.platform(),
        "binary": str(args.binary.resolve()),
        "binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
        "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "fixtures": {key: {"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
                     for key, path in paths.items()},
        "notes": ["Real PTY receipt, not Ghostty display latency.",
                  "Fresh processes, warm filesystem; serial trials, no cache purge.",
                  "wait4 RSS is bytes on macOS. pyte and sampler are outside viewer RSS.",
                  "Three trials are diagnostic samples, not tail-latency acceptance."],
        "trials": [],
    }
    try:
        for trial in range(args.trials):
            for name, path in paths.items():
                sample = run(args.binary.resolve(), path, name, lines)
                sample["trial"] = trial
                result["trials"].append(sample)
                print(json.dumps(sample), flush=True)
    except BaseException as error:
        result["error"] = str(error)
        raise
    finally:
        args.output.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
