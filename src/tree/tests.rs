use super::*;

fn patch(path: &str) -> String {
    format!(
        "diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n@@ -1 +1,2 @@\n-old\n+new\n+extra\n"
    )
}

fn parse(input: &str) -> Vec<std::sync::Arc<FileIndex>> {
    let mut files = Vec::new();
    crate::storage::read(input.as_bytes(), |file| {
        files.push(file);
        Ok(())
    })
    .unwrap();
    files
}

#[test]
fn identity_order_totals_and_incremental_collapses_use_real_patch_metadata() {
    let files = parse(
        &[
            patch("z.rs"),
            patch("src/lib/core.rs"),
            patch("src"),
            patch("src/lib/core.rs"),
            patch("src/a.rs"),
            patch("a.rs"),
        ]
        .concat(),
    );
    let mut tree = Tree::default();
    for file in &files[..4] {
        tree.append(file).unwrap();
    }
    tree.refresh();
    let first = tree.file_node(FileId(1)).unwrap();
    let duplicate = tree.file_node(FileId(3)).unwrap();
    assert_ne!(first, duplicate);
    assert!(matches!(
        tree.node(first).kind,
        Kind::File { occurrence: 1, .. }
    ));
    assert!(matches!(
        tree.node(duplicate).kind,
        Kind::File { occurrence: 2, .. }
    ));
    let lib = tree.parent(first).unwrap();
    let src = tree.parent(lib).unwrap();
    assert_eq!(
        tree.node(src).totals,
        Totals {
            files: 2,
            additions: 4,
            deletions: 2,
            binary: 0
        }
    );
    tree.set_expanded(lib, false);
    for file in &files[4..] {
        tree.append(file).unwrap();
    }
    tree.refresh();
    assert_eq!(tree.file_node(FileId(1)), Some(first));
    assert_eq!(tree.position(first), None);
    let rows: Vec<_> = tree
        .rows()
        .iter()
        .map(|r| (r.depth, tree.node(r.node).label.as_str()))
        .collect();
    assert_eq!(
        rows,
        [
            (0, "src"),
            (1, "lib"),
            (1, "a.rs"),
            (0, "a.rs"),
            (0, "src"),
            (0, "z.rs")
        ]
    );
    assert_eq!(
        tree.node(src).totals,
        Totals {
            files: 3,
            additions: 6,
            deletions: 3,
            binary: 0
        }
    );
    tree.reveal(duplicate);
    tree.refresh();
    assert!(tree.position(first).unwrap() < tree.position(duplicate).unwrap());
}

#[test]
fn raw_git_path_bytes_and_literal_escape_names_stay_distinct() {
    let mut input = String::new();
    for name in [
        r"line\n.rs",
        r"line\\n.rs",
        r"bad\377.rs",
        r"bad\\xff.rs",
        r"caf\303\251.rs",
    ] {
        input.push_str(&format!("diff --git \"a/{name}\" \"b/{name}\"\n--- \"a/{name}\"\n+++ \"b/{name}\"\n@@ -1 +1 @@\n-a\n+b\n"));
    }
    let files = parse(&input);
    let mut tree = Tree::default();
    for file in &files {
        tree.append(file).unwrap();
    }
    let labels: Vec<_> = (0..5)
        .map(|n| tree.node(tree.file_node(FileId(n)).unwrap()).label.as_str())
        .collect();
    assert_eq!(
        labels,
        [
            r"line\n.rs",
            r"line\\n.rs",
            r"bad\xff.rs",
            r"bad\\xff.rs",
            "café.rs"
        ]
    );
    assert_eq!(&*files[0].raw_path, b"line\n.rs");
    assert_eq!(&*files[2].raw_path, b"bad\xff.rs");
}

#[test]
fn deep_paths_use_iterative_hierarchy_and_visibility() {
    let path = format!("{}leaf.rs", "directory/".repeat(2000));
    let files = parse(&patch(&path));
    let mut tree = Tree::default();
    tree.append(&files[0]).unwrap();
    tree.refresh();
    assert_eq!(tree.rows().len(), 2001);
    assert_eq!(tree.rows().last().unwrap().depth, 2000);
    let root_folder = tree.rows()[0].node;
    tree.set_expanded(root_folder, false);
    tree.refresh();
    assert_eq!(tree.rows().len(), 1);
    tree.reveal(tree.file_node(FileId(0)).unwrap());
    tree.refresh();
    assert_eq!(tree.rows().len(), 2001);
}
