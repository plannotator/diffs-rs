use std::io::BufRead;

use anyhow::{Context, Result, bail};

pub fn visit_files(
    mut input: impl BufRead,
    mut visit: impl FnMut(&[u8]) -> Result<()>,
) -> Result<()> {
    let mut unit = Vec::new();
    let mut line = Vec::new();
    let mut remaining = None;
    let mut git_unit = false;
    loop {
        line.clear();
        if input.read_until(b'\n', &mut line)? == 0 {
            break;
        }
        if line.starts_with(b"diff --cc ") || line.starts_with(b"diff --combined ") {
            bail!("Combined merge diffs are not supported. Supply a two-sided patch.");
        }
        if line.starts_with(b"diff --git ") {
            require_complete(remaining)?;
            if !unit.is_empty() {
                visit(&unit)?;
                unit.clear();
            }
            remaining = None;
            git_unit = true;
        } else if git_unit && line.starts_with(b"@@ ") {
            require_complete(remaining)?;
            remaining = Some(hunk_counts(&line)?);
        } else if let Some((old, new)) = remaining.as_mut() {
            if line.strip_suffix(b"\n").unwrap_or(&line) == b"\\ No newline at end of file" {
                // The parser validates the marker's association with a source line.
            } else {
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
        }
        unit.extend_from_slice(&line);
    }
    require_complete(remaining)?;
    if !unit.is_empty() {
        visit(&unit)?;
    }
    Ok(())
}

fn require_complete(remaining: Option<(u64, u64)>) -> Result<()> {
    if remaining.is_some_and(|counts| counts != (0, 0)) {
        bail!("Hunk counts do not agree with its source lines");
    }
    Ok(())
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
