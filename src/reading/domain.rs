use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result, ensure};

use crate::{
    engine::ReviewId,
    input::SourceState,
    source::Snapshot,
    storage::Document,
    view::{SourceOrder, View},
};

pub(crate) struct SourceFile {
    pub(crate) order: SourceOrder,
    pub(crate) context: Option<Arc<Snapshot>>,
}

/// Frozen diff-text scope. Source bodies stay in their existing immutable stores.
pub(crate) struct Domain {
    pub(crate) review: ReviewId,
    pub(crate) files: Box<[SourceFile]>,
    pub(crate) lease: Lease,
}

#[derive(Clone, Debug)]
pub(crate) struct Lease {
    active: Arc<AtomicBool>,
    revision: Arc<AtomicBool>,
}

impl Domain {
    pub(crate) fn capture(
        review: ReviewId,
        active: Arc<AtomicBool>,
        document: &Document,
        view: &mut View,
        sources: Option<&[SourceState]>,
    ) -> Result<Self> {
        let files = document
            .files
            .iter()
            .map(|file| {
                let order = view.source_order(file.clone());
                let context = if order.has_context() {
                    let Some(SourceState::Checked(pair)) =
                        sources.and_then(|sources| sources.get(file.id))
                    else {
                        anyhow::bail!("Expanded search context has no checked source");
                    };
                    ensure!(
                        Arc::ptr_eq(pair.file(), file),
                        "Search source belongs to another file occurrence"
                    );
                    Some(
                        pair.new_source()
                            .context("Expanded search context has no source")?
                            .clone(),
                    )
                } else {
                    None
                };
                Ok(SourceFile { order, context })
            })
            .collect::<Result<Vec<_>>>()?
            .into_boxed_slice();
        let domain = Self {
            review,
            files,
            lease: Lease {
                active,
                revision: view.source_revision(),
            },
        };
        domain.check()?;
        Ok(domain)
    }

    pub(crate) fn check(&self) -> Result<()> {
        self.lease.check()
    }
}

impl Lease {
    pub(crate) fn check(&self) -> Result<()> {
        ensure!(
            self.active.load(Ordering::Acquire),
            "Search belongs to a closed review"
        );
        ensure!(
            self.revision.load(Ordering::Acquire),
            "Search scope changed; search again"
        );
        Ok(())
    }
}
