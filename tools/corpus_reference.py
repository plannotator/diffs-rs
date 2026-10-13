"""Independent source/terminal contracts; no viewer parser or pairing code."""

from dataclasses import dataclass
from bisect import bisect_left
from functools import cached_property, lru_cache
import io
import json
from pathlib import Path
import re
import unicodedata

from patch_reference import HUNK, MARKER
from trace_reference import digest, metadata, safe
from sidebar_reference import SidebarReference, cell_text, diff_geometry, diff_panes
import regex
from wcwidth import wcswidth

ROOT = Path(__file__).resolve().parents[1]


@dataclass
class Source:
    id: int
    hunk: int
    sign: str
    old: int | None
    new: int | None
    text: str
    no_newline: bool = False


class File:
    def __init__(self, raw, ref):
        self.meta = metadata(raw, ref)
        self.lines = []
        self.headers = []
        self.hunks = []
        self.ranges = ref["hunks"]
        old = new = 0
        hunk = -1
        previous_physical = None
        for physical in io.BytesIO(raw):
            if physical.rstrip(b"\n") == MARKER:
                self.lines[-1].no_newline = True
                self.lines[-1].text = safe(previous_physical[1:].removesuffix(b"\n"))
                continue
            match = HUNK.match(physical)
            if match:
                hunk += 1
                old, new = int(match[1]), int(match[3])
                self.headers.append(safe(physical.rstrip(b"\n"), True))
                self.hunks.append([])
            elif hunk >= 0:
                sign = chr(physical[0])
                if sign not in " +-":
                    raise ValueError("Unexpected source in independently decoded file")
                data = physical[1:]
                text = safe(data[:-1].removesuffix(b"\r") if data.endswith(b"\n") else data)
                line = Source(ref["source_start"] + len(self.lines), hunk, sign,
                              old if sign != "+" else None, new if sign != "-" else None, text)
                self.lines.append(line)
                self.hunks[-1].append(line)
                previous_physical = physical
                old += sign != "+"
                new += sign != "-"
        if len(self.lines) != ref["source_end"] - ref["source_start"]:
            raise ValueError("Independent file source count differs")
        self.by_side = {True: {s.old: s for s in self.lines if s.old is not None},
                        False: {s.new: s for s in self.lines if s.new is not None}}
        self.by_id = {s.id: s for s in self.lines}

    def gap_before(self, hunk):
        ends = []
        for side in ("old", "new"):
            prior = next((h for h in reversed(self.ranges[:hunk]) if h[side + "_count"]), None)
            ends.append(prior[side + "_start"] + prior[side + "_count"] if prior else 1)
        current = self.ranges[hunk]
        return current["old_start"] > ends[0] or current["new_start"] > ends[1]


class Corpus:
    def __init__(self, fixture):
        self.fixture = fixture
        self.path = ROOT / fixture["local_path"]
        self.ref_path = ROOT / f"evidence/large-diff-baseline/{fixture['id'].split('-')[0]}.reference.json"
        self.ref = json.loads(self.ref_path.read_text())
        if digest(self.path) != fixture["sha256"] or self.ref["sha256"] != fixture["sha256"]:
            raise ValueError("Pinned patch identity differs")
        self.count = len(self.ref["files"])

    @cached_property
    def sidebar(self):
        return SidebarReference(self)

    @lru_cache(maxsize=4)
    def file(self, index):
        ref = self.ref["files"][index]
        with self.path.open("rb") as stream:
            stream.seek(ref["offset"])
            return File(stream.read(ref["bytes"]), ref)

    @lru_cache(maxsize=256)
    def meta(self, index):
        ref = self.ref["files"][index]
        length = ref["hunks"][0]["offset"] - ref["offset"] if ref["hunks"] else ref["bytes"]
        with self.path.open("rb") as stream:
            stream.seek(ref["offset"])
            return metadata(stream.read(length), ref)

    def source(self, source_id):
        lo, hi = 0, self.count
        while lo < hi:
            mid = (lo + hi) // 2
            if self.ref["files"][mid]["source_end"] <= source_id:
                lo = mid + 1
            else:
                hi = mid
        return lo, self.file(lo).by_id[source_id]

    def targets(self):
        h = self.ref["largest_hunk"]
        largest = self.ref["files"][self.ref["largest_file"]]
        candidates = [
            ("hunk-mid", h["source_start"] + h["source_records"] // 2),
            ("hunk-end", h["source_start"] + h["source_records"] - 1),
            ("file-mid", (largest["source_start"] + largest["source_end"]) // 2),
            ("longest", self.ref["longest_line"]["source_ordinal"]),
        ]
        return [{"name": name, "source": source, "file": self.source(source)[0]}
                for name, source in candidates]


@lru_cache(maxsize=512)
def glyphs(text):
    result = []
    byte = column = 0
    for cluster in regex.findall(r"\X", text):
        width = 4 - column % 4 if cluster == "\t" else max(1, wcswidth(cluster))
        result.append((byte, column, cluster, width))
        byte += len(cluster.encode())
        column += width
    return result


@lru_cache(maxsize=512)
def starts(text, width):
    result, used = [0], 0
    for _, column, _, size in glyphs(text):
        if used and used + size > width:
            result.append(column)
            used = 0
        used += size
    return result


def segment(text, width, offset=0):
    cells = [" "] * width
    positions = []
    geometry = glyphs(text)
    first = bisect_left(geometry, offset, key=lambda glyph: glyph[1])
    for index in range(first, len(geometry)):
        byte, column, cluster, size = geometry[index]
        x = column - offset
        if x + size > width:
            break
        cells[x] = " " if cluster == "\t" else unicodedata.normalize("NFC", cluster)
        for n in range(1, size):
            cells[x + n] = " " if cluster == "\t" else ""
        positions.extend((x + n, byte) for n in range(size if cluster == "\t" else 1))
    return "".join(cells), positions


def header(meta, width):
    path = f" {meta['old_path']} → {meta['path']}" if meta["old_path"] != meta["path"] else f" {meta['path']}"
    summary = f" {meta['status']}   +{meta['additions']} −{meta['deletions']} "
    tail = min(width, wcswidth(summary))
    return segment(path, width - tail)[0] + segment(summary, tail)[0]


def snapshot(screen):
    styles = []
    for y in range(screen.lines):
        runs = []
        for x in range(screen.columns):
            c = screen.buffer[y][x]
            style = [c.fg, c.bg, c.bold, c.italics, c.underscore, c.reverse]
            if runs and runs[-1][2:] == style:
                runs[-1][1] = x + 1
            else:
                runs.append([x, x + 1, *style])
        styles.append(runs)
    return {"width": screen.columns, "height": screen.lines,
            "rows": screen.display, "styles": styles}


def restore(capture):
    """Decode a captured cell artifact; this does not execute or simulate the viewer."""
    import pyte
    screen = pyte.Screen(capture["width"], capture["height"])
    stream = pyte.Stream(screen)
    for y, row in enumerate(capture["rows"]):
        stream.feed(f"\x1b[{y + 1};1H" + row)
    for y, runs in enumerate(capture["styles"]):
        for start, end, fg, bg, bold, italics, underscore, reverse in runs:
            for x in range(start, end):
                screen.buffer[y][x] = screen.buffer[y][x]._replace(
                    fg=fg, bg=bg, bold=bold, italics=italics, underscore=underscore, reverse=reverse)
    if snapshot(screen) != capture:
        raise ValueError("Captured cell artifact did not round-trip")
    return screen


def check_screen(screen, corpus, expected_file, split=True, wrapped=False, horizontal=0,
                 anchor=None, require_anchor=False, require_file=False,
                 anchor_old=None, anchor_byte=None, sidebar=False, coverage=None,
                 horizontal_bound=False):
    """Check original-source semantics, not candidate-generated row expectations."""
    origin, width = diff_geometry(screen.columns, screen.lines, sidebar)
    display = [cell_text(screen, y, origin, width) for y in range(screen.lines)]
    if ("SPLIT" if split else "UNIFIED") not in screen.display[0]:
        raise AssertionError("Incorrect layout")
    candidates = range(max(0, expected_file - 48), min(corpus.count, expected_file + 49))
    headers = {}
    for f in candidates:
        headers.setdefault(header(corpus.meta(f), width), []).append(f)
    current = None
    previous = {True: None, False: None}
    unified_previous = None
    current_hunk = None
    announced = False
    pending_gap = False
    metadata_seen = False
    markers = set()
    records, active = [], []
    visible_files = set()
    left = (width - 1) // 2
    panes = diff_panes(screen.columns, screen.lines, split, sidebar)
    cursor_cells = [(y, pane) for y in range(2, screen.lines - 1)
                    for _, x, pane in panes if screen.buffer[y][x].data == "▎"]

    def parts(line, pane):
        gutter = file.meta["digits"] + 3 if split else 2 * file.meta["digits"] + 4
        return starts(line.text, pane - gutter) if wrapped else [horizontal]

    def complete(prior, side, pane, require_marker=True):
        if prior is None:
            return
        line, part = prior
        if part != len(parts(line, pane)) - 1:
            raise AssertionError("Missing wrapped source suffix")
        if require_marker and line.no_newline and (side, line.id) not in markers:
            raise AssertionError("Missing no-newline marker")

    def finish_hunk():
        if current_hunk is None:
            return
        for side, _, pane in panes:
            prior = previous[side] if split else unified_previous
            complete(prior, side if split else (prior[0].sign == "-" if prior else None), pane)
            source = [s for s in file.hunks[current_hunk]
                      if not split or (s.old if side else s.new) is not None]
            if prior and source and prior[0].id != source[-1].id:
                raise AssertionError("Missing source before hunk boundary")
            if announced and source and prior is None:
                raise AssertionError("Missing entire source side")

    for y in range(1, screen.lines - 1):
        row = display[y]
        if row in headers:
            choices = headers[row]
            if current is None:
                if require_file and expected_file not in choices:
                    raise AssertionError("Wrong requested file header")
                if len(choices) > 1 and not require_file:
                    raise AssertionError("Ambiguous truncated file header")
                selected = expected_file if require_file else choices[0]
            else:
                finish_hunk()
                if pending_gap or (file.hunks and current_hunk != len(file.hunks) - 1):
                    raise AssertionError("Missing hunk before file boundary")
                if not file.hunks and not metadata_seen:
                    raise AssertionError("Missing metadata card")
                selected = current + 1
                if selected not in choices:
                    raise AssertionError("Missing, duplicate, or reordered file header")
            current = selected
            visible_files.add(selected)
            previous = {True: None, False: None}
            unified_previous = None
            current_hunk = None
            announced = pending_gap = metadata_seen = False
            markers.clear()
            continue
        if current is None:
            raise AssertionError(f"Missing independently expected file header at row {y}: {row!r}")
        file = corpus.file(current)
        if row.lstrip().startswith("@@"):
            matches = [i for i, h in enumerate(file.headers)
                       if row == " " + segment(h, width - 2)[0] + " "]
            if len(matches) != 1:
                raise AssertionError("Wrong or ambiguous hunk header")
            index = matches[0]
            if current_hunk is not None:
                finish_hunk()
                if index != current_hunk + 1:
                    raise AssertionError("Missing, duplicate, or reordered hunk header")
                if file.gap_before(index) != pending_gap:
                    raise AssertionError("Missing or unexpected context-gap marker")
            elif pending_gap and not file.gap_before(index):
                raise AssertionError("Unexpected context-gap marker")
            current_hunk = index
            announced = True
            pending_gap = False
            previous = {True: None, False: None}
            unified_previous = None
            continue
        if row.lstrip().startswith("···  Context not included"):
            if row != "  " + segment("···  Context not included in patch", width - 4)[0] + "  ":
                raise AssertionError("Wrong missing-context marker")
            if pending_gap:
                raise AssertionError("Repeated missing-context marker")
            finish_hunk()
            pending_gap = True
            continue
        if "no textual hunks" in row and not file.lines:
            m = file.meta
            wanted = f" {m['status']}  ·  mode {m['old_mode']} → {m['new_mode']}  ·  no textual hunks"
            if row != " " + segment(wanted, width - 2)[0] + " ":
                raise AssertionError("Wrong metadata card")
            if metadata_seen:
                raise AssertionError("Repeated metadata card")
            metadata_seen = True
            continue
        if not row.strip():
            if any(line.strip() for line in display[y:screen.lines - 1]):
                raise AssertionError("Blank row interrupts the source view")
            if current != corpus.count - 1 or pending_gap:
                raise AssertionError("Blank tail precedes the end of the patch")
            if file.hunks:
                finish_hunk()
                if current_hunk != len(file.hunks) - 1:
                    raise AssertionError("Blank tail hides source hunks")
            elif not metadata_seen:
                raise AssertionError("Blank tail hides metadata")
            break
        if pending_gap:
            raise AssertionError("Context-gap marker is not followed by a hunk header")
        pane_texts = ["".join(screen.buffer[y][column].data for column in range(x, x + pane))
                      for _, x, pane in panes]
        marker_row = any(text.startswith(" No newline") or
                         (not split and text.startswith("  No newline")) for text in pane_texts)
        if split and not marker_row:
            pending = [(side, prior, pane) for side, _, pane in panes
                       if (prior := previous[side]) is not None]
            all_complete = all(part == len(parts(line, pane)) - 1
                               for _, (line, part), pane in pending)
            if all_complete and any(line.no_newline and (side, line.id) not in markers
                                    for side, (line, _), _ in pending):
                raise AssertionError("Missing no-newline row before next source")
        if split and screen.buffer[y][origin + left].data != (" " if marker_row else "│"):
            raise AssertionError("Missing split separator")
        same_row = []
        for (side, x, pane), text in zip(panes, pane_texts):
            if not text.strip():
                continue
            if marker_row:
                note = " No newline at end of file"
                wanted = segment(note, pane)[0] if split else " " + segment(note, width - 2)[0] + " "
                if text != wanted:
                    raise AssertionError("Wrong no-newline marker")
                prior = previous[side] if split else unified_previous
                if prior is None and y == 2 and not announced:
                    possible = [s for s in file.lines if s.no_newline and
                                (not split or (s.old if side else s.new) is not None)]
                    if len(possible) == 1:
                        line = possible[0]
                        prior = (line, len(parts(line, pane)) - 1)
                        if current_hunk is not None and current_hunk != line.hunk:
                            raise AssertionError("No-newline sides belong to different hunks")
                        current_hunk = line.hunk
                        if split:
                            previous[side] = prior
                        else:
                            unified_previous = prior
                            previous[line.sign == "-"] = prior
                if prior is None or not prior[0].no_newline:
                    raise AssertionError("Unowned no-newline marker")
                actual_side = side if split else prior[0].sign == "-"
                complete(prior, actual_side, pane, require_marker=False)
                marker = (actual_side, prior[0].id)
                if marker in markers:
                    raise AssertionError("Repeated no-newline marker")
                markers.add(marker)
                continue
            digits = file.meta["digits"]
            gutter = digits + 3 if split else 2 * digits + 4
            mark = text[0]
            if mark not in (" ", "▎"):
                raise AssertionError(f"Invalid reading cursor at {current}:{y}: {text!r}")
            sign = text[gutter - 2]
            if sign == "↪":
                if not wrapped:
                    raise AssertionError("Unexpected wrapped continuation")
                prior = previous[side] if split else unified_previous
                if prior is None:
                    possible = file.lines
                    if anchor is not None:
                        possible = [s for s in possible if abs(s.id - anchor) < screen.lines + 1]
                    matches = []
                    for line in possible:
                        if split and (line.old if side else line.new) is None:
                            continue
                        expected_part = None
                        if anchor_byte is not None and line.id == anchor and len(cursor_cells) == 1:
                            cursor_y, cursor_pane = cursor_cells[0]
                            column = next((column for byte, column, _, _ in glyphs(line.text)
                                           if byte == anchor_byte), None)
                            if column is not None:
                                anchor_part = max(i for i, start in enumerate(parts(line, cursor_pane))
                                                  if start <= column)
                                expected_part = anchor_part - (cursor_y - y)
                        for part, offset in enumerate(starts(line.text, pane - gutter)[1:], 1):
                            if expected_part is not None and part != expected_part:
                                continue
                            if segment(line.text, pane - gutter, offset)[0] == text[gutter:]:
                                matches.append((line, part))
                    if len(matches) != 1:
                        raise AssertionError(f"Ambiguous off-screen wrapped source: {len(matches)}")
                    line, part = matches[0]
                else:
                    line, part = prior[0], prior[1] + 1
                if not split:
                    side = line.sign == "-"
                wanted_gutter = mark + " " * (gutter - 3) + "↪ "
            else:
                old = int(text[1:1 + digits]) if text[1:1 + digits].strip() else None
                if split:
                    number = old
                else:
                    new = int(text[digits + 2:digits * 2 + 2]) if text[digits + 2:digits * 2 + 2].strip() else None
                    side = sign == "−"
                    number = old if side else new
                line = file.by_side[side].get(number)
                if line is None:
                    raise AssertionError(f"Unknown source number {current}:{side}:{number}")
                if not split and (old, new) != (line.old, line.new):
                    raise AssertionError("Wrong unified side numbers")
                if sign != ("−" if line.sign == "-" else line.sign):
                    raise AssertionError("Wrong source sign")
                part = 0
                prior = previous[side]
                complete(prior if split else unified_previous,
                         side if split else (unified_previous[0].sign == "-" if unified_previous else None), pane)
                if prior and prior[0].hunk == line.hunk and (line.old if side else line.new) != (prior[0].old if side else prior[0].new) + 1:
                    raise AssertionError("Missing, duplicate, or reordered source row")
                if not split and unified_previous and line.id != unified_previous[0].id + 1:
                    raise AssertionError("Wrong unified source sequence")
                if announced and prior is None:
                    first = next((s for s in file.hunks[current_hunk]
                                  if (s.old if side else s.new) is not None), None)
                    if first is None or first.id != line.id:
                        raise AssertionError("Missing first source after hunk header")
                if announced and not split and unified_previous is None and line.id != file.hunks[current_hunk][0].id:
                    raise AssertionError("Wrong first unified source after hunk header")
                number_text = lambda value: " " * digits if value is None else f"{value:>{digits}}"
                wanted_gutter = (mark + number_text(line.old if side else line.new) + sign + " "
                                 if split else mark + number_text(line.old) + " " + number_text(line.new) + sign + " ")
            if text[:gutter] != wanted_gutter:
                raise AssertionError("Wrong source gutter")
            if current_hunk is None:
                current_hunk = line.hunk
            elif current_hunk != line.hunk:
                raise AssertionError("Source belongs to a different hunk")
            offsets = parts(line, pane)
            if part >= len(offsets):
                raise AssertionError("Wrapped source extends beyond original text")
            wanted, positions = segment(line.text, pane - gutter, offsets[part])
            if not (horizontal_bound and not wrapped) and text[gutter:] != wanted:
                raise AssertionError(f"Wrong source text {current}:{line.id}:{part}: {text[gutter:]!r} != {wanted!r}")
            previous[side] = (line, part)
            if not split:
                unified_previous = (line, part)
                if line.sign == " ":
                    previous[not side] = (line, part)
            record = {"file": current, "id": line.id, "old": side, "part": part,
                      "y": y, "x": x + gutter, "positions": positions}
            records.append(record)
            same_row.append((line, part, side))
            if mark == "▎":
                active.append(record)
        if not same_row and not marker_row:
            raise AssertionError("Empty source row")
        if split and marker_row:
            if any(prior and prior[0].no_newline and (side, prior[0].id) not in markers
                   for side, prior in previous.items()):
                raise AssertionError("Missing no-newline marker side")
        if split and len(same_row) == 2 and same_row[0][1] != same_row[1][1]:
            raise AssertionError("Split wrap parts are not aligned")
        if split and any(s.sign == " " for s, _, _ in same_row):
            line, part, side = next(r for r in same_row if r[0].sign == " ")
            if any(s.id != line.id or p != part for s, p, _ in same_row):
                raise AssertionError("Shared context is not aligned")
            if len(same_row) == 1:
                other_pane = left if not side else width - left - 1
                if part < len(parts(line, other_pane)):
                    raise AssertionError("Missing shared-context pane")
    if horizontal_bound and not wrapped:
        limit = 0
        for record in records:
            line = corpus.file(record["file"]).by_id[record["id"]]
            geometry = glyphs(line.text)
            columns = geometry[-1][1] + geometry[-1][3] if geometry else 0
            _, x, pane = next(p for p in panes if not split or p[0] == record["old"])
            limit = max(limit, columns - (x + pane - record["x"]))
        offset = min(horizontal, limit)
        for record in records:
            line = corpus.file(record["file"]).by_id[record["id"]]
            _, x, pane = next(p for p in panes if not split or p[0] == record["old"])
            available = x + pane - record["x"]
            wanted, record["positions"] = segment(line.text, available, offset)
            if cell_text(screen, record["y"], record["x"], available) != wanted:
                raise AssertionError("Wrong independently bounded source text")
        if coverage is not None:
            coverage["horizontal"] = {"requested": horizontal, "limit": limit, "expected": offset}
    if require_anchor and not any(r["id"] == anchor for r in active):
        raise AssertionError(f"Wrong active source: expected {anchor}, found {[r['id'] for r in active]}")
    if len(active) > 1:
        raise AssertionError("Multiple active reading cursors")
    if require_anchor and anchor_old is not None and split and active[0]["old"] != anchor_old:
        raise AssertionError("Wrong active source side")
    if require_anchor and anchor_byte is not None:
        line = corpus.file(active[0]["file"]).by_id[anchor]
        if anchor_byte not in {byte for _, byte in active[0]["positions"]} and (line.text or anchor_byte != 0):
            raise AssertionError("Active text position is not visible")
    if origin:
        if active:
            selected_files = {active[0]["file"]}
            basis = "active-source"
        elif require_file:
            selected_files = {expected_file}
            basis = "required-file"
        else:
            selected_files = visible_files
            basis = "single-visible-file" if len(visible_files) == 1 else "visible-file-set"
        sidebar_check = corpus.sidebar.check(screen, selected_files)
        sidebar_check["basis"] = basis
        if coverage is not None:
            coverage["sidebar"] = sidebar_check
    return records
