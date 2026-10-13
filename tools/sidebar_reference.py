"""Independent default-sidebar expectations from patch paths and hunk counts."""

from collections import Counter, defaultdict
from dataclasses import dataclass
import unicodedata

import regex
from wcwidth import wcswidth

from trace_reference import metadata, safe


def diff_geometry(columns, lines, sidebar=False):
    origin = 33 if sidebar and columns >= 97 and lines >= 10 else 0
    return origin, columns - origin


def diff_panes(columns, lines, split, sidebar=False):
    origin, width = diff_geometry(columns, lines, sidebar)
    left = (width - 1) // 2
    return ([(True, origin, left), (False, origin + left + 1, width - left - 1)]
            if split else [(None, origin, width)])


def cell_text(screen, y, start, width):
    return "".join(screen.buffer[y][x].data for x in range(start, start + width))


def diff_region(screen, sidebar=False):
    origin, width = diff_geometry(screen.columns, screen.lines, sidebar)
    rows = [cell_text(screen, y, origin, width) for y in range(1, screen.lines - 1)]
    styles = []
    for y in range(1, screen.lines - 1):
        cells = (screen.buffer[y][x] for x in range(origin, origin + width))
        styles.append([(c.fg, c.bg, c.bold, c.italics, c.underscore, c.reverse) for c in cells])
    return rows, styles


def clipped(text, width):
    text = unicodedata.normalize("NFC", text)
    if wcswidth(text) <= width:
        return text
    result, used = [], 0
    for cluster in regex.findall(r"\X", text):
        used += wcswidth(cluster)
        if used >= width:
            break
        result.append(cluster)
    return "".join(result) + ("…" if width else "")


def padded(text, width):
    return text + " " * (width - wcswidth(text))


@dataclass
class TreeRow:
    name: bytes
    depth: int
    summary: str
    file: int | None = None
    occurrence: int = 0

    def text(self, current):
        name = "\\\\".join(safe(part, label=True) for part in self.name.split(b"\\"))
        if self.file is None:
            name = "▾ " + name + "/"
        indent = min(self.depth * 2, 6)
        prefix = (" *" if self.file == current else "  ") + " " * indent
        if self.depth * 2 > indent:
            prefix += "…"
        suffix = f" [{self.occurrence}]" if self.occurrence else ""
        summary_width = min(wcswidth(self.summary), 16)
        name_width = 32 - summary_width - 3
        label = clipped(prefix + name, max(0, name_width - wcswidth(suffix))) + suffix
        return (padded(clipped(label, name_width), name_width) + "  "
                + padded(clipped(self.summary, summary_width), summary_width) + " ")


class SidebarReference:
    """The corpus trace never focuses, collapses, or manually scrolls the tree."""

    def __init__(self, corpus):
        entries = []
        with corpus.path.open("rb") as stream:
            for index, ref in enumerate(corpus.ref["files"]):
                length = ref["hunks"][0]["offset"] - ref["offset"] if ref["hunks"] else ref["bytes"]
                stream.seek(ref["offset"])
                meta = metadata(stream.read(length), ref, raw_paths=True)
                entries.append((index, tuple(meta["path"].split(b"/")), meta))
        duplicates = Counter(path for _, path, _ in entries)
        occurrences = Counter()
        children = defaultdict(list)
        totals = defaultdict(lambda: [0, 0, 0, 0])
        for index, path, meta in entries:
            for depth in range(1, len(path)):
                folder = path[:depth]
                if folder not in totals:
                    children[folder[:-1]].append((False, folder[-1], -1, folder))
                value = totals[folder]
                for i, amount in enumerate((1, meta["additions"], meta["deletions"],
                                             int(meta["status"] == "Binary change"))):
                    value[i] += amount
            occurrences[path] += 1
            summary = ("B binary" if meta["status"] == "Binary change" else
                       f"{meta['status'][0]} +{meta['additions']} −{meta['deletions']}")
            row = TreeRow(path[-1], len(path) - 1, summary, index,
                          occurrences[path] if duplicates[path] > 1 else 0)
            children[path[:-1]].append((True, path[-1], index, row))
        self.rows, self.positions = [], {}
        stack = list(reversed(sorted(children[()])))
        while stack:
            leaf, name, index, value = stack.pop()
            if leaf:
                self.positions[index] = len(self.rows)
                self.rows.append(value)
            else:
                files, additions, deletions, binary = totals[value]
                self.rows.append(TreeRow(name, len(value) - 1,
                    f"{files} +{additions} −{deletions}" + (" B" if binary else "")))
                stack.extend(reversed(sorted(children[value])))
        self.count = len(entries)

    def check(self, screen, candidates):
        if cell_text(screen, 1, 0, 32) != padded(f" Files · {self.count}", 32):
            raise AssertionError("Wrong sidebar heading or incomplete index")
        if any(screen.buffer[y][32].data != "│" for y in range(1, screen.lines - 1)):
            raise AssertionError("Wrong sidebar divider")
        marked = [y for y in range(2, screen.lines - 1) if screen.buffer[y][1].data == "*"]
        if len(marked) != 1:
            raise AssertionError("Sidebar needs exactly one visible current file")
        page = screen.lines - 3
        actual = [cell_text(screen, offset + 2, 0, 32) for offset in range(page)]
        matches = []
        for current in sorted(candidates):
            top = self.positions[current] - (marked[0] - 2)
            if not 0 <= top <= max(0, len(self.rows) - page):
                continue
            expected = [self.rows[index].text(current) if index < len(self.rows) else " " * 32
                        for index in range(top, top + page)]
            if actual == expected:
                matches.append(current)
        if len(matches) != 1:
            raise AssertionError(f"Wrong or ambiguous sidebar rows for current-file candidates {sorted(candidates)}")
        return {"current_identity": "exact" if len(candidates) == 1 else "visible-file-only",
                "candidates": sorted(candidates), "displayed_current": matches[0]}

    def check_colors(self, screen, palette, current):
        marked = next(y for y in range(2, screen.lines - 1) if screen.buffer[y][1].data == "*")
        top = self.positions[current] - (marked - 2)
        for y in range(1, screen.lines - 1):
            index = top + y - 2
            item = self.rows[index] if y > 1 and index < len(self.rows) else None
            name_end = 32 - min(wcswidth(item.summary), 16) - 3 if item else 32
            for x in range(33):
                cell = screen.buffer[y][x]
                if cell.bg != palette["chrome"]:
                    raise AssertionError("Wrong sidebar background")
                if not cell.data.strip():
                    continue
                foreground = "dim" if x == 32 or (y > 1 and x >= name_end) else "fg"
                bold = x < 32 and (y == 1 or (x < name_end and item and item.file == current))
                if cell.fg != palette[foreground] or bool(cell.bold) != bool(bold):
                    raise AssertionError("Wrong sidebar foreground or current-file emphasis")
