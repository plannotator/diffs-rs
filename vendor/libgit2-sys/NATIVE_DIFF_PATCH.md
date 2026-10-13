# Local parser correction

Base: the published `libgit2-sys` crate `0.18.8+1.9.7`, containing libgit2 1.9.7.
The original crate SHA-256 is
`7f7c568b25d7489bc3fb2988ed69ab111d2944d2f5fec3d5c987fe545ea97b50`.
Its VCS record identifies git2-rs commit
`6c93812dbc1c34aef6e6464a645545b4a4299807`.

The complete published crate remains present. Only `build.rs` and
`libgit2/src/libgit2/patch_parse.c` differ from that crate, apart from this note.

## Behavior

Git rename and copy headers contain an unprefixed filename. Unquoted filenames
can contain leading, interior, and trailing spaces. The parser consumes these
names through the line ending without trimming filename whitespace.
Quoted names retain the existing token boundary and escape decoder. Extra data
after a quoted name remains an error. The temporary path buffer is released
on decoding failure.

The common path helper retains its previous trimming behavior for other headers.
No patch bytes, source offsets, source records, or application rendering rules
change. The application enables `vendored-libgit2`. The build rejects a nonzero
`LIBGIT2_NO_VENDOR` override to prevent an unpatched system library from bypassing
the correction.

The regression contracts generate real Git rename and copy patches. They check
exact safe-display names, status, supplied source bytes, line numbers, newline
markers, retained source export, both terminal layouts, and file navigation.
Malformed quoted names remain errors.

## Source and notices

- [Published crate](https://crates.io/crates/libgit2-sys/0.18.8+1.9.7)
- [Git patch format](https://git-scm.com/docs/diff-format)
- Bindings: `LICENSE-MIT` and `LICENSE-APACHE`, unchanged.
- libgit2: `libgit2/COPYING`, including its linking exception, unchanged.
- Bundled component notices remain in their original directories.

The changed C file carries a dated modification notice. This local correction
does not change the license of the original components.
