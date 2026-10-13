"""Real process-group and frozen-view checks for the macOS RSS observer."""

import argparse
import gzip
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import threading
import time

from corpus_acceptance import LIMITS, MeasuredSession, travel
from corpus_measure import expected_region, hold, observe
from corpus_reference import ROOT, digest, snapshot
from rss_sampling import PROTOCOL, query_group, verify_quit_boundary


def resident(fd, family):
    connection = socket.socket(fileno=fd)
    if family:
        child = os.fork()
        if child:
            connection.close()
            sys.stdin.buffer.read(1)
            return
    signal.signal(signal.SIGHUP, signal.SIG_IGN)
    pages = [bytearray(b"a" * (32 * 1024**2))]
    connection.sendall(f"{os.getpid()}\n".encode())
    while command := connection.recv(1):
        if command == b"q":
            return
        if command != b"g":
            raise ValueError("Unknown resident command")
        pages.append(bytearray(b"b" * (32 * 1024**2)))
        connection.sendall(b"grown\n")


def launch(family):
    connection, child = socket.socketpair()
    connection.settimeout(5)
    process = subprocess.Popen([sys.executable, __file__, "--resident", str(child.fileno()),
                                *(["--family"] if family else [])], stdin=subprocess.PIPE,
                               pass_fds=(child.fileno(),), start_new_session=True)
    child.close()
    reply = connection.makefile("rb")
    pid = int(reply.readline())
    return process, connection, reply, pid


def family_check(report):
    owned = []
    queries = []
    report.update(status="running", queries=queries)
    try:
        leader, connection, reply, child_pid = launch(True)
        owned.append((leader, connection, reply))
        outsider, other_connection, other_reply, other_pid = launch(False)
        owned.append((outsider, other_connection, other_reply))

        def sample(wanted):
            query = query_group(leader.pid)
            queries.append(query)
            assert query["status"] == "ok", query
            assert {m["pid"] for m in query["members"]} == wanted, query
            assert other_pid not in wanted
            assert query["rss_bytes"] == sum(m["rss_bytes"] for m in query["members"])
            return query["rss_bytes"]

        before = sample({leader.pid, child_pid})
        connection.sendall(b"g")
        assert reply.readline() == b"grown\n"
        after = sample({leader.pid, child_pid})
        assert after - before >= 16 * 1024**2, (before, after)
        os.kill(child_pid, signal.SIGSTOP)
        sample({leader.pid, child_pid})
        assert next(m for m in queries[-1]["members"] if m["pid"] == child_pid)["state"].startswith("T")
        os.kill(child_pid, signal.SIGCONT)
        leader.stdin.write(b"q")
        leader.stdin.flush()
        assert leader.wait(timeout=5) == 0
        sample({child_pid})
        connection.sendall(b"q")
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            query = query_group(leader.pid)
            queries.append(query)
            if query["status"] == "absent":
                break
            assert query["status"] in ("ok", "inactive"), query
            time.sleep(.02)
        assert queries[-1]["status"] == "absent", queries[-1]
        assert outsider.poll() is None
        report.update(status="pass", growth_bytes=after - before)
    finally:
        for process, connection, reply in owned:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait(timeout=5)
            process.stdin.close()
            reply.close()
            connection.close()


def viewer_check(binary, trace, report):
    hold_labels = ("quantile-0-split", "quantile-0-unified", "hunk-mid-resize-100", "hunk-mid-resize-160")
    positions = [[i for i, step in enumerate(trace["steps"]) if step["label"] == label]
                 for label in hold_labels]
    if any(len(indices) != 1 for indices in positions):
        raise ValueError("Observer control requires each declared hold destination exactly once")
    steps = trace["steps"][:max(indices[0] for indices in positions) + 1]
    session = MeasuredSession(binary, {"path": str(ROOT / trace["fixture"]["local_path"])}, LIMITS)
    checked = []
    report.update(status="running", checked_views=checked, queries=session.rss_queries, holds=session.rss_holds)
    try:
        observe(session, trace, trace["opening"], session.started_ns, expected_region(trace["opening"]))
        try:
            hold(session, trace["opening"]["expected"], 1, foreground_rss=True)
        except ValueError:
            assert not session.rss_holds
        else:
            raise AssertionError("Foreground query accepted alongside background sampling")
        hold(session, trace["opening"]["expected"], 1)
        width, height = 160, 50
        for step in steps:
            action = step["action"]
            if "travel" in action:
                started, _ = travel(session, bytes.fromhex(action["travel"]), width, height)
            else:
                if "resize" in action:
                    width, height = action["resize"]
                started, _ = session.inject(action)
            observe(session, trace, step, started, expected_region(step))
            checked.append(step["label"])
            if step["label"] in hold_labels:
                hold(session, step["expected"], 1)
        finished = time.monotonic_ns()
        quit_result = session.quit()
        report["quit"] = quit_result
        verify_quit_boundary(session.rss_shutdown, finished, quit_result["input_ns"])
        assert quit_result["exit_code"] == 0 and quit_result["termios_restored"], quit_result
        assert all(quit_result["cleanup_sequences_received"].values()), quit_result
    finally:
        session.close()
    assert session.rss_error is None, session.rss_error
    assert session.rss_shutdown == quit_result["rss_shutdown"]
    assert session.rss_shutdown["query_count"] == len(session.rss_queries)
    assert all(q["end_ns"] <= session.rss_shutdown["finished_ns"] for q in session.rss_queries)
    assert len(session.rss_holds) == 5
    for record in session.rss_holds:
        assert record["status"] == "complete" and record["samples"] >= 5, record
        for index in record["query_indices"]:
            query = session.rss_queries[index]
            assert record["start_ns"] <= query["start_ns"] <= query["end_ns"] <= record["deadline_ns"]
            assert {m["pid"] for m in query["members"]} == {session.pid}, query
    report["status"] = "pass"


def safety_check(binary, trace, report):
    session = MeasuredSession(binary, {"path": str(ROOT / trace["fixture"]["local_path"])},
                              dict(LIMITS, sampled_rss_stop_bytes=1))
    report.update(status="running", queries=session.rss_queries, holds=session.rss_holds)
    try:
        deadline = time.monotonic() + 5
        while not session.rss_error and time.monotonic() < deadline:
            packet = session.read(.01)
            if packet:
                list(session.frames.feed(packet[0]))
        assert session.rss_error == "Sampled process-group RSS exceeded the safety stop", session.rss_error
        try:
            session.inject({"input_hex": "6a"})
        except RuntimeError as error:
            assert str(error) == session.rss_error
        else:
            raise AssertionError("Input accepted after the RSS safety stop")
        try:
            hold(session, snapshot(session.frames.screen), 1)
        except (RuntimeError, AssertionError):
            failed = session.rss_holds[-1]
            assert failed["status"] == "incomplete" and failed["error"], failed
            assert failed["start_ns"] <= failed["end_ns"] < failed["deadline_ns"], failed
            assert failed["samples"] == len(failed["query_indices"])
        else:
            raise AssertionError("A hold accepted an actual RSS safety failure")
        try:
            session.quit()
        except RuntimeError as error:
            assert str(error) == session.rss_error
        else:
            raise AssertionError("Quit accepted a retained RSS safety failure")
    finally:
        session.close()
    assert session.returncode is not None and session.restored, session.returncode
    assert not session.sampler.is_alive()
    assert query_group(session.pid)["status"] == "absent"
    report.update(status="pass", error=session.rss_error, exit_code=session.returncode,
                  termios_restored=session.restored, rss_shutdown=session.rss_shutdown)


def lifecycle_check(binary, trace, report):
    report.update(status="running", sessions=[])
    for mode in ("normal", "forced-cleanup", "premature-exit", "exit-during-cadence"):
        session = MeasuredSession(binary, {"path": str(ROOT / trace["fixture"]["local_path"])}, LIMITS)
        row = {"mode": mode, "queries": session.rss_queries}
        report["sessions"].append(row)
        try:
            observe(session, trace, trace["opening"], session.started_ns, expected_region(trace["opening"]))
            hold(session, trace["opening"]["expected"], 1)
            finished = time.monotonic_ns()
            if mode == "normal":
                row["quit"] = session.quit()
                verify_quit_boundary(session.rss_shutdown, finished, row["quit"]["input_ns"])
                assert row["quit"]["exit_code"] == 0 and row["quit"]["exit_observation_ms"] >= 0
            elif mode == "premature-exit":
                session.stop_rss("quit")
                session.inject({"input_hex": "03"})
                deadline = time.monotonic() + 2
                while session.returncode is None and time.monotonic() < deadline:
                    session.read(.005)
                assert session.returncode is not None
                try:
                    session.quit()
                except RuntimeError as error:
                    assert str(error) == "Viewer exited before quit input"
                    row["rejected_quit"] = str(error)
                else:
                    raise AssertionError("Premature exit accepted as a normal quit")
            elif mode == "exit-during-cadence":
                session.stop_rss("quit")
                session.trace = dict(LIMITS, cadence_ms=1000)
                session.last_input = time.monotonic_ns()
                interrupt = threading.Timer(.05, os.kill, args=(session.pid, signal.SIGINT))
                interrupt.start()
                try:
                    session.quit()
                except RuntimeError as error:
                    assert str(error) == "Viewer exited before input"
                    row["rejected_quit"] = str(error)
                else:
                    raise AssertionError("Exit during cadence accepted as a normal quit")
                finally:
                    interrupt.join(timeout=2)
                assert not interrupt.is_alive()
        finally:
            session.close()
            row.update(exit_code=session.returncode, termios_restored=session.restored,
                       rss_shutdown=session.rss_shutdown, sampler_stopped=not session.sampler.is_alive(),
                       rss_error=session.rss_error)
        assert row["exit_code"] is not None and row["termios_restored"] and row["sampler_stopped"]
        assert row["rss_error"] is None
        assert session.rss_shutdown["query_count"] == len(session.rss_queries)
        assert all(q["end_ns"] <= session.rss_shutdown["finished_ns"] for q in session.rss_queries)
        assert query_group(session.pid)["status"] == "absent"
        if mode == "forced-cleanup":
            assert session.rss_shutdown["reason"] == "cleanup"
        if mode == "normal":
            assert session.rss_shutdown == row["quit"]["rss_shutdown"]
        row["status"] = "pass"
    report["status"] = "pass"


def foreground_check(binary, trace, report):
    report.update(status="running", sessions=[])
    for mode in ("normal", "unstarted-cleanup", "safety-stop"):
        session = MeasuredSession(binary, {"path": str(ROOT / trace["fixture"]["local_path"])},
                                  dict(LIMITS), background_rss=False)
        row = {"mode": mode, "queries": session.rss_queries, "holds": session.rss_holds}
        report["sessions"].append(row)
        try:
            observe(session, trace, trace["opening"], session.started_ns, expected_region(trace["opening"]))
            ready_ns = time.monotonic_ns()
            assert not session.rss_queries and session.sampler.ident is None
            expected = trace["opening"]["expected"]
            if mode == "normal":
                record = hold(session, expected, 1, foreground_rss=True)
                assert record["status"] == "complete" and record["samples"] >= 5
                assert record["sampling_scope"] == "settled-hold-only-v1"
                assert all(ready_ns <= q["start_ns"] <= q["end_ns"] for q in session.rss_queries)
                assert all(q["status"] == "ok" and q["rss_bytes"] > 0 for q in session.rss_queries)
                assert all({m["pid"] for m in q["members"]} == {session.pid} for q in session.rss_queries)
                row["quit"] = session.quit()
                verify_quit_boundary(session.rss_shutdown, record["end_ns"], row["quit"]["input_ns"])
                assert row["quit"]["exit_code"] == 0 and all(row["quit"]["cleanup_sequences_received"].values())
                count = len(session.rss_queries)
                try:
                    hold(session, expected, 1, foreground_rss=True)
                except ValueError:
                    assert len(session.rss_queries) == count
                else:
                    raise AssertionError("Foreground query accepted after shutdown")
            elif mode == "safety-stop":
                session.trace["sampled_rss_stop_bytes"] = 1
                try:
                    hold(session, expected, 1, foreground_rss=True)
                except RuntimeError as error:
                    assert str(error) == "Sampled process-group RSS exceeded the safety stop"
                    assert session.rss_holds[-1]["status"] == "incomplete"
                else:
                    raise AssertionError("Foreground hold accepted a real safety failure")
                try:
                    session.quit()
                except RuntimeError as error:
                    assert str(error) == session.rss_error
                else:
                    raise AssertionError("Quit accepted the foreground safety failure")
        finally:
            session.close()
            row.update(exit_code=session.returncode, termios_restored=session.restored,
                       rss_shutdown=session.rss_shutdown, sampler_stopped=not session.sampler.is_alive(),
                       rss_error=session.rss_error)
        assert session.returncode is not None and session.restored
        assert session.sampler.ident is None and not session.sampler.is_alive()
        assert query_group(session.pid)["status"] == "absent"
        if mode == "unstarted-cleanup":
            assert not session.rss_queries and session.rss_shutdown["query_count"] == 0
            assert session.rss_shutdown["last_query_end_ns"] is None
        row["status"] = "pass"
    report["status"] = "pass"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--resident", type=int)
    parser.add_argument("--family", action="store_true")
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--trace", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.resident is not None:
        resident(args.resident, args.family)
        return
    if not all((args.binary, args.trace, args.output)):
        parser.error("--binary, --trace, and --output are required")
    report = {"status": "running", "protocol": PROTOCOL,
              "binary_sha256": digest(args.binary), "trace_sha256": digest(args.trace),
              "identities": {str(path): digest(path) for path in sorted((ROOT / "tools").glob("*.py"))}}
    with args.output.open("x") as output:
        try:
            trace = json.loads(gzip.decompress(args.trace.read_bytes()))
            rejected = query_group(2**63)
            assert rejected["status"] == "error" and rejected["stderr"] and rejected["rss_bytes"] is None, rejected
            report["rejected_query"] = rejected
            report["process_group"] = {}
            family_check(report["process_group"])
            report["viewer"] = {}
            viewer_check(args.binary.resolve(), trace, report["viewer"])
            report["safety_stop"] = {}
            safety_check(args.binary.resolve(), trace, report["safety_stop"])
            report["lifecycle"] = {}
            lifecycle_check(args.binary.resolve(), trace, report["lifecycle"])
            report["foreground"] = {}
            foreground_check(args.binary.resolve(), trace, report["foreground"])
            report["status"] = "pass"
        except Exception as error:
            report.update(status="failed", error=str(error))
            raise
        finally:
            json.dump(report, output, indent=2)
            output.write("\n")
    print("Process-group membership, frozen PTY views, holds, and safety cleanup pass.")


if __name__ == "__main__":
    main()
