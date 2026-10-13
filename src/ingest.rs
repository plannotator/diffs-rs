use std::io::BufRead;

use anyhow::{Context, Result, bail};

/// Splits Git patch text into file-patch units and checks their hunk boundaries.
///
/// Each unit starts at a `diff --git` line. `visit` receives the unit's byte offset in the
/// original stream and its exact bytes. Text outside units is skipped: commit headers and
/// messages from `git show` and `git log -p`, mail headers and `-- ` signatures from
/// `git format-patch`, and diffstats from `--stat -p`. The skipped bytes still count toward
/// offsets, so callers that address the original stream stay exact. Returns the total number
/// of bytes read.
///
/// Like `git apply`, a unit's extended header ends at the first line that is not a Git header
/// line, and a valid hunk header outside a unit is an error. A complete hunk followed by another
/// source record remains an error, so incorrect hunk counts cannot silently drop source lines.
/// Input containing only whitespace yields no units; other text without a `diff --git` line
/// is an error.
pub fn visit_files(
    mut input: impl BufRead,
    mut visit: impl FnMut(u64, &[u8]) -> Result<()>,
) -> Result<u64> {
    let mut splitter = Splitter::default();
    let mut line = Vec::new();
    loop {
        line.clear();
        if input.read_until(b'\n', &mut line)? == 0 {
            break;
        }
        splitter.line(&line, &mut visit)?;
    }
    splitter.finish(&mut visit)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// Text outside any file patch. Records whether the previous line began with `--- `.
    Outside { old_file_header: bool },
    /// Git extended header lines after `diff --git`, before any hunk.
    Header,
    /// Remaining old/new line counts of the current hunk; `(0, 0)` once it is complete.
    Hunk(u64, u64),
    /// A `GIT binary patch` body with its completed binary hunks and whether data lines follow.
    Binary { hunks: u8, data: bool },
    /// A complete binary patch. Only another file patch or outside text can follow.
    Ended,
}

impl Default for State {
    fn default() -> Self {
        Self::Outside {
            old_file_header: false,
        }
    }
}

#[derive(Default)]
struct Splitter {
    state: State,
    unit: Vec<u8>,
    unit_start: u64,
    offset: u64,
    units: usize,
    text_outside: bool,
}

const NO_NEWLINE: &[u8] = b"\\ No newline at end of file";

/// Extended header lines that Git and libgit2 recognize after `diff --git`.
const HEADER_LINES: &[&[u8]] = &[
    b"--- ",
    b"+++ ",
    b"old mode ",
    b"new mode ",
    b"deleted file mode ",
    b"new file mode ",
    b"copy from ",
    b"copy to ",
    b"rename old ",
    b"rename new ",
    b"rename from ",
    b"rename to ",
    b"similarity index ",
    b"dissimilarity index ",
    b"index ",
    b"Binary files ",
];

impl Splitter {
    fn line(
        &mut self,
        line: &[u8],
        visit: &mut impl FnMut(u64, &[u8]) -> Result<()>,
    ) -> Result<()> {
        if line.starts_with(b"diff --cc ") || line.starts_with(b"diff --combined ") {
            bail!("Combined merge diffs are not supported. Supply a two-sided patch.");
        }
        let file_header = line.starts_with(b"diff --git ");
        let content = trim_eol(line);
        loop {
            match self.state {
                State::Outside { old_file_header } => {
                    if file_header {
                        self.begin(line);
                        return Ok(());
                    }
                    if old_file_header && line.starts_with(b"+++ ") {
                        bail!(
                            "Cannot parse this patch: file headers without a `diff --git` line. Supply Git patch output."
                        );
                    }
                    if line.starts_with(b"@@ -") && hunk_counts(line).is_ok() {
                        bail!("Cannot parse this patch: hunk header outside a file patch");
                    }
                    self.text_outside |= !line.iter().all(u8::is_ascii_whitespace);
                    self.state = State::Outside {
                        old_file_header: line.starts_with(b"--- "),
                    };
                    self.offset += line.len() as u64;
                    return Ok(());
                }
                State::Header => {
                    if file_header {
                        self.end(visit)?;
                        self.begin(line);
                        return Ok(());
                    }
                    if line.starts_with(b"@@ ") {
                        let (old, new) = hunk_counts(line)?;
                        self.state = State::Hunk(old, new);
                    } else if content == b"GIT binary patch" {
                        self.state = State::Binary {
                            hunks: 0,
                            data: false,
                        };
                    } else if !HEADER_LINES.iter().any(|prefix| line.starts_with(prefix)) {
                        self.end(visit)?;
                        continue;
                    }
                    self.push(line);
                    return Ok(());
                }
                State::Hunk(0, 0) => {
                    if file_header {
                        self.end(visit)?;
                        self.begin(line);
                        return Ok(());
                    }
                    if line.starts_with(b"@@ ") {
                        let (old, new) = hunk_counts(line)?;
                        self.state = State::Hunk(old, new);
                    } else if content == NO_NEWLINE {
                        // The parser validates the marker's association with a source line.
                    } else if content == b"-- " {
                        // The `git format-patch` signature separator ends the last file patch.
                        self.end(visit)?;
                        continue;
                    } else if matches!(line.first(), Some(b' ' | b'+' | b'-')) {
                        bail!("Unexpected source record or incorrect hunk counts");
                    } else {
                        self.end(visit)?;
                        continue;
                    }
                    self.push(line);
                    return Ok(());
                }
                State::Hunk(ref mut old, ref mut new) => {
                    if file_header || line.starts_with(b"@@ ") {
                        bail!("Hunk counts do not agree with its source lines");
                    }
                    if content != NO_NEWLINE {
                        match line.first() {
                            Some(b' ') if *old > 0 && *new > 0 => {
                                *old -= 1;
                                *new -= 1;
                            }
                            Some(b'-') if *old > 0 => *old -= 1,
                            Some(b'+') if *new > 0 => *new -= 1,
                            _ => bail!("Unexpected source record or incorrect hunk counts"),
                        }
                    }
                    self.push(line);
                    return Ok(());
                }
                State::Binary { hunks, data } => {
                    let blank = content.is_empty();
                    let hunk_header = line.starts_with(b"literal ") || line.starts_with(b"delta ");
                    if !data && hunks > 0 && !hunk_header {
                        // Git writes a forward and an optional reverse hunk, each ending in a blank line.
                        self.state = State::Ended;
                        continue;
                    }
                    self.state = match (data, blank) {
                        (true, true) if hunks > 0 => State::Ended,
                        (true, true) => State::Binary {
                            hunks: hunks + 1,
                            data: false,
                        },
                        // The parser validates binary hunk headers and data lines.
                        _ => State::Binary { hunks, data: true },
                    };
                    self.push(line);
                    return Ok(());
                }
                State::Ended => {
                    if file_header {
                        self.end(visit)?;
                        self.begin(line);
                        return Ok(());
                    }
                    self.end(visit)?;
                }
            }
        }
    }

    fn begin(&mut self, line: &[u8]) {
        self.unit_start = self.offset;
        self.state = State::Header;
        self.push(line);
    }

    fn push(&mut self, line: &[u8]) {
        self.unit.extend_from_slice(line);
        self.offset += line.len() as u64;
    }

    /// Publishes the current unit and returns to outside text.
    fn end(&mut self, visit: &mut impl FnMut(u64, &[u8]) -> Result<()>) -> Result<()> {
        if let State::Hunk(old, new) = self.state
            && (old, new) != (0, 0)
        {
            bail!("Hunk counts do not agree with its source lines");
        }
        self.state = State::default();
        if !self.unit.is_empty() {
            visit(self.unit_start, &self.unit)?;
            self.unit.clear();
            self.units += 1;
        }
        Ok(())
    }

    fn finish(mut self, visit: &mut impl FnMut(u64, &[u8]) -> Result<()>) -> Result<u64> {
        self.end(visit)?;
        if self.units == 0 && self.text_outside {
            bail!(
                "Cannot parse this patch: no `diff --git` file header found. Supply Git patch output."
            );
        }
        Ok(self.offset)
    }
}

fn trim_eol(line: &[u8]) -> &[u8] {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r").unwrap_or(line)
}

fn hunk_counts(header: &[u8]) -> Result<(u64, u64)> {
    let header = header.strip_suffix(b"\n").unwrap_or(header);
    let mut parts = header.split(|b| *b == b' ');
    anyhow::ensure!(
        parts.next() == Some(b"@@".as_slice()),
        "Invalid hunk header"
    );
    let count = |range: Option<&[u8]>, sign: u8| -> Result<u64> {
        let range = range.context("Missing hunk range")?;
        anyhow::ensure!(range.first() == Some(&sign), "Invalid hunk range");
        let mut values = range[1..].split(|b| *b == b',');
        let start = values.next().context("Missing hunk start")?;
        std::str::from_utf8(start)?.parse::<u64>()?;
        let count = match values.next() {
            Some(count) => std::str::from_utf8(count)?.parse()?,
            None => 1,
        };
        anyhow::ensure!(values.next().is_none(), "Invalid hunk range");
        Ok(count)
    };
    let old = count(parts.next(), b'-')?;
    let new = count(parts.next(), b'+')?;
    anyhow::ensure!(
        parts.next() == Some(b"@@".as_slice()),
        "Invalid hunk header"
    );
    Ok((old, new))
}

#[cfg(test)]
mod tests;
