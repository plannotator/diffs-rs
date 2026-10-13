use std::collections::BTreeMap;

use anyhow::{Context, Result, bail, ensure};

use crate::storage::FileIndex;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct NodeId(usize);

type ChildKey = (bool, Box<[u8]>, usize);

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Totals {
    pub files: usize,
    pub additions: usize,
    pub deletions: usize,
    pub binary: usize,
}

impl Totals {
    fn add(self, other: Self) -> Result<Self> {
        Ok(Self {
            files: self
                .files
                .checked_add(other.files)
                .context("Too many files")?,
            additions: self
                .additions
                .checked_add(other.additions)
                .context("Addition count overflow")?,
            deletions: self
                .deletions
                .checked_add(other.deletions)
                .context("Deletion count overflow")?,
            binary: self
                .binary
                .checked_add(other.binary)
                .context("Binary count overflow")?,
        })
    }
}

pub(crate) enum Kind {
    Folder {
        children: BTreeMap<ChildKey, NodeId>,
        expanded: bool,
    },
    File {
        id: FileId,
        occurrence: usize,
    },
}

pub(crate) struct Node {
    pub parent: NodeId,
    pub label: String,
    pub kind: Kind,
    pub totals: Totals,
}

impl Node {
    pub fn file(&self) -> Option<FileId> {
        match self.kind {
            Kind::File { id, .. } => Some(id),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct TreeRow {
    pub node: NodeId,
    pub depth: usize,
}

pub(crate) struct Tree {
    nodes: Vec<Node>,
    files: Vec<NodeId>,
    rows: Vec<TreeRow>,
    positions: Vec<Option<usize>>,
    dirty: bool,
}

const ROOT: NodeId = NodeId(0);

impl Default for Tree {
    fn default() -> Self {
        Self {
            nodes: vec![Node {
                parent: ROOT,
                label: String::new(),
                kind: Kind::Folder {
                    children: BTreeMap::new(),
                    expanded: true,
                },
                totals: Totals::default(),
            }],
            files: Vec::new(),
            rows: Vec::new(),
            positions: vec![None],
            dirty: false,
        }
    }
}

impl Tree {
    pub fn len(&self) -> usize {
        self.files.len()
    }
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id.0]
    }
    pub fn rows(&self) -> &[TreeRow] {
        &self.rows
    }
    pub fn position(&self, id: NodeId) -> Option<usize> {
        self.positions[id.0]
    }
    pub fn file_node(&self, id: FileId) -> Option<NodeId> {
        self.files.get(id.0).copied()
    }
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        let parent = self.node(id).parent;
        (parent != ROOT).then_some(parent)
    }

    pub fn append(&mut self, file: &FileIndex) -> Result<()> {
        ensure!(
            file.id == self.files.len(),
            "Tree file IDs must follow document order"
        );
        let totals = Totals {
            files: 1,
            additions: file.additions,
            deletions: file.deletions,
            binary: usize::from(file.status.as_ref() == "Binary change"),
        };
        self.nodes[0].totals.add(totals)?;
        let mut components = file.raw_path.split(|b| *b == b'/').peekable();
        let mut parent = ROOT;
        while let Some(component) = components.next() {
            let is_file = components.peek().is_none();
            let key = (
                is_file,
                Box::<[u8]>::from(component),
                if is_file { file.id } else { 0 },
            );
            let Kind::Folder { children, .. } = &self.nodes[parent.0].kind else {
                bail!("Tree parent is not a folder");
            };
            if !is_file && let Some(id) = children.get(&key) {
                parent = *id;
                continue;
            }
            let previous = if is_file {
                children
                    .range(..key.clone())
                    .next_back()
                    .filter(|(other, _)| other.0 && other.1.as_ref() == component)
                    .map(|(_, id)| *id)
            } else {
                None
            };
            let occurrence = if let Some(id) = previous {
                let Kind::File { occurrence, .. } = &mut self.nodes[id.0].kind else {
                    bail!("Duplicate tree entry is not a file");
                };
                *occurrence = (*occurrence).max(1);
                occurrence
                    .checked_add(1)
                    .context("Too many duplicate paths")?
            } else {
                0
            };
            let id = NodeId(self.nodes.len());
            self.nodes.push(Node {
                parent,
                label: component_label(component),
                totals: Totals::default(),
                kind: if is_file {
                    Kind::File {
                        id: FileId(file.id),
                        occurrence,
                    }
                } else {
                    Kind::Folder {
                        children: BTreeMap::new(),
                        expanded: true,
                    }
                },
            });
            self.positions.push(None);
            let Kind::Folder { children, .. } = &mut self.nodes[parent.0].kind else {
                bail!("Tree parent is not a folder");
            };
            children.insert(key, id);
            parent = id;
            if is_file {
                self.files.push(id);
            }
        }
        loop {
            self.nodes[parent.0].totals = self.nodes[parent.0].totals.add(totals)?;
            if parent == ROOT {
                break;
            }
            parent = self.nodes[parent.0].parent;
        }
        self.dirty = true;
        Ok(())
    }

    pub fn set_expanded(&mut self, id: NodeId, value: bool) {
        if let Kind::Folder { expanded, .. } = &mut self.nodes[id.0].kind {
            self.dirty |= *expanded != value;
            *expanded = value;
        }
    }

    pub fn reveal(&mut self, id: NodeId) {
        let mut parent = self.parent(id);
        while let Some(id) = parent {
            self.set_expanded(id, true);
            parent = self.parent(id);
        }
    }

    pub fn refresh(&mut self) {
        if !self.dirty {
            return;
        }
        self.rows.clear();
        self.positions.fill(None);
        let mut stack = vec![(ROOT, 0)];
        while let Some((id, depth)) = stack.pop() {
            if id != ROOT {
                self.positions[id.0] = Some(self.rows.len());
                self.rows.push(TreeRow {
                    node: id,
                    depth: depth - 1,
                });
            }
            if let Kind::Folder {
                children,
                expanded: true,
            } = &self.node(id).kind
            {
                stack.extend(children.values().rev().map(|id| (*id, depth + 1)));
            }
        }
        self.dirty = false;
    }
}

fn component_label(bytes: &[u8]) -> String {
    bytes
        .split(|b| *b == b'\\')
        .map(crate::text::safe_label)
        .collect::<Vec<_>>()
        .join("\\\\")
}
