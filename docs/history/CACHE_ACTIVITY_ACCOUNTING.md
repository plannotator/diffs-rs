# Style-cache activity accounting

Qualified 2026-09-26. This change affects only the opt-in `owner-census` diagnostic feature and its reader.

Source readiness, replacement, and revocation already removed affected styles. The diagnostic activity ledger did not count those removals. Its admission-minus-eviction equation could therefore reject a valid complete-source cache.

## Contract

Activity schema 2 adds seven cumulative counters for one cache instance:

- Admitted and budget-evicted payload bytes.
- Invalidation calls, removed entries, and removed payload bytes.
- Usable map-capacity increases and decreases during invalidation.

The ledger reconciles entries, payload, and usable map capacity with the observed cache. Invalidation is separate from budget eviction. An invalidation call with no matching entries increments only the call counter. Existing peaks remain high-water observations; replacing the cache, such as on a theme change, starts a new ledger.

Counters add seven `usize` fields, 56 bytes on the tested arm64 target. They retain no source, style, or event collections. The existing last-admission records remain bounded. Default builds contain neither the counters nor the observations around invalidation.

Payload means the existing style charge: inline values plus owned vector capacity. Usable hash-map capacity is not an allocation-size measurement. This ledger does not partition process RSS or measure allocator retention.

The reader keeps activity schema 1 under its original equations. It does not infer missing removals or promote historical evidence to invalidation-qualified evidence. Schema 2 requires the new counters. Corrupted or unsupported records are rejected.

## Verification

The new engine test uses real patch storage, captured full files, checked correspondence, the syntax worker, and `App::set_source`. It checks plain-to-complete readiness, replacement with different hidden syntax context, revocation, repeated empty removal, recovery, stale replies, unchanged unrelated-file styles and FIFO order, theme reset, canonical source stability, and source-owner release.

Ten emitted snapshots reconcile. The reader rejects 40 deliberately corrupted versions and accepts three supported historical records, including records before activity accounting and both source-key widths. An older record without the later style-storage schema is outside this reader's existing contract and remains rejected.

The existing budget-eviction tests verify admitted and evicted payload totals. The targeted diagnostic checks passed. The ordinary suite passed all 124 enabled checks, including 61 real-terminal checks; four opt-in checks remained excluded from that suite. The compatible-baseline check ran separately and matched 96 complete terminal grids and two canonical source streams.

Seven real-process census controls passed: startup, bounded repeated snapshots, three output-collision cases, feature-disabled behavior, and no output request. They checked unchanged settled cells and process cleanup. Formatting and strict application Clippy passed. The pre-existing vendored Syntect lifetime warning remains.

The verification bundle (temporary verification record `2026-09-26-invalidation.I2cuvL/report.html`, not retained) preserves the plan, build recipe and source archive, logs, real snapshots, reader controls, and review. The normal executable remains byte-for-byte unchanged at `53d28a87bd1669910adfc6f28aedb20063ddb9db270310ad056da5a20a3bd838`.

## Next investigation

Use the corrected diagnostics for a bounded complete-source ownership capture: initial checked view, expanded context, a deep visit, and a revisit. Keep source bytes, checkpoints, cached styles, and process RSS separate. The public CLI uses pinned sources; engine-level replacement qualification does not add a reload command.

First locate the retained owners, then decide whether a storage or lifetime change is justified. No new memory optimization, cache-policy change, performance comparison, or physical terminal acceptance is claimed here.
