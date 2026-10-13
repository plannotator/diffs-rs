# Local engine release readiness

Checkpoint: 2026-10-03. No public release is authorized or complete.
The [current project state](CURRENT_STATE.md) owns the overall status.
This page records local build and distribution preparation, not performance acceptance.

## Current local candidate

The local candidate qualifies macOS arm64, not other platforms or a stable published library API.
The standalone viewer and host example share one engine. Hosts retain terminal ownership, as described in [EMBEDDING.md](EMBEDDING.md).

- Both isolated installations, with Rust 1.96.0 and minimum Rust 1.88.0, passed 74 viewer checks and six host checks.
  Each viewer suite excluded ten opt-in cases. The separate host command selected six of those cases.
  The remaining four require pinned corpus or preserved-reference inputs, outside the installation cohort.
- Acquisition used an initially empty private Cargo home. Builds used locked dependencies offline, separate targets, and isolated prefixes.
  Both installations passed canonical-output, startup/error, and system-library checks. Default Rust and Cargo settings stayed unchanged.
  Native compilation used existing Xcode command-line tools. Local-source commands require Git >=2.50, with Git 2.50.1 qualified.
- The installed binaries differ from the measured ordinary binaries. Functional installation evidence does not transfer performance results.
  Bit-for-bit reproducibility is not established.
- The notice inventory covers all 194 locked packages, including development and non-host dependencies, native sources, grammars, and theme material.
  Ten packages need pinned upstream notice supplements. The bundle retains the objc2 Apple SDK licensing caveat without a legal-clearance claim.
- The fresh advisory/yanked scan found no known vulnerability or yanked warning. It retained maintenance warnings for `bincode` and `yaml-rust`.
  The scoped dependency review records their exposure and retention rationale. No scan establishes absence of vulnerabilities or legal clearance.

The installation record (external record `native-diff-production-hardening/INSTALLATION_RESULTS.md`, not in this repository) contains exact source and executable identities.
The dependency record (external record `native-diff-production-hardening/DEPENDENCIES.md`, not in this repository) records the current scan, notices, and bounded native-library assessment.
The resource record (external record `native-diff-production-hardening/RESOURCE_RESULTS.md`, not in this repository) covers the final five-patch, complete-source coloring, repeated-host, and reading-tool cohorts.
Large-input memory, indexing, and complete-search costs remain support limitations. The unchanged targets still apply.

Literal search, source-line selection, exact-byte save, and explicit clipboard copy are implemented.
Native clipboard fidelity and physical-terminal acceptance remain pending. Ghostty is the intended physical target.
Other terminals and multiplexers remain unqualified, outside this local-candidate requirement.
The final package remains pending. No signing, notarization, publishing, global install, or repository initialization is authorized.

## Historical 2026-09-28 local-engine milestone

The preceding milestone passed 61 viewer and two host contracts with both compilers.
Its notice inventory covered 169 packages and retained pinned upstream supplements for two Windows import-library packages.
The prior dependency/install record (external record `native-diff-local-engine-milestone/DEPENDENCIES.md`, not in this repository) preserves those versions, hashes, notices, and limitations.
The milestone closeout (external record `native-diff-local-engine-milestone/CLOSEOUT.md`, not in this repository) records its durable package and review.
Those results do not qualify the changed dependencies, executable bytes, or final package of the current candidate.

## Historical 2026-09-21 local builds

The following sections preserve the earlier checkpoint and its original limits. The older binary and package counts are not current qualification.

| Check | Result | Limit |
| --- | --- | --- |
| Normal Rust 1.96.0 source build | Pass | macOS 26.3 arm64, SDK 26.5, cached dependencies |
| Declared Rust 1.88.0 minimum | Pass | Same host, isolated compiler, empty target directory |
| Isolated installation | Pass with both compilers | No global installation or PATH change |
| Minimum-compiler terminal contracts | 25 passed | Rendering, syntax, geometry, delivery, horizontal movement, and Git paths |
| Minimal environment | Pass | Version, canonical source digest, and nonterminal error checks |
| Runtime library paths | Pass | Only macOS system libraries and frameworks |
| Byte-for-byte rebuild reproducibility | Not established | Different build locations produced different hashes |
| Other platforms and architectures | Not qualified | No support claim from these local checks |
| Physical terminal and multiplexer matrix | Incomplete | Emitted-cell checks do not establish physical display behavior |

The minimum-compiler executable has this SHA-256:

`4844dba661d93ccd0e523b774ee07d0ef0ca144e4ec6ac369b7ef3dbcee8d96c`

The installation produced identical executable bytes.
The normal compiler remains Rust 1.96.0. The normal application executable remains unchanged.
The minimum-compiler build uses no diagnostic features.

## Historical dependency inventory

The locked metadata contains 152 resolved packages, including development dependencies.
Each package has license metadata and top-level notice files in the local sources.
This inventory is not a complete release notice bundle or an advisory scan.

The original archives for all three vendored crates match their documented SHA-256 values.
No published file is missing. All published license and notice files remain unchanged.
The inventory records each changed and added file separately.

| Vendored crate | Existing maintenance record | Remaining work |
| --- | --- | --- |
| Syntect 5.3.0 | [Patch note](vendor/syntect/NATIVE_DIFF_PATCH.md), source provenance, standalone checks | Summary updated 2026-09-22 for required-byte rejection, retained-heap APIs, and affected files. Keep the documented checks during updates. |
| Fancy Regex 0.16.2 | [Patch note](vendor/fancy-regex/NATIVE_DIFF_PATCH.md), source provenance, standalone checks | Summary updated 2026-09-22 for required-byte metadata, public error behavior, and affected files. Keep the documented checks during updates. |
| libgit2-sys 0.18.8+1.9.7 | [Parser correction](vendor/libgit2-sys/NATIVE_DIFF_PATCH.md), unchanged notices | Retain the parser regression checks during updates |

The bundled grammar package supplies notices in addition to its crate license.
The local export contains 66 syntax notice entries from the pinned package's public API.
The export does not select the package's unused theme notices.
Distribution preparation still needs the runtime dependency notices, bundled-component notices, and existing theme attribution.
No dependency, vendored source, or notice changed during this inventory.
The later [local-source hardening pass](docs/history/LOCAL_SOURCE_HARDENING_REPORT.md) updated the two patch notes without changing runtime source or notices.

## Historical release gates

1. Define a patch-viewer release scope from implemented behavior.
2. Record explicit platform, terminal, and multiplexer support from actual checks.
3. Complete current-build correctness and controlled performance qualification.
4. Check source installation with a fresh dependency cache and documented native prerequisites.
5. Keep the vendored patch summaries current. The 2026-09-22 maintenance update completes this documentation step for the current source.
6. Assemble the notices for the exact release artifact.
7. Complete a current dependency advisory review.
8. Produce and check the release artifact, its source archive, and its checksums.

The earlier patch-viewer checkpoint did not include the current reading tools.
The current candidate implements literal search, source-line selection, exact-byte save, and explicit copy. A file picker remains outside this milestone.
Direct Git comparison arrived in the later local-source stage. This earlier patch-viewer build checkpoint does not qualify that feature for release.
This checkpoint does not add those features or mark the broader product specification complete.

## Historical evidence

The local evidence folder is:

`native-diff-battery-readiness/` (external record, not in this repository)

It contains build recipes, source archives, installation checks, terminal logs, dependency inventories, and the grammar notice export.
