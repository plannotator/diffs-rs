# Local source inputs

This checkpoint implements two-file and direct two-commit inputs. It does not implement full-context coloring or context expansion.

The existing patch command stays unchanged. New inputs retain the patch view while a separate source check runs.
The footer distinguishes source readiness from patch-context coloring until the coloring stage is complete.

## Implementation order

1. Add immutable captures, buffered disk line indexes, and byte correspondence through EOF.
2. Add supervised, local-only Git commands and raw metadata association.
3. Add regular-file capture and the two input commands.
4. Prepare Git source pairs on demand, with one worker and one replaceable pending request.
5. Check real files, repositories, terminal sessions, cancellation, and the preserved patch behavior.

Source checks compare supplied lines and omitted ranges. Equal line counts do not establish a match.
Completed snapshots share the existing page allowance. Failed or canceled jobs reclaim unpublished tails.
The helper writes stdout to a private temporary file. This avoids pipe backpressure during cancellation and bounds response RAM.
Object delivery temporarily adds a disk copy until capture finishes. Future resource measurements must include this cost and helper memory.

## Narrow dependency decision

Regular-file inputs use a nonblocking open before the descriptor type check.
This prevents a concurrent path replacement with a pipe from blocking capture.
The implementation declares the already-resolved `libc` package directly for its platform-specific `O_NONBLOCK` constant.
It adds no resolved package, unsafe code, runtime, or allocator. Hard-coded operating-system flag values are not appropriate.

## Completion boundary

New modes retain existing patch-context coloring, with an explicit status that does not claim full-context syntax.
This refines the staged presentation boundary in the broader feature spec.
The next coloring stage replaces that policy. Source readiness alone cannot remove the patch-context limitation.

GitHub, working-tree inputs, physical terminal captures, and controlled performance measurements remain outside this checkpoint.
