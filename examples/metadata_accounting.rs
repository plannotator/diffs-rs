use std::{fs::File, sync::Arc};

use anyhow::{Context, Result, ensure};
use diffs_rs::storage;
use serde::Serialize;
use serde_json::json;

#[derive(Default, Serialize)]
struct Extent {
    count: usize,
    len: usize,
    capacity: usize,
}

impl Extent {
    fn add(&mut self, len: usize, capacity: usize) {
        self.count += 1;
        self.len += len;
        self.capacity += capacity;
    }
}

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let path = args.next().context("Supply a patch path")?;
    ensure!(args.next().is_none(), "Supply exactly one patch path");
    let mut doc = storage::Document::default();
    let spool = storage::read(File::open(path)?, |file| doc.append(file))?;

    let mut strings: [Extent; 5] = std::array::from_fn(|_| Extent::default());
    let mut raw_paths = Extent::default();
    let mut headers = Extent::default();
    let mut hunks = Extent::default();
    let mut max_hunks = 0;
    let mut metadata = 0;
    let mut source = 0;
    let mut projection = 0;
    let mut additions = 0;
    let mut deletions = 0;
    for file in &doc.files {
        raw_paths.add(file.raw_path.len(), file.raw_path.len());
        let labels = [
            &file.path,
            &file.old_path,
            &file.status,
            &file.old_mode,
            &file.new_mode,
        ];
        let mut label_capacity = 0;
        for (extent, label) in strings.iter_mut().zip(labels) {
            extent.add(label.len(), label.len());
            label_capacity += label.len();
        }
        let mut header_capacity = 0;
        for hunk in &file.hunks {
            headers.add(hunk.header.len(), hunk.header.len());
            header_capacity += hunk.header.len();
        }
        hunks.add(file.hunks.len(), file.hunks.len());
        max_hunks = max_hunks.max(file.hunks.len());
        let allocation = file.raw_path.len()
            + label_capacity
            + header_capacity
            + file.hunks.len() * size_of::<storage::HunkMetadata>();
        ensure!(
            allocation == file.metadata_bytes(),
            "Metadata accounting disagrees for file {}",
            file.id
        );
        metadata += allocation;
        source += file.source_index_bytes();
        projection += file.projection_bytes();
        additions += file.additions;
        deletions += file.deletions;
    }

    let text_len = strings.iter().map(|s| s.len).sum::<usize>() + headers.len;
    let text_capacity = strings.iter().map(|s| s.capacity).sum::<usize>() + headers.capacity;
    let used_hunk_bytes = hunks.len * size_of::<storage::HunkMetadata>();
    let spare_hunk_bytes = (hunks.capacity - hunks.len) * size_of::<storage::HunkMetadata>();
    ensure!(
        metadata == raw_paths.capacity + text_capacity + used_hunk_bytes + spare_hunk_bytes,
        "Aggregate metadata accounting disagrees"
    );
    let file_descriptors = doc.files.len() * size_of::<storage::FileIndex>();
    let file_list = doc.files.capacity() * size_of::<Arc<storage::FileIndex>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "representation": "immutable boxed metadata; capacity denotes retained extent, not allocator size",
            "scope": "Completed retained document; borrowed extents, no source reads or renderer/highlighter. Counts omit allocator rounding/headers, Arc headers, spool/cache containers, stacks and library allocations. Not heap profiling or RSS.",
            "files": doc.files.len(),
            "source_lines": doc.source_len(),
            "split_rows": doc.row_len(true),
            "unified_rows": doc.row_len(false),
            "additions": additions,
            "deletions": deletions,
            "hunks": hunks,
            "raw_paths": raw_paths,
            "max_hunks_per_file": max_hunks,
            "strings": {
                "path": strings[0], "old_path": strings[1], "status": strings[2],
                "old_mode": strings[3], "new_mode": strings[4], "hunk_header": headers,
            },
            "type_sizes": {
                "word": size_of::<usize>(), "string": size_of::<Box<str>>(),
                "hunk": size_of::<storage::HunkMetadata>(), "hunk_vector": size_of::<Box<[storage::HunkMetadata]>>(),
                "file_metadata": size_of::<storage::FileMetadata>(),
                "file_index": size_of::<storage::FileIndex>(),
            },
            "metadata_components": {
                "raw_path_bytes": raw_paths.capacity,
                "used_string_bytes": text_len,
                "spare_string_bytes": text_capacity - text_len,
                "used_hunk_descriptor_bytes": used_hunk_bytes,
                "spare_hunk_descriptor_bytes": spare_hunk_bytes,
                "inline_file_metadata_bytes": doc.files.len() * size_of::<storage::FileMetadata>(),
            },
            "disjoint_storage": {
                "source_index_bytes": source, "projection_bytes": projection,
                "metadata_allocation_bytes": metadata, "file_descriptor_bytes": file_descriptors,
                "file_list_capacity_bytes": file_list,
                "total_bytes": source + projection + metadata + file_descriptors + file_list,
            },
            "spool_bytes": spool.len(),
            "cached_source_bytes": spool.cached_bytes(),
            "reconciled": true,
        }))?
    );
    Ok(())
}
