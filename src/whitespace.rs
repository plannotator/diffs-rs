//! Pairs removed and added lines that differ only in whitespace, for `Whitespace::IgnoreChange`
//! (`git diff -b`) and `Whitespace::IgnoreAll` (`git diff -w`).

use std::collections::HashSet;

use similar::{Algorithm, DiffOp, capture_diff_slices};

use crate::engine::Whitespace;

/// The most lines, both sides together, that are compared in order after the matching lines at
/// the start and end of a block are paired. A larger middle keeps only those pairs, so one
/// enormous block cannot stall the caller.
const MIDDLE_LIMIT: usize = 4096;

/// Whitespace as Git's `-b` and `-w` see it: space, tab, line feed, vertical tab, form feed and
/// carriage return.
fn space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// The text of a line as `whitespace` compares it, in `out`.
pub(crate) fn normalize(whitespace: Whitespace, line: &[u8], out: &mut Vec<u8>) {
    out.clear();
    match whitespace {
        Whitespace::IgnoreAll => out.extend(line.iter().copied().filter(|&b| !space(b))),
        Whitespace::IgnoreChange => {
            // Each run of whitespace becomes one space, and a run at the end goes.
            let mut run = false;
            for &byte in line {
                if space(byte) {
                    run = true;
                } else {
                    if run {
                        out.push(b' ');
                        run = false;
                    }
                    out.push(byte);
                }
            }
        }
        _ => out.extend_from_slice(line),
    }
}

/// The pairs `(old, new)` of positions in `old` and `new` whose normalized texts are equal, in
/// increasing order on both sides and as many as can be: a longest common subsequence. Lines
/// that are already equal are never given, since a patch does not remove and add the same line,
/// but they would pair too.
pub(crate) fn pairs(old: &[Vec<u8>], new: &[Vec<u8>]) -> Vec<(usize, usize)> {
    if old.is_empty() || new.is_empty() {
        return Vec::new();
    }
    // Lines with no equal on the other side can never pair. Without them the longest common
    // subsequence is the same, and a block of real changes costs only these two passes.
    let in_new: HashSet<&[u8]> = new.iter().map(Vec::as_slice).collect();
    let old_candidates: Vec<usize> = (0..old.len())
        .filter(|&n| in_new.contains(old[n].as_slice()))
        .collect();
    if old_candidates.is_empty() {
        return Vec::new();
    }
    let in_old: HashSet<&[u8]> = old_candidates.iter().map(|&n| old[n].as_slice()).collect();
    let new_candidates: Vec<usize> = (0..new.len())
        .filter(|&n| in_old.contains(new[n].as_slice()))
        .collect();
    let a: Vec<&[u8]> = old_candidates.iter().map(|&n| old[n].as_slice()).collect();
    let b: Vec<&[u8]> = new_candidates.iter().map(|&n| new[n].as_slice()).collect();

    let prefix = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..]
        .iter()
        .rev()
        .zip(b[prefix..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let mut found: Vec<(usize, usize)> = (0..prefix).map(|n| (n, n)).collect();
    let (a_middle, b_middle) = (prefix..a.len() - suffix, prefix..b.len() - suffix);
    if a_middle.len() + b_middle.len() <= MIDDLE_LIMIT {
        for op in capture_diff_slices(Algorithm::Myers, &a[a_middle.clone()], &b[b_middle.clone()])
        {
            if let DiffOp::Equal {
                old_index,
                new_index,
                len,
            } = op
            {
                found.extend((0..len).map(|n| (prefix + old_index + n, prefix + new_index + n)));
            }
        }
    }
    found.extend((0..suffix).map(|n| (a.len() - suffix + n, b.len() - suffix + n)));
    found
        .into_iter()
        .map(|(x, y)| (old_candidates[x], new_candidates[y]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalized(whitespace: Whitespace, line: &str) -> String {
        let mut out = Vec::new();
        normalize(whitespace, line.as_bytes(), &mut out);
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn ignore_change_is_git_dash_b_and_ignore_all_is_dash_w() {
        let b = Whitespace::IgnoreChange;
        let w = Whitespace::IgnoreAll;
        let same = |mode, x: &str, y: &str| normalized(mode, x) == normalized(mode, y);
        // Re-indentation, tabs for spaces, and trailing whitespace: both.
        for (x, y) in [
            ("\tlet a = 1;\n", "    let a = 1;\n"),
            ("a  b\n", "a\tb\n"),
            ("end\n", "end   \n"),
            ("end\r\n", "end\n"),
            ("x\x0b\x0cy\n", "x y\n"),
        ] {
            assert!(same(b, x, y), "-b {x:?} {y:?}");
            assert!(same(w, x, y), "-w {x:?} {y:?}");
        }
        // Whitespace where there was none: only -w.
        for (x, y) in [
            ("a+b\n", "a + b\n"),
            ("f(a,b)\n", "f(a, b)\n"),
            ("x\n", "  x\n"),
        ] {
            assert!(!same(b, x, y), "-b {x:?} {y:?}");
            assert!(same(w, x, y), "-w {x:?} {y:?}");
        }
        // Real changes: neither.
        for (x, y) in [("a\n", "b\n"), ("ab\n", "a\n"), ("a b\n", "b a\n")] {
            assert!(!same(b, x, y));
            assert!(!same(w, x, y));
        }
        // Unicode spaces are not whitespace to Git.
        assert!(!same(w, "a\u{a0}b\n", "ab\n"));
    }

    fn lines(mode: Whitespace, text: &[&str]) -> Vec<Vec<u8>> {
        text.iter()
            .map(|line| {
                let mut out = Vec::new();
                normalize(mode, line.as_bytes(), &mut out);
                out
            })
            .collect()
    }

    #[test]
    fn pairs_keep_both_orders_and_are_as_many_as_can_be() {
        let w = Whitespace::IgnoreAll;
        let old = lines(w, &["a", "b", "c", "d"]);
        let new = lines(w, &[" b", "x", " c", "a ", " d"]);
        // b, c, d in order beat a alone.
        assert_eq!(pairs(&old, &new), [(1, 0), (2, 2), (3, 4)]);
        assert!(pairs(&old, &lines(w, &["q", "r"])).is_empty());
        assert!(pairs(&old, &[]).is_empty());
        assert!(pairs(&[], &old).is_empty());
        // Repeated lines pair in order, once each.
        let old = lines(w, &["x", "x", "y"]);
        let new = lines(w, &["x ", "y ", "x "]);
        let found = pairs(&old, &new);
        assert_eq!(found.len(), 2);
        assert!(found.windows(2).all(|p| p[0].0 < p[1].0 && p[0].1 < p[1].1));
    }

    #[test]
    fn a_huge_block_still_pairs_its_matching_start_and_end() {
        let w = Whitespace::IgnoreAll;
        let count = MIDDLE_LIMIT;
        let old: Vec<String> = (0..count).map(|n| format!("line {n}")).collect();
        // The same lines reindented, with the middle reversed.
        let mut new: Vec<String> = (0..count).map(|n| format!("  line {n}")).collect();
        new[100..count - 100].reverse();
        let old = lines(w, &old.iter().map(String::as_str).collect::<Vec<_>>());
        let new = lines(w, &new.iter().map(String::as_str).collect::<Vec<_>>());
        let found = pairs(&old, &new);
        assert!(found.len() >= 200);
        assert!(found[..100].iter().enumerate().all(|(n, &p)| p == (n, n)));
        assert!(found.windows(2).all(|p| p[0].0 < p[1].0 && p[0].1 < p[1].1));
        assert!(found.iter().all(|&(x, y)| old[x] == new[y]));
    }
}
