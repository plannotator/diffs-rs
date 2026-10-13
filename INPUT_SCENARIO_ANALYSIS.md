# Diff inputs: scenarios, evidence, and source guarantees

Date: 2026-09-22.
Status: research and proposed scope correction. No input adapter was implemented or exercised against an account.

## 1. Decision

The previous exact-source spec covered a useful first implementation slice, not the complete product input model.
GitHub pull requests must be a named planned input, not an incidental future use of local Git.
Local edits, staged changes, other hosting services, and application-supplied sources also need explicit boundaries.
This document covers those boundaries without proposing every adapter for the first implementation.

The renderer must consume a stable review and its source capabilities.
It must not assume that every review comes from a local checkout or two commit IDs.
The same snapshot storage, coloring worker, and context projection can serve each supported input.
Provider-specific acquisition, authentication, and comparison semantics stay outside those components.

## 2. Research boundary

The local Plannotator checkout was clean at commit:

`8f2a8a81a384f1cd39c5f083d3c6fcd35a956422`

The inspection covered input parsing, local source selection, GitHub/GitLab providers, workspace routing, source endpoints, and context augmentation.
Public GitHub documentation supplied the external API contracts cited below.
No authenticated API calls, repository fetches, checkouts, tests, or remote mutations occurred in this research.
Source inspection establishes implemented paths, not successful end-to-end qualification of every path.

### What Plannotator demonstrates

| Observed path | Evidence in the inspected checkout | Lesson for this viewer |
| --- | --- | --- |
| Local Git review | `packages/shared/review-core.ts`, `getFileContentsForDiff` | Staged, unstaged, committed, branch, and working-tree reviews need different source pairs. |
| GitHub review | `packages/shared/pr-github.ts`, `fetchGhPR` and `fetchGhPRFileContent` | Review metadata, patch retrieval, and file content are separate operations. |
| GitLab review | `packages/shared/pr-gitlab.ts`, `fetchGlMR` and `fetchGlFileContent` | Each platform has distinct revision and omitted-content rules. |
| Local PR support | `apps/hook/server/index.ts`, PR initialization | Local materialization is a separate capability, not a prerequisite for every platform read. |
| Multiple repositories | `packages/shared/review-workspace.ts`, `getFileContents` | File identity includes its repository. Equal paths in different repositories are not the same source. |
| Other version-control systems | `packages/server/vcs.ts`, registered providers | Git, Jujutsu, GitButler, and Perforce do not share all commands or capabilities. |
| Static patch review | `packages/server/review.ts`, static source mode and `/api/file-content` | A raw patch has no implicit source repository. Context expansion is explicitly unavailable. |
| UI expansion | `packages/review-editor/components/DiffViewer.tsx`, context augmentation | Full contents and source readiness must belong to the currently displayed diff. |
| Staleness handling | `packages/server/review.ts`, snapshot/fingerprint checks | Source reads must not silently use a newer review state. |

Plannotator is a useful reference for breadth, but not proof that all inputs have complete context.
The inspected local content path has a 5 MiB regular-file limit.
Its GitHub content helper reads the base64 `content` field, whose availability depends on the API response and file size.
Static patches cannot expand. Platform fallback paths can explicitly report incomplete patches.
These are source-mode limits, not reasons to copy the whole application architecture.

### Safeguards we must retain or strengthen

- Separate raw patches from source-backed reviews.
- Bind content requests to the captured review and exact side.
- Preserve old paths on renames and the original comparison scope.
- Expose missing content without turning it into an empty or binary file.
- Reject mismatched source bytes rather than relying only on line-count agreement.
- If the comparison ancestor is unavailable, do not substitute the base branch tip.
- Do not invent a rename similarity percentage or file mode to satisfy a renderer.

The last three requirements are deliberate design choices, not claims of equivalent safeguards in every reference path.
The inspected `patchConsistency.ts` checks line-count displacement, not complete byte correspondence.
The inspected GitHub fallback can synthesize metadata, and its caller can fall back from merge base to base tip.
Our proposed source guarantee is stricter: uncertainty remains explicit until the appropriate evidence exists.

## 3. Scenario matrix

Stages are delivery priorities, not present capabilities:

- **Existing:** current patch viewer, with its documented parser limits.
- **Foundation:** the first local implementation and provider-neutral source contract.
- **GitHub:** the next required source adapter in this feature roadmap.
- **Follow-on:** a named capability that needs its own adapter and acceptance evidence.
- **Unsupported:** no silent conversion into an ordinary two-sided review.

| User input or workflow | Actual comparison/source | Required behavior | Stage |
| --- | --- | --- | --- |
| Patch file or stdin | Only supplied patch records are known | Preserve the input. No repository guessing or automatic network access. | Existing |
| Downloaded GitHub `.diff` | A patch artifact, possibly without a trustworthy revision binding | Same patch-only behavior until exact provenance and matching sources are explicitly supplied. | Existing as a local file or pipe |
| Two regular files | Two captured byte sequences | Generate and expand from those captures, not later live reads. | Foundation |
| Two explicit Git commits | The first pinned tree versus the second pinned tree | Direct comparison. No implicit merge base. | Foundation |
| GitHub PR URL, public, no checkout | Platform PR scope and its pinned old/new source versions | Obtain metadata and matching sources without requiring a working checkout. Report incomplete responses. | GitHub |
| GitHub PR URL, private | Same source contract, with authorized read access | Explicit authentication errors. Never treat access failure as an empty diff or absent file. | GitHub qualification |
| GitHub PR from a fork | Base and head can belong to different repositories | Bind repository identity separately for each endpoint. Do not assume the base repository serves every source object. | GitHub |
| GitHub PR with a matching local repository | Same pinned PR comparison, local objects available | Read matching objects without checkout. Do not use local HEAD or the working tree as substitutes. | GitHub |
| GitHub commit URL | The selected commit versus an explicit parent policy | Ordinary commit uses its parent. Root commits use an empty tree. Merge commits require an explicit parent choice. | Follow-on using the GitHub transport |
| GitHub compare URL | Direct or merge-base comparison, as requested | Preserve the URL's comparison semantics and resolve immutable endpoints. Do not interpret every link as a PR. | Follow-on |
| Staged changes | Captured HEAD versus captured index entries | Index changes after capture must not alter this review. Handle unborn HEAD explicitly. | Follow-on local source milestone |
| Unstaged changes | Captured index versus captured working files | Snapshot source before comparison. Never mix old diff rows with newer working-file contents. | Follow-on local source milestone |
| All uncommitted changes | Captured HEAD versus captured working files | Declare untracked-file inclusion and exclusions. Do not concatenate staged and unstaged patches as one final comparison. | Follow-on local source milestone |
| Changes since a branch point | Pinned merge base versus captured worktree, or versus committed head | Name the selected endpoint. These two views are not interchangeable. | Follow-on |
| Patch plus explicitly supplied matching source | A supplied patch with additional source evidence | Check every supplied and omitted range. If the source describes another comparison, refuse enrichment. | Follow-on |
| Plannotator/Herdr supplies a review | Host-owned patch and immutable side contents or content handles | Accept checked source snapshots without forcing the host through our CLI, Git, or authentication flow. | Foundation contract, integration deferred |
| Multi-repository workspace | Several independent comparison identities | Namespace source identity by repository and file occurrence. Keep each repository's directory hierarchy intact. | Follow-on |
| GitLab MR or enterprise host | Provider-specific revision and API contracts | Reuse the core, not GitHub endpoint assumptions or credentials. | Follow-on |
| Jujutsu, GitButler, Perforce | Provider-specific revision/workspace state | Host-supplied snapshots can bridge the core. Native adapters need separate qualification. | Follow-on |
| Email patch series or stacked review | Several sequential or layer-specific comparisons | Do not bind every repeated file occurrence to one global old/new pair or silently squash the series. | Not yet qualified as a series |
| Combined conflict/merge diff | More than two source sides | Existing explicit rejection remains. An explicitly selected two-tree comparison is a different operation. | Unsupported format |

The core supports capabilities, not a claim that every row above is already implemented.
Network URLs need a supported adapter. An arbitrary URL is not automatically a safe patch or source endpoint.

## 4. GitHub-specific correctness

### The base is not necessarily the base-branch tip

GitHub PRs use a three-dot comparison: the common ancestor and the topic head.
A direct two-tree comparison can include changes that are not part of that PR.
The provider must retain the base tip, head, and actual comparison ancestor as separate identities.
Multiple possible ancestors or an unavailable ancestor require an explicit unresolved state, not an arbitrary selection.
See GitHub's [comparison semantics](https://docs.github.com/en/pull-requests/reference/branches#three-dot-and-two-dot-git-diff-comparisons).

Stacked PR layer scope is not the same as the full stack against the default branch.
Closed or merged PRs also need an explicit captured comparison identity. A synthetic test-merge commit is not automatically the proposed head.
The review records which scope it opened. Changing scope creates a new review snapshot.

### Freeze the review, not just its display name

The proposed GitHub acquisition sequence is:

1. Parse a supported PR URL and bind host, repository identity, and PR number.
2. Read metadata and record immutable base/head identities and their repositories, including any fork repository.
3. Resolve the actual comparison ancestor for those pinned endpoints.
4. Acquire a patch and file inventory for that comparison, with explicit completeness evidence.
5. If a request used a PR number or branch, recheck mutable metadata.
6. Read requested old/new source objects by pinned identity and perform the core correspondence checks.
7. Publish completed source pairs only into the same review snapshot.

A metadata recheck is a race detector, not proof that every response belongs together.
Byte correspondence and file-inventory evidence remain required.
If the PR changes during acquisition, cancel the mixed result and offer a bounded retry or an explicit stale-review state.
Do not enter an automatic retry loop during repeated force pushes.
An already captured review remains readable without automatic polling or refresh.

### API success does not always mean complete content

The GitHub PR files endpoint has a maximum of 3,000 entries, even with pagination.
An omitted per-file patch cannot be interpreted as proof of a binary file or pure rename.
See the [PR files endpoint](https://docs.github.com/en/rest/pulls/pulls#list-pull-requests-files).

The compare endpoint includes at most 300 changed files in its file list.
Commit pagination does not expand that file list.
See the [compare endpoint](https://docs.github.com/en/rest/commits/commits#compare-two-commits).

The contents endpoint changes behavior for files larger than 1 MB and does not support files larger than 100 MB.
It can dereference repository symlinks. It is not an unconditional raw Git-blob reader.
See the [contents endpoint](https://docs.github.com/en/rest/repos/contents#get-repository-content).

The Git blobs endpoint provides raw data by object ID but also has a 100 MB limit.
See the [blob endpoint](https://docs.github.com/en/rest/git/blobs#get-a-blob).
Recursive tree responses can be truncated. The API documents non-recursive subtree traversal for that case.
See the [tree endpoint](https://docs.github.com/en/rest/git/trees#get-a-tree).

These limits belong to particular endpoints, not universal limits on Git or this viewer.
API limits cannot become an undocumented maximum diff size in the renderer.

### Recovery and provenance

If exact local objects exist in a user-selected repository, prefer those objects.
Otherwise, use authenticated platform reads into the same private snapshot storage.
Do not clone a repository merely to color one file.

If platform delivery is incomplete, preserve readable content with a persistent partial-review status.
Recovery can use an explicitly authorized fetch into an isolated Git object store and local comparison of the pinned endpoints.
It must not check out files, move the user's branch, update their refs, or recurse into submodules.
Fetch size, cancellation, disk use, and private-cache lifetime need qualification before this recovery is enabled.
Missing history in a shallow repository is not evidence of a different merge base.

A locally recomputed comparison is labeled as locally computed from those revisions.
It need not have identical boundaries, pairing, or rename choices to the platform patch.
Switching to it creates a new review snapshot. It must not silently replace a supplied patch during expansion.
Known file byte differences must remain complete under the selected comparison policy.

### Authentication and network scope

The initial GitHub adapter can use supervised `gh` read commands for existing authentication and host handling.
This avoids a new token store inside the viewer. The core does not depend on `gh`.
Commands must explicitly use read-only endpoints and methods. No review submission or viewed-file mutation belongs here.
Public reads, private reads, expired credentials, and denied permissions need separate acceptance evidence.
Private or enterprise support is not inferred from a public PR demonstration.

Credentials must not appear in commands, diagnostics, saved responses, or artifacts.
Credentials must not follow arbitrary cross-host redirects or a URL supplied by diff content.
The first URL adapter recognizes supported GitHub PR URLs. Enterprise hosts require an explicit host/authentication policy and separate qualification.
Access failures, rate limits, deleted forks, unavailable objects, and offline operation remain distinct from legitimate absent source sides.
Rate-limit handling follows server retry instructions with bounded retry and cancellation, not request flooding.
See the [GitHub CLI API contract](https://cli.github.com/manual/gh_api) and [API rate limits](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api).

## 5. The common contract needs three separate answers

1. **What comparison is this?** Origin, requested scope, repository identities, pinned endpoint identities, and a review snapshot ID.
2. **Did we receive the whole comparison?** Complete, known partial, or unknown, with the reason and supporting inventory evidence.
3. **Can this file supply exact source?** Each side is available, legitimately absent, pending, or unavailable for a concrete reason.

Successful input parsing answers a narrower question: all received bytes formed a valid patch.
It does not prove that a hosting service supplied every changed file or every change within a file.
Complete source for one visited file does not prove completeness of the entire review.
File and folder totals remain partial when the underlying comparison is partial.
An unknown total is not zero. A missing patch is not a checked zero-change file.

A complete-review claim needs a checked complete inventory for the pinned comparison.
Local Git generation can provide that inventory.
A remote adapter must establish equivalent evidence or keep completeness explicitly unknown/partial.
An immutable tree comparison is one possible source of inventory evidence, but a truncated tree response is not sufficient.
The remote acquisition checkpoint must decide and qualify this mechanism before claiming full GitHub review coverage.
Counts and a successful HTTP response alone do not satisfy that checkpoint.
Completeness evidence must distinguish file-inventory coverage from the coverage of changes within each file.
Do not download every full source at startup merely to obtain a complete-review label.
Keep unresolved coverage explicit while requested files undergo lazy source checks.

Represent these states with small enums and owned descriptors.
Raw patch records and disk-backed source bytes keep their current roles.
Snapshot identity includes repository and side provenance, not merely the filename or PR number.
Each file occurrence can name its own pair of source versions.
No assumption that one global commit pair covers patch series, workspace reviews, or host-supplied comparisons is permitted.

The renderer and highlighter never issue HTTP, inspect credentials, discover repositories, or decide merge-base semantics.
Adapters deliver immutable source handles through the existing bounded preparation path.
New provider support must not multiply source caches or instantiate a highlighter per repository.
Use a small input enum and focused adapters initially. A shared trait is justified only by concrete implemented consumers.

## 6. Cross-cutting cases

| Case | Required policy |
| --- | --- |
| Same path in different repos or patch entries | Distinct identity. No cache or source collision. |
| Rename/copy, including changed extension | Retain both paths and source identities. Choose syntax separately for each side. |
| Symlink | Review the stored link text, not its target. |
| Submodule | Review metadata. No implicit clone or recursive diff. |
| LFS pointer | Review committed pointer bytes unless a separate explicit payload mode exists. No automatic LFS download. |
| Invalid UTF-8, controls, CRLF, missing final newline | Retain exact bytes. Apply the existing safe display transformation without lossy decoding. |
| Binary, generated, or very large file | Never infer classification from a missing API patch. Hidden UI content is not absent source. |
| Whitespace-ignored diff | Record the comparison policy. Byte-different omitted ranges cannot masquerade as ordinary unchanged context. Initial expansion rejects an incompatible mapping. |
| Filters and text conversion | Raw-source and transformed-source comparisons are different. Initial source-backed modes do not execute repository filters. |
| Local file/index changes during capture | Compare captured bytes consistently. Detect ordinary changes, but do not claim a filesystem-wide atomic snapshot. |
| API error, denied access, timeout, missing object | Explicit unavailability. Never turn these into an empty or deleted file. |
| Refresh, PR switch, or scope switch | New snapshot identity. Old results cannot enter its caches or replace its contents. |

## 7. Revised order and qualification gates

1. **Broaden the contract now.** Add comparison identity, completeness, and per-side source capabilities to the design before storage interfaces become fixed.
2. **Build the local foundation.** Keep the planned snapshot storage, two-file/direct-Git inputs, coloring, and expansion stages.
3. **Add GitHub PR acquisition.** Reuse the foundation and qualify public, fork, private, large, and changing-review cases.
4. **Add captured local-change modes.** Qualify staged, unstaged, and uncommitted snapshots separately, including untracked-file policy.
5. **Extend through real demand.** Commit/compare URLs, GitLab, enterprise hosts, workspaces, and application integration use the same core boundaries.

GitHub is now part of the named source-capable roadmap, not proof that stage 2 alone covers production input needs.
The reference application demonstrates useful scenarios. It does not require us to reproduce every provider before shipping any feature.

The GitHub adapter checkpoint must establish:

- A complete file-inventory strategy tied to immutable comparison endpoints.
- Exact patch-to-source correspondence across forks and moving branch tips.
- Raw-byte retrieval, including empty, renamed, symlink, and oversized-file behavior.
- Explicit limits and an authorized isolated recovery path, or honest incomplete status.
- No network or authentication operations on the UI thread.
- Bounded request concurrency, backpressure, cancellation, and retained response metadata.
- A supported CLI/API version and redacted diagnostics.

Real local repositories can prove comparison semantics and source correctness without remote writes.
Pinned public read-only API cases must then exercise actual GitHub delivery.
Private and enterprise tests require an authorized environment. Until then, those capabilities remain unqualified.
Recorded real responses can exercise truncation/error parsers, but cannot replace live transport and authentication checks.
Existing source-corruption challenges and PTY checks remain required at the shared core boundary.

Performance reports separate network acquisition from source preparation, first readable text, coloring, and expansion latency.
Report bytes transferred, request counts, disk use, helper memory, and warmed/offline behavior where applicable.
A fast partial response is not a successful complete-review performance result.
No benchmark campaign or remote test was started by this analysis.
