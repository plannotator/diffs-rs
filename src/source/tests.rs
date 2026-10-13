use super::*;

#[test]
fn range_readers_reject_corrupt_line_boundaries_before_reading_other_snapshots() {
    let store = StoreWriter::new().unwrap();
    let mut sources = SourceStore::sharing(&store.store()).unwrap();
    let snapshot = sources
        .capture(b"one\ntwo\n".as_slice(), &|| Ok(()))
        .unwrap();
    let following = sources
        .capture(b"different snapshot\n".as_slice(), &|| Ok(()))
        .unwrap();
    assert!(following.len() > snapshot.len());

    for offsets in [[0u64, 5, 4], [0, 4, 9]] {
        sources.index.rollback(0).unwrap();
        for offset in offsets {
            sources.index.append(&offset.to_le_bytes()).unwrap();
        }
        assert!(snapshot.range(1..2).is_err());
        assert!(snapshot.uncached_range(1..2).is_err());
        assert!(snapshot.uncached_lines().line(2, 0).is_err());
    }
}

#[test]
fn uncached_lines_preserve_snapshot_bounds_across_index_and_body_windows() {
    let store = StoreWriter::new().unwrap();
    let mut sources = SourceStore::sharing(&store.store()).unwrap();
    sources
        .capture(b"preceding snapshot\n".as_slice(), &|| Ok(()))
        .unwrap();
    let mut lines: Vec<Vec<u8>> = (0..8_200)
        .map(|line| format!("\t日本語 {line}\r\n").into_bytes())
        .collect();
    lines[1] = vec![b'x'; CHUNK + 17];
    lines[1].push(b'\n');
    let last = lines.last_mut().unwrap();
    last.truncate(last.len() - 2);
    last.extend_from_slice(b"\xfftail");
    let snapshot = sources
        .capture(lines.concat().as_slice(), &|| Ok(()))
        .unwrap();
    sources
        .capture(b"following snapshot\n".as_slice(), &|| Ok(()))
        .unwrap();
    assert_eq!(snapshot.lines(), lines.len() as u32);
    assert!(!snapshot.final_newline());

    let mut reader = snapshot.uncached_lines();
    for index in [8_199, 8_189, 8_191, 0, 1, 8_190, 8_192, 2] {
        for from in [0, 1, lines[index].len()] {
            let mut actual = Vec::new();
            reader
                .line(index as u32 + 1, from as u64)
                .unwrap()
                .read_to_end(&mut actual)
                .unwrap();
            assert_eq!(actual, lines[index][from..]);
        }
    }
    assert!(reader.line(0, 0).is_err());
    assert!(reader.line(snapshot.lines() + 1, 0).is_err());
    assert!(reader.line(1, lines[0].len() as u64 + 1).is_err());
    assert_eq!(store.store().cached_bytes(), 0);
}

#[test]
fn search_readers_survive_rollback_of_unrelated_unpublished_sources() {
    let store = StoreWriter::new().unwrap();
    let mut sources = SourceStore::sharing(&store.store()).unwrap();
    let snapshot = sources
        .capture(b"kept\r\nfinal".as_slice(), &|| Ok(()))
        .unwrap();
    let checkpoint = sources.checkpoint();
    let unfinished = b"unpublished\n".repeat(CHUNK);
    sources.capture(unfinished.as_slice(), &|| Ok(())).unwrap();
    let mut reader = snapshot.uncached_lines();
    sources.rollback(checkpoint).unwrap();
    for (number, expected) in [(2, b"final".as_slice()), (1, b"kept\r\n".as_slice())] {
        let mut actual = Vec::new();
        reader
            .line(number, 0)
            .unwrap()
            .read_to_end(&mut actual)
            .unwrap();
        assert_eq!(actual, expected);
    }
    assert_eq!(store.store().cached_bytes(), 0);
}
