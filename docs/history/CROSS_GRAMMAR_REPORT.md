# Cross-grammar sharing: implemented, qualification incomplete

Date: 2026-09-13. Ordinary candidate: `da5754e8…`.

## Result

Identical static regex patterns now share compiled engines across grammars in one
syntax-set family. The source/correctness checks pass. One declared allocation
pair shows a net live-heap reduction that includes the sharing table's overhead.
The ordinary responsiveness gate failed on an RSS observation during shutdown,
so no paired timing series or current five-patch sustained-memory cohort ran.
The combined candidate remains unpromoted.

| Allocation snapshot | Reference live heap | Candidate live heap | Net saving |
| --- | ---: | ---: | ---: |
| Opening | 2,609,168 bytes | 2,609,504 bytes | −336 bytes |
| Tour 1 | 224.45 MiB | 186.23 MiB | 38.21 MiB |
| Tour 2 | 275.10 MiB | 213.19 MiB | 61.91 MiB |
| Tour 3 | 275.27 MiB | 213.40 MiB | 61.87 MiB |

These are sequential, stack-logged snapshots of the existing syntax-heavy
showcase trace, not ordinary RSS, latency measurements, or a universal saving.
Both processes matched all 412 frozen views and exited cleanly. The named
`regex_automata`/`fancy_regex` subtotal fell by 49,581,280 bytes, or 47.28 MiB,
at tour 3. That subtotal is a partial engine total. The remaining 15,297,552 bytes
of the whole-heap difference are not assigned by that subtotal.

Whole-heap totals include registry keys, capacity, and weak-reference overhead.
Native stacks identify a 49,152-byte table allocation, and candidate classes
labeled `share_regexes` total 200,624 bytes. Those are visible components, not
an exact complete registry census. Independent review reconciled all heap class
rows to their totals and confirmed the net reduction.

## Implementation and correctness

The production change is confined to two vendored Syntect files. A family-owned
`Arc<Mutex<RegexCache>>` interns exact pattern strings into weak references to
existing `Arc<OnceCell<compiled engine>>` cells. The table is consulted only
during lazy grammar initialization. Deserialization happens outside its lock;
compilation and matching never take that lock.

Clones keep the family; independently built or deserialized sets get separate
families. Serialization omits runtime ownership. Standalone syntax-reference
deserialization retains local sharing. The final strong owner releases the
compiled value even while a weak table entry remains. Keys and weak allocation
headers remain until replacement or family release and are counted as overhead.

Patterns, engine settings, grammar assets, rule ordering/actions, captures,
old/new parser state, source-derived delimiters, fallback policy, source/projection
storage, UI behavior, and cache limits are unchanged. There is no fixture-specific
branch, global registry, unsafe code, new dependency, or matching-path lookup.

Verification completed:

- Nine library checks, 32 terminal/CLI process checks, one worker check, and six
  real-grammar checks pass. Two existing opt-in process tests remain ignored.
- Five private vendored regex tests pass, including real concurrent capture use,
  release while the weak cache remains, and replacement of an expired entry.
- Grammar controls cover concurrent cold/partially warm clones, distinct scopes
  and rule actions, independent families, serialization/rebuilding, and standalone
  references. Existing process contracts cover old/new delimiters, inline spans,
  clipping, wrapping, theme/layout changes, rapid resizing, and cleanup.
- Complete source checks pass on all five patches, including all 19,299,175
  supplied source records in the largest. The existing projection evaluator's
  answers match byte-for-byte: 2,888 accepted structural cases, 716 mutually
  rejected cases, five real patches, and five challenge patches.
- Formatting and ordinary/diagnostic application Clippy pass. The existing
  upstream lifetime-spelling warning remains. No actual terminal pixels were
  inspected, and these checks do not establish every possible diff's correctness.

## Measurement-helper findings

The first profile stopped before its opening check or any allocation snapshot.
Temporarily enabling stack logging in the parent let the sampler's `ps` child
inherit it. The repair passes an explicit environment only to the viewer's
`execve`. A real instrumented opening verifies allocation stacks, clean sampler
queries, unchanged parent environment, the frozen screen, and normal cleanup.
The failed attempt remains; the corrected pair uses the same application binaries,
actions, and snapshot commands under a separately recorded declaration.

Review also found missing post-join sampler-error checks in capture acceptance.
The helper now rejects late errors and writes explicit shutdown evidence in
schema-2 recordings. Replay requires that evidence. An offline audit verifies
rejection of copied real recordings with a sampler error, an unjoined sampler,
or a missing sampler-error field. It does not simulate a live shutdown race.

The first new correctness gate then caught a real shutdown observation: `ps`
reported `81843 81843 0 ?Es+`. The viewer had matched all seven action screens,
returned exit code 0, restored terminal settings, and emitted cleanup sequences.
The `E` flag indicates an exiting process in [Apple's ps source](https://github.com/apple-oss-distributions/adv_cmds/blob/main/ps/print.c).
An unavailable memory value is not counted as zero or silently discarded.

All four visually supported cases have matching action cells and source; three
have successful complete captures and one remains failed. The joined-emoji case
remains source-only. The gate stopped before its built-in fault-control stage.
Separate offline copied-artifact controls passed, but do not convert that gate
into a pass. No ordinary comparison or five-patch checkpoint followed it.

## Next bounded work

Fix the RSS observer's lifecycle before another ordinary series. Scope a bounded
stop-and-join barrier after the last completed viewing action, before injecting
the intentional quit command. Preserve all prior sampler errors; do not accept
unknown process state or synthesize a zero sample. Record the sampling-end
boundary. Keep the quit timer starting at actual command injection, and retain
the OS-reported peak over the full process lifetime.

Failure cleanup must still reap the child even if stopping the sampler fails.
Verify the boundary with real viewer/process-group checks, normal quits, and the
existing safety-stop case. This lifecycle change needs its own declared cohort;
do not relabel the failed capture or repeat runs until a preferred result appears.
Then resume the existing capture gate, five alternating pairs, and conditionally
the unchanged five-patch checkpoint. No further application optimization belongs
ahead of that qualification.

The prior coloring regression, memory goals, and actual Ghostty/Kitty presentation
remain unresolved. The allocation saving does not settle them. No viewer, build,
profile, or measurement from this stage remains running.

## Evidence

- [Design and ordered gates](CROSS_GRAMMAR_PLAN.md)
- [Candidate build](../../evidence/cross-grammar-v1/candidate/build.json) and
  [executable](../../evidence/cross-grammar-v1/candidate/native-diff)
- [Control identity check](../../evidence/cross-grammar-v1/control-identity.json)
- [Allocation declaration](../../evidence/cross-grammar-v2/DECLARATION.md),
  [summary](../../evidence/cross-grammar-v2/allocation-summary.json), and
  [frozen identities](../../evidence/cross-grammar-v2/profile-freeze.json)
- [Retained failed capture gate](../../evidence/cross-grammar-v1/capture-check/report.json)
- [Offline final audit](../../evidence/cross-grammar-v1/final-verification.json)

The clean control builds differ in raw identity only within their Mach-O UUID and
code-signature regions. Both original files remain untouched; the first is the
sole measured reference. Neither file is presented as byte-identical to the other.
