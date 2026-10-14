use super::*;

pub(crate) fn copy_index(file: &mut FileIndex) -> (StoreWriter, tempfile::TempPath) {
    let records: Vec<_> = (0..file.source.len())
        .map(|n| {
            let (offset, len) = file.source.range(n).unwrap();
            (offset, len, file.source.no_newline(n))
        })
        .collect();
    let (mut writer, path) = StoreWriter::named_sharing(&file.spool).unwrap();
    file.source = SourceSpans::new(
        &mut writer,
        0,
        records.iter().copied(),
        file.spool.len(),
        &|| Ok(()),
    )
    .unwrap();
    (writer, path)
}

#[test]
fn cached_geometry_does_not_resolve_paged_spans_but_misses_propagate_errors() {
    let mut doc = Document::default();
    read(
        include_bytes!("../../fixtures/position/review.patch").as_slice(),
        |file| doc.append(file),
    )
    .unwrap();
    let (mut index, _path) = copy_index(Arc::get_mut(&mut doc.files[0]).unwrap());
    let cache = crate::geometry::Cache::default();
    let line = cache.get(&doc, 0, Some(12)).unwrap();
    index.rollback(0).unwrap();
    assert!(std::rc::Rc::ptr_eq(
        &line,
        &cache.get(&doc, 0, Some(12)).unwrap()
    ));
    for (id, width) in [(0, None), (0, Some(13)), (1, Some(12))] {
        let error = cache.get(&doc, id, width).err().unwrap();
        assert!(error.is::<crate::source_spans::ReadFailure>());
    }
    assert!(
        doc.files[0]
            .write_canonical(&mut Vec::new())
            .unwrap_err()
            .is::<crate::source_spans::ReadFailure>()
    );
}

#[test]
fn canonical_export_preserves_the_callers_output_buffer() {
    use std::io::{BufWriter, Seek};

    let mut doc = Document::default();
    read(
        b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -0,0 +1 @@\n+value\n".as_slice(),
        |file| doc.append(file),
    )
    .unwrap();
    let file = &doc.files[0];
    let mut output = BufWriter::new(tempfile::tempfile().unwrap());
    file.write_canonical(&mut output).unwrap();
    assert_eq!(output.get_ref().metadata().unwrap().len(), 0);
    let expected = output.buffer().to_vec();
    assert!(expected.ends_with(b"value\n"));
    output.flush().unwrap();
    let mut output = output.into_inner().unwrap();
    output.rewind().unwrap();
    let mut actual = Vec::new();
    output.read_to_end(&mut actual).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(file.spool.cached_bytes(), 0);
}

#[test]
fn canonical_export_rejects_reordered_spans() {
    let mut doc = Document::default();
    read(
        include_bytes!("../../fixtures/review.patch").as_slice(),
        |file| doc.append(file),
    )
    .unwrap();
    let file = Arc::get_mut(&mut doc.files[0]).unwrap();
    let mut records: Vec<_> = (0..file.source.len())
        .map(|n| {
            let (offset, len) = file.source.range(n).unwrap();
            (offset, len, file.source.no_newline(n))
        })
        .collect();
    records.swap(1, 2);
    let mut index = StoreWriter::sharing(&file.spool).unwrap();
    file.source = SourceSpans::new(
        &mut index,
        0,
        records.iter().copied(),
        file.spool.len(),
        &|| Ok(()),
    )
    .unwrap();
    let error = file.write_canonical(&mut Vec::new()).unwrap_err();
    assert!(error.to_string().contains("overlap or are out of order"));
}

#[test]
fn canonical_export_propagates_real_body_read_and_write_failures() {
    use std::{fs::OpenOptions, os::unix::net::UnixStream, thread, time::Duration};

    let mut patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -0,0 +1 @@\n+".to_vec();
    patch.extend(std::iter::repeat_n(b'z', 2 * 1024 * 1024));
    patch.push(b'\n');
    let cache = StoreWriter::new().unwrap();
    let (writer, path) = StoreWriter::named_sharing(&cache.store()).unwrap();
    let mut doc = Document::default();
    read_with(patch.as_slice(), writer, &|| Ok(()), |file, _| {
        doc.append(file)
    })
    .unwrap();
    let file = &doc.files[0];
    let (mut output, mut peer) = UnixStream::pair().unwrap();
    output
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let consumer = thread::spawn(move || {
        let mut bytes = [0; 1024];
        peer.read_exact(&mut bytes).unwrap();
        bytes
    });
    let error = file.write_canonical(&mut output).unwrap_err();
    let received = consumer.join().unwrap();
    assert_eq!(&received[received.len() - 16..], b"zzzzzzzzzzzzzzzz");
    assert_eq!(
        error.downcast_ref::<io::Error>().unwrap().kind(),
        io::ErrorKind::BrokenPipe
    );
    assert_eq!(file.spool.cached_bytes(), 0);

    let (offset, _) = file.source.cursor(0..1).unwrap().next().unwrap().unwrap();
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(offset + 64 * 1024 + 1)
        .unwrap();
    let mut output = tempfile::tempfile().unwrap();
    let error = file.write_canonical(&mut output).unwrap_err();
    assert_eq!(
        error.downcast_ref::<io::Error>().unwrap().kind(),
        io::ErrorKind::UnexpectedEof
    );
    assert!(output.metadata().unwrap().len() > 64 * 1024);
    assert_eq!(file.spool.cached_bytes(), 0);
}

#[test]
fn source_ranges_propagate_real_index_and_body_truncation() {
    use std::fs::OpenOptions;

    let mut patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -0,0 +1 @@\n+".to_vec();
    patch.extend(std::iter::repeat_n(b'z', 256 * 1024));
    patch.push(b'\n');
    let cache = StoreWriter::new().unwrap();
    let (writer, path) = StoreWriter::named_sharing(&cache.store()).unwrap();
    let mut doc = Document::default();
    read_with(patch.as_slice(), writer, &|| Ok(()), |file, _| {
        doc.append(file)
    })
    .unwrap();
    let (mut index, _index_path) = copy_index(Arc::get_mut(&mut doc.files[0]).unwrap());
    let file = &doc.files[0];
    let (offset, _) = file.source.cursor(0..1).unwrap().next().unwrap().unwrap();
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(offset + 65537)
        .unwrap();
    let mut output = tempfile::tempfile().unwrap();
    let error = file
        .write_source_lines(false, 1..=1, &mut output, &|| Ok(()))
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<io::Error>().unwrap().kind(),
        io::ErrorKind::UnexpectedEof
    );
    assert_eq!(output.metadata().unwrap().len(), 65537);
    assert_eq!(file.spool.cached_bytes(), 0);

    index.rollback(0).unwrap();
    let mut output = tempfile::tempfile().unwrap();
    let error = file
        .write_source_lines(false, 1..=1, &mut output, &|| Ok(()))
        .unwrap_err();
    assert!(error.is::<crate::source_spans::ReadFailure>());
    assert_eq!(output.metadata().unwrap().len(), 0);
}

#[test]
fn range_read_skips_opposite_side_payload_without_losing_source_order() {
    let mut patch = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1,2 +1 @@\n-old 1\n+".to_vec();
    patch.extend(std::iter::repeat_n(b'z', 256 * 1024));
    patch.extend_from_slice(b"\n-old 2\n");
    let mut doc = Document::default();
    read(patch.as_slice(), |file| doc.append(file)).unwrap();
    let mut output = Vec::new();
    let written = doc.files[0]
        .write_source_lines(true, 1..=2, &mut output, &|| Ok(()))
        .unwrap();
    assert_eq!(written, 12);
    assert_eq!(output, b"old 1\nold 2\n");
    assert_eq!(doc.files[0].spool.cached_bytes(), 0);
}

#[test]
fn private_temporary_files_are_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    // Linux creates unnamed files with the umask-derived mode unless it is set explicitly.
    let file = private_tempfile().unwrap();
    let metadata = file.metadata().unwrap();
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
}
