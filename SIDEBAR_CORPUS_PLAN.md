# Sidebar-aware large-patch qualification

Date: 2026-09-23. This document fixes the scope. The report records completion.

## Scope

The viewer and normal executable remain unchanged. This work updates the existing
independent terminal checker for the default sidebar. Historical packets retain
their original helpers, geometry, and measurements.

The workload retains the five pinned patches, navigation categories, and source
targets. The sidebar remains in its default automatic mode. At 160 columns, a
visible sidebar leaves 127 diff columns. At 100 columns, it leaves 67 columns.
Below 97 columns, the sidebar hides. Explicit split mode remains split.

## Checker changes

1. Use one declared geometry function for headers, panes, gutters, source records,
   wrap destinations, syntax colors, and corruption controls.
2. Keep source-record coordinates relative to the whole terminal. Keep complete
   screen captures, including the sidebar, for settled-state comparisons.
3. Derive the default expanded tree independently from raw patch paths and hunk
   counts. Check visible names, hierarchy, statuses, totals, current-file marker,
   clipping, and the divider. Reject unsupported focused or collapsed states.
   When no source cursor is visible, multiple visible files can make current-file
   identity unobservable. Check the tree against that independently decoded set.
   Record this weaker scope per view. Do not claim exact current identity there.
   Even a known source identity proves tree-slice consistency, not unique identity
   from truncated tree labels alone.
4. Preserve the old no-sidebar decoder when the trace declares no sidebar.
5. Bind new geometry, helpers, executable, actions, and expectations before any
   accepted measurement. Reject old freezes in the new measurement mode.

## Evidence

The short preparation uses the actual viewer and actual captured cells. It covers
split and unified layouts, wrap, both sidebar widths, automatic hiding, and file
navigation. Deliberate corruption must fail at both nonzero and zero diff origins.
The checks must distinguish source, sidebar, separator, and color failures.

After this preparation passes, each large-patch trace needs independent source
and sequential syntax checks. Memory traces need wrap offsets from the declared
diff width, not the whole terminal width. Keep every failed attempt and explain
each driver correction before a replacement run.
The horizontal limit comes from independently decoded visible source lines and
their available widths. A dedicated corrupted-source control checks this path.

## Measurement boundary

Preparation can perform expensive validation while the viewer runs. It produces
no accepted timing values. Performance capture must keep expensive validation
outside the timed input/output path.
Frozen region preparation occurs before process launch or input injection.
The retained runner still parses terminal output and compares frozen cells live.
Its validation overhead remains visible. This is not a capture-only measurement.

The automatic sidebar can appear during loading. A final-layout match does not
prove the first earlier usable text. Report final-viewport receipt as an upper
bound unless separate progressive-opening expectations establish that milestone.
Tree publication must not redefine syntax completion as full index completion.
Full-screen settled correctness remains mandatory even when a diff-only milestone
finishes earlier.

The normal executable, AC power, serial schedule, full trial retention, and uniform
development policy remain required. Physical terminal paint and appearance stay
separate from PTY receipt. No runtime optimization belongs in this checkpoint
unless the evidence establishes a correctness defect.
