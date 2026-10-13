"""Prepare checked references, capture five declared cases, and replay saved output."""

import argparse
import base64
from collections import defaultdict
import gzip
import json
import os
from pathlib import Path
import subprocess

from capture_delivery import (ACTIONS, Capture, check_protocol, replay_recording,
                              run as capture, source_check, wanted_case, write_json)
from check_delivery import Challenge
from corpus_acceptance import LIMITS, await_state, check_styles
from corpus_reference import ROOT, check_screen, digest, restore
from patch_reference import decode
from syntax_diagnostic import grid as deep_grid

MANIFEST = ROOT / "evidence/delivery-v1/cases/manifest.json"
EXISTING = ROOT / "evidence/delivery-v1/baseline-cases-v3/report.json"
CASE_IDS = ["deep", "guard", "many-small", "ratatui", "ruby"]
HELPERS = ["first_visit_highlighting.py", "capture_delivery.py", "check_delivery.py",
           "corpus_acceptance.py", "corpus_reference.py", "patch_reference.py",
           "trace_reference.py", "terminal_baseline.py", "rss_sampling.py",
           "syntax_diagnostic.py", "reading_diagnostic.py"]
DEEP = [("opening", None, 1, None, False),
        ("cold-end", "47", 11955, 12001, False),
        ("reverse", "1b5b357e1b5b357e", 11909, 11909, False),
        ("theme-change", "74", 11909, 11909, True),
        ("uncached-forward", "1b5b367e1b5b367e", 11955, 12001, True)]


def state(anchor=None, file=0, old=False):
    result = {"expected_file": file, "split": True, "wrapped": False, "horizontal": 0}
    if anchor is not None:
        result.update(anchor=anchor, require_anchor=True, anchor_old=old)
    return result


def protocol(case):
    corpus = Challenge(case)
    if case["id"] == "deep":
        steps = [{"label": label, "action": None if keys is None else {"input_hex": keys},
                  "state": state(None if active is None else active - 1)}
                 for label, keys, _, active, _ in DEEP]
    elif case["id"] == "ruby":
        end = corpus.ref["last"]["source_ordinal"]
        steps = [{"label": "opening", "action": None, "state": state()}]
        for number, theme in enumerate(["dark", "light", "dark-again"]):
            steps.append({"label": theme + "-end", "action": {"input_hex": "47" if number == 0 else "746747"},
                          "state": state(end)})
            for page in range(1, 4):
                steps.append({"label": f"{theme}-back-{page}", "action": {"input_hex": "1b5b357e"},
                              "state": state(end - 46 * page)})
    else:
        steps, current = [], state()
        for label, action in ACTIONS:
            if label in ("end", "start"):
                endpoint = corpus.ref["last" if label == "end" else "first"]
                current = state(endpoint["source_ordinal"], endpoint["file"], endpoint["new"] is None)
            steps.append({"label": label, "action": action, "state": current.copy()})
    return check_protocol({"window_ns": 5_000_000_000 if case["id"] in ("deep", "ruby") else 500_000_000,
                           "steps": steps})


def generated_cases(out):
    deep = ROOT / "evidence/next-stage-v1/syntax-fixtures/deep.patch"
    expected = ("diff --git a/deep.ts b/deep.ts\n--- a/deep.ts\n+++ b/deep.ts\n"
                "@@ -1,12000 +1,12001 @@\n"
                + "".join(f" const value_{i:05d} = true;\n" for i in range(1, 12001))
                + "+const finished = true;\n").encode()
    if deep.read_bytes() != expected:
        raise ValueError("Preserved deep fixture differs from the syntax diagnostic")
    deep_path = out / "deep.patch"
    with deep_path.open("xb") as stream:
        stream.write(expected)
    ruby_dir = out / "ruby"
    ruby_dir.mkdir()
    body = "".join(f"return 1 # shared_{n:04}\n" for n in range(320))
    ending = "OLD_STOP\nreturn 2 # independent\nNEW_STOP\nreturn 3 # done\n"
    tail = "".join(f"return 4 # tail_{n:04}\n" for n in range(100))
    for name, delimiter in [("old.rb", "OLD_STOP"), ("new.rb", "NEW_STOP")]:
        with (ruby_dir / name).open("x") as stream:
            stream.write(f"value = <<{delimiter}\n{body}{ending}{tail}")
    command = ["git", "diff", "--no-index", "--no-ext-diff", "--no-textconv", "--no-color",
               "--unified=100000", "old.rb", "new.rb"]
    result = subprocess.run(command, cwd=ruby_dir, capture_output=True, timeout=30)
    write_json(ruby_dir / "generation.json", {"command": command, "exit_code": result.returncode,
               "stderr": result.stderr.decode(errors="replace"),
               "contract_sha256": digest(ROOT / "tests/process/syntax_contract.rs")})
    if result.returncode != 1:
        raise ValueError("Captured-delimiter patch generation failed")
    ruby_path = ruby_dir / "input.patch"
    with ruby_path.open("xb") as stream:
        stream.write(result.stdout)
    cases = []
    for name, path in [("deep", deep_path), ("ruby", ruby_path)]:
        ref = path.with_suffix(".reference.json")
        write_json(ref, decode(path))
        cases.append({"id": name, "path": str(path.resolve()), "sha256": digest(path),
                      "reference": str(ref.resolve()), "reference_sha256": digest(ref)})
    return cases


def appearance(label):
    return "light" if label in ("light", "narrow", "wide", "theme-change", "uncached-forward") or label.startswith("light-") else "dark"


def check_explicit(case, index, screen):
    if case["id"] == "deep":
        _, _, first, active, light = DEEP[index]
        if not deep_grid(screen, first, active, light, initial=index == 0, styled=True):
            raise AssertionError("Deep source destination or explicit keyword colors differ")
        return "exact syntax_diagnostic grid and keyword roles"
    if case["id"] == "ruby" and index in (4, 8, 12):
        light = index == 8
        for text, old_string, new_string in [("return 1 # shared_0319", True, True),
                                             ("return 2 # independent", False, True),
                                             ("return 3 # done", False, False)]:
            for x, string in [(0, old_string), (80, new_string)]:
                rows = [y for y in range(2, 49) if screen.display[y][x + 6:x + 6 + len(text)] == text]
                if len(rows) != 1:
                    raise AssertionError("Captured-delimiter source destination differs")
                color = ("199f43" if light else "5ecc71") if string else ("d32a61" if light else "ff678d")
                if any(screen.buffer[rows[0]][x + col].fg != color for col in range(6, 12)):
                    raise AssertionError("Captured-delimiter old/new syntax role differs")
        return "three explicit old/new Ruby delimiter roles"
    return None


def reference_views(binary, case, declared, out):
    out.mkdir()
    result = {"status": "running", "id": case["id"], "steps": [],
              "scope": "Unmeasured reference preparation; no latency acceptance"}
    corpus, session = Challenge(case), None
    try:
        source_check(binary, corpus)
        session = Capture(str(binary.resolve()), {"path": str(corpus.path)}, LIMITS)
        for index, step in enumerate(declared["steps"]):
            started = session.started_ns if step["action"] is None else session.inject(step["action"])[0]
            observed = await_state(session, corpus, step["state"], started)
            check_explicit(case, index, session.frames.screen)
            result["steps"].append({"label": step["label"], "screen": observed["screen"]})
        result["quit"] = session.quit()
        if (result["quit"]["exit_code"] != 0 or not result["quit"]["termios_restored"]
                or not all(result["quit"]["cleanup_sequences_received"].values())):
            raise AssertionError("Reference terminal cleanup failed")
        result["status"] = "complete"
    except BaseException as error:
        result.update(status="failed", error=str(error))
        raise
    finally:
        try:
            if session is not None:
                session.close()
                if session.rss_error or session.capture_error:
                    raise RuntimeError(session.rss_error or session.capture_error)
        except BaseException as error:
            result.update(status="failed", cleanup_error=str(error))
            raise
        finally:
            if session is not None:
                with gzip.open(out / "preparation-packets.jsonl.gz", "xb") as stream:
                    for data, ns in session.packets:
                        stream.write((json.dumps({"received_ns": ns, "base64": base64.b64encode(data).decode()}) + "\n").encode())
                result.update(cleanup_exit_code=session.returncode, rss_error=session.rss_error,
                              capture_error=session.capture_error, sampler_stopped=not session.sampler.is_alive())
            write_json(out / "preparation.json", result)
    return result["steps"]


def check_reference(case, declared, steps, oracle, out):
    corpus, verified = Challenge(case), []
    for index, (step, expected) in enumerate(zip(declared["steps"], steps, strict=True)):
        if expected["label"] != step["label"]:
            raise ValueError("Reference schedule differs")
        screen = restore(expected["screen"])
        records = check_screen(screen, corpus, **step["state"])
        verified.append({"label": step["label"], "expected": expected["screen"], "state": step["state"],
                         "records": records, "appearance": appearance(step["label"]),
                         "explicit": check_explicit(case, index, screen)})
    checks = []
    for theme in ("dark", "light"):
        selected = [step for step in verified if step["appearance"] == theme]
        groups = defaultdict(set)
        for step in selected:
            for record in step["records"]:
                line = corpus.file(record["file"]).by_id[record["id"]]
                groups[record["file"], line.hunk].add(line.id)
        request = {"appearance": theme, "hunks": []}
        for (file_id, hunk), wanted in sorted(groups.items()):
            file = corpus.file(file_id)
            request["hunks"].append({"id": f"{file_id}:{hunk}", "path": file.meta["path"], "wanted": sorted(wanted),
                                     "lines": [{"id": line.id, "text": line.text,
                                                "old": line.old is not None, "new": line.new is not None}
                                               for line in file.hunks[hunk] if line.id <= max(wanted)]})
        encoded = json.dumps(request).encode()
        with gzip.open(out / f"{theme}-request.json.gz", "xb") as stream:
            stream.write(encoded)
        result = subprocess.run([str(oracle.resolve())], input=encoded, capture_output=True, timeout=600)
        with (out / f"{theme}-stderr.txt").open("xb") as stream:
            stream.write(result.stderr)
        if result.returncode:
            raise RuntimeError("Sequential syntax reference failed")
        answer = json.loads(result.stdout)
        with gzip.open(out / f"{theme}-answer.json.gz", "xb") as stream:
            stream.write(result.stdout)
        checks.append({"appearance": theme, **check_styles(corpus, selected, answer)})
    return {"id": case["id"], "protocol": declared, "steps": steps, "checks": checks}


def prepare(reference, oracle, out):
    out.mkdir(parents=True, exist_ok=False)
    report = {"status": "running", "cases": [], "scope": "Reference preparation, not a measurement cohort"}
    try:
        if "NATIVE_DIFF_DELIVERY_TRACE" in os.environ:
            raise ValueError("Unset delivery tracing for ordinary reference preparation")
        identities = {str(path.resolve()): digest(path) for path in
                      [reference, oracle, MANIFEST, EXISTING, ROOT / "tests/process/syntax_contract.rs",
                       ROOT / "tests/process.rs", ROOT / "examples/sequential_style_reference.rs",
                       *[ROOT / "tools" / name for name in HELPERS]]}
        write_json(out / "preparation-freeze.json", {"identities": identities, "case_ids": CASE_IDS})
        generated = generated_cases(out)
        existing = json.loads(MANIFEST.read_text())["cases"]
        cases = [next(case for case in generated + existing if case["id"] == name) for name in CASE_IDS]
        expected = []
        for case in cases:
            declared = protocol(case)
            directory = out / (case["id"] + "-reference")
            if case["id"] in ("deep", "ruby"):
                steps = reference_views(reference, case, declared, directory)
            else:
                directory.mkdir()
                steps = [{"label": step["label"], "screen": step["screen"]}
                         for step in wanted_case(EXISTING, case["id"])["steps"]]
            expected.append(check_reference(case, declared, steps, oracle, directory))
            report["cases"].append({"id": case["id"], "status": "checked"})
        expected_path = out / "expected.json"
        write_json(expected_path, {"cases": expected})
        for path, sha in identities.items():
            if digest(Path(path)) != sha:
                raise ValueError(f"Preparation input changed: {path}")
        identities[str(expected_path.resolve())] = digest(expected_path)
        for case in cases:
            identities[str((ROOT / case["path"]).resolve())] = case["sha256"]
            identities[str((ROOT / case["reference"]).resolve())] = case["reference_sha256"]
        packet = {"schema": 1, "cases": cases, "expected_path": str(expected_path.resolve()),
                  "reference_binary": str(reference.resolve()), "oracle_binary": str(oracle.resolve()),
                  "identities": identities, "scope": "One ordinary and one diagnostic process per declared case"}
        write_json(out / "packet.json", packet)
        report["status"] = "complete"
    except BaseException as error:
        report.update(status="failed", error=str(error))
        raise
    finally:
        write_json(out / "preparation.json", report)


def verify_packet(path):
    packet = json.loads(path.read_text())
    if packet["schema"] != 1 or [case["id"] for case in packet["cases"]] != CASE_IDS:
        raise ValueError("Declared field differs")
    for name, sha in packet["identities"].items():
        if digest(Path(name)) != sha:
            raise ValueError(f"Frozen input changed: {name}")
    return packet


def run(packet_path, binary, role, out):
    out.mkdir(parents=True, exist_ok=False)
    report = {"status": "running", "role": role, "cases": [], "packet_path": str(packet_path.resolve()),
              "scope": "Single diagnostic cohort; no distribution or production speedup claim"}
    try:
        if role not in ("ordinary", "diagnostic"):
            raise ValueError("Unknown capture role")
        if "NATIVE_DIFF_DELIVERY_TRACE" in os.environ:
            raise ValueError("Use the diagnostic role, not an inherited trace path")
        packet = verify_packet(packet_path)
        binary_sha = digest(binary)
        report.update(packet_sha256=digest(packet_path), binary=str(binary.resolve()), binary_sha256=binary_sha)
        write_json(out / "run-freeze.json", report)
        expected_path = Path(packet["expected_path"])
        for case in packet["cases"]:
            directory = out / case["id"]
            expected = wanted_case(expected_path, case["id"])
            environment = ({"NATIVE_DIFF_DELIVERY_TRACE": str((directory / "delivery.json").resolve())}
                           if role == "diagnostic" else None)
            actual = capture(binary, case, expected_path, directory, protocol=expected["protocol"], child_env=environment)
            replayed = replay_recording(directory)
            if replayed["steps"] != [step["observation"] for step in actual["steps"]]:
                raise AssertionError("Persisted raw replay differs")
            if role == "diagnostic" and not (directory / "delivery.json").is_file():
                raise ValueError("Diagnostic trace is missing")
            artifacts = ["report.json", "packets.jsonl.gz"]
            if role == "diagnostic":
                artifacts.append("delivery.json")
            report["cases"].append({"id": case["id"], "status": actual["status"], "replay": replayed,
                                    "artifacts": {name: digest(directory / name) for name in artifacts}})
        verify_packet(packet_path)
        if digest(binary) != binary_sha:
            raise ValueError("Executable changed during capture")
        report["status"] = "complete"
    except BaseException as error:
        report.update(status="failed", error=str(error))
        raise
    finally:
        write_json(out / "report.json", report)


def replay(out):
    report = json.loads((out / "report.json").read_text())
    if report["status"] != "complete" or digest(Path(report["packet_path"])) != report["packet_sha256"]:
        raise ValueError("Incomplete cohort or changed packet")
    packet = verify_packet(Path(report["packet_path"]))
    frozen = json.loads((out / "run-freeze.json").read_text())
    if any(frozen[key] != report[key] for key in ("role", "packet_path", "packet_sha256", "binary", "binary_sha256")):
        raise ValueError("Recorded cohort identity differs from its run freeze")
    if report["role"] not in ("ordinary", "diagnostic"):
        raise ValueError("Unknown recorded capture role")
    if [case["id"] for case in report["cases"]] != CASE_IDS:
        raise ValueError("Incomplete recorded field")
    results = []
    for case, recorded in zip(packet["cases"], report["cases"], strict=True):
        directory = out / case["id"]
        required = {"report.json", "packets.jsonl.gz"}
        if report["role"] == "diagnostic":
            required.add("delivery.json")
        if set(recorded["artifacts"]) != required:
            raise ValueError("Recorded case artifact field differs")
        for name, sha in recorded["artifacts"].items():
            if digest(directory / name) != sha:
                raise ValueError("Recorded case artifact changed")
        captured = json.loads((directory / "report.json").read_text())
        if (captured["case"] != case or captured["binary_sha256"] != report["binary_sha256"]
                or captured["expected_path"] != packet["expected_path"]):
            raise ValueError("Recorded case identity differs from cohort")
        checked = replay_recording(directory)
        if checked != recorded["replay"]:
            raise ValueError("Persisted case observations differ")
        results.append({"id": case["id"], **checked})
    return {"status": "pass", "cases": results}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    prep = commands.add_parser("prepare", help="Runs unmeasured reference viewers and the sequential oracle")
    prep.add_argument("reference", type=Path)
    prep.add_argument("oracle", type=Path)
    prep.add_argument("output", type=Path)
    record = commands.add_parser("run")
    record.add_argument("packet", type=Path)
    record.add_argument("binary", type=Path)
    record.add_argument("role", choices=("ordinary", "diagnostic"))
    record.add_argument("output", type=Path)
    check = commands.add_parser("replay", help="Reads saved packets; never launches the viewer")
    check.add_argument("directory", type=Path)
    args = parser.parse_args()
    if args.command == "prepare":
        prepare(args.reference, args.oracle, args.output)
    elif args.command == "run":
        run(args.packet, args.binary, args.role, args.output)
    else:
        print(json.dumps(replay(args.directory), indent=2))


if __name__ == "__main__":
    main()
