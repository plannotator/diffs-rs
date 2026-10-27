# diffs-fancy-regex: scoped engine sharing, search scratch, and required-byte metadata

This is `fancy-regex 0.16.2` with local patches, packaged as `diffs-fancy-regex` for diffs-rs. The published archive SHA-256 is
`998b056554fbe42e03ae0e152895cd1a7e1002aec800fdc6635d20270260c46f`.
The upstream MIT license remains in `LICENSE`.

`SearchScratch` gives a caller ownership of temporary VM branch, save-history,
and delegated-slot vectors. `captures_from_pos_with_scratch` borrows those buffers.
Each search clears their logical lengths and constructs fresh VM state.
Successful captures retain their existing owned save vector, independently of scratch.
The existing capture API creates temporary scratch and uses the same VM implementation.
Matching options, instruction order, errors, limits, and compiled engines are unchanged.
The type works with alloc/no-std and has checked capacity accounting.
Scratch lifetime belongs to the caller. Syntect releases its instance after each parsed line.

The optional `shared-engines` feature requires `std` and adds caller-scoped
`EngineCache` admission through `RegexBuilder::build_with_cache`. Keys contain the
exact cooked expression, all syntax settings, and both optional compiler limits.
Weak values share complete meta-engine handles and their synchronized search
pools. Compilation occurs outside the lock with a publication recheck. Captures
and VM mappings stay private. Ordinary regex cloning still makes a separate pool.
No default feature changes; compilation without admission and the alloc/no-std
route remain available. VM delegates retain their original default options.

The optional `owner-census` feature adds a borrowed visitor in
`src/owner_census.rs`. It exposes existing inner engines, their compiled
expressions and capture mappings. It does not change compilation, matching,
cloning, pool behavior, or default features. The ordinary build excludes it.

Engine instance identity is not allocation identity. Cloned engines can share
strategy storage. References to the same cached handle must be deduplicated before
summing strategy estimates. Syntect does not clone inner handles during parsing.
Search-buffer bytes are unavailable: the private pool has no safe inspection
API that leaves its state unchanged.

The fork is a member of the diffs-rs workspace. Its own tests
(`cargo test -p diffs-fancy-regex`) use the workspace's root lockfile.

## Required-byte metadata

`Regex::required_ascii_bytes()` exposes a `u128` mask of necessary ASCII bytes.
The compiler derives the mask from the parsed expression before optimization.
Bit `b` requires byte `b` somewhere in the complete input text, not necessarily after the search position.
Zero means that no requirement was derived. The mask does not predict captures or search errors.

Case-sensitive literals contribute their ASCII bytes. Concatenation combines requirements, and alternation retains only requirements common to all branches.
Positive lookarounds contribute requirements. Optional repetitions and negative lookarounds contribute no required bytes of their own.
Conditional and subroutine constructs use conservative fallback.
Subroutine and self-recursion flags disable mask derivation. Compilation still reports unsupported expressions through the existing errors.
Case-insensitive literals do not contribute literal-byte requirements.

The public matching and capture methods do not reject searches with this mask.
Backtracking limits and other runtime errors remain observable through those methods.
Syntect separately uses the mask to skip searches that cannot match its complete input line.
This metadata is available in ordinary builds and is independent of the inspection features.

The change adds `src/required_bytes.rs` and modifies `src/lib.rs`.
The diffs-rs tests in the repository's `tests/required_bytes.rs` check real captures at each character boundary, lookarounds, Unicode, conservative fallback, and public errors.
Standalone Fancy Regex tests alone do not cover that application contract.

## Changed files and maintenance

The local runtime changes affect `src/lib.rs`, `src/compile.rs`, and `src/vm.rs`.
They add `src/engine_cache.rs`, `src/owner_census.rs`, and `src/required_bytes.rs`.
The original license and notice files remain unchanged.

Dependency updates need the upstream suite and the diffs-rs required-byte, search-scratch, engine-sharing, and full-source coloring checks.
Capture equivalence and error behavior remain requirements even when the application rejects an impossible search before calling the matcher.

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
- Upstream's `codecov.yml` is removed.
