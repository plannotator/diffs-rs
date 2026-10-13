use std::{fmt, io::Read, ops::Range};

use anyhow::{Context, Result, ensure};

use super::append_bytes;

#[derive(Default)]
struct Length(usize);

impl fmt::Write for Length {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0 = self.0.checked_add(text.len()).ok_or(fmt::Error)?;
        Ok(())
    }
}

fn prefix_length(
    input: &mut impl Read,
    mut bytes: u64,
    check: &impl Fn() -> Result<()>,
) -> Result<usize> {
    const CHUNK: usize = 64 * 1024;
    let mut buffer = vec![0; CHUNK + 3];
    let mut pending = 0;
    let mut length = Length::default();
    while bytes != 0 {
        check()?;
        let count = bytes.min(CHUNK as u64) as usize;
        input.read_exact(&mut buffer[pending..pending + count])?;
        bytes -= count as u64;
        let end = pending + count;
        let consumed = append_bytes(&mut length, &buffer[..end], bytes == 0)
            .context("Display text length overflow")?;
        pending = end - consumed;
        buffer.copy_within(consumed..end, 0);
    }
    check()?;
    Ok(length.0)
}

/// Boundaries come from a matched UTF-8 query, never the interior of a UTF-8 character.
pub(crate) fn source_offsets(
    input: &mut impl Read,
    range: Range<u64>,
    check: &impl Fn() -> Result<()>,
) -> Result<Range<usize>> {
    ensure!(range.start <= range.end, "Invalid search match range");
    let start = prefix_length(input, range.start, check)?;
    let length = prefix_length(input, range.end - range.start, check)?;
    Ok(start
        ..start
            .checked_add(length)
            .context("Display text length overflow")?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{Cancellation, Cancelled};
    use std::io::{Seek, Write};

    #[test]
    fn original_byte_offsets_follow_the_real_sanitizer_across_chunks() {
        let mut bytes = vec![b'x'; 64 * 1024 - 1];
        bytes.extend_from_slice("日\t\u{1b}e\u{301}\u{202e}".as_bytes());
        bytes.extend_from_slice(b"\xff\xe2\x82!");
        let start = bytes.len();
        bytes.extend_from_slice("needle\t👩‍💻".as_bytes());
        let end = bytes.len();
        bytes.extend_from_slice(b"\r\n");
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(&bytes).unwrap();
        file.rewind().unwrap();
        let actual = source_offsets(&mut file, start as u64..end as u64, &|| Ok(())).unwrap();
        let prefix = format!(
            "{}日\t\\u{{1b}}e\u{301}\\u{{202e}}\\xff\\xe2\\x82!",
            "x".repeat(64 * 1024 - 1)
        );
        assert_eq!(actual, prefix.len()..prefix.len() + "needle\t👩‍💻".len());
        assert_eq!(super::super::safe_text(&bytes[..start]), prefix);
        assert_eq!(&super::super::display_text(&bytes)[actual], "needle\t👩‍💻");

        file.rewind().unwrap();
        let cancel = Cancellation::default();
        cancel.stop();
        assert!(
            source_offsets(&mut file, 0..1, &|| cancel.check())
                .unwrap_err()
                .is::<Cancelled>()
        );
        file.rewind().unwrap();
        file.set_len(10).unwrap();
        assert!(source_offsets(&mut file, start as u64..end as u64, &|| Ok(())).is_err());
    }
}
