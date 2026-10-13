# Cross-grammar compiled-pattern sharing

Date: 2026-09-13. Scope declared before implementation and measurements.

## Design

The completed census found 827 live compiled cells for 563 exact static patterns.
Share those cells during grammar initialization, using one table owned by a syntax
set and references/clones descended from it. Unrelated sets remain independent.
Do not change regex matching or add a lookup to its hot path.

Keep `Regex` as an exact pattern string and `Arc<OnceCell<compiled engine>>`.
Change the existing interning table's values to weak cell references. On an exact
key hit, upgrade the weak reference and reuse the cell. On a missing or expired
entry, register the grammar's current cell. No pattern is compiled by interning.

Each `SyntaxReference` has an optional, serialization-skipped shared table owner.
Building a set attaches one `Arc<Mutex<RegexCache>>` to all its references. A
custom deserializer on the set's syntax vector does the same after reading that
vector, without loading its context bodies. Standalone reference deserialization
has no shared owner and retains temporary per-grammar interning.

During lazy context loading, deserialize outside the table lock, intern stored
match patterns under the lock, then release it before publishing contexts. The
existing `OnceCell` compiles on first actual use. Concurrent grammar loading
selects the same cell; concurrent first use compiles through that cell. No
compilation, matching, or nested grammar lookup occurs under the table lock.

Cloning preserves the family table and existing compiled-cell ownership.
Serializing omits runtime sharing state. Deserializing or rebuilding via a
builder makes a fresh family. A dropped final strong regex owner destroys its
compiled value even if the table remains. Weak entries retain allocation headers,
keys, and table capacity until replacement or family release. Count that overhead;
do not describe weak references as zero-retention. The table is bounded by the
distinct static patterns in loaded grammars, not by viewed source or requests.

## Correctness boundaries

Preserve pattern bytes, compiler options, grammar assets, rule order/actions,
parser search-cache identities, separate old/new state, caller-owned captures,
error behavior, and lazy initialization. Source-derived backreference expressions
never enter the table. Keep source/projection storage, UI behavior, fallback and
highlighting limits, caches, and measurement targets unchanged. No global
registry, unsafe Rust, engine substitution, or input-specific behavior.

## Ordered verification and measurement

1. Preserve and reproduce the current ordinary control in clean target
   directories before editing. Preserve the candidate separately using the same
   plain release recipe. Keep all historical artifacts and failed preparations.
2. Add real grammar/capture controls for sharing across grammar loads, rule
   actions, concurrent initialization/searches, independent captures, clones,
   serialization/rebuild isolation, standalone references, and final-owner
   release while the table remains alive. Update the census expectation for
   within-family sharing, retaining an independent-set negative control.
3. Run existing library/worker/terminal contracts, formatting, application Clippy,
   complete five-patch source checks, and the existing projection evaluator.
   Keep independent source/style answers and existing opt-in exclusions.
4. Run exactly one stack-logged ordinary control process and then one ordinary
   candidate process on the existing showcase memory trace, each with opening
   and three tour-end snapshots. Use the existing allocation tools and exact
   frozen views. Record allocator environment. Compare named engine allocation
   classes and whole live heap, including table/key/weak-cell overhead. A lower
   RSS value alone cannot pass this gate. Stop if ownership attribution fails.
5. If correctness and net ownership reduction pass, run the existing capture-first
   correctness gate and five alternating ordinary pairs over its unchanged cases.
   Keep every observation and decoder exclusion; do not claim physical paint.
   A material new regression blocks the next gate and requires attribution.
6. If the focused comparison supports continuing, characterize the candidate once
   on the unchanged five-patch workload: checked captures against the original
   sequential style oracle, 25 latency processes, and five three-tour memory
   processes using the current RSS observer. Stop and retain a failed batch.

Builds, tests, profiles, and viewer measurements run serially. Read-only review
can overlap them. Freeze commands and input hashes before each measurement gate.
No repeated performance series to obtain preferred results. A source repair gets
a new candidate version and declaration. A material owner reduction does not
erase the preceding combined candidate's coloring regression or imply promotion.

No GUI control or external coordination is part of this work.

## Preparation findings

The two clean controls have distinct raw hashes. A read-only Mach-O comparison
found exactly 48 differing bytes, all in the 16-byte UUID field or the embedded
code-signature region. Every other byte, source file, compiler setting, and
dependency feature matches. The original identities remain distinct; neither
binary is normalized or edited. `evidence/cross-grammar-v1/control-identity.json`
records offsets and identities. The first build is the sole measured control.
The fields are identified from [Apple's loader definitions](https://github.com/apple/darwin-xnu/blob/main/EXTERNAL_HEADERS/mach-o/loader.h).

Before either profile, the shared profile helper records checked view labels and
rechecks RSS errors after joining the sampler, preserving evidence if cleanup
fails. Both profiles use this same hash-bound helper. This changes reporting and
failure detection, not their source actions or snapshot schedule.
