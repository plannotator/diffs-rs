use super::*;
use crate::source::CheckedPair;

#[derive(Clone)]
pub enum FileSource {
    Patch(Arc<FileIndex>),
    Plain(Arc<FileIndex>),
    Complete(Arc<CheckedPair>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Context {
    Patch,
    Plain,
    Complete(u64),
}

/// Identifies the immutable file and parser context that produced a style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Origin {
    store: u64,
    file: usize,
    context: Context,
}

impl Origin {
    #[cfg(test)]
    pub(super) fn patch(store: u64, file: usize) -> Self {
        Self {
            store,
            file,
            context: Context::Patch,
        }
    }
}

impl FileSource {
    pub(crate) fn info(&self, id: crate::view::LineId) -> anyhow::Result<crate::storage::LineInfo> {
        anyhow::ensure!(
            id.belongs_to(self.file()),
            "Style source belongs to another file"
        );
        match id {
            crate::view::LineId::Patch(id) => Ok(self.file().info(id)),
            crate::view::LineId::Context { old, new, .. } => match self {
                Self::Complete(pair) => pair.context_info(old.get(), new.get()),
                _ => anyhow::bail!("Context requires checked source"),
            },
        }
    }

    pub fn file(&self) -> &Arc<FileIndex> {
        match self {
            Self::Patch(file) | Self::Plain(file) => file,
            Self::Complete(pair) => pair.file(),
        }
    }

    pub fn origin(&self) -> Origin {
        Origin {
            store: self.file().spool.identity(),
            file: self.file().id,
            context: match self {
                Self::Patch(_) => Context::Patch,
                Self::Plain(_) => Context::Plain,
                Self::Complete(pair) => Context::Complete(pair.identity),
            },
        }
    }
}

impl From<Arc<FileIndex>> for FileSource {
    fn from(file: Arc<FileIndex>) -> Self {
        Self::Patch(file)
    }
}
