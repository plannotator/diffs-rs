use std::{fs::File, path::Path, rc::Rc};

use native_diff::{geometry, model::RowKind, storage, text};

#[test]
fn source_references_keep_patch_destinations_and_separate_geometry_between_documents() {
    let mut documents = Vec::new();
    for fixture in ["fixtures/review.patch", "fixtures/position/review.patch"] {
        let mut doc = storage::Document::default();
        let mut retained = Vec::new();
        storage::read(
            File::open(Path::new(env!("CARGO_MANIFEST_DIR")).join(fixture)).unwrap(),
            |file| {
                assert!(file.source_ref(file.source_end()).is_err());
                if file.source_base > 0 {
                    assert!(file.source_ref(file.source_base - 1).is_err());
                }
                for id in file.source_base..file.source_end() {
                    retained.push((
                        id,
                        file.source_id(id)?,
                        file.source_ref(id)?,
                        file.bytes(id)?,
                    ));
                }
                doc.append(file)?;
                for (id, identity, reference, bytes) in &retained {
                    let file = doc.file_for_source(*id);
                    assert_eq!(file.source_id(*id)?, *identity);
                    assert_eq!(file.source_ref(*id)?, *reference);
                    assert_eq!(file.spool.read_source(*reference)?, *bytes);
                    for split in [false, true] {
                        let row = doc.row(doc.row_for_source(*id, split), split).unwrap();
                        assert!(matches!(row.kind, RowKind::Code { left, right }
                            if left == Some(*id) || right == Some(*id)));
                    }
                }
                Ok(())
            },
        )
        .unwrap();
        documents.push(doc);
    }
    let geometry = geometry::Cache::default();
    for width in [None, Some(12), Some(70)] {
        let first = geometry.get(&documents[0], 0, width).unwrap();
        let second = geometry.get(&documents[1], 0, width).unwrap();
        assert!(!Rc::ptr_eq(&first, &second));
        for (doc, line) in documents.iter().zip([&first, &second]) {
            let actual: String = line.glyphs_from_column(0).map(|g| g.text).collect();
            assert_eq!(actual, text::display_text(&doc.bytes(0).unwrap()));
        }
        assert!(Rc::ptr_eq(
            &first,
            &geometry.get(&documents[0], 0, width).unwrap()
        ));
        assert!(Rc::ptr_eq(
            &second,
            &geometry.get(&documents[1], 0, width).unwrap()
        ));
    }
    let old = documents[0].files[0].source_ref(0).unwrap();
    assert!(documents[1].files[0].spool.read_source(old).is_err());
}
