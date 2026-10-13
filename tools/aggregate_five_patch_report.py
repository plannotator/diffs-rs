#!/usr/bin/env python3
"""Summarize supplied five-patch evidence, including incomplete runs."""

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path


FIXTURES = [
    ("pierre-0800fb", "pierre-rehearsal", "pierre-latency2", "pierre-memory-rehearsal", "pierre-memory-results"),
    ("ghostty-12291", "ghostty-12291-rehearsal", "ghostty-12291-latency2", "ghostty-12291-memory-rehearsal", "ghostty-12291-memory-results"),
    ("node-59805", "node-59805-rehearsal", "node-59805-latency2", "node-59805-memory-rehearsal", "node-59805-memory-results"),
    ("bun-30412", "bun-30412-rehearsal", "bun-30412-latency2", "bun-30412-memory-rehearsal", "bun-30412-memory-results"),
    ("linux-v6-v7", "linux-v6-v7-rehearsal", "linux-v6-v7-latency2", "linux-v6-v7-memory-rehearsal", "linux-v6-v7-memory-results"),
]


def read_json(path):
    return json.loads(path.read_text())


def read_last_json(path):
    for line in reversed(path.read_text().splitlines()):
        if line.startswith("{"):
            return json.loads(line)
    raise ValueError(f"No JSON result in {path}")


def mib(value):
    return round(value / 1024**2, 1) if value is not None else None


def item(item_id, title, status, summary, evidence, check_type="code"):
    return {"id": item_id, "title": title, "check_type": check_type, "status": status,
            "summary": summary, "findings": [{"severity": "info", "text": evidence}]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run_dir", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    run = args.run_dir.resolve()
    patches = []
    for fixture, rehearsal_name, latency_name, _memory_rehearsal_name, memory_name in FIXTURES:
        rehearsal = read_last_json(run / f"{rehearsal_name}.log")
        oracle_name = "pierre-oracle.json" if fixture == "pierre-0800fb" else f"{fixture}-oracle.json"
        oracle = read_json(run / oracle_name)
        latency = read_json(run / latency_name / "summary.json")
        memory = read_json(run / memory_name / "summary.json")
        patches.append({
            "fixture": fixture,
            "rehearsal": {
                "trace": str((run / rehearsal_name / "trace.json.gz").resolve()),
                "steps": rehearsal["steps"],
                "checked_foreground_cells": rehearsal["checked_foreground_cells"],
                "checked_background_cells": rehearsal["checked_background_cells"],
                "hunks": rehearsal["hunks"],
                "source_records": rehearsal["source_records"],
                "reference_sha256": rehearsal["reference_sha256"],
            },
            "oracle": {
                "artifact": oracle["artifact"],
                "status": oracle["status"],
                "unchanged_screens_checked": oracle["unchanged_screens_checked"],
                "corruption_cases": len(oracle["cases"]),
                "all_corruptions_rejected": bool(oracle["cases"]) and all(case["status"] == "rejected" for case in oracle["cases"]),
            },
            "latency": {
                "summary": str((run / latency_name / "summary.json").resolve()),
                "complete": latency["complete"],
                "targets": latency.get("targets"),
                "navigation_target_pass": latency["navigation_target_pass"],
                "opening_target_pass": latency["opening_target_pass"],
                "quit_target_pass": latency["quit_target_pass"],
                "os_peak_rss_mib": mib(latency["os_peak_rss_bytes"]),
                "opening_ready_ms": [round(row["index_ready_upper_bound_ms"], 2) if row else None for row in latency["opening"]],
                "categories": latency["categories"],
            },
            "memory": {
                "summary": str((run / memory_name / "summary.json").resolve()),
                "complete": memory["complete"],
                "targets": memory.get("targets"),
                "memory_target_pass": memory["memory_target_pass"],
                "quit_target_pass": memory["quit_target_pass"],
                "tour_settled_rss_mib": [mib(value) for value in memory["tour_settled_rss_max_bytes"]],
                "steady_rss_mib": mib(memory["steady_rss_bytes"]),
                "os_peak_rss_mib": mib(memory["os_peak_rss_bytes"]),
            },
        })

    report = {
        "schema_version": 1,
        "title": "Native diff: supplied five-patch evidence summary",
        "mode": "list",
        "selection": "selective",
        "selection_basis": "Supplied interior PTY, latency, and sustained-memory evidence for the five listed corpus entries. This aggregation starts no viewer and performs no new performance measurement.",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "repo": "/Users/ramos/oss/native-diff",
        "list_source": None,
        "notes": "Summarizes supplied PTY evidence, not a new run or verification of the current executable. Target outcomes retain the policy used by that run; a missing targets field denotes legacy policy recorded in its archived runner. PTY observations are not Ghostty paint timestamps or screenshots. Source digests and engineering checks are not verified by this generator.",
        "patches": patches,
    }
    items = []
    for patch in patches:
        fixture = patch["fixture"]
        presentation_pass = (patch["oracle"]["status"] == "pass" and patch["oracle"]["all_corruptions_rejected"]
                             and patch["oracle"]["corruption_cases"] >= 8 and patch["oracle"]["unchanged_screens_checked"] > 0)
        latency_pass = (patch["latency"]["complete"] and patch["latency"]["navigation_target_pass"]
                        and patch["latency"]["opening_target_pass"] and patch["latency"]["quit_target_pass"])
        memory_pass = patch["memory"]["complete"] and patch["memory"]["memory_target_pass"] and patch["memory"]["quit_target_pass"]
        items.append(item(f"interior-{fixture}", f"{fixture}: independent interior screens",
                          "pass" if presentation_pass else "fail",
                          f"{patch['rehearsal']['steps']} real-PTY states recorded; {patch['oracle']['unchanged_screens_checked']} unchanged screens checked. Corruption controls: {patch['oracle']['corruption_cases']}; all rejected={patch['oracle']['all_corruptions_rejected']}.",
                          f"{patch['rehearsal']['trace']}; {patch['oracle']['artifact']}"))
        items.append(item(f"latency-{fixture}", f"{fixture}: latency in supplied run",
                          "pass" if latency_pass else "fail",
                          f"Requested five fresh-process repetitions; full series complete={patch['latency']['complete']}. Opening, category distributions, and clean exit are retained; navigation guard={'pass' if patch['latency']['navigation_target_pass'] else 'fail'}.",
                          patch["latency"]["summary"], "code"))
        items.append(item(f"memory-{fixture}", f"{fixture}: sustained memory",
                          "pass" if memory_pass else "fail",
                          f"Requested three tours; full series complete={patch['memory']['complete']}. Settled RSS {patch['memory']['steady_rss_mib']} MiB; OS peak {patch['memory']['os_peak_rss_mib']} MiB; target={'pass' if patch['memory']['memory_target_pass'] else 'fail'}.",
                          patch["memory"]["summary"], "code"))
    items.extend([
        item("source-digests", "Complete supplied-source digests", "skipped",
             "Unverified here: this generator does not consume the source-check results. Consult separately retained checks bound to the measured executable.",
             "No source-check evidence was supplied to this generator."),
        item("engineering-checks", "Build and real-process regression checks", "skipped",
             "Unverified here: this generator does not run or consume process tests, formatting, or Clippy. Consult separately retained checks for the measured source and executable.",
             "No engineering-check evidence was supplied to this generator."),
        item("ghostty-pixels", "Actual Ghostty pixel and gesture acceptance", "skipped",
             "Not measured here. PTY cell captures do not establish Ghostty paint timing, pixel parity, or trackpad feel; no GUI automation was used.",
             "/Users/ramos/oss/native-diff/CURRENT_STATE.md", "mixed"),
    ])
    report["items"] = items
    (run / "aggregate.json").write_text(json.dumps({"schema_version": 1, "run_dir": str(run), "patches": patches}, indent=2) + "\n")
    report.pop("patches")
    args.output.write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
