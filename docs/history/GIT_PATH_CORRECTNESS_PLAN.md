# Git rename and copy paths

Status: scoped correctness follow-up, 2026-09-14. No parser change is implemented.
Start after the retained metadata qualification reaches its keep/reject decision.

## Confirmed failure

A real Git repository generated a metadata-only rename from `old name.txt` to
`new name.txt`. Both preserved metadata executables rejected that valid patch:

```text
native-diff: Cannot parse this patch: trailing data at line 3; class=Patch (31)
```

The [patch and rejection records](../../evidence/document-metadata-v1/git-rename-control/rejections.json)
retain the evidence. The generator is
[check_git_metadata.py](../../evidence/document-metadata-v1/check_git_metadata.py).
This failure precedes the immutable metadata change.

The pinned libgit2 parser uses `header_path_len` for `parse_header_rename`.
That function stops an unquoted path at its first whitespace character.
The header parser then rejects the remaining filename as trailing data.
Copy headers use the same function. Their failure remains a source-backed risk
until a real Git control reproduces it.

## Required behavior

The viewer must accept valid Git rename and copy headers without changing source
bytes, paths, file status, modes, source numbers, or newline markers.
The existing safe-label policy still applies to terminal display.
Malformed input must retain explicit errors.

The fix must cover metadata-only changes and changes with textual hunks.
It must not depend on the filename, patch size, or corpus identity.
Ordinary paths must retain their existing parsing behavior.

## Implementation boundary

First assess a narrow fix in the parser dependency. Preserve the existing parser
and streaming publication path. A dependency change needs an explicit source and
license record, a reproducible build, and focused regression coverage.

Do not patch the shared Cargo registry. Do not add a second general patch parser
or a permissive fallback that ignores rejected headers.

Header rewriting is not an automatic safe alternative.
[model.rs](../../src/model.rs) checks parser offsets against original patch bytes.
Inserted quoting changes those offsets for every later source record.
Any normalization proposal must preserve original offsets and prove exact source
identity. Replacing spaces, trimming names, or searching for matching source text
does not satisfy that requirement.

## Real-process checks

1. Generate rename and copy patches with an isolated temporary Git repository.
2. Use ordinary names, interior spaces, leading/trailing spaces, quotes, tabs,
   and Unicode names. Derive expected names from the generated repository.
3. Include unchanged contents and edited contents with a final newline marker.
4. Check exact metadata and all supplied source records through the executable.
5. Check split and unified terminal views, file navigation, and clean exit.
6. Keep a malformed-header case that must fail without a partial false success.
7. Run the ordinary release contracts and complete source/projection checks.

The generator supplies inputs, not parser-derived expected answers.
The current executable must reproduce the focused failure before the fix runs.
The fixed executable must pass the same retained input and independent answer.

## Completion

Record the smallest justified implementation, exact build identity, and check
results. Update the current-state report with any remaining input limitations.
This correctness fix does not claim complete Git-format support or a performance
improvement. Reuse existing measurement protocols if its parsing work changes
opening cost materially.
