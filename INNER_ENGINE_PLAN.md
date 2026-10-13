# Scoped inner-engine sharing

Declared 2026-09-13 before runtime edits or measurements.

Share identical inner compiled regex handles inside one syntax-set family. The
owner census motivates this work; its 74.21 MiB estimate is not an acceptance
target or a promised saving. Rendering/source correctness remains the gate.

## Ownership and identity

Add an explicit, optional `EngineCache` to the pinned Fancy Regex dependency,
behind a `shared-engines` feature requiring `std`. Leave the alloc/no-std route
available. Native-diff enables sharing through its pinned Syntect dependency.
The existing syntax-family regex registry owns one cloneable cache. Static rules
receive that cache during context initialization. Source-derived substitutions,
validation-only compilation, and first-line detection remain outside admission.

Cache identity is the exact cooked fragment, all eleven syntax settings, and the
two optional compiler size limits. Use typed equality, not debug strings. Keep
`None` and explicit values distinct. Other meta-engine settings are fixed by the
pinned compiler. Wrapped expressions use their builder settings; VM delegates
retain the compiler's existing default settings. Pass only the new cache into
that compiler, never substitute outer settings or change its instructions.

Store weak engine handles. Look up under the cache lock, compile outside it, then
recheck before publication so concurrent successful misses converge. Do not cache
errors. Keys and weak headers last until replacement or cache-family release;
count that overhead. No global registry, eviction policy, or unsafe access.

Share full `Arc<meta::Regex>` handles. The engine's existing synchronized scratch
pool is shared, but each search checks out its own temporary state. Captures,
VM slots, old/new parser state, and delegate-to-outer capture mappings stay
independent. A small private wrapper preserves public regex cloning: ordinary
`Clone` still invokes meta-regex cloning and receives a separate pool. Only cache
hits share a handle. Matching call sites need no registry lookup or new lock.

Compiled rules need not own the cache. Lazy static rule objects keep its family
available for later compilation. Deserialization/rebuilding makes a fresh family;
clones keep their existing family. Final engine release must work while the cache
remains alive. No grammar, threshold, source/index, navigation, or style-cache
policy changes are included.

## Verification sequence

1. Use preserved ordinary `18177558…` as the sole control; verify its source and
   binary before edits. Preserve a new ordinary candidate and separate diagnostic
   build with the same release recipe. Retain all failures and source identities.
2. Exercise real wrapped/VM captures, unequal outer capture mappings, nonzero
   offsets, Unicode/lookaround/backreferences, configuration and error isolation,
   concurrent compilation/search, separate families, normal clones, and final
   release through weak entries. Run upstream dependency checks where available.
   Retain existing grammar/serialization/source-derived delimiter controls.
3. Update diagnostic accounting to count a shared inner handle once while keeping
   each reference's instruction and capture mapping. No storage estimate may
   double-count shared references. Run current library, worker, terminal/source,
   formatting and Clippy checks. Compare full five-patch source/projection data
   with the existing independent expectations and preserved evaluator.
4. Freeze and run one ordinary control and one ordinary candidate allocation
   diagnostic, in that order, on the existing 138-action addition-heavy trace,
   opening plus three tours. Use the existing heap/stack tools and exact views.
   Require a net owner reduction including registry overhead; RSS alone cannot
   pass this gate. The pair is not a latency or sustained-memory comparison.
5. If correctness and allocation attribution pass, use the existing capture-first
   gate and five alternating ordinary pairs for focused responsiveness and memory
   observations. Preserve every result. A material unexplained regression stops
   qualification; do not run favorable retries or change workloads to pass.

Each gate is serial. Stop at a failed acceptance gate, diagnose it, and retain the
unpromoted candidate or withdraw it with an explanation. Full five-patch timing
qualification and physical terminal presentation remain separate follow-up work;
this stage does not erase existing coloring or memory uncertainties.
