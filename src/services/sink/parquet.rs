//! Streaming Parquet sink that writes traversal output incrementally.

use super::{ScanSink, SinkFinish};
use crate::io::snapshot::{
    create_entries_batch, create_errors_batch, create_metadata_batch, snapshot_schema,
};
use crate::{DirectoryEntry, ErrorItem, SnapshotMeta};
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use std::fs::File;
use std::io::{Error, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Default number of entries buffered before flushing to Parquet.
const DEFAULT_BUFFER_CAPACITY: usize = 4_096;

/// Sink implementation that streams entries directly into a Parquet file.
pub struct ParquetStreamSink {
    writer: Option<ArrowWriter<File>>,
    schema: Arc<arrow_schema::Schema>,
    buffer: Vec<DirectoryEntry>,
    buffer_capacity: usize,
    errors: Vec<ErrorItem>,
    entry_count: u64,
    metadata: Option<SnapshotMeta>,
    output_path: PathBuf,
}

impl ParquetStreamSink {
    /// Create a new streaming sink targeting the provided snapshot path.
    pub fn try_new<P: AsRef<Path>>(path: P, buffer_capacity: Option<usize>) -> Result<Self> {
        let path_ref = path.as_ref();

        if let Some(parent) = path_ref.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = File::create(path_ref)?;
        let schema = snapshot_schema();
        let props = WriterProperties::builder().build();
        let writer =
            ArrowWriter::try_new(file, schema.clone(), Some(props)).map_err(Error::other)?;

        Ok(Self {
            writer: Some(writer),
            schema,
            buffer: Vec::new(),
            buffer_capacity: buffer_capacity.unwrap_or(DEFAULT_BUFFER_CAPACITY).max(1),
            errors: Vec::new(),
            entry_count: 0,
            metadata: None,
            output_path: path_ref.to_path_buf(),
        })
    }

    fn flush_entries(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let batch = create_entries_batch(&self.schema, &self.buffer)?;
        if let Some(writer) = self.writer.as_mut() {
            writer.write(&batch).map_err(Error::other)?;
            self.buffer.clear();
        } else {
            return Err(Error::other("Parquet writer already closed before flush"));
        }

        Ok(())
    }
}

impl ScanSink for ParquetStreamSink {
    fn record_entry(&mut self, entry: DirectoryEntry) -> Result<()> {
        self.entry_count = self.entry_count.saturating_add(1);
        self.buffer.push(entry);

        if self.buffer.len() >= self.buffer_capacity {
            self.flush_entries()?;
        }

        Ok(())
    }

    fn record_error(&mut self, error: ErrorItem) -> Result<()> {
        self.errors.push(error);
        Ok(())
    }

    fn set_metadata(&mut self, meta: &SnapshotMeta) -> Result<()> {
        self.metadata = Some(meta.clone());
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> Result<SinkFinish> {
        self.flush_entries()?;

        let mut writer = self.writer.take().ok_or_else(|| {
            Error::other(format!(
                "Parquet writer for {} already closed",
                self.output_path.display()
            ))
        })?;

        if !self.errors.is_empty() {
            let error_batch = create_errors_batch(&self.schema, &self.errors)?;
            writer.write(&error_batch).map_err(Error::other)?;
        }

        let meta = self.metadata.ok_or_else(|| {
            Error::other("snapshot metadata must be provided before finishing the Parquet sink")
        })?;

        let metadata_batch = create_metadata_batch(&self.schema, &meta)?;
        writer.write(&metadata_batch).map_err(Error::other)?;

        writer.close().map_err(Error::other)?;

        Ok(SinkFinish::new(Vec::new(), self.errors, self.entry_count))
    }
}
