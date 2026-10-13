use super::*;
use std::{
    fs::OpenOptions,
    io::{Seek, SeekFrom, Write},
};

fn source(bytes: &[u8]) -> std::fs::File {
    let mut file = tempfile::tempfile().unwrap();
    file.write_all(bytes).unwrap();
    file.rewind().unwrap();
    file
}

fn expected(
    bytes: &[u8],
    query: &str,
    starts: Range<u64>,
    direction: Direction,
) -> Option<Range<u64>> {
    let mut matches = bytes
        .windows(query.len())
        .enumerate()
        .filter_map(|(offset, bytes)| {
            (bytes == query.as_bytes() && starts.contains(&(offset as u64)))
                .then_some(offset as u64..(offset + query.len()) as u64)
        });
    match direction {
        Direction::Forward => matches.next(),
        Direction::Backward => matches.next_back(),
    }
}

#[test]
fn overlapping_matches_obey_start_boundaries_in_both_directions() {
    let text = b"ababa";
    let mut file = source(text);
    let mut literal = Literal::new("aba").unwrap();
    for start in 0..=text.len() as u64 {
        for end in start..=text.len() as u64 + 1 {
            for direction in [Direction::Forward, Direction::Backward] {
                file.seek(SeekFrom::Start(start)).unwrap();
                let actual = literal
                    .find(&mut file, start..end, direction, &|| Ok(()))
                    .unwrap();
                assert_eq!(
                    actual,
                    expected(text, "aba", start..end, direction),
                    "{start}..{end} {direction:?}"
                );
            }
        }
    }
    file.rewind().unwrap();
    assert_eq!(
        literal
            .find(&mut file, 0..2, Direction::Backward, &|| Ok(()))
            .unwrap(),
        Some(0..3)
    );
}

#[test]
fn unicode_queries_cross_chunk_boundaries_without_sanitizing_source_offsets() {
    for query in [
        "日本語 café e\u{301} 👩‍💻".to_string(),
        "é".repeat(QUERY_BYTES / 2),
    ] {
        let mut text = vec![b'.'; 3 * CHUNK];
        text[0] = 0xff;
        let first = CHUNK - query.len() / 2;
        let second = 2 * CHUNK - 1;
        text[first..first + query.len()].copy_from_slice(query.as_bytes());
        text[second..second + query.len()].copy_from_slice(query.as_bytes());
        let mut file = source(&text);
        let mut literal = Literal::new(&query).unwrap();
        for starts in [
            0..u64::MAX,
            (first + 1) as u64..u64::MAX,
            first as u64..first as u64 + 1,
            0..second as u64,
            (second + 1) as u64..text.len() as u64,
        ] {
            for direction in [Direction::Forward, Direction::Backward] {
                file.seek(SeekFrom::Start(starts.start)).unwrap();
                let actual = literal
                    .find(&mut file, starts.clone(), direction, &|| Ok(()))
                    .unwrap();
                assert_eq!(
                    actual,
                    expected(&text, &query, starts.clone(), direction),
                    "{starts:?} {direction:?}"
                );
                assert!(literal.buffer.capacity() < CHUNK + QUERY_BYTES);
            }
        }
    }
}

#[test]
fn query_validation_and_real_store_errors_are_not_reported_as_no_match() {
    for query in [
        String::new(),
        "one\ntwo".into(),
        "one\rtwo".into(),
        "é".repeat(QUERY_BYTES / 2 + 1),
    ] {
        assert!(Literal::new(&query).is_err());
    }
    assert!(Literal::new("\t日本語").is_ok());

    let shared = crate::storage::StoreWriter::new().unwrap();
    let (mut writer, path) = crate::storage::StoreWriter::named_sharing(&shared.store()).unwrap();
    writer.append(&vec![b'x'; 3 * CHUNK]).unwrap();
    let store = writer.finish();
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(CHUNK as u64 + 7)
        .unwrap();
    let mut reader = store.reader(0, store.len()).unwrap();
    let mut literal = Literal::new("needle").unwrap();
    let error = literal
        .find(&mut reader, 0..u64::MAX, Direction::Forward, &|| Ok(()))
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<io::Error>().unwrap().kind(),
        io::ErrorKind::UnexpectedEof
    );
}

#[test]
fn no_match_scan_remains_cancellable_and_the_buffer_is_reused() {
    let mut file = source(&vec![b'x'; 4 * CHUNK]);
    let mut literal = Literal::new("needle").unwrap();
    let cancel = crate::input::Cancellation::default();
    let checks = std::cell::Cell::new(0);
    let error = literal
        .find(&mut file, 0..u64::MAX, Direction::Backward, &|| {
            let count = checks.get() + 1;
            checks.set(count);
            if count == 4 {
                cancel.stop();
            }
            cancel.check()
        })
        .unwrap_err();
    assert!(error.is::<crate::input::Cancelled>());
    assert!(file.stream_position().unwrap() <= CHUNK as u64);
    let capacity = literal.buffer.capacity();
    file.rewind().unwrap();
    assert_eq!(
        literal
            .find(&mut file, 0..u64::MAX, Direction::Forward, &|| Ok(()))
            .unwrap(),
        None
    );
    assert_eq!(literal.buffer.capacity(), capacity);
}
