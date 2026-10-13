use std::io::BufRead;

use anyhow::{Context, Result, ensure};

use super::{Cancellation, git};
use crate::{model::PatchIdentity, storage::FileIndex};

#[derive(Debug)]
pub(super) struct Entry {
    pub old: Endpoint,
    pub new: Endpoint,
    pub status: u8,
}

#[derive(Clone, Debug)]
pub(super) struct Endpoint {
    pub path: Vec<u8>,
    pub mode: u32,
    pub oid: git2::Oid,
}

pub(super) fn envelope(input: &mut impl BufRead, cancel: &Cancellation) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    loop {
        cancel.check()?;
        let header = field(input, 0)?;
        if header.is_empty() {
            return Ok(entries);
        }
        let mut fields = header
            .strip_prefix(b":")
            .context("Invalid Git raw header")?
            .split(|b| *b == b' ');
        let old_mode = mode(fields.next())?;
        let new_mode = mode(fields.next())?;
        let old_oid = git::oid(fields.next().context("Missing old object ID")?)?;
        let new_oid = git::oid(fields.next().context("Missing new object ID")?)?;
        let status = fields.next().context("Missing Git change status")?;
        ensure!(
            fields.next().is_none() && !status.is_empty(),
            "Invalid Git raw fields"
        );
        ensure!(
            matches!(status[0], b'A' | b'D' | b'M' | b'T' | b'R'),
            "Unsupported Git change status"
        );
        ensure!(
            status[1..].iter().all(u8::is_ascii_digit),
            "Invalid Git change score"
        );
        let old_path = field(input, 0)?;
        ensure!(!old_path.is_empty(), "Missing Git path");
        let new_path = if status[0] == b'R' {
            field(input, 0)?
        } else {
            old_path.clone()
        };
        ensure!(!new_path.is_empty(), "Missing Git destination path");
        ensure!(
            (old_mode == 0) == old_oid.is_zero() && (new_mode == 0) == new_oid.is_zero(),
            "Git object absence disagrees with its mode"
        );
        let entry = Entry {
            old: Endpoint {
                path: old_path,
                mode: old_mode,
                oid: old_oid,
            },
            new: Endpoint {
                path: new_path,
                mode: new_mode,
                oid: new_oid,
            },
            status: status[0],
        };
        if entry.status == b'T' {
            // Git emits a deletion and addition patch for a raw type-change entry.
            entries.push(Entry {
                old: entry.old.clone(),
                new: Endpoint {
                    path: entry.old.path.clone(),
                    mode: 0,
                    oid: git2::Oid::ZERO_SHA1,
                },
                status: b'D',
            });
            entries.push(Entry {
                old: Endpoint {
                    path: entry.new.path.clone(),
                    mode: 0,
                    oid: git2::Oid::ZERO_SHA1,
                },
                new: entry.new,
                status: b'A',
            });
        } else {
            entries.push(entry);
        }
    }
}

fn mode(value: Option<&[u8]>) -> Result<u32> {
    let value = value.context("Missing Git mode")?;
    ensure!(value.len() == 6, "Invalid Git mode");
    Ok(u32::from_str_radix(std::str::from_utf8(value)?, 8)?)
}

pub(super) fn field(input: &mut impl BufRead, delimiter: u8) -> Result<Vec<u8>> {
    let mut value = Vec::new();
    loop {
        let buffer = input.fill_buf()?;
        if buffer.is_empty() {
            ensure!(value.is_empty(), "Truncated Git field");
            return Ok(value);
        }
        let end = buffer.iter().position(|b| *b == delimiter);
        let len = end.map_or(buffer.len(), |i| i + 1);
        ensure!(
            value.len() + len <= 1024 * 1024,
            "Git metadata field exceeds the supported size"
        );
        value.extend_from_slice(&buffer[..len]);
        input.consume(len);
        if end.is_some() {
            value.pop();
            return Ok(value);
        }
    }
}

impl Entry {
    pub fn matches(&self, file: &FileIndex, identity: &PatchIdentity) -> Result<()> {
        ensure!(
            identity.old_path == self.old.path && identity.new_path == self.new.path,
            "Git metadata and patch paths disagree"
        );
        let mode_omitted = self.status == b'R'
            && file.hunks.is_empty()
            && file.old_mode.as_ref() == "0"
            && file.new_mode.as_ref() == "0"
            && identity.old_oid.is_none()
            && identity.new_oid.is_none();
        ensure!(
            mode_omitted
                || (file.old_mode.as_ref() == format!("{:o}", self.old.mode)
                    && file.new_mode.as_ref() == format!("{:o}", self.new.mode)),
            "Git metadata and patch modes disagree for {}",
            file.path
        );
        // Pure renames and mode-only patches need not contain an index header.
        if let Some(id) = identity.old_oid {
            ensure!(id == self.old.oid, "Git old object IDs disagree");
        }
        if let Some(id) = identity.new_oid {
            ensure!(id == self.new.oid, "Git new object IDs disagree");
        }
        let status = match self.status {
            b'A' => "Added",
            b'D' => "Deleted",
            b'R' => "Renamed",
            b'T' => "Type changed",
            _ => "Modified",
        };
        ensure!(
            file.status.as_ref() == status || file.status.as_ref() == "Binary change",
            "Git metadata and patch statuses disagree"
        );
        Ok(())
    }
}
