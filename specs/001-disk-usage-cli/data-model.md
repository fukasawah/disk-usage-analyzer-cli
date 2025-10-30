# Data Model (Phase 1)

This defines entities and relationships used by the Disk Usage CLI MVP.

## Entities

### DirectoryEntry
- path: string (absolute or path relative to scan root)
- parent_path: string? (optional, empty for root)
- depth: u16 (distance from root)
- size_bytes: u64 (aggregate size for this directory item under the chosen basis)
- file_count: u32
- dir_count: u32

Validation:
- depth(root) == 0
- size_bytes must be >= sum of child sizes if using inclusive aggregation per directory; we will report per-node totals inclusive of descendants for summary

### SnapshotMeta
- scan_root: string
- started_at: datetime (RFC3339)
- finished_at: datetime (RFC3339)
- size_basis: enum [logical, physical]
- hardlink_policy: enum [dedupe, count]
- excludes: string[] (MVP: default-only; record for transparency)

### ErrorItem
- path: string
- code: string (ENOENT, EACCES, IO, etc.)
- message: string

## Relationships
- DirectoryEntry forms a tree through `parent_path`. Entries are stored flat with depth for fast filtering; parent_path enables reconstruction.
- Snapshot contains many DirectoryEntry and ErrorItem rows.

## State Transitions
- Scan Initiated -> Traversal -> Aggregation -> Output printed
- Optional: Aggregation -> Snapshot Write (.parquet)
- Viewing: Snapshot Read -> Filter/Sort -> Output printed

## Parquet Schema (initial)
- Discriminator column: `table` with values `entries` or `meta` or `errors`.

Entries columns:
- table: literal "entries"
- path: BYTE_ARRAY (UTF8)
- parent_path: BYTE_ARRAY (UTF8, optional)
- depth: INT32 (unsigned logical)
- size_bytes: INT64
- file_count: INT32
- dir_count: INT32

Meta columns:
- table: literal "meta"
- scan_root: BYTE_ARRAY (UTF8)
- started_at: BYTE_ARRAY (RFC3339 string)
- finished_at: BYTE_ARRAY (RFC3339 string)
- size_basis: BYTE_ARRAY (enum string)
- hardlink_policy: BYTE_ARRAY (enum string)
- excludes: LIST<BYTE_ARRAY UTF8>

Errors columns:
- table: literal "errors"
- path: BYTE_ARRAY (UTF8)
- code: BYTE_ARRAY (UTF8)
- message: BYTE_ARRAY (UTF8)
