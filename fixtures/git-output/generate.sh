#!/bin/sh
# Regenerates the Git command-output fixtures in this directory from a scratch repository.
# Each `<name>.patch` is real output from the Git command shown below. Each
# `<name>.units.patch` holds only the file patches of the same commits, produced by
# `git diff-tree` or `git diff`, as the independent expectation for what the viewer reads.
set -eu
out=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=Fixture GIT_AUTHOR_EMAIL=fixture@example.com
export GIT_COMMITTER_NAME=Fixture GIT_COMMITTER_EMAIL=fixture@example.com
export GIT_AUTHOR_DATE=2026-01-01T00:00:00Z GIT_COMMITTER_DATE=2026-01-01T00:00:00Z
cd "$work"
git init -q -b main repo
cd repo
git config core.hooksPath /dev/null

printf 'fn main() {\n    println!("one");\n}\n' > main.rs
printf 'all:\n\techo hi\n' > Makefile
printf 'alpha\nbeta\ngamma\n' > notes.txt
printf 'echo run\n' > script.sh
: > empty.txt
git add . && git commit -qm "initial commit"

printf 'fn main() {\n    println!("two");\n}\n' > main.rs
git commit -qam "change main

This body mentions diff --git and --- a/main.rs inside the message."

git mv notes.txt docs.txt
git commit -qm "rename notes"

chmod +x script.sh
git commit -qam "make script executable"

printf '\000\001\002binary' > blob.bin
git add blob.bin && git commit -qm "add binary"

git checkout -qb side HEAD~2
printf 'side\n' > side.txt
git add side.txt && git commit -qm "side change"
git checkout -q main
git merge -q --no-edit side -m "merge side"

printf 'all:\n\techo bye' > Makefile
printf 'fn main() {\n    println!("three");\n}\n' > main.rs
git commit -qam "last change"

diff_tree() {
    # $1: extra options; remaining: commit IDs in output order.
    options=$1
    shift
    for commit in "$@"; do
        # shellcheck disable=SC2086
        git diff-tree -p -M --root --no-commit-id $options "$commit"
    done
}
commits() { sed -n "s/$1/\1/p" "$2"; }

git show HEAD > "$out/show.patch"
diff_tree "" HEAD > "$out/show.units.patch"

git show --stat -p HEAD > "$out/show-stat.patch"
cp "$out/show.units.patch" "$out/show-stat.units.patch"

git log -p > "$out/log.patch"
diff_tree "" $(commits '^commit \([0-9a-f]*\).*' "$out/log.patch") > "$out/log.units.patch"

git log -p -3 > "$out/log-3.patch"
diff_tree "" $(commits '^commit \([0-9a-f]*\).*' "$out/log-3.patch") > "$out/log-3.units.patch"

git log -p --stat --binary > "$out/log-stat-binary.patch"
diff_tree "--binary" $(commits '^commit \([0-9a-f]*\).*' "$out/log-stat-binary.patch") > "$out/log-stat-binary.units.patch"

git format-patch --stdout --root HEAD > "$out/format-patch.patch"
diff_tree "--binary" $(commits '^From \([0-9a-f]\{40\}\) .*' "$out/format-patch.patch") > "$out/format-patch.units.patch"

root=$(git rev-list --max-parents=0 HEAD)
git diff --stat -p "$root" HEAD > "$out/diff-stat.patch"
git diff -p "$root" HEAD > "$out/diff-stat.units.patch"

git --version > "$out/GIT_VERSION"
