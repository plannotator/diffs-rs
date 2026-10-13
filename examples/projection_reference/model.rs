use std::{io::BufRead, ops::Range};

use anyhow::{Context, Result, bail};
use git2::{Delta, Diff, Patch};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Kind {
    Context,
    Addition,
    Deletion,
}

#[derive(Debug, Serialize)]
pub struct SourceLine {
    pub kind: Kind,
    pub old: Option<u32>,
    pub new: Option<u32>,
    pub bytes: Vec<u8>,
    pub no_newline: bool,
    #[serde(skip)]
    pub offset: usize,
}

#[derive(Debug, Serialize)]
pub struct Hunk {
    pub header: String,
    pub old_start: u32,
    pub new_start: u32,
    pub lines: Range<usize>,
}

#[derive(Debug, Serialize)]
pub struct FileDiff {
    pub path: String,
    pub old_path: String,
    pub status: String,
    pub old_mode: String,
    pub new_mode: String,
    pub additions: usize,
    pub deletions: usize,
    pub digits: usize,
    pub hunks: Vec<Hunk>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    Header,
    Hunk(usize),
    Gap,
    Metadata,
    Code {
        left: Option<usize>,
        right: Option<usize>,
    },
    NoNewline {
        left: bool,
        right: bool,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub file: usize,
    pub kind: RowKind,
}

#[derive(Debug, Default, Serialize)]
pub struct Document {
    pub files: Vec<FileDiff>,
    pub lines: Vec<SourceLine>,
    #[serde(skip)]
    pub split: Vec<Row>,
    #[serde(skip)]
    pub unified: Vec<Row>,
    #[serde(skip)]
    pub split_locations: Vec<usize>,
    #[serde(skip)]
    pub unified_locations: Vec<usize>,
}

impl Document {
    pub fn parse(input: &[u8]) -> Result<Self> {
        Self::read(std::io::Cursor::new(input))
    }

    pub fn read(input: impl BufRead) -> Result<Self> {
        let mut doc = Self::default();
        crate::ingest::visit_files(input, |unit| {
            let mut part = Self::parse_file(unit)?;
            let base = doc.lines.len();
            for file in &mut part.files {
                for hunk in &mut file.hunks {
                    hunk.lines = (hunk.lines.start + base)..(hunk.lines.end + base);
                }
            }
            doc.lines.append(&mut part.lines);
            doc.files.append(&mut part.files);
            Ok(())
        })?;
        doc.index_rows();
        Ok(doc)
    }

    pub fn parse_file(input: &[u8]) -> Result<Self> {
        if input.iter().all(u8::is_ascii_whitespace) {
            return Ok(Self::default());
        }
        if input
            .split(|b| *b == b'\n')
            .any(|l| l.starts_with(b"diff --cc ") || l.starts_with(b"diff --combined "))
        {
            bail!("Combined merge diffs are not supported. Supply a two-sided patch.");
        }
        let normalized = empty_file_headers(input);
        let diff = Diff::from_buffer(normalized.as_deref().unwrap_or(input))
            .context("Cannot parse this patch")?;
        if diff.deltas().len() == 0 {
            bail!("The input contains no recognized file changes.");
        }
        let mut doc = Self::default();
        for (file_index, delta) in diff.deltas().enumerate() {
            let old_path =
                crate::text::safe_label(delta.old_file().path_bytes().unwrap_or(b"/dev/null"));
            let path =
                crate::text::safe_label(delta.new_file().path_bytes().unwrap_or(b"/dev/null"));
            let mut file = FileDiff {
                path: if delta.status() == Delta::Deleted {
                    old_path.clone()
                } else {
                    path
                },
                old_path,
                status: match delta.status() {
                    Delta::Added => "Added",
                    Delta::Deleted => "Deleted",
                    Delta::Renamed => "Renamed",
                    Delta::Copied => "Copied",
                    Delta::Typechange => "Type changed",
                    _ => "Modified",
                }
                .into(),
                old_mode: format!("{:o}", u32::from(delta.old_file().mode())),
                new_mode: format!("{:o}", u32::from(delta.new_file().mode())),
                additions: 0,
                deletions: 0,
                digits: 3,
                hunks: Vec::new(),
            };
            if let Some(patch) = Patch::from_diff(&diff, file_index)? {
                let (mut old_end, mut new_end) = (0, 0);
                for hunk_index in 0..patch.num_hunks() {
                    let (hunk, count) = patch.hunk(hunk_index)?;
                    let old_start = u64::from(hunk.old_start()) + u64::from(hunk.old_lines() == 0);
                    let new_start = u64::from(hunk.new_start()) + u64::from(hunk.new_lines() == 0);
                    if old_start < old_end || new_start < new_end {
                        bail!("Hunks overlap or appear out of source order");
                    }
                    old_end = old_start + u64::from(hunk.old_lines());
                    new_end = new_start + u64::from(hunk.new_lines());
                    let start = doc.lines.len();
                    let (mut old_count, mut new_count) = (0, 0);
                    for i in 0..count {
                        let line = patch.line_in_hunk(hunk_index, i)?;
                        let kind = match line.origin() {
                            ' ' => Kind::Context,
                            '+' => Kind::Addition,
                            '-' => Kind::Deletion,
                            '=' | '<' | '>' => {
                                if doc.lines.len() == start {
                                    bail!("Newline marker without a source line in this hunk");
                                }
                                let previous = doc
                                    .lines
                                    .last_mut()
                                    .context("Newline marker without a source line")?;
                                previous.no_newline = true;
                                if previous.bytes.last() == Some(&b'\n') {
                                    previous.bytes.pop();
                                }
                                continue;
                            }
                            c => bail!("Unsupported patch line type: {c:?}"),
                        };
                        let old = line.old_lineno();
                        let new = line.new_lineno();
                        if let Some(n) = old {
                            if n == 0 || Some(n) != hunk.old_start().checked_add(old_count) {
                                bail!("Non-contiguous old source line");
                            }
                            old_count += 1;
                        }
                        if let Some(n) = new {
                            if n == 0 || Some(n) != hunk.new_start().checked_add(new_count) {
                                bail!("Non-contiguous new source line");
                            }
                            new_count += 1;
                        }
                        file.additions += usize::from(kind == Kind::Addition);
                        file.deletions += usize::from(kind == Kind::Deletion);
                        let largest = old.unwrap_or(0).max(new.unwrap_or(0)).max(1);
                        file.digits = file.digits.max(largest.ilog10() as usize + 1);
                        let offset = usize::try_from(line.content_offset())?
                            .checked_add(1)
                            .context("Source offset overflow")?;
                        let end = offset
                            .checked_add(line.content().len())
                            .context("Source length overflow")?;
                        anyhow::ensure!(
                            input.get(offset..end) == Some(line.content()),
                            "Parser source offset does not match the supplied patch"
                        );
                        doc.lines.push(SourceLine {
                            kind,
                            old,
                            new,
                            bytes: line.content().to_vec(),
                            no_newline: false,
                            offset,
                        });
                    }
                    if old_count != hunk.old_lines() || new_count != hunk.new_lines() {
                        bail!("Hunk counts do not agree with its source lines");
                    }
                    file.hunks.push(Hunk {
                        header: crate::text::safe_label(
                            hunk.header().strip_suffix(b"\n").unwrap_or(hunk.header()),
                        ),
                        old_start: hunk.old_start(),
                        new_start: hunk.new_start(),
                        lines: start..doc.lines.len(),
                    });
                }
            }
            if delta.flags().is_binary() {
                file.status = "Binary change".into();
            }
            doc.files.push(file);
        }
        Ok(doc)
    }

    fn push_both(&mut self, file: usize, kind: RowKind) {
        self.split.push(Row { file, kind });
        self.unified.push(Row { file, kind });
    }

    pub(crate) fn index_rows(&mut self) {
        self.split_locations.resize(self.lines.len(), 0);
        self.unified_locations.resize(self.lines.len(), 0);
        for f in 0..self.files.len() {
            self.push_both(f, RowKind::Header);
            if self.files[f].hunks.is_empty() {
                self.push_both(f, RowKind::Metadata);
            }
            let (mut old_end, mut new_end) = (1, 1);
            for h in 0..self.files[f].hunks.len() {
                let hunk = &self.files[f].hunks[h];
                let range = hunk.lines.clone();
                if hunk.old_start > old_end || hunk.new_start > new_end {
                    self.push_both(f, RowKind::Gap);
                }
                self.push_both(f, RowKind::Hunk(h));
                let mut i = range.start;
                while i < range.end {
                    if self.lines[i].kind == Kind::Context {
                        self.add_split(f, Some(i), Some(i));
                        self.add_unified(f, i);
                        i += 1;
                    } else {
                        let start = i;
                        while i < range.end && self.lines[i].kind != Kind::Context {
                            i += 1;
                        }
                        let left: Vec<_> = (start..i)
                            .filter(|n| self.lines[*n].kind == Kind::Deletion)
                            .collect();
                        let right: Vec<_> = (start..i)
                            .filter(|n| self.lines[*n].kind == Kind::Addition)
                            .collect();
                        for id in left.iter().chain(&right) {
                            self.add_unified(f, *id);
                        }
                        let offset = alignment_offset(&self.lines, &left, &right);
                        let left_pad = if right.len() > left.len() { offset } else { 0 };
                        let right_pad = if left.len() > right.len() { offset } else { 0 };
                        for row in 0..(left.len() + left_pad).max(right.len() + right_pad) {
                            let l = row.checked_sub(left_pad).and_then(|n| left.get(n)).copied();
                            let r = row
                                .checked_sub(right_pad)
                                .and_then(|n| right.get(n))
                                .copied();
                            self.add_split(f, l, r);
                        }
                    }
                }
                for line in &self.lines[range] {
                    if let Some(n) = line.old {
                        old_end = n.saturating_add(1);
                    }
                    if let Some(n) = line.new {
                        new_end = n.saturating_add(1);
                    }
                }
            }
        }
    }

    fn add_split(&mut self, file: usize, left: Option<usize>, right: Option<usize>) {
        for id in left.into_iter().chain(right) {
            self.split_locations[id] = self.split.len();
        }
        self.split.push(Row {
            file,
            kind: RowKind::Code { left, right },
        });
        let l = left.is_some_and(|i| self.lines[i].no_newline);
        let r = right.is_some_and(|i| self.lines[i].no_newline);
        if l || r {
            self.split.push(Row {
                file,
                kind: RowKind::NoNewline { left: l, right: r },
            });
        }
    }

    fn add_unified(&mut self, file: usize, id: usize) {
        let line = &self.lines[id];
        self.unified_locations[id] = self.unified.len();
        let left = line.old.map(|_| id);
        let right = line.new.map(|_| id);
        self.unified.push(Row {
            file,
            kind: RowKind::Code { left, right },
        });
        if line.no_newline {
            self.unified.push(Row {
                file,
                kind: RowKind::NoNewline {
                    left: left.is_some(),
                    right: right.is_some(),
                },
            });
        }
    }
}

fn empty_file_headers(input: &[u8]) -> Option<Vec<u8>> {
    let mut lines = input.split_inclusive(|b| *b == b'\n');
    let (header, mode, index) = (lines.next()?, lines.next()?, lines.next()?);
    if lines.next().is_some() {
        return None;
    }
    let added = mode.starts_with(b"new file mode ");
    let deleted = mode.starts_with(b"deleted file mode ");
    if !(added || deleted) || !index.starts_with(b"index ") {
        return None;
    }
    let header = header.strip_prefix(b"diff --git ")?.strip_suffix(b"\n")?;
    let (old, new) = split_identical_paths(header)?;
    let ids = index.strip_prefix(b"index ")?.strip_suffix(b"\n")?;
    let (old_id, new_id) = std::str::from_utf8(ids).ok()?.split_once("..")?;
    let empty = "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391";
    let matches_empty = |id: &str| id.len() >= 7 && empty.starts_with(id);
    let zero = |id: &str| id.len() >= 7 && id.bytes().all(|b| b == b'0');
    if !((added && zero(old_id) && matches_empty(new_id))
        || (deleted && matches_empty(old_id) && zero(new_id)))
    {
        return None;
    }
    let mut result = input.to_vec();
    result.extend_from_slice(b"--- ");
    result.extend_from_slice(if added { b"/dev/null" } else { old });
    result.extend_from_slice(b"\n+++ ");
    result.extend_from_slice(if deleted { b"/dev/null" } else { new });
    result.extend_from_slice(b"\n");
    Some(result)
}

fn split_identical_paths(header: &[u8]) -> Option<(&[u8], &[u8])> {
    let middle = header.len() / 2;
    if header.get(middle) != Some(&b' ') {
        return None;
    }
    let (old, new) = (&header[..middle], &header[middle + 1..]);
    let unquoted = old.strip_prefix(b"a/").zip(new.strip_prefix(b"b/"));
    let quoted = old.strip_prefix(b"\"a/").zip(new.strip_prefix(b"\"b/"));
    if unquoted.or(quoted).is_some_and(|(a, b)| a == b) {
        Some((old, new))
    } else {
        None
    }
}

// Only shift uneven replacements when shared content clearly beats positional pairing.
fn alignment_offset(lines: &[SourceLine], left: &[usize], right: &[usize]) -> usize {
    let pairs = left.len().min(right.len());
    let extra = left.len().abs_diff(right.len());
    if pairs == 0 || extra == 0 || pairs.saturating_mul(extra + 1) > 4096 {
        return 0;
    }
    if left
        .iter()
        .chain(right)
        .any(|i| lines[*i].bytes.len() > 4096)
    {
        return 0;
    }
    let score = |offset: usize| -> f64 {
        (0..pairs)
            .map(|p| {
                let a = &lines[left[p + if left.len() > right.len() { offset } else { 0 }]].bytes;
                let b = &lines[right[p + if right.len() > left.len() { offset } else { 0 }]].bytes;
                let a: Vec<_> = a
                    .iter()
                    .copied()
                    .filter(|c| !c.is_ascii_whitespace())
                    .collect();
                let b: Vec<_> = b
                    .iter()
                    .copied()
                    .filter(|c| !c.is_ascii_whitespace())
                    .collect();
                let prefix = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
                let suffix = a[prefix..]
                    .iter()
                    .rev()
                    .zip(b[prefix..].iter().rev())
                    .take_while(|(x, y)| x == y)
                    .count();
                (prefix + suffix) as f64 / a.len().max(b.len()).max(1) as f64
            })
            .sum()
    };
    let mut best = score(0) + pairs as f64 * 0.5;
    let mut offset = 0;
    for n in 1..=extra {
        let s = score(n);
        if s > best {
            best = s;
            offset = n;
        }
    }
    offset
}
