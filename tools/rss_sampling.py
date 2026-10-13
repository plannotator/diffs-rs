"""Conservative macOS process-group RSS observations, with query intervals."""

import os
import subprocess
import time

PROTOCOL = {"id": "process-group-rss-v3", "command": "/bin/ps",
            "arguments": ["-g", "{pgid}", "-x", "-o", "pid=,pgid=,rss=,state="],
            "environment": {"COMMAND_MODE": "unix2003", "LC_ALL": "C"},
            "timeout_seconds": 1, "post_query_wait_seconds": .1,
            "hold_policy": "full-query-containment", "minimum_hold_samples": 5,
            "shutdown_policy": "stop-and-join-before-intentional-quit", "join_timeout_seconds": 2}


def verify_quit_boundary(record, viewing_finished_ns, input_ns):
    if not isinstance(record, dict):
        raise ValueError("Missing RSS shutdown boundary")
    fields = ("requested_ns", "finished_ns", "query_count", "last_query_end_ns")
    if (record.get("clock") != "monotonic_ns" or record.get("status") != "complete"
            or record.get("reason") != "quit"
            or any(type(record.get(key)) is not int for key in fields)
            or record["query_count"] < 1
            or not viewing_finished_ns <= record["requested_ns"] <= record["finished_ns"] <= input_ns
            or not 0 < record["last_query_end_ns"] <= record["finished_ns"]):
        raise ValueError("Invalid RSS shutdown boundary")


def query_group(pgid):
    if type(pgid) is not int or pgid <= 0:
        raise ValueError("RSS query requires a positive process-group ID")
    query = {"start_ns": time.monotonic_ns(), "end_ns": None, "pgid": pgid,
             "status": "error", "members": [], "rss_bytes": None,
             "returncode": None, "stdout": "", "stderr": "", "error": None}
    try:
        command = [PROTOCOL["command"], *[arg.format(pgid=pgid) for arg in PROTOCOL["arguments"]]]
        result = subprocess.run(command, capture_output=True, text=True,
                                env=dict(os.environ, **PROTOCOL["environment"]),
                                timeout=PROTOCOL["timeout_seconds"])
        query.update(returncode=result.returncode, stdout=result.stdout, stderr=result.stderr)
        if result.stderr or result.returncode not in (0, 1):
            raise ValueError(f"ps failed: status {result.returncode}, stderr {result.stderr!r}")
        if result.returncode == 1 and not result.stdout.strip():
            query["status"] = "absent"
            return query
        if result.returncode != 0 or not result.stdout.strip():
            raise ValueError("Unexpected ps status or empty successful query")
        seen = set()
        for line in result.stdout.splitlines():
            fields = line.split()
            if len(fields) != 4 or not all(value.isascii() and value.isdecimal() for value in fields[:3]):
                raise ValueError(f"Malformed ps record: {line!r}")
            pid, group, kib = map(int, fields[:3])
            state = fields[3]
            if pid <= 0 or pid in seen or group != pgid:
                raise ValueError(f"Unexpected ps membership: {line!r}")
            seen.add(pid)
            query["members"].append({"pid": pid, "pgid": group, "rss_bytes": kib * 1024, "state": state})
            if "?" in state or (kib == 0 and not state.startswith("Z")):
                raise ValueError(f"Unavailable RSS or state: {line!r}")
        live = any(not member["state"].startswith("Z") for member in query["members"])
        query["status"] = "ok" if live else "inactive"
        if live:
            query["rss_bytes"] = sum(member["rss_bytes"] for member in query["members"])
    except subprocess.TimeoutExpired as error:
        query.update(error=f"ps exceeded {PROTOCOL['timeout_seconds']} second timeout",
                     stdout=(error.stdout or b"").decode(errors="replace"),
                     stderr=(error.stderr or b"").decode(errors="replace"))
    except (OSError, ValueError) as error:
        query["error"] = str(error)
    finally:
        query["end_ns"] = time.monotonic_ns()
    return query
