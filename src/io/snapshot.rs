//! Parquet snapshot read/write operations
//!
//! This module provides functionality to save and load directory scan results
//! using Apache Parquet format for efficient storage and retrieval.

use crate::{DirectoryEntry, ErrorItem, SnapshotMeta};
use parquet::arrow::ArrowWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::file::properties::WriterProperties;
use std::fs::File;
use std::io::{Error, ErrorKind, Result};
use std::path::Path;
use std::sync::Arc;

use arrow_array::{
    Array, ArrayRef, RecordBatch, StringArray, UInt16Array, UInt32Array, UInt64Array,
};
use arrow_schema::{DataType, Field, Schema};

/// Write a snapshot to a Parquet file
///
/// Stores metadata, entries, and errors in a single Parquet file using
/// a simple flat schema approach for MVP implementation.
pub fn write_snapshot(
    path: &str,
    meta: &SnapshotMeta,
    entries: &[DirectoryEntry],
    errors: &[ErrorItem],
) -> Result<()> {
    let file_path = Path::new(path);

    // Create parent directory if it doesn't exist
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = File::create(file_path)?;

    // Define schema for entries (all fields nullable for flexibility)
    let schema = Arc::new(Schema::new(vec![
        Field::new("path", DataType::Utf8, true),
        Field::new("parent_path", DataType::Utf8, true),
        Field::new("depth", DataType::UInt16, true),
        Field::new("size_bytes", DataType::UInt64, true),
        Field::new("file_count", DataType::UInt32, true),
        Field::new("dir_count", DataType::UInt32, true),
        // Metadata fields (repeated for each entry row)
        Field::new("meta_scan_root", DataType::Utf8, true),
        Field::new("meta_started_at", DataType::Utf8, true),
        Field::new("meta_finished_at", DataType::Utf8, true),
        Field::new("meta_size_basis", DataType::Utf8, true),
        Field::new("meta_hardlink_policy", DataType::Utf8, true),
        // Error fields (stored separately, empty for entry rows)
        Field::new("error_path", DataType::Utf8, true),
        Field::new("error_code", DataType::Utf8, true),
        Field::new("error_message", DataType::Utf8, true),
    ]));

    let props = WriterProperties::builder().build();
    let mut writer =
        ArrowWriter::try_new(file, schema.clone(), Some(props)).map_err(Error::other)?;

    // Write entries
    if !entries.is_empty() {
        let batch = create_entries_batch(&schema, entries, meta)?;
        writer.write(&batch).map_err(Error::other)?;
    }

    // Write errors as separate rows
    if !errors.is_empty() {
        let batch = create_errors_batch(&schema, errors)?;
        writer.write(&batch).map_err(Error::other)?;
    }

    // If both are empty, write metadata only
    if entries.is_empty() && errors.is_empty() {
        let batch = create_metadata_batch(&schema, meta)?;
        writer.write(&batch).map_err(Error::other)?;
    }

    writer.close().map_err(Error::other)?;

    Ok(())
}

/// Read a snapshot from a Parquet file
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

        // Extract metadata from first row (it's repeated in all rows)
        if meta.is_none() && batch.num_rows() > 0 {
            meta = Some(extract_metadata(&batch)?);
        }

        // Extract entries and errors
        for row_idx in 0..batch.num_rows() {
            // Check if this is an error row
            let error_path_col = batch
                .column_by_name("error_path")
                .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing error_path column"))?;

            if let Some(error_path_array) = error_path_col.as_any().downcast_ref::<StringArray>()
                && !error_path_array.is_null(row_idx)
            {
                // This is an error row
                let error = extract_error(&batch, row_idx)?;
                errors.push(error);
                continue;
            }

            // Check if this is an entry row
            let path_col = batch
                .column_by_name("path")
                .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing path column"))?;

            if let Some(path_array) = path_col.as_any().downcast_ref::<StringArray>()
                && !path_array.is_null(row_idx)
                && !path_array.value(row_idx).is_empty()
            {
                let entry = extract_entry(&batch, row_idx)?;
                entries.push(entry);
            }
        }
    }

    let meta = meta.ok_or_else(|| Error::new(ErrorKind::InvalidData, "No metadata found"))?;

    Ok((meta, entries, errors))
}

fn create_entries_batch(
    schema: &Arc<Schema>,
    entries: &[DirectoryEntry],
    meta: &SnapshotMeta,
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

    // Metadata (repeated for each row)
    let meta_roots: ArrayRef =
        Arc::new(StringArray::from(vec![Some(meta.scan_root.as_str()); len]));
    let meta_started: ArrayRef =
        Arc::new(StringArray::from(vec![Some(meta.started_at.as_str()); len]));
    let meta_finished: ArrayRef = Arc::new(StringArray::from(vec![
        Some(meta.finished_at.as_str());
        len
    ]));
    let meta_basis: ArrayRef =
        Arc::new(StringArray::from(vec![Some(meta.size_basis.as_str()); len]));
    let meta_policy: ArrayRef =
        Arc::new(StringArray::from(vec![
            Some(meta.hardlink_policy.as_str());
            len
        ]));

    // Empty error fields for entry rows
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
            error_paths,
            error_codes,
            error_messages,
        ],
    )
    .map_err(Error::other)
}

fn create_errors_batch(schema: &Arc<Schema>, errors: &[ErrorItem]) -> Result<RecordBatch> {
    let len = errors.len();

    // Empty entry fields for error rows
    let paths: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let parent_paths: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let depths: ArrayRef = Arc::new(UInt16Array::from(vec![None::<u16>; len]));
    let sizes: ArrayRef = Arc::new(UInt64Array::from(vec![None::<u64>; len]));
    let file_counts: ArrayRef = Arc::new(UInt32Array::from(vec![None::<u32>; len]));
    let dir_counts: ArrayRef = Arc::new(UInt32Array::from(vec![None::<u32>; len]));

    // Empty metadata for error rows
    let meta_roots: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let meta_started: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let meta_finished: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let meta_basis: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));
    let meta_policy: ArrayRef = Arc::new(StringArray::from(vec![None::<&str>; len]));

    // Error fields
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
            error_paths,
            error_codes,
            error_messages,
        ],
    )
    .map_err(Error::other)
}

fn create_metadata_batch(schema: &Arc<Schema>, meta: &SnapshotMeta) -> Result<RecordBatch> {
    // Single row with just metadata
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
            error_paths,
            error_codes,
            error_messages,
        ],
    )
    .map_err(Error::other)
}

fn extract_metadata(batch: &RecordBatch) -> Result<SnapshotMeta> {
    let scan_root = get_string_value(batch, "meta_scan_root", 0)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing scan_root"))?;
    let started_at = get_string_value(batch, "meta_started_at", 0)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing started_at"))?;
    let finished_at = get_string_value(batch, "meta_finished_at", 0)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing finished_at"))?;
    let size_basis = get_string_value(batch, "meta_size_basis", 0)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing size_basis"))?;
    let hardlink_policy = get_string_value(batch, "meta_hardlink_policy", 0)?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing hardlink_policy"))?;

    Ok(SnapshotMeta {
        scan_root,
        started_at,
        finished_at,
        size_basis,
        hardlink_policy,
        excludes: vec![],
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
