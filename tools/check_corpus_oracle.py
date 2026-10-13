"""Challenge source checks by corrupting captured real-process terminal artifacts."""

import argparse
import copy
import gzip
import json
from pathlib import Path

from corpus_reference import Corpus, check_screen, digest, glyphs, restore, segment, snapshot
from corpus_acceptance import check_styles
from sidebar_reference import cell_text, diff_geometry, diff_panes


def check(path):
    run = json.loads(gzip.decompress(path.read_bytes()))
    corpus = Corpus(run["fixture"])
    steps = [run["opening"], *run["steps"]]
    records, coverages = [], []
    for step in steps:
        coverage = {}
        records.append(check_screen(restore(step["expected"]), corpus, **step["state"], coverage=coverage))
        if "coverage" in step and step["coverage"] != coverage:
            raise AssertionError(f"Recorded checker coverage differs: {step['label']}")
        coverages.append(coverage)
    for step, actual in zip(steps, records):
        if json.loads(json.dumps(actual)) != step["records"]:
            raise AssertionError(f"Captured source map differs from independent decode: {step['label']}")
    cases = []

    def reject(name, index, corrupt, state=None):
        screen = restore(steps[index]["expected"])
        before = snapshot(screen)
        corrupt(screen)
        if state is None and snapshot(screen) == before:
            raise AssertionError(f"Corruption did not change cells: {name}")
        try:
            check_screen(screen, corpus, **(state or steps[index]["state"]))
        except (AssertionError, ValueError, KeyError) as error:
            cases.append({"name": name, "step": steps[index]["label"],
                          "status": "rejected", "reason": str(error)})
        else:
            cases.append({"name": name, "step": steps[index]["label"], "status": "accepted-corruption"})

    def cell(screen, x, y, value):
        screen.buffer[y][x] = screen.buffer[y][x]._replace(data=value)

    def blank(screen, y, origin=0):
        for x in range(origin, screen.columns):
            cell(screen, x, y, " ")

    def row(screen, y, text, origin=0):
        import pyte
        blank(screen, y, origin)
        pyte.Stream(screen).feed(f"\x1b[{y + 1};{origin + 1}H" + text)

    first = next(i for i, items in enumerate(records) if items)
    source = records[first][0]
    first_screen = steps[first]["expected"]
    first_origin, _ = diff_geometry(first_screen["width"], first_screen["height"],
                                    steps[first]["state"].get("sidebar", False))
    glyph = next((r, x) for r in records[first] for x, _ in r["positions"])
    reject("source-glyph", first,
           lambda screen: cell(screen, glyph[0]["x"] + glyph[1], glyph[0]["y"], "§"))
    digits = corpus.meta(source["file"])["digits"]
    gutter = digits + 3 if steps[first]["state"]["split"] else 2 * digits + 4
    reject("source-number", first,
           lambda screen: cell(screen, source["x"] - gutter + digits, source["y"],
                               "8" if screen.buffer[source["y"]][source["x"] - gutter + digits].data == "9" else "9"))
    reject("source-sign", first,
           lambda screen: cell(screen, source["x"] - 2, source["y"], "!"))
    reject("blank-source-row", first, lambda screen: blank(screen, source["y"], first_origin))
    last = records[first][-1]
    reject("erased-visible-tail", first,
           lambda screen: [blank(screen, y, first_origin) for y in range(last["y"], screen.lines - 1)])
    reject("blank-body", first,
           lambda screen: [blank(screen, y, first_origin) for y in range(2, screen.lines - 1)])

    for index, step in enumerate(steps):
        capture = step["expected"]
        origin, _ = diff_geometry(capture["width"], capture["height"], step["state"].get("sidebar", False))
        if step["state"].get("sidebar") and not origin and records[index]:
            r, x = next((r, x) for r in records[index] for x, _ in r["positions"])
            reject("hidden-sidebar-source-glyph", index,
                   lambda screen: cell(screen, r["x"] + x, r["y"], "§"))
            break

    indexed = next(((i, [r for r in items if r["part"] == 0])
                    for i, items in enumerate(records) if len(items) >= 2), None)
    if indexed and len(indexed[1]) >= 2:
        index, sources = indexed
        a, b = sources[:2]
        def duplicate_cursor(screen):
            for r in (a, b):
                n = corpus.meta(r["file"])["digits"]
                g = n + 3 if steps[index]["state"]["split"] else 2 * n + 4
                cell(screen, r["x"] - g, r["y"], "▎")
        reject("duplicate-source-cursor", index, duplicate_cursor)

    for index, items in enumerate(records):
        if not steps[index]["state"]["split"]:
            continue
        shared = next((r for r in items if corpus.file(r["file"]).by_id[r["id"]].sign == " "
                       and r["part"] == 0), None)
        if shared:
            def erase_peer(screen):
                panes = diff_panes(screen.columns, screen.lines, True,
                                   steps[index]["state"].get("sidebar", False))
                _, lo, width = next(p for p in panes if p[0] != shared["old"])
                for x in range(lo, lo + width):
                    cell(screen, x, shared["y"], " ")
            reject("missing-context-side", index, erase_peer)
            break

    for index, items in enumerate(records):
        if steps[index]["state"]["split"]:
            continue
        pair = next(((a, b) for a, b in zip(items, items[1:])
                     if a["y"] + 1 == b["y"] and a["old"] and not b["old"] and
                     corpus.file(a["file"]).by_id[a["id"]].sign == "-" and
                     corpus.file(b["file"]).by_id[b["id"]].sign == "+"), None)
        if pair:
            a, b = pair
            def swap_rows(screen):
                origin, width = diff_geometry(screen.columns, screen.lines, steps[index]["state"].get("sidebar", False))
                first_row, second_row = (cell_text(screen, y, origin, width) for y in (a["y"], b["y"]))
                row(screen, a["y"], second_row, origin)
                row(screen, b["y"], first_row, origin)
            reject("unified-addition-before-deletion", index, swap_rows)
            break

    for index, items in enumerate(records):
        continuation = next((r for r in items if r["part"] > 0), None)
        if continuation:
            r = continuation
            n = corpus.meta(r["file"])["digits"]
            g = n + 3 if steps[index]["state"]["split"] else 2 * n + 4
            reject("numbered-wrapped-continuation", index,
                   lambda screen: cell(screen, r["x"] - g + 1, r["y"], "9"))
            break

    bounded = [i for i, step in enumerate(steps)
               if step["state"].get("horizontal_bound") and not step["state"]["wrapped"]]
    bounded.sort(key=lambda i: coverages[i].get("horizontal", {}).get("requested", 0)
                 > coverages[i].get("horizontal", {}).get("limit", 0), reverse=True)
    if bounded:
        index = bounded[0]
        r, x = next((r, x) for r in records[index] for x, _ in r["positions"])
        reject("bounded-horizontal-source-glyph", index,
               lambda screen: cell(screen, r["x"] + x, r["y"], "§"))

    for index, items in enumerate(records):
        pair = next(((a, b) for a, b in zip(items, items[1:]) if a["part"] > 0 and
                     a["y"] + 1 == b["y"] and a["id"] != b["id"]), None)
        if pair and not steps[index]["state"]["split"]:
            a, b = pair
            capture = steps[index]["expected"]
            origin, width = diff_geometry(capture["width"], capture["height"], steps[index]["state"].get("sidebar", False))
            reject("omitted-wrap-suffix", index,
                   lambda screen: row(screen, a["y"], cell_text(screen, b["y"], origin, width), origin))
            break

    for index, step in enumerate(steps):
        state = step["state"]
        if state.get("require_file") and corpus.count > 1:
            wrong = dict(state, expected_file=(state["expected_file"] + 1) % corpus.count)
            reject("wrong-required-file", index, lambda screen: None, wrong)
            break
    for index, step in enumerate(steps):
        state = step["state"]
        if state.get("require_anchor") and state["split"] and state.get("anchor_old") is not None:
            reject("wrong-active-side", index, lambda screen: None,
                   dict(state, anchor_old=not state["anchor_old"]))
            break
    for index, step in enumerate(steps):
        state = step["state"]
        if not state.get("require_anchor") or state.get("anchor_byte") is None:
            continue
        screen = restore(step["expected"])
        for r in records[index]:
            n = corpus.meta(r["file"])["digits"]
            g = n + 3 if state["split"] else 2 * n + 4
            if screen.buffer[r["y"]][r["x"] - g].data != "▎":
                continue
            visible = {byte for _, byte in r["positions"]}
            text = corpus.file(r["file"]).by_id[r["id"]].text
            wrong_byte = next((byte for byte, _, _, _ in glyphs(text) if byte not in visible), None)
            if wrong_byte is not None:
                reject("wrong-active-text-segment", index, lambda screen: None,
                       dict(state, anchor_byte=wrong_byte))
                break
        else:
            continue
        break

    style_path = path.parent / "sequential-reference.json.gz"
    if style_path.exists():
        answer = json.loads(gzip.decompress(style_path.read_bytes()))
        check_styles(corpus, steps, answer)
        r, x = glyph
        for name, field in [("wrong-syntax-foreground", 2), ("wrong-source-background", 3)]:
            corrupted = copy.deepcopy(steps[first])
            run = next(run for run in corrupted["expected"]["styles"][r["y"]]
                       if run[0] <= r["x"] + x < run[1])
            run[field] = "ff00ff"
            try:
                check_styles(corpus, [corrupted], answer)
            except AssertionError as error:
                cases.append({"name": name, "step": corrupted["label"], "status": "rejected", "reason": str(error)})
            else:
                cases.append({"name": name, "step": corrupted["label"], "status": "accepted-corruption"})
    for index, step in enumerate(steps):
        capture = step["expected"]
        origin, width = diff_geometry(capture["width"], capture["height"], step["state"].get("sidebar", False))
        screen = restore(capture)
        y = next((y for y in range(1, screen.lines - 1)
                  if any(cell_text(screen, y, x, pane).lstrip().startswith("No newline at end of file")
                         for _, x, pane in diff_panes(screen.columns, screen.lines, step["state"]["split"],
                                                     step["state"].get("sidebar", False)))), None)
        if y is not None:
            reject("no-newline-marker-text", index,
                   lambda screen: row(screen, y, cell_text(screen, y, origin, width).replace("No newline", "No garbage"), origin))
            break

    def replace_tree(screen, current, top):
        import pyte
        stream = pyte.Stream(screen)
        for y in range(2, screen.lines - 1):
            position = top + y - 2
            text = corpus.sidebar.rows[position].text(current) if position < len(corpus.sidebar.rows) else " " * 32
            for x in range(32):
                cell(screen, x, y, " ")
            stream.feed(f"\x1b[{y + 1};1H" + text)

    for index, step in enumerate(steps):
        capture = step["expected"]
        origin, width = diff_geometry(capture["width"], capture["height"], step["state"].get("sidebar", False))
        if origin and coverages[index].get("sidebar", {}).get("current_identity") == "exact":
            screen = restore(capture)
            marked = next(y for y in range(2, screen.lines - 1) if screen.buffer[y][1].data == "*")
            reject("sidebar-heading-count", index, lambda s: cell(s, 9, 1, "!"))
            reject("sidebar-divider", index, lambda s: cell(s, 32, 2, "!"))
            reject("sidebar-row-label", index, lambda s: cell(s, 3, marked, "!"))
            reject("sidebar-row-count", index, lambda s: cell(s, 29, marked, "!"))
            reject("sidebar-missing-current", index, lambda s: cell(s, 1, marked, " "))
            other = 2 if marked != 2 else 3
            reject("sidebar-duplicate-current", index, lambda s: cell(s, 1, other, "*"))
            reject("sidebar-unexpected-focus", index, lambda s: cell(s, 0, marked, ">"))
            if style_path.exists():
                for name, field in [("sidebar-foreground", 2), ("sidebar-background", 3)]:
                    corrupted = copy.deepcopy(step)
                    style = next(r for r in corrupted["expected"]["styles"][1] if r[0] <= 2 < r[1])
                    style[field] = "ff00ff"
                    try:
                        check_styles(corpus, [corrupted], answer)
                    except AssertionError as error:
                        cases.append({"name": name, "step": step["label"], "status": "rejected", "reason": str(error)})
                    else:
                        cases.append({"name": name, "step": step["label"], "status": "accepted-corruption"})
            break

    for identity, name in [("exact", "sidebar-wrong-current"),
                           ("visible-file-only", "sidebar-current-outside-visible-files")]:
        for index, coverage in enumerate(coverages):
            info = coverage.get("sidebar", {})
            if info.get("current_identity") != identity:
                continue
            screen = restore(steps[index]["expected"])
            marked = next(y for y in range(2, screen.lines - 1) if screen.buffer[y][1].data == "*")
            top = corpus.sidebar.positions[info["displayed_current"]] - (marked - 2)
            page = screen.lines - 3
            other = next((r.file for r in corpus.sidebar.rows[top:top + page]
                          if r.file is not None and r.file not in info["candidates"]), None)
            if other is None:
                continue
            reject(name, index, lambda s: replace_tree(s, other, top))
            break

    for index, step in enumerate(steps):
        file = corpus.file(records[index][0]["file"]) if records[index] else None
        if file is None or len(file.headers) < 2:
            continue
        capture = step["expected"]
        origin, width = diff_geometry(capture["width"], capture["height"], step["state"].get("sidebar", False))
        screen = restore(capture)
        y = next((y for y in range(1, screen.lines - 1) if cell_text(screen, y, origin, width).lstrip().startswith("@@")), None)
        if y is not None:
            different = next((h for h in file.headers
                              if " " + segment(h, width - 2)[0] + " " != cell_text(screen, y, origin, width)), None)
            if different:
                reject("wrong-existing-hunk-header", index,
                       lambda screen: row(screen, y, " " + segment(different, width - 2)[0] + " ", origin))
                break

    return {"artifact": str(path), "artifact_sha256": digest(path),
            "unchanged_screens_checked": len(steps), "cases": cases,
            "sidebar_coverage": {kind: sum(c.get("sidebar", {}).get("current_identity") == kind for c in coverages)
                                 for kind in ("exact", "visible-file-only")},
            "bounded_horizontal_views": len(bounded),
            "status": "pass" if all(c["status"] == "rejected" for c in cases) else "fail",
            "coverage": "Corrupted captured cells, not a simulated application. Cases absent from the artifact are not claimed."}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifact", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise FileExistsError(args.output)
    result = check(args.artifact.resolve())
    with args.output.open("x") as stream:
        json.dump(result, stream, indent=2)
        stream.write("\n")
    print(json.dumps(result))
    if result["status"] != "pass":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
