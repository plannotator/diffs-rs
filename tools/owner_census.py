"""Unscored, bounded engine/cache observations on a frozen real-PTY trace."""

import argparse
from collections import defaultdict
from contextlib import contextmanager
import gzip
import hashlib
import json
import os
from pathlib import Path
import time

from corpus_acceptance import LIMITS, MeasuredSession, await_state, travel
from corpus_measure import bindings
from corpus_reference import ROOT, digest, snapshot
from regex_census import summarize, write_json

REQUEST = {"input_hex": "1b5b32347e"}  # Diagnostic build only: F12.


@contextmanager
def process(binary, trace, out, *, enabled=True):
    out.mkdir(parents=True, exist_ok=False)
    environment = {"NATIVE_DIFF_OWNER_CENSUS": str(out)} if enabled else None
    session = MeasuredSession(binary, {"path": str(ROOT / trace["fixture"]["local_path"])},
                              LIMITS, child_env=environment)
    report = {"status": "running", "pid": session.pid, "snapshots": [], "checked_views": []}
    try:
        yield session, report
        assert report["quit"]["exit_code"] == report["quit"].get("expected_exit_code", 0)
        assert report["quit"]["termios_restored"]
        assert all(report["quit"]["cleanup_sequences_received"].values())
        report["status"] = "complete"
    except BaseException as error:
        report.update(status="failed", error=str(error), error_type=type(error).__name__)
        raise
    finally:
        try:
            session.close()
            assert not session.sampler.is_alive()
            assert not session.rss_error, session.rss_error
            try:
                os.killpg(session.pid, 0)
            except ProcessLookupError:
                report["process_group_absent"] = True
            else:
                raise RuntimeError("Viewer process group remains alive")
        except BaseException as error:
            report.update(status="failed", cleanup_error=str(error), cleanup_error_type=type(error).__name__)
            raise
        finally:
            report.update(output_sha256=session.raw_hash.hexdigest(), output_bytes=session.raw_bytes,
                          exit_code=session.returncode, termios_restored=session.restored,
                          rss_shutdown=session.rss_shutdown)
            (out / "tail.ansi").write_bytes(session.raw_tail)
            write_json(out / "process.json", report)


def opening(session, trace, report):
    await_state(session, None, trace["opening"]["state"], session.started_ns,
                expected=trace["opening"]["expected"], timeout=120)
    report["checked_views"].append("opening")


def check_cache_activity(cache):
    activity = cache["activity"]
    assert type(activity["schema"]) is int and activity["schema"] in (1, 2, 3)
    assert activity["scope"] == "current-cache-instance"
    assert 0 < activity["counter_storage_bytes"] <= 4096
    counts = activity["observations"]
    for value in counts.values():
        if not isinstance(value, dict):
            assert type(value) is int and value >= 0
    invalidated, capacity_gained, capacity_lost = 0, 0, 0
    new_fields = {"admitted_payload_bytes", "evicted_payload_bytes", "invalidations",
                  "invalidated_entries", "invalidated_payload_bytes",
                  "invalidation_capacity_gained", "invalidation_capacity_lost"}
    released = released_bytes = release_gained = release_lost = peak_visible = 0
    release_fields = {"releases", "released_entries", "released_payload_bytes",
                      "release_capacity_gained", "release_capacity_lost", "peak_visible_payload_bytes"}
    if activity["schema"] == 3:
        assert release_fields <= counts.keys() and "visible_protection" in cache
        released, released_bytes = counts["released_entries"], counts["released_payload_bytes"]
        release_gained, release_lost = counts["release_capacity_gained"], counts["release_capacity_lost"]
        peak_visible = counts["peak_visible_payload_bytes"]
        assert cache["visible_protection"]["payload_bytes"] <= peak_visible <= counts["peak_settled_payload_bytes"]
        assert (released == 0) == (released_bytes == 0)
        if counts["releases"] == 0:
            assert released == 0
        if released == 0:
            assert release_gained == release_lost == 0
    else:
        assert not release_fields.intersection(counts)
        assert "visible_protection" not in cache
    if activity["schema"] >= 2:
        assert new_fields <= counts.keys()
        invalidated = counts["invalidated_entries"]
        capacity_gained = counts["invalidation_capacity_gained"]
        capacity_lost = counts["invalidation_capacity_lost"]
        assert counts["admitted_payload_bytes"] - counts["evicted_payload_bytes"] - counts["invalidated_payload_bytes"] - released_bytes == cache["payload_counter_bytes"]
        assert (counts["admissions"] == 0) == (counts["admitted_payload_bytes"] == 0)
        assert (counts["evictions"] == 0) == (counts["evicted_payload_bytes"] == 0)
        assert (invalidated == 0) == (counts["invalidated_payload_bytes"] == 0)
        if counts["invalidations"] == 0:
            assert invalidated == 0
        if invalidated == 0:
            assert capacity_gained == capacity_lost == 0
    else:
        assert not new_fields.intersection(counts)
    assert counts["admissions"] - counts["evictions"] - invalidated - released == cache["entries"]
    assert counts["admissions_with_eviction"] <= min(counts["admissions"], counts["evictions"])
    assert cache["entries"] <= counts["peak_settled_entries"] <= counts["peak_entries"]
    assert counts["peak_entries"] <= counts["peak_settled_entries"] + 1
    assert counts["peak_entries"] <= counts["capacity_high_water"]
    assert cache["map_capacity"] <= counts["capacity_high_water"]
    assert cache["payload_counter_bytes"] <= counts["peak_settled_payload_bytes"] <= counts["peak_payload_bytes"]
    assert counts["peak_settled_payload_bytes"] <= max(32 * 1024**2, peak_visible)
    assert counts["insertion_capacity_gained"] + counts["removal_capacity_gained"] + capacity_gained + release_gained == (
        cache["map_capacity"] + counts["insertion_capacity_lost"] + counts["removal_capacity_lost"] + capacity_lost + release_lost)
    increases = counts["capacity_increases"]
    high_water = counts["high_water_increases"]
    assert high_water["count"] <= increases["count"] <= counts["admissions"]
    for name, group in (("capacity_increases", increases), ("high_water_increases", high_water)):
        for key, value in group.items():
            if key != "last":
                assert type(value) is int and 0 <= value <= group["count"]
        event = group["last"]
        assert (event is None) == (group["count"] == 0)
        if event is None:
            continue
        assert 1 <= event["ordinal"] <= counts["admissions"]
        before, inserted, settled = (event[key] for key in ("before", "inserted", "settled"))
        for point in (before, inserted, settled):
            assert all(type(value) is int and value >= 0 for value in point.values())
            assert point["entries"] <= point["capacity"] <= counts["capacity_high_water"]
        assert inserted["entries"] == before["entries"] + 1
        assert settled["entries"] <= inserted["entries"] <= counts["peak_entries"]
        for point in (before, inserted, settled):
            if activity["schema"] == 3:
                assert point["visible_payload_bytes"] <= point["payload_bytes"]
            else:
                assert "visible_payload_bytes" not in point
        for point in (before, settled):
            assert point["payload_bytes"] <= max(32 * 1024**2, point.get("visible_payload_bytes", 0))
        assert before["payload_bytes"] < inserted["payload_bytes"] <= counts["peak_payload_bytes"]
        assert settled["payload_bytes"] <= inserted["payload_bytes"]
        assert inserted["capacity"] > before["capacity"]
        assert before["entries"] <= event["prior_peak_entries"] <= counts["peak_entries"]
        assert before["capacity"] <= event["prior_capacity_high_water"] <= counts["capacity_high_water"]
        if name == "high_water_increases":
            assert inserted["capacity"] > event["prior_capacity_high_water"]
        for field, condition in (
            ("at_capacity", before["entries"] == before["capacity"]),
            ("with_eviction", inserted["entries"] > settled["entries"]),
            ("without_new_entry_peak", inserted["entries"] <= event["prior_peak_entries"]),
            ("within_prior_capacity_high_water", inserted["entries"] <= event["prior_capacity_high_water"]),
        ):
            if condition:
                assert group[field] > 0
    for field in ("at_capacity", "with_eviction", "without_new_entry_peak", "within_prior_capacity_high_water"):
        assert high_water[field] <= increases[field]


def check_style_storage(cache):
    visible_capacity = 0
    if "visible_protection" in cache:
        visible = cache["visible_protection"]
        assert all(type(value) is int and value >= 0 for value in visible.values())
        assert visible["schema"] == 1 and visible["retained_target_bytes"] == 32 * 1024**2
        assert visible["cached_entries"] <= min(cache["entries"], visible["ids"])
        assert visible["ids"] <= visible["capacity"]
        assert visible["capacity_bytes"] == visible["capacity"] * cache["key_size_bytes"]
        assert visible["payload_bytes"] <= cache["payload_counter_bytes"]
        assert (visible["cached_entries"] == 0) == (visible["payload_bytes"] == 0)
        assert cache["payload_counter_bytes"] <= max(visible["retained_target_bytes"], visible["payload_bytes"])
        visible_capacity = visible["capacity_bytes"]
    if "activity" in cache:
        check_cache_activity(cache)
    key_size = cache.get("key_size_bytes")
    if "key_size_bytes" in cache:
        assert type(key_size) is int and key_size > 0
        assert cache["occupied_key_bytes"] == cache["entries"] * key_size
    else:
        assert "occupied_key_bytes" not in cache
    if "fifo_accounting" in cache:
        assert cache["fifo_accounting"] == "recomputed-from-styles"
        assert type(cache["fifo_element_size_bytes"]) is int
        if key_size is not None:
            assert cache["fifo_element_size_bytes"] == key_size
        else:
            assert cache["fifo_element_size_bytes"] in (4, 8)
        assert cache["fifo_capacity_bytes"] == cache["fifo_capacity"] * cache["fifo_element_size_bytes"]
    else:
        assert "fifo_element_size_bytes" not in cache and "fifo_capacity_bytes" not in cache
    storage = cache["style_storage"]
    assert storage["schema"] in (1, 2, 3)
    if storage["schema"] == 2:
        assert storage["representation"] == "boxed-slice"
        assert storage["capacity_scope"] == "owned-element-extent"
    token_size = storage["token_size_bytes"]
    range_size = storage["changed_range_size_bytes"]
    assert token_size > 0 and range_size > 0 and storage["line_style_size_bytes"] > 0
    primary, old, changed = storage["primary"], storage["old"], storage["changed"]
    for vectors in (primary["vectors"], old["vectors"], changed):
        assert 0 <= vectors["length"] <= vectors["capacity"]
        assert 0 <= vectors["max_length"] <= vectors["max_capacity"] <= vectors["capacity"]
        assert vectors["max_length"] <= vectors["length"]
        assert vectors["count"] >= 0
        if storage["schema"] == 2:
            assert vectors["length"] == vectors["capacity"]
            assert vectors["max_length"] == vectors["max_capacity"]
    assert primary["vectors"]["count"] == changed["count"] == cache["entries"]
    assert old["vectors"]["count"] <= cache["entries"]
    for side in (primary, old):
        assert 0 <= side["invalid_vectors"] <= side["vectors"]["count"]
        assert 0 <= side["empty_ranges"] <= side["vectors"]["length"]
        assert 0 <= side["hypothetical_merged_tokens"] <= side["eligible_nonempty_tokens"]
        assert side["eligible_nonempty_tokens"] <= side["vectors"]["length"] - side["empty_ranges"]
        if side["invalid_vectors"] == 0:
            assert side["eligible_nonempty_tokens"] == side["vectors"]["length"] - side["empty_ranges"]
    assert 0 <= storage["old_comparison_ineligible"] <= old["vectors"]["count"]
    assert 0 <= storage["identical_old_vectors"] <= storage["equivalent_old_vectors"]
    assert storage["equivalent_old_vectors"] <= old["vectors"]["count"] - storage["old_comparison_ineligible"]
    assert 0 <= storage["equivalent_old_capacity_bytes"] <= old["vectors"]["capacity"] * token_size
    used = (primary["vectors"]["length"] + old["vectors"]["length"]) * token_size + changed["length"] * range_size
    capacity = (primary["vectors"]["capacity"] + old["vectors"]["capacity"]) * token_size + changed["capacity"] * range_size
    objects = cache["entries"] * storage["line_style_size_bytes"]
    assert storage["dynamic_used_bytes"] == used
    assert storage["dynamic_capacity_bytes"] == capacity
    assert storage["dynamic_spare_bytes"] == capacity - used
    if storage["schema"] == 3:
        assert storage["representation"] == "boxed-value" and storage["vectors"] == "vec"
        assert storage["extent_scope"] == "rust-values-excluding-allocator-overhead"
        assert storage["map_value_size_bytes"] in (4, 8)
        assert storage["inline_value_bytes"] == cache["entries"] * storage["map_value_size_bytes"]
        assert storage["boxed_value_bytes"] == objects
    else:
        assert storage["inline_value_bytes"] == objects
    assert objects + capacity == cache["recomputed_payload_bytes"]
    if "container_storage" in cache:
        container = cache["container_storage"]
        assert type(container["schema"]) is int and container["schema"] in (1, 2)
        if container["schema"] == 2:
            assert "visible_protection" in cache
            assert type(container["visible_capacity_bytes"]) is int
            assert container["visible_capacity_bytes"] == visible_capacity
        else:
            assert "visible_protection" not in cache and "visible_capacity_bytes" not in container
        assert storage["schema"] == 1
        assert container["scope"] == "retained-requested-storage-excluding-allocator-overhead"
        assert container["table_provider"] == "hashbrown-0.16.1-allocation_size"
        assert container["inline_scope"] == "observed-owner-census-build"
        for field in (
            "table_allocation_bytes", "vector_capacity_bytes", "fifo_capacity_bytes",
            "reported_heap_bytes", "inline_cache_bytes", "diagnostic_inline_bytes",
            "reported_value_and_heap_bytes",
        ):
            assert type(container[field]) is int and container[field] >= 0, field
        assert container["table_allocation_bytes"] >= objects + cache["occupied_key_bytes"]
        assert container["vector_capacity_bytes"] == capacity
        assert container["fifo_capacity_bytes"] == cache["fifo_capacity_bytes"]
        assert container["diagnostic_inline_bytes"] == cache["activity"]["counter_storage_bytes"]
        assert container["inline_cache_bytes"] > container["diagnostic_inline_bytes"]
        heap = container["table_allocation_bytes"] + capacity + cache["fifo_capacity_bytes"] + visible_capacity
        assert container["reported_heap_bytes"] == heap
        assert container["reported_value_and_heap_bytes"] == heap + container["inline_cache_bytes"]


def storage_controls(cache):
    check_style_storage(cache)
    check_style_storage({key: value for key, value in cache.items() if key != "container_storage"})
    cases = {
        "zero-key-size": {"key_size_bytes": 0},
        "negative-key-size": {"key_size_bytes": -1},
        "noninteger-key-size": {"key_size_bytes": float(cache["key_size_bytes"])},
        "occupied-key-bytes": {"occupied_key_bytes": cache["occupied_key_bytes"] + 1},
        "fifo-key-size-mismatch": {"fifo_element_size_bytes": cache["key_size_bytes"] + 1},
        "noninteger-fifo-size": {"fifo_element_size_bytes": float(cache["fifo_element_size_bytes"])},
        "fifo-capacity-bytes": {"fifo_capacity_bytes": cache["fifo_capacity_bytes"] + 1},
    }
    if "container_storage" in cache:
        container = cache["container_storage"]
        changes = {
            field: container[field] + 1 for field in (
                "table_allocation_bytes", "vector_capacity_bytes", "fifo_capacity_bytes",
                "reported_heap_bytes", "inline_cache_bytes", "diagnostic_inline_bytes",
                "reported_value_and_heap_bytes",
            )
        }
        changes.update({
            "schema": True, "scope": "RSS", "table_provider": "capacity-estimate",
            "inline_scope": "ordinary-build",
        })
        cases.update({
            f"container-{field}": {"container_storage": {**container, field: value}}
            for field, value in changes.items()
        })
        cases["container-noninteger-heap"] = {
            "container_storage": {**container, "reported_heap_bytes": float(container["reported_heap_bytes"])}
        }
        if container["schema"] == 2:
            cases["container-visible-capacity"] = {
                "container_storage": {**container, "visible_capacity_bytes": container["visible_capacity_bytes"] + 1}
            }
    if "visible_protection" in cache:
        visible = cache["visible_protection"]
        changes = {"schema": True, "retained_target_bytes": visible["retained_target_bytes"] + 1,
                   "ids": visible["capacity"] + 1, "cached_entries": cache["entries"] + 1,
                   "payload_bytes": cache["payload_counter_bytes"] + 1,
                   "capacity_bytes": visible["capacity_bytes"] + 1}
        cases.update({f"visible-{field}": {"visible_protection": {**visible, field: value}}
                      for field, value in changes.items()})
    for name, changes in cases.items():
        try:
            check_style_storage({**cache, **changes})
        except AssertionError:
            continue
        raise AssertionError(f"Accepted corrupted storage accounting: {name}")
    return list(cases)


def check_retained(syntax, app, *, required=False):
    if not required and "retained" not in syntax and "retained" not in app:
        return
    worker, main = syntax["retained"], app["retained"]
    assert type(worker["schema"]) is int and worker["schema"] == 1
    assert type(main["schema"]) is int and main["schema"] == 1

    def sizes(record, optional=()):
        for name, value in record.items():
            if name in optional and value is None:
                continue
            assert type(value) is int and value >= 0, (name, value)

    for name in ("source_pages", "geometry"):
        cache = main[name]
        if name == "source_pages" and cache is None:
            continue
        sizes(cache)
        assert cache["payload_counter_bytes"] == cache["recomputed_payload_bytes"] <= cache["payload_limit_bytes"]
        assert cache["entries"] == cache["fifo_length"] <= cache["fifo_capacity"]
        assert cache["entries"] <= cache["map_capacity"]
        assert cache["fifo_element_size_bytes"] > 0
        if name == "geometry":
            assert cache["entries"] <= cache["entry_limit"]
            assert cache["text_bytes"] <= cache["payload_counter_bytes"]
    checkpoint = worker["checkpoints"]
    sizes(checkpoint, ("recomputed_heap_bytes",))
    assert checkpoint["heap_counter_bytes"] == checkpoint["recomputed_heap_bytes"]
    assert checkpoint["entries"] <= checkpoint["entry_limit"]
    assert checkpoint["entries"] <= checkpoint["capacity"]
    assert checkpoint["protected_entries"] == 0
    assert checkpoint["retained_charge_bytes"] == (checkpoint["inline_size_bytes"] +
            checkpoint["capacity"] * checkpoint["entry_size_bytes"] + checkpoint["heap_counter_bytes"])
    assert checkpoint["retained_charge_bytes"] <= checkpoint["retained_limit_bytes"]
    patch, full = worker["patch"], worker["full_source"]
    for record in (patch, full):
        assert type(record["inline_size_bytes"]) is int and record["inline_size_bytes"] > 0
    states = [patch["state"], full["old"], full["new"]]
    for state in states:
        if state is None:
            continue
        assert type(state["next"]) is int and state["next"] >= 0
        sides = (state["old"], state["new"]) if "old" in state else (state["side"],)
        for side in sides:
            assert set(side) == {"parse_heap_bytes", "highlight_heap_bytes"}
            sizes(side, ("parse_heap_bytes", "highlight_heap_bytes"))


def observe(session, out, ordinal, expected, *, inject=True, allow_loading=False):
    marker = out / f"snapshot-{ordinal:02}.ready"
    if inject:
        assert not marker.exists()
        session.inject(REQUEST)
    deadline = time.monotonic() + 120
    while not marker.exists():
        packet = session.read(.01)
        if packet:
            list(session.frames.feed(packet[0]))
        if session.returncode is not None or session.rss_error:
            raise RuntimeError(f"Census failed: exit={session.returncode}, rss={session.rss_error}")
        if time.monotonic() > deadline:
            raise RuntimeError("Census completion timed out")
    while packet := session.read(0):
        list(session.frames.feed(packet[0]))
    assert snapshot(session.frames.screen) == expected, "Census changed the settled terminal cells"
    syntax = json.loads((out / f"syntax-{ordinal:02}.json").read_text())
    app = json.loads((out / f"app-{ordinal:02}.json").read_text())
    assert syntax["schema"] == app["schema"] == 1
    assert syntax["started_ns"] <= syntax["observed_ns"] <= app["started_ns"] <= app["observed_ns"]
    cache = app["cache"]
    assert cache["payload_counter_bytes"] == cache["recomputed_payload_bytes"] == cache["fifo_accounted_bytes"]
    assert cache["payload_counter_bytes"] <= max(32 * 1024**2, cache.get("visible_protection", {}).get("payload_bytes", 0))
    assert cache["entries"] == cache["fifo_length"] <= cache["fifo_capacity"]
    assert cache["entries"] <= cache["map_capacity"]
    check_style_storage(cache)
    check_retained(syntax, app)
    assert allow_loading or not app["loading"]
    return syntax, app


def analyze(syntax, app):
    census = syntax["census"]
    rules = summarize(census["rules"])
    expected_owners = {str(rule["compiled_owner"])
                       for grammar in census["rules"]["loaded_syntaxes"]
                       for rule in grammar["patterns"] if rule["compiled_owner"] is not None}
    expected_owners.update(str(rule["compiled_owner"]) for rule in census["rules"]["first_line_patterns"]
                           if rule["compiled_owner"] is not None)
    assert set(census["compiled"]) == expected_owners
    instances = {}
    groups = defaultdict(list)
    references = 0
    for owner, engines in census["compiled"].items():
        for engine in engines:
            identity = engine["instance"]
            assert identity > 0
            assert engine["scratch_bytes"] is None
            assert not engine["engine_config"]["custom_prefilter_present"], "Custom prefilters need distinct identities"
            references += 1
            intrinsic = {key: value for key, value in engine.items()
                         if key not in ("instruction", "capture_groups", "explicit_capture_group_0")}
            if identity in instances:
                assert instances[identity] == intrinsic, "Shared handle has inconsistent compilation data"
                continue
            instances[identity] = intrinsic
            key = json.dumps([engine["pattern"], engine["syntax_config"], engine["engine_config"]], sort_keys=True)
            groups[key].append(engine)
    duplicates = []
    for key, engines in groups.items():
        if len(engines) > 1:
            sizes = [engine["approximate_strategy_bytes"] for engine in engines]
            duplicates.append({"pattern_config_sha256": hashlib.sha256(key.encode()).hexdigest(),
                               "instances": sorted(engine["instance"] for engine in engines),
                               "instance_count": len(engines), "strategy_estimate_bytes": sum(sizes),
                               "additional_instance_estimate_bytes": sum(sizes) - max(sizes),
                               "min_instance_bytes": min(sizes), "max_instance_bytes": max(sizes)})
    duplicates.sort(key=lambda row: (-row["additional_instance_estimate_bytes"], row["pattern_config_sha256"]))
    return {"rules": rules, "inner_instances": len(instances), "inner_references": references,
            "approximate_strategy_bytes": sum(engine["approximate_strategy_bytes"] for engine in instances.values()),
            "scratch_bytes": None, "cache": app["cache"], "instances": instances,
            "retained": {"worker": syntax.get("retained"), "app": app.get("retained")},
            "pattern_config_groups": len(groups), "duplicate_groups": duplicates,
            "scope": "Strategy estimates exclude scratch, outer VM, strings and allocator overhead. Equal groups are sharing candidates, not proven recoverable bytes. Map capacity is usable slots, not allocation bytes."}


def expected_failure(session, message):
    session.stop_rss("expected diagnostic failure")
    session.inject(REQUEST)
    deadline = time.monotonic() + 120
    while session.returncode is None and time.monotonic() < deadline:
        session.read(.01)
    while session.read(0) is not None:
        pass
    assert session.returncode == 1, session.returncode
    assert session.restored and all(session.cleanup_bytes.values())
    assert message in session.raw_tail
    return {"exit_code": session.returncode, "expected_exit_code": 1, "termios_restored": session.restored,
            "cleanup_sequences_received": session.cleanup_bytes.copy()}


def controls(binary, ordinary, trace, out):
    startup = out / "startup"
    with process(binary, trace, startup) as (session, report):
        deadline = time.monotonic() + 120
        while not session.frames.screen.display[0].lstrip().startswith("native-diff"):
            packet = session.read(.01)
            if packet:
                list(session.frames.feed(packet[0]))
            if session.returncode is not None or session.rss_error or time.monotonic() > deadline:
                raise RuntimeError("No initial application frame")
        report["request_footer"] = session.frames.screen.display[-1]
        session.inject(REQUEST)
        if snapshot(session.frames.screen) != trace["opening"]["expected"]:
            opening(session, trace, report)
        first = observe(session, startup, 0, trace["opening"]["expected"], inject=False, allow_loading=True)
        report["snapshots"].append(analyze(*first))
        report["quit"] = session.quit()
    repeated = out / "repeated"
    with process(binary, trace, repeated) as (session, report):
        opening(session, trace, report)
        first = observe(session, repeated, 0, trace["opening"]["expected"])
        rejected = storage_controls(first[1]["cache"])
        report["snapshots"].append(analyze(*first))
        for ordinal in range(1, 8):
            current = observe(session, repeated, ordinal, trace["opening"]["expected"])
            assert current[0]["census"] == first[0]["census"]
            assert current[1]["cache"] == first[1]["cache"]
            assert current[0]["retained"] == first[0]["retained"]
            assert current[1]["retained"] == first[1]["retained"]
            report["snapshots"].append(analyze(*current))
        report["quit"] = expected_failure(session, b"Owner census snapshot limit reached")
        assert not (repeated / "syntax-08.json").exists()
    for name in ["syntax-00.json", "app-00.json", "snapshot-00.ready"]:
        target = out / f"collision-{name}"
        with process(binary, trace, target) as (session, report):
            opening(session, trace, report)
            sentinel = target / name
            sentinel.write_bytes(b"preserve-existing-evidence")
            report["quit"] = expected_failure(session, b"Cannot create owner census")
            assert sentinel.read_bytes() == b"preserve-existing-evidence"
            if name != "snapshot-00.ready":
                assert not (target / "snapshot-00.ready").exists()
    for name, executable, enabled in [("feature-off", ordinary, True), ("no-output-requested", binary, False)]:
        target = out / name
        with process(executable, trace, target, enabled=enabled) as (session, report):
            opening(session, trace, report)
            session.inject(REQUEST)
            started, _ = travel(session, b"")
            await_state(session, None, trace["opening"]["state"], started,
                        expected=trace["opening"]["expected"], timeout=120)
            assert not list(target.iterdir())
            report["quit"] = session.quit()
    return {"status": "complete", "rejected_storage_corruptions": rejected,
            "checks": "startup snapshot with settled-view check, 8 stable snapshots, bounded 9th rejection, three collision boundaries, feature off, no output requested"}


def replay(binary, trace, out, tours=3):
    with process(binary, trace, out) as (session, report):
        opening(session, trace, report)
        previous = None
        for tour in range(tours + 1):
            if tour:
                width, height = 160, 50
                for step in trace["steps"]:
                    action = step["action"]
                    if "travel" in action:
                        started, _ = travel(session, bytes.fromhex(action["travel"]), width, height)
                    else:
                        if "resize" in action:
                            width, height = action["resize"]
                        started, _ = session.inject(action)
                    await_state(session, None, step["state"], started, expected=step["expected"], timeout=120)
                    report["checked_views"].append({"tour": tour, "label": step["label"]})
            expected = trace["opening"]["expected"] if not tour else trace["steps"][-1]["expected"]
            current = analyze(*observe(session, out, tour, expected))
            if previous is not None:
                assert previous["instances"].items() <= current["instances"].items(), "Retained engine changed or disappeared"
                new = current["instances"].keys() - previous["instances"].keys()
                current["new_inner_instances"] = len(new)
                current["new_strategy_estimate_bytes"] = sum(current["instances"][key]["approximate_strategy_bytes"] for key in new)
            report["snapshots"].append(current)
            previous = current
            print(f"Checkpoint {tour}: {current['inner_instances']} inner engines; all {len(report['checked_views'])} views checked", flush=True)
        report["quit"] = session.quit()
    return {"status": "complete", "checked_views": len(report["checked_views"]), "snapshots": len(report["snapshots"])}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("trace", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--controls", type=Path, help="Ordinary executable for negative controls; no tours")
    parser.add_argument("--tours", type=int, choices=(1, 3), default=3,
                        help="Whole memory tours; no timing or ordinary RSS claims")
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    report = {"status": "running"}
    try:
        if not __debug__:
            raise RuntimeError("Owner census requires Python assertions; do not use -O or PYTHONOPTIMIZE")
        environment = {name: os.environ.get(name) for name in [
            "MallocStackLogging", "MallocStackLoggingNoCompact", "MallocScribble", "MallocGuardEdges",
            "MallocNanoZone", "DYLD_INSERT_LIBRARIES", "NATIVE_DIFF_DELIVERY_TRACE",
            "NATIVE_DIFF_REGEX_CENSUS", "NATIVE_DIFF_OWNER_CENSUS"]}
        for name, value in environment.items():
            if value is not None and name != "MallocNanoZone":
                raise ValueError(f"Unset inherited instrumentation: {name}")
        trace = json.loads(gzip.decompress(args.trace.read_bytes()))
        assert trace["kind"] == "memory" and trace["steps"] and not trace.get("error")
        assert digest(args.reference) == trace["style_check"]["reference_sha256"]
        assert digest(ROOT / trace["fixture"]["local_path"]) == trace["fixture"]["sha256"]
        hashes = bindings(args.binary, args.reference)
        for path in [args.trace, ROOT / trace["fixture"]["local_path"], Path(__file__),
                     ROOT / "tools/regex_census.py", ROOT / "evidence/owner-census-v1/PLAN.md"]:
            hashes[str(path.resolve())] = digest(path)
        if args.controls:
            hashes[str(args.controls.resolve())] = digest(args.controls)
        write_json(out / "declaration.json", {"started_ns": time.time_ns(), "sha256": hashes, "environment": environment,
                   "mode": "controls" if args.controls else "three-tour-owner-census" if args.tours == 3 else "one-tour-owner-census",
                   "tours": 0 if args.controls else args.tours, "ordinary_performance": False})
        report = controls(args.binary.resolve(), args.controls.resolve(), trace, out) if args.controls else replay(args.binary.resolve(), trace, out / "process", args.tours)
        for path, expected in hashes.items():
            assert digest(Path(path)) == expected, f"Bound source changed: {path}"
    except BaseException as error:
        report.update(status="failed", error=str(error), error_type=type(error).__name__)
        raise
    finally:
        write_json(out / "report.json", report)


if __name__ == "__main__":
    main()
