# Checkpoint retention: decision and remaining qualification

Date: 2026-09-23. Status: retention adopted after matched diagnostics, ordinary comparison, and normal-build qualification.
Basis: [the completed local comparison](LOCAL_SOURCE_REPLAY_COMPARISON.md).
Findings: [the retention investigation](LOCAL_SOURCE_RETENTION_REPORT.md).
Implementation and measurements: [the prototype report](LOCAL_SOURCE_RETENTION_PROTOTYPE.md).
Opening attribution: [the diagnostic report](LOCAL_SOURCE_OPENING_REPORT.md).
Git completion: [the independent notification report](LOCAL_SOURCE_GIT_WAKE_REPORT.md).
Decision: [the retention adoption report](../../LOCAL_SOURCE_RETENTION_DECISION.md).

## Completed decision

The renewed comparison used identical Git-wait code in both roles. Retention was the only runtime difference.
All 72 targeted diagnostic views and 198 ordinary views passed. The normal rebuild then passed 120 checks and 33 reference views.
The interrupted memory observation remains preserved with an explicit resumption declaration. No acceptance rule changed.

The policy reduces repeated parsing and large-source recoloring without increasing cache limits.
Adoption accepts the observed smaller-workload costs. It does not claim faster opening, lower total memory, or universal improvement.
The targeted diagnostic separates Enter from gap selection and distinguishes first arrivals from warm revisits.
Cached layout and resize actions issue no coloring requests. Their costs are not direct checkpoint-planning work.
Earlier adverse observations remain preserved, including opening differences without a complete causal explanation.

## Next checkpoint

1. Preserve the adopted ordinary executable and source identity before any new runtime change.
2. Extend the existing source-aware reference checks into one declared interaction trace.
   Include actual viewport movement, repeated context expansion, revisits, and width changes during active coloring.
   Include many-file source updates and pressure across the shared cache budgets.
3. Check exact source, colors, reader position, cancellation, and cleanup before accepting timing observations.
   Keep source correspondence and independent sequential colors authoritative.
4. Observe first text, completed coloring, responsiveness, and sustained process memory separately.
   Use the existing capture tools and offline validation. Preserve every attempt and adverse observation.
5. Keep the current-build five-patch campaign and physical terminal checks as separate qualification work.
   Complete local qualification before extending source acquisition to GitHub.

No additional cache, budget increase, or scheduling change is selected at this checkpoint.
The remaining sections preserve the original experiment design, not active instructions to repeat that investigation.

## Original prototype design

The real-worker and terminal traces reproduce eviction of reusable states during the current request.
The large sparse case reaches the entry limit, not the aggregate byte limit.
Dense requests still reuse saved states. A larger allowance is not the selected next step.
All production source and the ordinary executable remain unchanged.

The first prototype makes checkpoint admission aware of the current complete-source file job.
It will protect existing compatible states that the request can reuse before an eviction decision removes them.
Protection ends with that file job, including cancellation and error paths.
Unprotected states retain the existing eviction order.
If all eligible entries are protected, admission declines rather than exceeding either allowance.

This remains a candidate policy, not an accepted improvement across workloads.
It can reduce new coverage while protecting old coverage.
It does not solve arbitrary pressure from unrelated files or guarantee a maximum replay distance.
An ordinary hit-based recency change alone does not address states evicted before their first lookup.

## Prototype contract

The protection decision uses only immutable source identity and requested source coordinates.
It must distinguish old/new sides and preserve exact parser state.
It must not predict destinations from a filename, language, workload, or benchmark.
It must not clone protected states into a second cache or keep evicted states outside accounting.
Any additional metadata must be bounded and included in the declared accounting.
Count, aggregate-byte, and per-entry allowances stay unchanged.
Patch-only continuation, source text, diff pairing, syntax output, and public controls remain unchanged.

The first implementation stays inside the existing highlighting worker and checkpoint module.
No new dependency, background worker, cache setting, or source-acquisition behavior is required.

Before adoption, the pressure probes must include requested lines inside divergent old/new syntax states and later syntax transitions.
The earlier investigation's retention probes did not cover that full condition.
The completed prototype adds those cases; its report records the results.
Existing exact-color, replacement, cancellation, oversized-state, and real-process contracts remain acceptance gates.
The byte-pressure behavior needs a meaningful real-parser check, not an invented oversized mock state.

An isolated Enter action will measure expansion after gap selection settles.
Cold endpoint acquisition and warm endpoint revisits will have separate labels.
Feature-gated boundaries will separate input handling, rendering/output, source preparation/application, and coloring.
Deeper timers are justified only after that partition identifies expensive work.

## Questions

The large local-source case reaches the checkpoint entry limit and repeats all hidden parsing on theme changes.
Smaller cases retain useful checkpoints and reduce repeated parsing. Some navigation and expansion observations are slower despite correct output.

The next checkpoint must explain both outcomes before another storage or scheduling change.
It must not infer a runtime regression solely from a shared-workstation timing difference.
It must not dismiss an adverse observation solely because it is small.

## Ordered work

1. Attribute retained checkpoint coverage and eviction during real source requests.
   Reuse existing traces first. Add only the diagnostic identity fields necessary to distinguish source side, saved position, and request generation.
   Keep diagnostic code absent from ordinary builds.
2. Reproduce the covered patterns with real source workers and sequential color references.
   Include sparse and dense requests, backward travel, interleaved files, side-length differences, and theme changes.
   Cover cases within and beyond both entry and byte limits. Include large parser states and checkpoint rejection.
3. Compare bounded retention or admission strategies only after the failure mechanism is reproducible.
   Prefer a small change to the existing cache over a new cache, worker, or public setting.
   Retain source identity, exact parser state, cancellation checks, and the current count and byte allowances.
4. Isolate the slower Git endpoint, context expansion, and real-source layout observations.
   Separate acquisition, text delivery, coloring, and layout work with the minimum necessary attribution.
   Freeze matched ordinary actions before a short AC-powered comparison. Preserve all results and background conditions.
5. Qualify an implementation only if correctness passes and its resource tradeoffs are understood.
   Keep text readiness, complete coloring, responsiveness, peak memory, and retained memory separate.

## Non-negotiable behavior

The optimization must not depend on filenames, languages, fixture identity, expected destinations, or timing targets.
It must not skip an unknown source prefix or reuse an incompatible parser state.
Source bytes, line identities, diff pairing, wrapping, layout, and exact old/new colors remain authoritative.

Losing a saved checkpoint must remain safe. It can increase replay work, but it must not change output or block input indefinitely.
No policy promises a fixed replay distance for every file or grammar.
No cache allowance becomes a total-process memory guarantee.

## Acceptance and limits

Real-worker evidence must show correct tokens and the actual work performed, including evictions and rejected checkpoints.
Real-process checks must preserve navigation, context expansion, wrapping, resizing, cancellation, and cleanup.
The ordinary comparison must include any workload that becomes slower or retains more memory.

A demonstrated benefit on the 12,000-line case alone is insufficient.
The decision must account for other request shapes and pressure patterns, without enlarging the declared cache limits.
If no small policy improves the tradeoff, retain the existing implementation and document the limit.

The subsequent resource campaign must include repeated expansion, revisits, actual scrolling, cache pressure, and width changes during active coloring.
It must retain source-checked views and separate raw capture from expensive validation.
The existing five-patch campaign remains separate from the complete-source workloads.
The initial attribution and prototype checkpoints are complete.
The measured opening differences remain an adoption gate.
