# diffs-syntect: parser ownership and search patches

This is Syntect 5.3.0 with local ownership, search, and accounting changes, packaged as `diffs-syntect` for diffs-rs.
The patches retain the existing regex engine, compilation options, grammar data, rule order, and match results.
The package retains its original MIT license in `LICENSE.txt`.

## Provenance

- Published crate SHA-256: `656b45c05d95a5704399aeef6bd0ddec7b2b3531b7c9e900abbf7c4d2190c925`.
- Upstream revision: `e4670846ecf16d8832db6c43d531bec466214e27` in `trishume/syntect`.
- `testdata/` comes from that revision. The published crate omits it, and so does `diffs-syntect`, but upstream library tests require it.
- Its `testdata/Packages` submodule is restored at the recorded `sublimehq/Packages` revision `fa6b8629c95041bf262d4c1dab95c456a0530122`. These are test inputs, not replacement runtime grammars. Other upstream test submodules are not restored. Without the `testdata/spacegray` theme submodule, the upstream unit test `highlighting::theme_set::tests::can_parse_common_themes` fails; the other library, integration and doc tests pass.
- The fork is a member of the diffs-rs workspace. Its own tests (`cargo test -p diffs-syntect`) use the workspace's root lockfile.

The ownership patch changes `src/parsing/regex.rs` and `src/parsing/syntax_set.rs`.
The former also contains ownership and admission tests.
Family propagation also changes `src/parsing/syntax_definition.rs`.
The manifest and `src/parsing/mod.rs` declare the local features and modules.

The line-local search patch also changes `src/parsing/parser.rs`.
Its non-cloneable `LineSearch` groups the existing match cache and region with
temporary Fancy Regex scratch. The scratch reuses VM branch, save-history,
and delegated-slot capacity within one `parse_line` call, then releases it.
It is not part of `ParseState`, checkpoints, compiled regexes, or cloned regions.
Source-derived regexes use the same temporary owner without entering an interner.
The Oniguruma path has empty scratch and preserves its existing search call.
Rule priority, parser-cache identities, owned captures, and miss/error behavior are unchanged.

The optional `shared-engines` feature admits initialized static rules to the
family's pinned Fancy Regex `EngineCache`. Exact compiled fragments can then
share complete engine handles across different outer rules. Captures, rule
actions, and parser state remain separate. Source-derived expressions,
validation, and first-line compilation do not receive admission. Ordinary regex
clones retain the family; deserialization creates an unadmitted regex until
static context initialization. The cache owns only weak engine values.

The optional `regex-census` feature additionally exposes a borrowed snapshot in
`src/parsing/regex_census.rs`. It inspects initialized static cells only, records
owner identities, and does not load grammars, compile regexes, or clone owners.
It is disabled in ordinary application builds. The application-level scope and
real-grammar controls are in `docs/measurements/REGEX_CENSUS_PLAN.md` and `tests/regex_census.rs`
in the diffs-rs repository.

The optional `owner-census` feature adds `src/parsing/owner_census.rs` for the
Fancy Regex backend. It deduplicates initialized outer cells before visiting
their existing inner engines through the pinned dependency's borrowed API.
It reports approximate strategy storage, not scratch or total heap. Inspection
does not load a grammar, compile an expression, clone an engine, or search.

With this feature, `HighlightState::owned_heap_bytes()` also reports style-vector,
scored-style-vector, and scope-stack capacities using checked arithmetic.
It borrows the state without cloning or rebuilding it. This count excludes the
inline value, allocator overhead, and shared syntax/theme storage. The application
uses it for idle-worker diagnostics only, not for cache admission.

The optional `syntax-timing` feature records grammar and outer-regex initialization
inside their lazy initializer closures. Fixed-size thread-local intervals report
counts, elapsed time, maxima, and failures. Source-dependent pattern construction
has a separate count. The observer retains no source text or engine owner.
Request totals include parser-call subsets, so those totals are not additive.
Outer-regex initialization can reuse an inner engine. It is not a count of new engines.
Normal application builds do not enable this feature.

## Required-byte rejection

The ordinary Fancy Regex path also uses a conservative search filter.
The pinned dependency derives ASCII bytes that every match requires somewhere in the complete input text.
`LineSearch` computes the bytes present in the complete line once per `parse_line` call.
If a required byte is absent, the parser skips that capture search.
Otherwise, the existing capture search runs unchanged.

The filter uses the complete line, not the suffix after the current search position.
This distinction preserves lookbehind requirements.
The filter applies to static and source-derived expressions. Source-derived expressions retain their existing exclusion from the match cache and interners.
The predicate initializes the existing lazy compiled cell. It does not avoid regex compilation.
The Oniguruma path always permits the search.

The dependency's public capture methods do not use this filter and retain their runtime-error behavior.
Syntect retains its existing conversion of capture errors to search misses.
The application checks necessary-byte masks, real captures, character-boundary searches, conservative fallback, and public errors in `tests/required_bytes.rs`.
These checks are in the application suite, not the standalone Syntect suite.

## Checkpoint accounting

`ParseState::owned_heap_bytes()` counts owned stack, prototype, captured-text, and capture-region capacities.
`ScopeStack::owned_heap_bytes()` includes scope vectors saved behind nested clears.
Both methods use checked arithmetic and exclude the inline value, allocator overhead, and shared repositories.
An uncountable capture region returns `None`. This includes the Oniguruma region path.
The application uses these methods to admit or reject parser checkpoints, not to estimate total process memory.

The accounting changes affect `src/parsing/parser.rs`, `scope.rs`, and `regex.rs`.
Their tests cover captured delimiters, prototypes, nested clears, region growth, and independent clones.
The application checkpoint tests cover cache admission, rejection, eviction, and restored parser state.

## Ownership

Each regex owns an `Arc` to its lazy compiled cell. A syntax set attaches one
mutex-protected map of exact pattern strings to weak cell references to its
grammars. Context deserialization occurs outside that lock. Interning occurs
before publication, without compiling patterns or locking during matching.
Source-derived regexes never enter the map. Unrelated sets remain independent.

Set/reference clones retain family affinity. Deserialization and builder output
create fresh families without eagerly loading grammar contexts. A standalone
reference deserializes without a family and uses temporary local interning.
The map's weak values do not retain compiled values after their final strong
owner drops. Keys, table capacity, and expired cell allocation headers remain
until replacement or family release; the map is bounded by loaded static patterns.

Match rules remain distinct objects with their original captures, scopes, actions, and parser-cache identities. Old and new source states remain separate. A search still writes to the caller's capture region. The compiled Fancy Regex wrapper has its own thread-safe internal search-cache pool; sharing the wrapper also shares that pool.

Source-derived expressions, such as heredoc closing delimiters, are created after grammar initialization. They do not enter the temporary map. Cloning any regex now shares its lazy or compiled cell instead of resetting compilation. The last owner releases the compiled object. Serialization and equality still use only the pattern string.

## Acceptance

Check real captures, concurrent searches, distinct rule actions, clone/serialization behavior, and final-owner release. Check viewer source and terminal-cell contracts, including different heredoc terminators on old and new sides. Compare large-patch cells to the preserved, unpatched sequential highlighter.

Measure allocation attribution separately from ordinary release performance. Grammar initialization now pays for temporary string hashing and reference-counted cells. Accept the patch only if real workload memory falls without a correctness failure or a material responsiveness regression. This change does not establish a total-memory bound.

## Packaging changes

These affect only how the fork is built and published, not its library code.

- The package is renamed and versioned independently (see `README.md`). The library
  keeps its upstream name, so `use` paths, tests, examples and doctests are unchanged.
- The manifest is the upstream `Cargo.toml.orig` with the fork's name, version,
  description, repository, `include` list and `rust-version`. Dependents rename the
  package back to the upstream dependency name with `package = "..."`.
- Upstream lints are allowed in the manifest, because diffs-rs runs Clippy with
  `-D warnings` across its workspace and the upstream code is not changed to satisfy it.
- Upstream benchmarks and their `criterion` dev-dependency are removed.
- The Fancy Regex dependency is `diffs-fancy-regex`, renamed to `fancy-regex`, so the
  `fancy-regex/...` features and `fancy_regex::` paths resolve to the fork.
- The upstream `public_api` test, its snapshot, and its `public-api`, `rustdoc-json`
  and `rustup-toolchain` dev-dependencies are removed. The fork's public API
  intentionally differs from the snapshot, and the test installs a nightly toolchain.
- Upstream's GitHub workflows, `.gitmodules` and `[profile]` settings are removed.
  Workspace members cannot set profiles.
