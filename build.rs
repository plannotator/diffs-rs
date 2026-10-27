//! Keeps the Git patch parser on the libgit2 source that `git2`'s `vendored-libgit2` feature
//! builds. libgit2-sys links a system library instead when `LIBGIT2_NO_VENDOR` is set to a
//! nonzero value, and that library's parser has not been checked against this crate.

fn main() {
    println!("cargo:rerun-if-env-changed=LIBGIT2_NO_VENDOR");
    if std::env::var_os("LIBGIT2_NO_VENDOR").is_some_and(|value| value != "0") {
        panic!(
            "diffs-rs requires the libgit2 source vendored by libgit2-sys. \
             Unset LIBGIT2_NO_VENDOR or set it to 0."
        );
    }
}
