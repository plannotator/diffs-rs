"""Source-derived terminal expectations. Does not import or execute the viewer."""

import codecs
from dataclasses import dataclass
from functools import lru_cache
import hashlib
import io
from pathlib import Path
import re
import unicodedata

import regex
from wcwidth import wcswidth

from patch_reference import HUNK, MARKER


def digest(path):
    with Path(path).open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def safe(data, label=False):
    text = data.decode("utf-8", "surrogateescape")
    result = ""
    for char in text:
        n = ord(char)
        if 0xDC80 <= n <= 0xDCFF:
            result += f"\\x{n - 0xDC00:02x}"
        elif char in "\n\t":
            result += {"\n": "\\n", "\t": "\\t"}[char] if label else char
        elif n < 32 or 127 <= n <= 159 or 0x202A <= n <= 0x202E or 0x2066 <= n <= 0x2069:
            result += f"\\u{{{n:x}}}"
        else:
            result += char
    return result


def path_bytes(data, prefix=False):
    if data.startswith(b'"') and data.endswith(b'"'):
        data = codecs.escape_decode(data[1:-1])[0]
    if prefix and data.startswith((b"a/", b"b/")):
        data = data[2:]
    return data


def path_text(data, prefix=False):
    return safe(path_bytes(data, prefix), label=True)


def metadata(raw, reference, *, raw_paths=False):
    records = list(io.BytesIO(raw))
    header = records[0].rstrip(b"\n").removeprefix(b"diff --git ")
    split = re.match(rb'("(?:\\.|[^"\\])*"|a/.*?) ("(?:\\.|[^"\\])*"|b/.*)$', header)
    if not split:
        raise ValueError(f"Ambiguous file header: {header!r}")
    decode_path = path_bytes if raw_paths else path_text
    old, new = [decode_path(item, True) for item in split.groups()]
    status, old_mode, new_mode = "Modified", "0", "0"
    for record in records[1:]:
        record = record.rstrip(b"\n")
        if record.startswith(b"@@ "):
            break
        if record.startswith(b"index "):
            parts = record.split()
            if len(parts) == 3:
                old_mode = new_mode = parts[2].decode("ascii")
        elif record.startswith(b"old mode "):
            old_mode = record[9:].decode("ascii")
        elif record.startswith(b"new mode "):
            new_mode = record[9:].decode("ascii")
        elif record.startswith(b"new file mode "):
            status, new_mode = "Added", record[14:].decode("ascii")
        elif record.startswith(b"deleted file mode "):
            status, old_mode = "Deleted", record[18:].decode("ascii")
        elif record.startswith(b"rename from "):
            status, old = "Renamed", decode_path(record[12:])
        elif record.startswith(b"rename to "):
            new = decode_path(record[10:])
        elif record.startswith(b"copy from "):
            status, old = "Copied", decode_path(record[10:])
        elif record.startswith(b"copy to "):
            new = decode_path(record[8:])
        elif record.startswith((b"Binary files ", b"GIT binary patch")):
            status = "Binary change"
    if status == "Modified" and old_mode != "0" and new_mode != "0" and old_mode[:-3] != new_mode[:-3]:
        status = "Type changed"
    hunks = reference["hunks"]
    digits = max([3] + [len(str(h[s + "_start"] + h[s + "_count"] - 1))
                        for h in hunks for s in ["old", "new"] if h[s + "_count"]])
    return {
        "path": old if status == "Deleted" else new, "old_path": old, "status": status,
        "old_mode": old_mode, "new_mode": new_mode, "digits": digits,
        "additions": sum(h["source_records"] - h["old_count"] for h in hunks),
        "deletions": sum(h["source_records"] - h["new_count"] for h in hunks),
    }


@dataclass(slots=True)
class Line:
    sign: str
    old: int | None
    new: int | None
    data: bytes
    ordinal: int
    no_newline: bool = False

    def text(self):
        data = self.data
        if data.endswith(b"\n"):
            data = data[:-1].removesuffix(b"\r")
        return safe(data)


def pairing(left, right):
    count, extra = min(len(left), len(right)), abs(len(left) - len(right))
    if not count or not extra or count * (extra + 1) > 4096 or any(len(l.data) > 4096 for l in left + right):
        return 0
    a = [re.sub(rb"[\x09-\x0d ]", b"", l.data) for l in left]
    b = [re.sub(rb"[\x09-\x0d ]", b"", l.data) for l in right]

    def score(shift):
        pairs = zip(a[shift:] if len(a) > len(b) else a, b[shift:] if len(b) > len(a) else b)
        total = 0.0
        for x, y in pairs:
            prefix = 0
            while prefix < min(len(x), len(y)) and x[prefix] == y[prefix]:
                prefix += 1
            suffix = 0
            while suffix < min(len(x), len(y)) - prefix and x[-suffix - 1] == y[-suffix - 1]:
                suffix += 1
            total += (prefix + suffix) / max(len(x), len(y), 1)
        return total
    best, offset = score(0) + count * 0.5, 0
    for candidate in range(1, extra + 1):
        value = score(candidate)
        if value > best:
            best, offset = value, candidate
    return offset


class FileView:
    def __init__(self, raw, reference):
        self.meta = metadata(raw, reference)
        self.lines = []
        self.hunks = []
        hunk = None
        old = new = 0
        for physical in io.BytesIO(raw):
            if physical.rstrip(b"\n") == MARKER:
                line = self.lines[-1]
                line.no_newline = True
                line.data = line.data.removesuffix(b"\n")
                continue
            match = HUNK.match(physical)
            if match:
                old, new = int(match[1]), int(match[3])
                hunk = {"header": safe(physical.rstrip(b"\n"), True), "old": old, "new": new, "lines": []}
                self.hunks.append(hunk)
            elif hunk is not None:
                sign = chr(physical[0])
                if sign not in " +-":
                    raise ValueError("Unexpected source record")
                line = Line(sign, old if sign != "+" else None, new if sign != "-" else None,
                            physical[1:], reference["source_start"] + len(self.lines))
                self.lines.append(line)
                hunk["lines"].append(line)
                old += sign != "+"
                new += sign != "-"
        if len(self.lines) != reference["source_end"] - reference["source_start"]:
            raise ValueError("Source record count differs from the full independent reference")
        self.split, self.unified = [], []
        self.project()

    def project(self):
        def both(row):
            self.split.append(row)
            self.unified.append(row)

        def code(rows, left, right):
            rows.append(("code", left, right))
            if (left and left.no_newline) or (right and right.no_newline):
                rows.append(("newline", bool(left and left.no_newline), bool(right and right.no_newline)))

        both(("file", None, None))
        if not self.hunks:
            both(("metadata", None, None))
        old_end = new_end = 1
        for h in self.hunks:
            if h["old"] > old_end or h["new"] > new_end:
                both(("gap", None, None))
            both(("hunk", h["header"], None))
            block = []

            def flush():
                if not block:
                    return
                left = [l for l in block if l.sign == "-"]
                right = [l for l in block if l.sign == "+"]
                for line in left + right:
                    code(self.unified, line if line.old is not None else None, line if line.new is not None else None)
                offset = pairing(left, right)
                if len(left) < len(right):
                    left = [None] * offset + left
                else:
                    right = [None] * offset + right
                for n in range(max(len(left), len(right))):
                    code(self.split, left[n] if n < len(left) else None, right[n] if n < len(right) else None)
                block.clear()

            for line in h["lines"]:
                if line.sign == " ":
                    flush()
                    code(self.split, line, line)
                    code(self.unified, line, line)
                else:
                    block.append(line)
                if line.old is not None:
                    old_end = line.old + 1
                if line.new is not None:
                    new_end = line.new + 1
            flush()

    def rows(self, split):
        return self.split if split else self.unified


class Document:
    def __init__(self, path, reference):
        self.path, self.reference = path, reference
        self.count = len(reference["files"])

    @lru_cache(maxsize=4)
    def file(self, index):
        ref = self.reference["files"][index]
        with self.path.open("rb") as source:
            source.seek(ref["offset"])
            raw = source.read(ref["bytes"])
        return FileView(raw, ref)

    def row(self, pos, split):
        return self.file(pos[0]).rows(split)[pos[1]]

    def step(self, pos, count, split):
        f, r = pos
        while count:
            if count > 0:
                take = min(count, len(self.file(f).rows(split)) - r - 1)
                r, count = r + take, count - take
                if count:
                    if f + 1 == self.count:
                        break
                    f, r, count = f + 1, 0, count - 1
            else:
                take = min(-count, r)
                r, count = r - take, count + take
                if count:
                    if not f:
                        break
                    f, count = f - 1, count + 1
                    r = len(self.file(f).rows(split)) - 1
        return f, r

    def source_pos(self, ordinal, split):
        refs = self.reference["files"]
        # Structural targets are few; this search is outside measurement.
        f = next(i for i, ref in enumerate(refs) if ref["source_start"] <= ordinal < ref["source_end"])
        r = next(i for i, row in enumerate(self.file(f).rows(split)) if row[0] == "code" and any(l and l.ordinal == ordinal for l in row[1:]))
        return f, r


def glyphs(text):
    column = 0
    for cluster in regex.findall(r"\X", text):
        width = 4 - column % 4 if cluster == "\t" else max(1, wcswidth(cluster, unicode_version="15.1.0"))
        yield column, " " * width if cluster == "\t" else cluster, width
        column += width


def put(cells, x, text, width):
    for column, glyph, size in glyphs(text):
        if column + size > width:
            break
        # pyte composes combining marks; raw byte identity is checked separately.
        cells[x + column] = unicodedata.normalize("NFC", glyph) if glyph.strip() else " "
        for n in range(1, size):
            cells[x + column + n] = "" if glyph.strip() else " "


class View:
    def __init__(self, doc):
        self.doc = doc
        self.top = self.cursor = (0, 0)
        self.split, self.old = True, False
        self.width, self.height = 160, 50

    def visible(self):
        y = 1 + (self.doc.row(self.top, self.split)[0] != "file")
        p = self.top
        result = []
        while y < self.height - 1:
            result.append((p, y))
            following = self.doc.step(p, 1, self.split)
            if following == p:
                break
            p, y = following, y + 1
        return result

    def scroll(self, amount):
        self.top = self.doc.step(self.top, amount, self.split)
        visible = self.visible()
        self.cursor = min(max(self.cursor, self.top), visible[-1][0])

    def jump(self, end, prototype=False):
        if prototype:
            self.cursor = (self.doc.count - 1, len(self.doc.file(self.doc.count - 1).rows(self.split)) - 1) if end else (0, 0)
        else:
            ordinal = self.doc.reference["last" if end else "first"]["source_ordinal"]
            self.cursor = self.doc.source_pos(ordinal, self.split)
        if self.cursor < self.top:
            self.top = self.cursor
        elif self.cursor not in [p for p, _ in self.visible()]:
            self.top = self.doc.step(self.cursor, -(self.height - 4), self.split)

    def locate(self, ordinal):
        self.cursor = self.doc.source_pos(ordinal, self.split)
        self.top = self.doc.step(self.cursor, -(self.height - 4), self.split)
        line = next(l for l in self.doc.row(self.cursor, self.split)[1:] if l and l.ordinal == ordinal)
        self.old = line.new is None

    def reflow(self, split, width, height):
        screen_row = next((i for i, (p, _) in enumerate(self.visible()) if p == self.cursor), 0)
        row = self.doc.row(self.cursor, self.split)
        source = (row[1] or row[2]) if self.old else (row[2] or row[1])
        if row[0] == "code" and split != self.split:
            self.cursor = self.doc.source_pos(source.ordinal, split)
            if source.sign != " ":
                self.old = source.sign == "-"
        self.split, self.width, self.height = split, width, height
        self.top = self.doc.step(self.cursor, -min(screen_row, height - 4), split)

    def grid(self, completed=None):
        width = self.width
        cells = [[" "] * width for _ in range(self.height - 2)]

        def header(f, y):
            m = self.doc.file(f).meta
            path = f" {m['old_path']} → {m['path']}" if m["old_path"] != m["path"] else f" {m['path']}"
            summary = f" {m['status']}   +{m['additions']} −{m['deletions']} "
            n = min(width, wcswidth(summary))
            put(cells[y-1], 0, path, width-n)
            put(cells[y-1], width-n, summary, n)

        if self.doc.row(self.top, self.split)[0] != "file":
            header(self.top[0], 1)
        sources = []
        for (f, r), y in self.visible():
            if completed is not None and f >= completed:
                break
            row = self.doc.file(f).rows(self.split)[r]
            kind, left, right = row
            m = self.doc.file(f).meta
            target = cells[y-1]
            if kind == "file":
                header(f, y)
            elif kind == "metadata":
                put(target, 1, f" {m['status']}  ·  mode {m['old_mode']} → {m['new_mode']}  ·  no textual hunks", width-2)
            elif kind == "gap":
                put(target, 2, "···  Context not included in patch", width-4)
            elif kind == "hunk":
                put(target, 1, left, width-2)
            elif kind == "newline":
                if self.split:
                    lw = (width-1)//2
                    for present, x, size in [(left, 0, lw), (right, lw+1, width-lw-1)]:
                        if present:
                            put(target, x, " No newline at end of file", size)
                else:
                    put(target, 1, " No newline at end of file", width-2)
            else:
                digits = m["digits"]
                number = lambda n: " " * digits if n is None else str(n).rjust(digits)
                if self.split:
                    lw = (width-1)//2
                    target[lw] = "│"
                    panes = [(left, 0, lw, True), (right, lw+1, width-lw-1, False)]
                else:
                    panes = [(right or left, 0, width, right is None)]
                for line, x, pane, old in panes:
                    if line is None:
                        continue
                    active = self.cursor == (f, r) and (not self.split or old == self.old)
                    mark = "▎" if active else " "
                    sign = "−" if line.sign == "-" else line.sign
                    if self.split:
                        gutter = mark + number(line.old if old else line.new) + sign + " "
                    else:
                        gutter = mark + number(line.old) + " " + number(line.new) + sign + " "
                    put(target, x, gutter, len(gutter))
                    put(target, x + len(gutter), line.text(), pane - len(gutter))
                    sources.append({"file": f, "source_ordinal": line.ordinal, "old": line.old, "new": line.new, "y": y, "x": x})
        return {"width": width, "height": self.height, "split": self.split,
                "body": ["".join(row) for row in cells], "sources": sources,
                "top": self.top, "cursor": self.cursor, "old_side": self.old}
