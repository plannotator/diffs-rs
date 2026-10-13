"""Five-patch source-checked PTY rehearsal, frozen replay, and memory tours."""

import argparse
from collections import defaultdict
import gzip
import json
import os
from pathlib import Path
import select
import re
import subprocess
import time

from corpus_reference import Corpus, ROOT, check_screen, digest, glyphs, restore, snapshot, starts
from terminal_baseline import Frames, Session as MeasuredSession
from sidebar_reference import diff_geometry, diff_panes, diff_region

LIMITS = {"sampled_rss_stop_bytes": 6 * 1024**3, "cadence_ms": 50}
PRESENTATION = {"sidebar": "auto", "input_mode": "patch",
                "terminal_start": [160, 50], "geometry": "sidebar-v1"}


def settled(screen):
    footer = screen.display[-1]
    return not any(word in footer for word in ["Loading", "Highlighting", "INCOMPLETE"])


def await_state(session, corpus, state, start, expected=None, timeout=30, *, wanted_region=None):
    first = None
    styled = None
    indexed = None
    validation_ns = 0
    initial_frames = session.frames.count
    last_error = None
    if expected is not None and wanted_region is None:
        wanted_region = diff_region(restore(expected), state.get("sidebar", False))
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        packet = session.read()
        if packet:
            data, received = packet
            before = time.monotonic_ns()
            for _ in session.frames.feed(data):
                try:
                    coverage = {}
                    records = check_screen(session.frames.screen, corpus, **state, coverage=coverage) if expected is None else []
                    if expected is None and not records and state.get("require_anchor"):
                        raise AssertionError("No expected source")
                    region = diff_region(session.frames.screen, state.get("sidebar", False)) if expected else None
                    if expected and region[0] != wanted_region[0]:
                        raise AssertionError("Frozen content destination differs")
                    if first is None:
                        first = (received - start) / 1e6
                    if indexed is None and "Loading" not in session.frames.screen.display[-1]:
                        indexed = (received - start) / 1e6
                    actual = snapshot(session.frames.screen)
                    if styled is None and expected and region[1] == wanted_region[1]:
                        styled = (received - start) / 1e6
                    if settled(session.frames.screen):
                        if expected and actual != expected:
                            raise AssertionError("Frozen completed cell styles differ")
                        validation_ns += time.monotonic_ns() - before
                        return {"content_ms": first, "index_ready_upper_bound_ms": indexed,
                                "styled_ms": styled if styled is not None else (received - start) / 1e6,
                                "ready_ms": (received - start) / 1e6, "validation_ms": validation_ns / 1e6,
                                "frames": session.frames.count - initial_frames,
                                "screen": actual, "records": records, "coverage": coverage}
                except (AssertionError, ValueError, KeyError) as error:
                    last_error = str(error)
            validation_ns += time.monotonic_ns() - before
        if session.rss_error:
            raise RuntimeError(session.rss_error)
        if session.returncode is not None:
            raise RuntimeError(f"Viewer exited: {session.returncode}")
    raise RuntimeError(f"State timeout: {last_error}\n" + "\n".join(session.frames.screen.display))


def travel(session, payload, width=160, height=50):
    """Drain unmeasured setup output, then request a real full redraw by resizing."""
    begin = time.monotonic()
    tokens = re.findall(rb"\x1b\[[0-9;]*[~A-Za-z]|[^\x1b]", payload)
    if b"".join(tokens) != payload:
        raise ValueError("Unrecognized public setup sequence")
    groups, pending = [], b""
    for token in tokens:
        if len(pending) + len(token) > 128:
            groups.append(pending)
            pending = b""
        pending += token
    if pending:
        groups.append(pending)
    for group in groups:
        pending = memoryview(group)
        quiet = time.monotonic()
        while pending or time.monotonic() - quiet < .05:
            if time.monotonic() - begin > 600:
                raise RuntimeError("Public setup exceeded its ten-minute watchdog")
            if pending:
                _, writable, _ = select.select([], [session.master], [], 0)
                if writable:
                    try:
                        sent = os.write(session.master, pending)
                        pending = pending[sent:]
                        quiet = time.monotonic()
                    except BlockingIOError:
                        pass
            packet = session.read(.005)
            if packet:
                quiet = time.monotonic()
            if session.returncode is not None or session.rss_error:
                raise RuntimeError(session.rss_error or "Viewer exited during public setup")
    quiet = time.monotonic()
    while time.monotonic() - quiet < .3:
        if time.monotonic() - begin > 600:
            raise RuntimeError("Public setup exceeded its ten-minute watchdog")
        packet = session.read(.005)
        if packet:
            quiet = time.monotonic()
        if session.returncode is not None or session.rss_error:
            raise RuntimeError(session.rss_error or "Viewer exited during public setup")
    alternate = width - 1
    session.frames = Frames(alternate, height)
    session.inject({"resize": [alternate, height]})
    until = time.monotonic() + .1
    while time.monotonic() < until:
        packet = session.read()
        if packet:
            list(session.frames.feed(packet[0]))
    session.frames = Frames(width, height)
    start, _ = session.inject({"resize": [width, height]})
    return start, (time.monotonic() - begin) * 1000


class Rehearsal:
    def __init__(self, binary, corpus, out):
        self.corpus, self.out = corpus, out
        self.session = MeasuredSession(binary, {"path": str(corpus.path)}, LIMITS)
        self.steps = []
        self.state = {"expected_file": 0, "split": True, "wrapped": False, "horizontal": 0,
                      "sidebar": corpus.count >= 2}
        self.file = 0
        self.old_side = False
        self.width, self.height = 160, 50

    def step(self, label, action, category="setup", anchor=None, hold=False):
        reflow = "resize" in action or "travel" in action or action.get("input_hex") in ["75", "77", "6c"]
        remembered = anchor if anchor is not None else self.state.get("anchor")
        if remembered is not None and reflow:
            _, line = self.corpus.source(remembered)
            if line.sign != " ":
                self.old_side = line.new is None
        if action.get("input_hex") in ["67", "47"]:
            endpoint = self.corpus.ref["first" if action["input_hex"] == "67" else "last"]
            self.old_side = self.corpus.source(endpoint["source_ordinal"])[1].new is None
        if action.get("input_hex") == "09":
            self.old_side = not self.old_side
        if anchor is not None:
            _, line = self.corpus.source(anchor)
            self.state.update(anchor=anchor, require_anchor=True,
                              anchor_old=self.old_side if line.sign == " " else line.new is None)
        else:
            self.state.pop("anchor", None)
            self.state.pop("require_anchor", None)
            self.state.pop("anchor_old", None)
            self.state.pop("anchor_byte", None)
        if "travel" in action:
            started, elapsed = travel(self.session, bytes.fromhex(action["travel"]), self.width, self.height)
            action["setup_ms"] = elapsed
        else:
            started, _ = self.session.inject(action)
        result = await_state(self.session, self.corpus, self.state, started)
        self.steps.append({"label": label, "category": category, "action": action.copy(),
                           "state": self.state.copy(), "expected": result["screen"],
                           "records": result["records"], "coverage": result["coverage"], "hold": hold})
        if category == "setup":
            print(label, flush=True)
        if result["records"]:
            self.file = next((r["file"] for r in result["records"] if r["id"] == anchor), self.state["expected_file"])
        return result

    def key(self, label, key, **kwargs):
        return self.step(label, {"input_hex": key.hex()}, **kwargs)

    def mode(self, split, label="layout", **kwargs):
        if self.state["split"] != split:
            self.state["split"] = split
            return self.key(label, b"u", **kwargs)

    def goto_file(self, target, label="file"):
        self.mode(False)
        if self.state["wrapped"] or self.state["horizontal"]:
            raise ValueError("File setup requires explicit wrap/horizontal reset")
        self.state.update(expected_file=target, require_file=True)
        def from_file(index):
            if index != target:
                return (b"n" if target > index else b"N") * abs(target - index)
            if self.corpus.count == 1:
                return b"k" * 4
            return b"nN" if target < self.corpus.count - 1 else b"Nn"

        routes = [b"g" + from_file(self.corpus.ref["first"]["file"]),
                  b"G" + from_file(self.corpus.ref["last"]["file"])]
        if target != self.file:
            routes.append((b"n" if target > self.file else b"N") * abs(target - self.file))
        elif self.corpus.count > 1:
            routes.append(b"nN" if target < self.corpus.count - 1 else b"Nn")
        payload = min(routes, key=len)
        if payload[:1] in [b"g", b"G"]:
            endpoint = self.corpus.ref["first" if payload[:1] == b"g" else "last"]
            self.old_side = self.corpus.source(endpoint["source_ordinal"])[1].new is None
        self.file = target
        self.state.pop("anchor", None)
        result = self.step(label, {"travel": payload.hex()})
        self.state.pop("require_file", None)
        return result

    def goto_source(self, source, label="source"):
        file_index, line = self.corpus.source(source)
        self.goto_file(file_index, label + "-file")
        file = self.corpus.file(file_index)
        h = self.corpus.ref["files"][file_index]["hunks"][line.hunk]
        before = [s for s in file.lines if h["source_start"] <= s.id < source]
        rows = len(before) + sum(s.no_newline for s in before) + 1
        payload = b"]" * (line.hunk + 1) + b"\x1b[6~" * (rows // 46) + b"j" * (rows % 46)
        self.state["expected_file"] = file_index
        self.state["anchor_byte"] = 0
        return self.step(label, {"travel": payload.hex()}, anchor=source)


def geometry_plan(driver):
    target = driver.corpus.targets()[-1]
    driver.goto_source(target["source"], "long-source")
    driver.state["wrapped"] = True
    driver.key("wrap", b"w", anchor=target["source"])
    for split in (True, False):
        driver.mode(split, anchor=target["source"])
        for width in (100, 96, 97, 160):
            driver.width = width
            driver.step(f"{'split' if split else 'unified'}-{width}", {"resize": [width, 50]},
                        anchor=target["source"])


def memory_plan(driver):
    corpus = driver.corpus
    for index in corpus.ref["file_quantiles"]:
        driver.goto_file(index, f"quantile-{index}")
        driver.mode(True, f"quantile-{index}-split", hold=True)
        driver.mode(False, f"quantile-{index}-unified", hold=True)
    for target in corpus.targets():
        driver.goto_source(target["source"], target["name"])
        driver.mode(True, target["name"] + "-split", anchor=target["source"], hold=True)
        for width, height in [(100, 30), (160, 50)]:
            driver.width, driver.height = width, height
            driver.step(f"{target['name']}-resize-{width}", {"resize": [width, height]},
                        anchor=target["source"], hold=True)
        driver.mode(False, target["name"] + "-unified", anchor=target["source"], hold=True)
    target = corpus.targets()[-1]
    driver.goto_source(target["source"], "long-line")
    file_index, source = corpus.source(target["source"])
    _, diff_width = diff_geometry(driver.width, driver.height, driver.state["sidebar"])
    available = diff_width - 2 * corpus.meta(file_index)["digits"] - 4
    offsets = starts(source.text, available)
    geometry = glyphs(source.text)
    byte_by_column = {column: byte for byte, column, _, _ in geometry}
    driver.state["wrapped"] = True
    driver.key("long-wrap", b"w", anchor=source.id, hold=True)
    part = 0

    def move_to(wanted, label, full_check=False):
        nonlocal part
        delta = wanted - part
        page = b"\x1b[6~" if delta > 0 else b"\x1b[5~"
        line = b"j" if delta > 0 else b"k"
        if full_check:
            for n in range(abs(delta) // 46):
                part += 46 if delta > 0 else -46
                driver.state["anchor_byte"] = byte_by_column.get(offsets[part], 0)
                driver.key(f"{label}-page-{n}", page, anchor=source.id, hold=True)
            if abs(delta) % 46:
                driver.state["anchor_byte"] = byte_by_column.get(offsets[wanted], 0)
                driver.key(label + "-last", line * (abs(delta) % 46), anchor=source.id, hold=True)
        elif delta:
            driver.state["anchor_byte"] = byte_by_column.get(offsets[wanted], 0)
            payload = page * (abs(delta) // 46) + line * (abs(delta) % 46)
            driver.step(label, {"travel": payload.hex()}, anchor=source.id, hold=True)
        part = wanted

    move_to(len(offsets) - 1, "long", full_check=True)
    # Reach distant text with existing wrap/page commands, then inspect it unwrapped.
    for label, wanted in [("first", 0), ("middle", len(offsets) // 2), ("end", len(offsets) - 1)]:
        move_to(wanted, "long-" + label)
        driver.state.update(wrapped=False, horizontal=offsets[wanted], horizontal_bound=True)
        driver.key("horizontal-" + label, b"w", anchor=source.id, hold=True)
        driver.state["wrapped"] = True
        driver.key("rewrap-" + label, b"w", anchor=source.id)
    first = corpus.ref["first"]
    driver.state.update(expected_file=first["file"], anchor_byte=0)
    driver.key("tour-source-reset", b"g", anchor=first["source_ordinal"])
    driver.state.update(wrapped=False, horizontal=0)
    driver.key("tour-wrap-reset", b"w", anchor=first["source_ordinal"])
    driver.goto_file(0, "tour-reset")
    driver.mode(True, "tour-reset-split")
    if driver.old_side:
        driver.key("tour-side-reset", b"\t")


def style_reference(corpus, steps, binary, out):
    groups = defaultdict(set)
    for step in steps:
        for record in step["records"]:
            source = corpus.file(record["file"]).by_id[record["id"]]
            groups[record["file"], source.hunk].add(source.id)
    request = {"appearance": "dark", "hunks": []}
    for (f, h), wanted in sorted(groups.items()):
        file = corpus.file(f)
        request["hunks"].append({"id": f"{f}:{h}", "path": file.meta["path"],
                                  "wanted": sorted(wanted), "lines": [
            {"id": s.id, "text": s.text, "old": s.old is not None, "new": s.new is not None}
            for s in file.lines if s.hunk == h and s.id <= max(wanted)]})
    request_path = out / "sequential-request.json.gz"
    request_path.write_bytes(gzip.compress(json.dumps(request).encode(), mtime=0))
    run = subprocess.run([str(binary)], input=json.dumps(request), capture_output=True,
                         text=True, timeout=600)
    (out / "sequential.stderr.txt").write_text(run.stderr)
    if run.returncode:
        raise RuntimeError(f"Sequential reference failed: {run.stderr}")
    answer = json.loads(run.stdout)
    (out / "sequential-reference.json.gz").write_bytes(gzip.compress(run.stdout.encode(), mtime=0))
    return {**check_styles(corpus, steps, answer), "reference_sha256": digest(binary)}


def check_styles(corpus, steps, answer):
    by_id = {line["id"]: line for h in answer["hunks"] for line in h["lines"]}
    default = "".join(f"{n:02x}" for n in answer["palette"]["fg"])
    checked = 0
    backgrounds = 0
    palette = {name: "".join(f"{n:02x}" for n in rgb) for name, rgb in answer["palette"].items()}
    for step in steps:
        screen = step["expected"]
        cells = restore(screen)
        if diff_geometry(screen["width"], screen["height"], step["state"].get("sidebar", False))[0]:
            coverage = {}
            check_screen(cells, corpus, **step["state"], coverage=coverage)
            corpus.sidebar.check_colors(cells, palette, coverage["sidebar"]["displayed_current"])
        panes = diff_panes(screen["width"], screen["height"], step["state"]["split"],
                           step["state"].get("sidebar", False))
        if step["state"]["split"]:
            occupied = {(r["y"], r["old"]) for r in step["records"]}
            for y in {r["y"] for r in step["records"]}:
                for old, lo, width in panes:
                    if (y, old) not in occupied:
                        if any(cells.buffer[y][x].bg != palette["empty"] for x in range(lo, lo + width)):
                            raise AssertionError(f"Incorrect empty-side background: {step['label']} row {y}")
        for r in step["records"]:
            source = corpus.file(r["file"]).by_id[r["id"]]
            split = step["state"]["split"]
            _, pane_start, pane_width = next(p for p in panes if not split or p[0] == r["old"])
            pane_end = pane_start + pane_width
            base, emphasis, number = {" ": ("context", "context", "dim"),
                                      "+": ("add_bg", "add_word", "added"),
                                      "-": ("del_bg", "del_word", "deleted")}[source.sign]
            active = cells.buffer[r["y"]][pane_start].data == "▎"
            for x in range(pane_start, pane_end):
                style = next(run for run in screen["styles"][r["y"]] if run[0] <= x < run[1])
                allowed = {palette[base]} if x < r["x"] else {palette[base], palette[emphasis]}
                if style[3] not in allowed:
                    raise AssertionError(f"Incorrect source background role: {step['label']} source {r['id']} cell {x}")
                if x < r["x"] and style[2] != palette["accent" if active else number]:
                    raise AssertionError(f"Incorrect source gutter color: {step['label']} source {r['id']} cell {x}")
                backgrounds += 1
            side = by_id[r["id"]]["old" if r["old"] else "new"]
            if side is None:
                raise AssertionError("Sequential reference side is absent")
            tokens = side["tokens"]
            for x, byte in r["positions"]:
                wanted = next(("".join(f"{n:02x}" for n in t["rgb"]) for t in tokens
                               if t["start"] <= byte < t["end"]), default)
                actual = next(run[2] for run in screen["styles"][r["y"]] if run[0] <= r["x"] + x < run[1])
                if actual != wanted:
                    raise AssertionError(f"Sequential color mismatch in {step['label']} source {r['id']} byte {byte}: {actual} != {wanted}")
                checked += 1
    return {"checked_foreground_cells": checked, "checked_background_cells": backgrounds,
            "hunks": len(answer["hunks"]),
            "source_records": len(by_id)}


def rehearse(binary, reference_binary, corpus, out, memory, geometry=False):
    out.mkdir(parents=True, exist_ok=False)
    driver = Rehearsal(binary, corpus, out)
    result = {"fixture": corpus.fixture, "steps": driver.steps,
              "kind": "geometry" if geometry else "memory" if memory else "latency",
              "presentation": dict(PRESENTATION)}
    result["binary_sha256"] = digest(binary)
    try:
        opening = await_state(driver.session, corpus, driver.state, driver.session.started_ns)
        initial = {"label": "opening", "state": driver.state.copy(),
                   "expected": opening["screen"], "records": opening["records"], "coverage": opening["coverage"]}
        result["opening"] = initial
        if geometry:
            geometry_plan(driver)
        elif not memory:
            for direction, button in [("forward", 65), ("reverse", 64)]:
                for i in range(40):
                    driver.key(f"{direction}-{i}", f"\x1b[<{button};90;10M".encode(), category=direction)
            for i in range(40):
                end = i % 2 == 0
                anchor = corpus.ref["last" if end else "first"]
                driver.state["expected_file"] = anchor["file"]
                driver.key(f"endpoint-{i}", b"G" if end else b"g", category="distant_seek", anchor=anchor["source_ordinal"])
            target = corpus.targets()[0]
            driver.goto_source(target["source"], "largest-hunk-mid")
            driver.mode(True, anchor=target["source"])
            for i in range(40):
                driver.mode(not driver.state["split"], f"layout-{i}", category="layout", anchor=target["source"])
            for i in range(40):
                driver.width, driver.height = (100, 30) if i % 2 == 0 else (160, 50)
                driver.step(f"resize-{i}", {"resize": [driver.width, driver.height]}, "resize", anchor=target["source"])
        else:
            memory_plan(driver)
        result["quit"] = driver.session.quit()
        if result["quit"]["exit_code"] != 0 or not result["quit"]["termios_restored"]:
            raise RuntimeError("Terminal cleanup failed")
    except BaseException as error:
        result["error"] = str(error)
        raise
    finally:
        driver.session.close()
        (out / "rehearsal.json.gz").write_bytes(gzip.compress(json.dumps(result).encode(), mtime=0))
        (out / "last-screen.txt").write_text("\n".join(driver.session.frames.screen.display))
    result["style_check"] = style_reference(corpus, [result["opening"], *driver.steps], reference_binary, out)
    if digest(binary) != result["binary_sha256"]:
        raise RuntimeError("Viewer changed during rehearsal")
    (out / "trace.json.gz").write_bytes(gzip.compress(json.dumps(result).encode(), mtime=0))
    print(json.dumps({"fixture": corpus.fixture["id"], "kind": result["kind"], "steps": len(driver.steps), **result["style_check"]}), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("reference_binary", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--fixture", required=True)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--memory", action="store_true")
    mode.add_argument("--geometry-check", action="store_true")
    parser.add_argument("--resume-style", type=Path)
    args = parser.parse_args()
    fixtures = json.loads((ROOT / "fixtures/diffshub/corpus.json").read_text())["fixtures"]
    fixture = next(f for f in fixtures if f["id"] == args.fixture)
    corpus = Corpus(fixture)
    if args.resume_style:
        args.output.mkdir(parents=True, exist_ok=False)
        result = json.loads(gzip.decompress(args.resume_style.read_bytes()))
        if result.get("error"):
            raise ValueError("Cannot certify a failed process rehearsal")
        result["style_check"] = style_reference(corpus, [result["opening"], *result["steps"]],
                                                 args.reference_binary.resolve(), args.output)
        (args.output / "trace.json.gz").write_bytes(gzip.compress(json.dumps(result).encode(), mtime=0))
        print(json.dumps(result["style_check"]), flush=True)
    else:
        rehearse(args.binary.resolve(), args.reference_binary.resolve(), corpus, args.output.resolve(), args.memory, args.geometry_check)


if __name__ == "__main__":
    main()
