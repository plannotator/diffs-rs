"""Optional caller progress, separate from authoritative result files and logs."""

import errno
import os


class Progress:
    """Write UTF-8 lines without buffering or owning the output descriptor.

    Use only for optional progress. Keep results and required logs on ordinary
    checked file writes. Do not mix buffered prints into this channel. This
    handles a closed pipe reader, not process termination or a stalled reader.
    """

    def __init__(self, *, enabled=True, fd=1):
        self.enabled = enabled
        self.fd = fd
        self.disconnected = False

    def __call__(self, message):
        if not self.enabled or self.disconnected:
            return
        remaining = memoryview((message + "\n").encode("utf-8"))
        try:
            while remaining:
                written = os.write(self.fd, remaining)
                if not written:
                    raise OSError(errno.EIO, "Progress write made no progress")
                remaining = remaining[written:]
        except BrokenPipeError:
            self.disconnected = True
