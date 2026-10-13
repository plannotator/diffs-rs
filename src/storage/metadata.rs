use std::ops::Range;

use serde::Serialize;

use crate::model;

#[derive(Debug, Serialize)]
pub struct HunkMetadata {
    pub header: Box<str>,
    pub old_start: u32,
    pub new_start: u32,
    pub lines: Range<usize>,
}

impl From<model::Hunk> for HunkMetadata {
    fn from(hunk: model::Hunk) -> Self {
        let model::Hunk {
            header,
            old_start,
            new_start,
            lines,
        } = hunk;
        Self {
            header: header.into_boxed_str(),
            old_start,
            new_start,
            lines,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FileMetadata {
    #[serde(skip)]
    pub raw_path: Box<[u8]>,
    pub path: Box<str>,
    pub old_path: Box<str>,
    pub status: Box<str>,
    pub old_mode: Box<str>,
    pub new_mode: Box<str>,
    pub additions: usize,
    pub deletions: usize,
    pub digits: usize,
    pub hunks: Box<[HunkMetadata]>,
}

impl From<model::FileDiff> for FileMetadata {
    fn from(file: model::FileDiff) -> Self {
        let model::FileDiff {
            identity: _,
            raw_path,
            path,
            old_path,
            status,
            old_mode,
            new_mode,
            additions,
            deletions,
            digits,
            hunks,
        } = file;
        Self {
            raw_path: raw_path.into_boxed_slice(),
            path: path.into_boxed_str(),
            old_path: old_path.into_boxed_str(),
            status: status.into_boxed_str(),
            old_mode: old_mode.into_boxed_str(),
            new_mode: new_mode.into_boxed_str(),
            additions,
            deletions,
            digits,
            hunks: hunks.into_iter().map(HunkMetadata::from).collect(),
        }
    }
}
