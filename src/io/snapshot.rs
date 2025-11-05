//! Parquet snapshot read/write operations
//!
//! This module provides functionality to save and load directory scan results
//! using Apache Parquet format for efficient storage and retrieval.

use crate::{DirectoryEntry, ErrorItem, SnapshotMeta};
use arrow_array::{
    Array, ArrayRef, RecordBatch, StringArray, UInt16Array, UInt32Array, UInt64Array,
};
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::ArrowWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::file::properties::WriterProperties;
use std::fs::File;
use std::io::{Error, ErrorKind, Result};
use std::path::Path;
use std::sync::Arc;

/// Return the Arrow schema shared by snapshot writers and readers.
#[must_use]
pub fn snapshot_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("path", DataType::Utf8, true),
        Field::new("parent_path", DataType::Utf8, true),
        Field::new("depth", DataType::UInt16, true),
        Field::new("size_bytes", DataType::UInt64, true),
        Field::new("file_count", DataType::UInt32, true),
        Field::new("dir_count", DataType::UInt32, true),
        Field::new("meta_scan_root", DataType::Utf8, true),
        Field::new("meta_started_at", DataType::Utf8, true),
        Field::new("meta_finished_at", DataType::Utf8, true),
        Field::new("meta_size_basis", DataType::Utf8, true),
        Field::new("meta_hardlink_policy", DataType::Utf8, true),
        Field::new("meta_strategy", DataType::Utf8, true),
        Field::new("error_path", DataType::Utf8, true),
        Field::new("error_code", DataType::Utf8, true),
        Field::new("error_message", DataType::Utf8, true),
    ]))
}

/// Write a snapshot to a Parquet file.
pub fn write_snapshot(
    path: &str,
    meta: &SnapshotMeta,
    entries: &[DirectoryEntry],
    errors: &[ErrorItem],
) -> Result<()> {
    let file_path = Path::new(path);

    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = File::create(file_path)?;
    let schema = snapshot_schema();
    let props = WriterProperties::builder().build();
    let mut writer =
        ArrowWriter::try_new(file, schema.clone(), Some(props)).map_err(Error::other)?;

    if !entries.is_empty() {
        let batch = create_entries_batch(&schema, entries)?;
        writer.write(&batch).map_err(Error::other)?;
    }

    if !errors.is_empty() {
        let batch = create_errors_batch(&schema, errors)?;
        writer.write(&batch).map_err(Error::other)?;
    }

    let metadata_batch = create_metadata_batch(&schema, meta)?;
    writer.write(&metadata_batch).map_err(Error::other)?;

    writer.close().map_err(Error::other)?;
    Ok(())
}

/// Read a snapshot from a Parquet file.
pub fn read_snapshot(path: &str) -> Result<(SnapshotMeta, Vec<DirectoryEntry>, Vec<ErrorItem>)> {
    let file = File::open(path)?;

    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

    let mut reader = builder
        .build()
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

    let mut entries = Vec::new();
    let mut errors = Vec::new();
    let mut meta: Option<SnapshotMeta> = None;

    for batch_result in &mut reader {
        let batch = batch_result.map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

        for row_idx in 0..batch.num_rows() {
            let meta_value = get_string_value(&batch, "meta_scan_root", row_idx)?;
            let path_value = get_string_value(&batch, "path", row_idx)?;
            let error_path_value = get_string_value(&batch, "error_path", row_idx)?;

            if meta.is_none() && meta_value.is_some() {
                meta = Some(extract_metadata(&batch, row_idx)?);

                if path_value.is_none() && error_path_value.is_none() {
                    continue;
                }
            }

            if error_path_value.is_some() {
                let error = extract_error(&batch, row_idx)?;
                errors.push(error);
                continue;
            }

            if let Some(path) = path_value
                && !path.is_empty()
            {
                let entry = extract_entry(&batch, row_idx)?;
                entries.push(entry);
            }
        }
    }

    let meta = meta.ok_or_else(|| Error::new(ErrorKind::InvalidData, "No metadata found"))?;

    Ok((meta, entries, errors))
}

pub fn create_entries_batch(
    schema: &Arc<Schema>,
    entries: &[DirectoryEntry],
) -> Result<RecordBatch> {
    let len = entries.len();

    let paths: ArrayRef = Arc::new(StringArray::from(
        entries
            .iter()
            .map(|e| Some(e.path.as_str()))
            .collect::<Vec<_>>(),
    ));

    let parent_paths: ArrayRef = Arc::new(StringArray::from(
        entries
            .iter()
            .map(|e| e.parent_path.as_deref())
            .collect::<Vec<_>>(),
    ));

    let depths: ArrayRef = Arc::new(UInt16Array::from(
        entries.iter().map(|e| Some(e.depth)).collect::<Vec<_>>(),
    ));

    let sizes: ArrayRef = Arc::new(UInt64Array::from(
        entries
            .iter()
            .map(|e| Some(e.size_bytes))
            .collect::<Vec<_>>(),
    ));

    let file_counts: ArrayRef = Arc::new(UInt32Array::from(
        entries
            .iter()
            .map(|e| Some(e.file_count))
            .collect::<Vec<_>>(),
    ));

    let dir_counts: ArrayRef = Arc::new(UInt32Array::from(
        entries
            .iter()
            .map(|e| Some(e.dir_count))
            .collect::<Vec<_>>(),
    ));

    let meta_roots: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let meta_started: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let meta_finished: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let meta_basis: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let meta_policy: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let meta_strategy: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));

    let error_paths: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let error_codes: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let error_messages: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));

    RecordBatch::try_new(
        schema.clone(),
        vec![
            paths,
            parent_paths,
            depths,
            sizes,
            file_counts,
            dir_counts,
            meta_roots,
            meta_started,
            meta_finished,
            meta_basis,
            meta_policy,
            meta_strategy,
            error_paths,
            error_codes,
            error_messages,
        ],
    )
    .map_err(Error::other)
}

pub fn create_errors_batch(schema: &Arc<Schema>, errors: &[ErrorItem]) -> Result<RecordBatch> {
    let len = errors.len();

    let paths: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let parent_paths: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let depths: ArrayRef = Arc::new(UInt16Array::from(vec![None::<u16>; len]));
    let sizes: ArrayRef = Arc::new(UInt64Array::from(vec![None::<u64>; len]));
    let file_counts: ArrayRef = Arc::new(UInt32Array::from(vec![None::<u32>; len]));
    let dir_counts: ArrayRef = Arc::new(UInt32Array::from(vec![None::<u32>; len]));

    let meta_roots: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let meta_started: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let meta_finished: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let meta_basis: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let meta_policy: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let meta_strategy: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));

    let error_paths: ArrayRef = Arc::new(StringArray::from(
        errors
            .iter()
            .map(|e| Some(e.path.as_str()))
            .collect::<Vec<_>>(),
    ));
    let error_codes: ArrayRef = Arc::new(StringArray::from(
        errors
            .iter()
            .map(|e| Some(e.code.as_str()))
            .collect::<Vec<_>>(),
    ));
    let error_messages: ArrayRef = Arc::new(StringArray::from(
        errors
            .iter()
            .map(|e| Some(e.message.as_str()))
            .collect::<Vec<_>>(),
    ));

    RecordBatch::try_new(
        schema.clone(),
        vec![
            paths,
            parent_paths,
            depths,
            sizes,
            file_counts,
            dir_counts,
            meta_roots,
            meta_started,
            meta_finished,
            meta_basis,
            meta_policy,
            meta_strategy,
            error_paths,
            error_codes,
            error_messages,
        ],
    )
    .map_err(Error::other)
}

pub fn create_metadata_batch(schema: &Arc<Schema>, meta: &SnapshotMeta) -> Result<RecordBatch> {
    let paths: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; 1]));
    let parent_paths: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; 1]));
    let depths: ArrayRef = Arc::new(UInt16Array::from(vec![None::<u16>; 1]));
    let sizes: ArrayRef = Arc::new(UInt64Array::from(vec![None::<u64>; 1]));
    let file_counts: ArrayRef = Arc::new(UInt32Array::from(vec![None::<u32>; 1]));
    let dir_counts: ArrayRef = Arc::new(UInt32Array::from(vec![None::<u32>; 1]));

    let meta_roots: ArrayRef = Arc::new(StringArray::from(vec![Some(meta.scan_root.as_str()); 1]));
    let meta_started: ArrayRef =
        Arc::new(StringArray::from(vec![Some(meta.started_at.as_str()); 1]));
    let meta_finished: ArrayRef =
        Arc::new(StringArray::from(vec![Some(meta.finished_at.as_str()); 1]));
    let meta_basis: ArrayRef = Arc::new(StringArray::from(vec![Some(meta.size_basis.as_str()); 1]));
    let meta_policy: ArrayRef =
        Arc::new(StringArray::from(vec![
            Some(meta.hardlink_policy.as_str());
            1
        ]));
    let meta_strategy: ArrayRef =
        Arc::new(StringArray::from(vec![Some(meta.strategy.as_str()); 1]));

    let error_paths: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; 1]));
    let error_codes: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; 1]));
    let error_messages: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; 1]));

    RecordBatch::try_new(
        schema.clone(),
        vec![
            paths,
            parent_paths,
            depths,
            sizes,
            file_counts,
            dir_counts,
            meta_roots,
            meta_started,
            meta_finished,
            meta_basis,
            meta_policy,
            meta_strategy,
            error_paths,
            error_codes,
            error_messages,
        ],
    )
    .map_err(Error::other)
}

fn extract_metadata(batch: &RecordBatch, row: usize) -> Result<SnapshotMeta> {
    let scan_root = get_string_value(batch, "meta_scan_root", row)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing scan_root"))?;
    let started_at = get_string_value(batch, "meta_started_at", row)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing started_at"))?;
    let finished_at = get_string_value(batch, "meta_finished_at", row)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing finished_at"))?;
    let size_basis = get_string_value(batch, "meta_size_basis", row)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing size_basis"))?;
    let hardlink_policy = get_string_value(batch, "meta_hardlink_policy", row)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing hardlink_policy"))?;
    let strategy =
        get_string_value(batch, "meta_strategy", row)?.unwrap_or_else(|| "legacy".to_string());

    Ok(SnapshotMeta {
        scan_root,
        started_at,
        finished_at,
        size_basis,
        hardlink_policy,
        excludes: vec![],
        strategy,
    })
}

fn extract_entry(batch: &RecordBatch, row: usize) -> Result<DirectoryEntry> {
    let path = get_string_value(batch, "path", row)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing path"))?;
    let parent_path = get_string_value(batch, "parent_path", row)?;
    let depth = get_u16_value(batch, "depth", row)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing depth"))?;
    let size_bytes = get_u64_value(batch, "size_bytes", row)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing size_bytes"))?;
    let file_count = get_u32_value(batch, "file_count", row)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing file_count"))?;
    let dir_count = get_u32_value(batch, "dir_count", row)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing dir_count"))?;

    Ok(DirectoryEntry {
        path,
        parent_path,
        depth,
        size_bytes,
        file_count,
        dir_count,
    })
}

fn extract_error(batch: &RecordBatch, row: usize) -> Result<ErrorItem> {
    let path = get_string_value(batch, "error_path", row)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing error_path"))?;
    let code = get_string_value(batch, "error_code", row)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing error_code"))?;
    let message = get_string_value(batch, "error_message", row)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing error_message"))?;

    Ok(ErrorItem {
        path,
        code,
        message,
    })
}

fn get_string_value(batch: &RecordBatch, col_name: &str, row: usize) -> Result<Option<String>> {
    let col = batch.column_by_name(col_name).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Missing column: {col_name}"),
        )
    })?;

    let array = col.as_any().downcast_ref::<StringArray>().ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Invalid type for: {col_name}"),
        )
    })?;

    if array.is_null(row) {
        Ok(None)
    } else {
        Ok(Some(array.value(row).to_string()))
    }
}

fn get_u16_value(batch: &RecordBatch, col_name: &str, row: usize) -> Result<Option<u16>> {
    let col = batch.column_by_name(col_name).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Missing column: {col_name}"),
        )
    })?;

    let array = col.as_any().downcast_ref::<UInt16Array>().ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Invalid type for: {col_name}"),
        )
    })?;

    if array.is_null(row) {
        Ok(None)
    } else {
        Ok(Some(array.value(row)))
    }
}

fn get_u32_value(batch: &RecordBatch, col_name: &str, row: usize) -> Result<Option<u32>> {
    let col = batch.column_by_name(col_name).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Missing column: {col_name}"),
        )
    })?;

    let array = col.as_any().downcast_ref::<UInt32Array>().ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Invalid type for: {col_name}"),
        )
    })?;

    if array.is_null(row) {
        Ok(None)
    } else {
        Ok(Some(array.value(row)))
    }
}

fn get_u64_value(batch: &RecordBatch, col_name: &str, row: usize) -> Result<Option<u64>> {
    let col = batch.column_by_name(col_name).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Missing column: {col_name}"),
        )
    })?;

    let array = col.as_any().downcast_ref::<UInt64Array>().ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Invalid type for: {col_name}"),
        )
    })?;

    if array.is_null(row) {
        Ok(None)
    } else {
        Ok(Some(array.value(row)))
    }
}
