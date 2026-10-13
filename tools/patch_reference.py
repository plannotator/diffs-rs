"""Independent supplied-patch reference. Does not import the viewer or libgit2."""

import argparse
import hashlib
import json
import re
import struct
from pathlib import Path


HUNK = re.compile(rb"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@")
MARKER = b"\\ No newline at end of file"


def number(value):
    return struct.pack("<Q", value)


def decode(path):
    body = hashlib.sha256()
    content = hashlib.sha256()
    files = []
    current = None
    hunk = None
    pending = None
    offset = 0
    source_count = additions = deletions = 0
    longest = None
    largest_hunk = None
    first = last = None

    def emit():
        nonlocal pending, source_count, additions, deletions, longest, first, last
        if pending is None:
            return
        sign, old, new, data, no_newline, start = pending
        record = b"L" + sign + number(old) + number(new)
        content.update(record + bytes([no_newline]) + number(len(data)) + data)
        anchor = {
            "file": len(files), "hunk": len(current["hunks"]),
            "source_ordinal": source_count, "old": old or None, "new": new or None,
            "offset": start, "bytes": len(data), "no_newline": no_newline,
            "prefix_hex": data[:160].hex(),
        }
        if first is None:
            first = anchor
        last = anchor
        source_length = len(data.removesuffix(b"\n"))
        if longest is None or source_length > longest["source_bytes"]:
            longest = dict(anchor, source_bytes=source_length)
        source_count += 1
        additions += sign == b"+"
        deletions += sign == b"-"
        hunk["source_records"] += 1
        pending = None

    def finish_hunk():
        nonlocal hunk, largest_hunk
        if hunk is None:
            return
        emit()
        if hunk["old_remaining"] or hunk["new_remaining"]:
            raise ValueError(f"Truncated hunk near byte {offset}")
        del hunk["old_remaining"], hunk["new_remaining"]
        if largest_hunk is None or hunk["source_records"] > largest_hunk["source_records"]:
            largest_hunk = dict(hunk, file=len(files), hunk=len(current["hunks"]))
        current["hunks"].append(hunk)
        hunk = None

    def finish_file(end):
        nonlocal current
        if current is None:
            return
        finish_hunk()
        current["bytes"] = end - current["offset"]
        current["metadata_sha256"] = current.pop("metadata_hash").hexdigest()
        current["source_end"] = source_count
        files.append(current)
        current = None

    with path.open("rb") as stream:
        for raw in stream:
            start = offset
            offset += len(raw)
            body.update(raw)
            if raw.startswith((b"diff --cc ", b"diff --combined ")):
                raise ValueError("Combined diff is outside the reference format")
            if raw.removesuffix(b"\n") == MARKER:
                if pending is None or pending[4] or not pending[3].endswith(b"\n"):
                    raise ValueError(f"Invalid newline marker at {start}")
                sign, old, new, data, _, source_start = pending
                pending = sign, old, new, data[:-1], True, source_start
                continue
            emit()
            if raw.startswith(b"diff --git "):
                finish_file(start)
                content.update(b"F" + number(len(files)))
                current = {
                    "offset": start, "header_hex": raw.hex(), "hunks": [],
                    "metadata_hash": hashlib.sha256(raw), "source_start": source_count,
                }
                continue
            if current is None:
                if raw.strip():
                    raise ValueError(f"Expected Git file header at byte {start}")
                continue
            match = HUNK.match(raw)
            if match:
                finish_hunk()
                old_start, old_count, new_start, new_count = (
                    int(match[1]), int(match[2] or b"1"),
                    int(match[3]), int(match[4] or b"1"),
                )
                if (old_count and not old_start) or (new_count and not new_start):
                    raise ValueError(f"Nonempty source range starts at zero near byte {start}")
                if current["hunks"]:
                    previous = current["hunks"][-1]
                    for side, line_start, count in [("old", old_start, old_count), ("new", new_start, new_count)]:
                        end = previous[side + "_start"] + previous[side + "_count"]
                        end += previous[side + "_count"] == 0
                        if line_start + (count == 0) < end:
                            raise ValueError(f"Overlapping {side} hunks at byte {start}")
                hunk = {
                    "offset": start, "old_start": old_start, "new_start": new_start,
                    "old_count": old_count, "new_count": new_count,
                    "old_remaining": old_count, "new_remaining": new_count,
                    "source_start": source_count, "source_records": 0,
                }
                content.update(b"H" + number(old_start) + number(new_start))
                continue
            if hunk is not None:
                if not raw or raw[:1] not in (b" ", b"+", b"-"):
                    raise ValueError(f"Unexpected hunk record at byte {start}")
                sign = raw[:1]
                old = new = 0
                if sign != b"+":
                    if hunk["old_remaining"] == 0:
                        raise ValueError(f"Extra old source at byte {start}")
                    old = hunk["old_start"] + hunk["old_count"] - hunk["old_remaining"]
                    hunk["old_remaining"] -= 1
                if sign != b"-":
                    if hunk["new_remaining"] == 0:
                        raise ValueError(f"Extra new source at byte {start}")
                    new = hunk["new_start"] + hunk["new_count"] - hunk["new_remaining"]
                    hunk["new_remaining"] -= 1
                pending = sign, old, new, raw[1:], False, start + 1
            else:
                current["metadata_hash"].update(raw)
    finish_file(offset)
    return {
        "schema_version": 1,
        "canonical_format": "F/u64-file; H/u64-old-start/u64-new-start; L/sign/u64-old-or-zero/u64-new-or-zero/u8-no-newline/u64-length/raw-bytes; integers little-endian",
        "sha256": body.hexdigest(), "bytes": offset,
        "source_sha256": content.hexdigest(), "source_records": source_count,
        "additions": additions, "deletions": deletions,
        "first": first, "last": last, "longest_line": longest, "largest_hunk": largest_hunk,
        "largest_file": max(range(len(files)), key=lambda i: files[i]["bytes"]) if files else None,
        "file_quantiles": sorted({i * (len(files) - 1) // 31 for i in range(32)}) if files else [],
        "files": files,
        "coverage": "All supplied source bytes, signs, side line numbers, hunk starts and newline markers. Raw metadata digests recorded separately; normalized metadata and terminal-grid expectations remain pending.",
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("patch", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    result = decode(args.patch)
    with args.output.open("x") as output:
        json.dump(result, output, separators=(",", ":"))
        output.write("\n")
    print(json.dumps({key: value for key, value in result.items() if key != "files"}))


if __name__ == "__main__":
    main()
