use std::{
    num::{NonZeroU32, TryFromIntError},
    ops::Range,
};

use super::{Block, BlockKind, Replacement, SourceRun};
use crate::model::Kind;

pub(super) fn compact_fits(source_len: usize, hunks: usize) -> bool {
    hunks
        .checked_mul(2)
        .and_then(|n| n.checked_add(2))
        .and_then(|n| n.checked_add(source_len))
        .is_some_and(|n| u32::try_from(n).is_ok())
}

const CHECKED_BOUND: &str = "File-local projection bound was checked before construction";

#[derive(Debug, Clone, Copy)]
enum CompactKind {
    Header,
    Hunk(u32),
    Gap,
    Metadata,
    Context(u32),
    Replacement(u32),
}

#[derive(Debug, Clone, Copy)]
struct CompactBlock {
    start: [u32; 2],
    kind: CompactKind,
}

impl TryFrom<Block> for CompactBlock {
    type Error = TryFromIntError;

    fn try_from(block: Block) -> Result<Self, Self::Error> {
        Ok(Self {
            start: [
                u32::try_from(block.start[0])?,
                u32::try_from(block.start[1])?,
            ],
            kind: match block.kind {
                BlockKind::Header => CompactKind::Header,
                BlockKind::Hunk(h) => CompactKind::Hunk(u32::try_from(h)?),
                BlockKind::Gap => CompactKind::Gap,
                BlockKind::Metadata => CompactKind::Metadata,
                BlockKind::Context { source_start } => {
                    CompactKind::Context(u32::try_from(source_start)?)
                }
                BlockKind::Replacement(r) => CompactKind::Replacement(u32::try_from(r)?),
            },
        })
    }
}

impl From<CompactBlock> for Block {
    fn from(block: CompactBlock) -> Self {
        Self {
            start: block.start.map(|n| n as usize),
            kind: match block.kind {
                CompactKind::Header => BlockKind::Header,
                CompactKind::Hunk(h) => BlockKind::Hunk(h as usize),
                CompactKind::Gap => BlockKind::Gap,
                CompactKind::Metadata => BlockKind::Metadata,
                CompactKind::Context(n) => BlockKind::Context {
                    source_start: n as usize,
                },
                CompactKind::Replacement(r) => BlockKind::Replacement(r as usize),
            },
        }
    }
}

#[derive(Debug)]
struct CompactReplacement {
    old_start: u32,
    new_start: u32,
    end: u32,
    old_pad: u32,
    new_pad: u32,
}

const _: () = assert!(size_of::<CompactReplacement>() == 5 * size_of::<u32>());

impl TryFrom<Replacement> for CompactReplacement {
    type Error = TryFromIntError;

    fn try_from(r: Replacement) -> Result<Self, Self::Error> {
        Ok(Self {
            old_start: u32::try_from(r.old_start)?,
            new_start: u32::try_from(r.new_start)?,
            end: u32::try_from(r.end)?,
            old_pad: u32::try_from(r.old_pad)?,
            new_pad: u32::try_from(r.new_pad)?,
        })
    }
}

impl From<&CompactReplacement> for Replacement {
    fn from(r: &CompactReplacement) -> Self {
        Self {
            old_start: r.old_start as usize,
            new_start: r.new_start as usize,
            end: r.end as usize,
            old_pad: r.old_pad as usize,
            new_pad: r.new_pad as usize,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CompactRun {
    start: u32,
    block: u32,
    ordinal: u32,
    old: Option<NonZeroU32>,
    new: Option<NonZeroU32>,
    kind: Kind,
}

impl TryFrom<SourceRun> for CompactRun {
    type Error = TryFromIntError;

    fn try_from(r: SourceRun) -> Result<Self, Self::Error> {
        Ok(Self {
            start: u32::try_from(r.start)?,
            block: u32::try_from(r.block)?,
            ordinal: u32::try_from(r.ordinal)?,
            old: r.old,
            new: r.new,
            kind: r.kind,
        })
    }
}

impl From<CompactRun> for SourceRun {
    fn from(r: CompactRun) -> Self {
        Self {
            start: r.start as usize,
            block: r.block as usize,
            ordinal: r.ordinal as usize,
            old: r.old,
            new: r.new,
            kind: r.kind,
        }
    }
}

#[derive(Debug)]
pub(super) struct Tables(Storage);

#[derive(Debug)]
enum Storage {
    Compact {
        blocks: Box<[CompactBlock]>,
        replacements: Box<[CompactReplacement]>,
        runs: Box<[CompactRun]>,
        sides: Box<[u32]>,
    },
    Wide {
        blocks: Box<[Block]>,
        replacements: Box<[Replacement]>,
        runs: Box<[SourceRun]>,
        sides: Box<[usize]>,
    },
}

impl Default for Tables {
    fn default() -> Self {
        TableBuilder::new(true).finish()
    }
}

impl Tables {
    pub fn block_count(&self) -> usize {
        match &self.0 {
            Storage::Compact { blocks, .. } => blocks.len(),
            Storage::Wide { blocks, .. } => blocks.len(),
        }
    }

    pub fn block(&self, n: usize) -> Block {
        match &self.0 {
            Storage::Compact { blocks, .. } => blocks[n].into(),
            Storage::Wide { blocks, .. } => blocks[n],
        }
    }

    pub fn block_at(&self, layout: usize, row: usize) -> Block {
        match &self.0 {
            Storage::Compact { blocks, .. } => {
                blocks[blocks.partition_point(|b| b.start[layout] as usize <= row) - 1].into()
            }
            Storage::Wide { blocks, .. } => {
                blocks[blocks.partition_point(|b| b.start[layout] <= row) - 1]
            }
        }
    }

    pub fn run_count(&self) -> usize {
        match &self.0 {
            Storage::Compact { runs, .. } => runs.len(),
            Storage::Wide { runs, .. } => runs.len(),
        }
    }

    pub fn run(&self, n: usize) -> SourceRun {
        match &self.0 {
            Storage::Compact { runs, .. } => runs[n].into(),
            Storage::Wide { runs, .. } => runs[n],
        }
    }

    pub fn run_at(&self, source: usize) -> SourceRun {
        match &self.0 {
            Storage::Compact { runs, .. } => {
                runs[runs.partition_point(|r| r.start as usize <= source) - 1].into()
            }
            Storage::Wide { runs, .. } => runs[runs.partition_point(|r| r.start <= source) - 1],
        }
    }

    pub fn run_end(&self, n: usize, source_len: usize) -> usize {
        match &self.0 {
            Storage::Compact { runs, .. } => {
                runs.get(n + 1).map_or(source_len, |r| r.start as usize)
            }
            Storage::Wide { runs, .. } => runs.get(n + 1).map_or(source_len, |r| r.start),
        }
    }

    pub fn replacement(&self, n: usize) -> Replacement {
        match &self.0 {
            Storage::Compact { replacements, .. } => (&replacements[n]).into(),
            Storage::Wide { replacements, .. } => replacements[n].clone(),
        }
    }

    pub fn side_run(&self, n: usize) -> usize {
        match &self.0 {
            Storage::Compact { sides, .. } => sides[n] as usize,
            Storage::Wide { sides, .. } => sides[n],
        }
    }

    pub fn side_run_at(&self, side: Range<usize>, ordinal: usize) -> Option<usize> {
        match &self.0 {
            Storage::Compact { runs, sides, .. } => {
                let side = &sides[side];
                let n = side.partition_point(|&r| runs[r as usize].ordinal as usize <= ordinal);
                n.checked_sub(1).map(|n| side[n] as usize)
            }
            Storage::Wide { runs, sides, .. } => {
                let side = &sides[side];
                let n = side.partition_point(|&r| runs[r].ordinal <= ordinal);
                n.checked_sub(1).map(|n| side[n])
            }
        }
    }

    pub fn source_run_bytes(&self) -> usize {
        match &self.0 {
            Storage::Compact { runs, .. } => size_of_val(&**runs),
            Storage::Wide { runs, .. } => size_of_val(&**runs),
        }
    }

    pub fn block_bytes(&self) -> usize {
        match &self.0 {
            Storage::Compact {
                blocks,
                replacements,
                ..
            } => size_of_val(&**blocks) + size_of_val(&**replacements),
            Storage::Wide {
                blocks,
                replacements,
                ..
            } => size_of_val(&**blocks) + size_of_val(&**replacements),
        }
    }

    pub fn side_index_bytes(&self) -> usize {
        match &self.0 {
            Storage::Compact { sides, .. } => size_of_val(&**sides),
            Storage::Wide { sides, .. } => size_of_val(&**sides),
        }
    }
}

pub(super) struct TableBuilder(Buffers);

enum Buffers {
    Compact {
        blocks: Vec<CompactBlock>,
        replacements: Vec<CompactReplacement>,
        runs: Vec<CompactRun>,
        sides: Vec<u32>,
    },
    Wide {
        blocks: Vec<Block>,
        replacements: Vec<Replacement>,
        runs: Vec<SourceRun>,
        sides: Vec<usize>,
    },
}

impl TableBuilder {
    pub fn new(compact: bool) -> Self {
        Self(if compact {
            Buffers::Compact {
                blocks: Vec::new(),
                replacements: Vec::new(),
                runs: Vec::new(),
                sides: Vec::new(),
            }
        } else {
            Buffers::Wide {
                blocks: Vec::new(),
                replacements: Vec::new(),
                runs: Vec::new(),
                sides: Vec::new(),
            }
        })
    }

    pub fn block_count(&self) -> usize {
        match &self.0 {
            Buffers::Compact { blocks, .. } => blocks.len(),
            Buffers::Wide { blocks, .. } => blocks.len(),
        }
    }

    pub fn replacement_count(&self) -> usize {
        match &self.0 {
            Buffers::Compact { replacements, .. } => replacements.len(),
            Buffers::Wide { replacements, .. } => replacements.len(),
        }
    }

    pub fn run_count(&self) -> usize {
        match &self.0 {
            Buffers::Compact { runs, .. } => runs.len(),
            Buffers::Wide { runs, .. } => runs.len(),
        }
    }

    pub fn side_count(&self) -> usize {
        match &self.0 {
            Buffers::Compact { sides, .. } => sides.len(),
            Buffers::Wide { sides, .. } => sides.len(),
        }
    }

    pub fn run(&self, n: usize) -> SourceRun {
        match &self.0 {
            Buffers::Compact { runs, .. } => runs[n].into(),
            Buffers::Wide { runs, .. } => runs[n],
        }
    }

    pub fn push_block(&mut self, value: Block) {
        match &mut self.0 {
            Buffers::Compact { blocks, .. } => blocks.push(value.try_into().expect(CHECKED_BOUND)),
            Buffers::Wide { blocks, .. } => blocks.push(value),
        }
    }

    pub fn push_replacement(&mut self, value: Replacement) {
        match &mut self.0 {
            Buffers::Compact { replacements, .. } => {
                replacements.push(value.try_into().expect(CHECKED_BOUND))
            }
            Buffers::Wide { replacements, .. } => replacements.push(value),
        }
    }

    pub fn push_run(&mut self, value: SourceRun) {
        match &mut self.0 {
            Buffers::Compact { runs, .. } => runs.push(value.try_into().expect(CHECKED_BOUND)),
            Buffers::Wide { runs, .. } => runs.push(value),
        }
    }

    pub fn push_side(&mut self, value: usize) {
        match &mut self.0 {
            Buffers::Compact { sides, .. } => {
                sides.push(u32::try_from(value).expect(CHECKED_BOUND))
            }
            Buffers::Wide { sides, .. } => sides.push(value),
        }
    }

    pub fn finish(self) -> Tables {
        Tables(match self.0 {
            Buffers::Compact {
                blocks,
                replacements,
                runs,
                sides,
            } => Storage::Compact {
                blocks: blocks.into_boxed_slice(),
                replacements: replacements.into_boxed_slice(),
                runs: runs.into_boxed_slice(),
                sides: sides.into_boxed_slice(),
            },
            Buffers::Wide {
                blocks,
                replacements,
                runs,
                sides,
            } => Storage::Wide {
                blocks: blocks.into_boxed_slice(),
                replacements: replacements.into_boxed_slice(),
                runs: runs.into_boxed_slice(),
                sides: sides.into_boxed_slice(),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_accounts_for_structural_rows_and_arithmetic_overflow() {
        let max = u32::MAX as usize;
        assert!(compact_fits(0, 0));
        assert!(compact_fits(max - 2, 0));
        assert!(!compact_fits(max - 1, 0));
        assert!(compact_fits(max - 8, 3));
        assert!(!compact_fits(max - 7, 3));
        assert!(!compact_fits(usize::MAX, 0));
        assert!(!compact_fits(0, usize::MAX));
        assert!(!compact_fits(usize::MAX / 2, usize::MAX / 2));
    }

    #[test]
    fn stored_fields_round_trip_at_compact_and_wide_limits() {
        let max = u32::MAX as usize;
        for (is_compact, n) in [(true, max), (false, max), (false, usize::MAX)] {
            let mut builder = TableBuilder::new(is_compact);
            let kinds = [
                BlockKind::Header,
                BlockKind::Gap,
                BlockKind::Metadata,
                BlockKind::Hunk(n),
                BlockKind::Context { source_start: n },
                BlockKind::Replacement(n),
            ];
            for kind in kinds {
                builder.push_block(Block {
                    start: [n, n - 1],
                    kind,
                });
            }
            let replacements = [
                (0, 0, 0),
                (n - 2, n - 1, n),
                (n, n, n),
                (0, 0, n),
                (0, n, n),
            ]
            .map(|(old_start, new_start, end)| Replacement {
                old_start,
                new_start,
                end,
                old_pad: n,
                new_pad: n - 1,
            });
            for replacement in &replacements {
                builder.push_replacement(replacement.clone());
            }
            let run = SourceRun {
                start: n,
                block: n - 1,
                ordinal: n - 2,
                old: NonZeroU32::new(u32::MAX),
                new: None,
                kind: Kind::Deletion,
            };
            builder.push_run(run);
            builder.push_side(n);
            assert_eq!(builder.run(0), run);
            let tables = builder.finish();
            assert_eq!(matches!(tables.0, Storage::Compact { .. }), is_compact);
            for (i, kind) in kinds.into_iter().enumerate() {
                assert_eq!(
                    tables.block(i),
                    Block {
                        start: [n, n - 1],
                        kind
                    }
                );
            }
            assert_eq!(tables.block_at(0, n), tables.block(kinds.len() - 1));
            assert_eq!(tables.block_at(1, n - 1), tables.block(kinds.len() - 1));
            for (i, replacement) in replacements.into_iter().enumerate() {
                assert_eq!(tables.replacement(i), replacement);
                assert_eq!(
                    replacement.old_range(),
                    replacement.old_start..replacement.new_start
                );
                assert_eq!(
                    replacement.new_range(),
                    replacement.new_start..replacement.end
                );
            }
            assert_eq!(tables.run(0), run);
            assert_eq!(tables.run_at(n), run);
            assert_eq!(tables.side_run(0), n);
        }
        if let Some(too_large) = max.checked_add(1) {
            assert!(
                CompactBlock::try_from(Block {
                    start: [too_large, 0],
                    kind: BlockKind::Header
                })
                .is_err()
            );
            assert!(
                CompactBlock::try_from(Block {
                    start: [0, 0],
                    kind: BlockKind::Context {
                        source_start: too_large
                    }
                })
                .is_err()
            );
            for fields in [
                [too_large, too_large, too_large, 0, 0],
                [0, too_large, too_large, 0, 0],
                [0, 0, too_large, 0, 0],
                [0, 0, 0, too_large, 0],
                [0, 0, 0, 0, too_large],
            ] {
                let [old_start, new_start, end, old_pad, new_pad] = fields;
                assert!(
                    CompactReplacement::try_from(Replacement {
                        old_start,
                        new_start,
                        end,
                        old_pad,
                        new_pad,
                    })
                    .is_err()
                );
            }
            assert!(
                CompactRun::try_from(SourceRun {
                    start: 0,
                    block: 0,
                    ordinal: too_large,
                    old: None,
                    new: NonZeroU32::new(1),
                    kind: Kind::Addition
                })
                .is_err()
            );
        }
    }
}
