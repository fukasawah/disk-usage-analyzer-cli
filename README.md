# Disk Usage CLI (dux)

A fast, memory-efficient command-line tool to analyze disk usage for directory trees, written in Rust.

## Features

- **Fast traversal**: Efficiently scan large directory structures
- **Multiple views**: Summarize immediate children, drill down into subdirectories
- **Flexible sizing**: Choose between physical (actual disk usage) or logical (file size) basis
- **Snapshot support**: Save scan results to Parquet format for later analysis without re-scanning
- **JSON output**: Machine-readable format for integration with other tools
- **Error resilient**: Continues scanning even when encountering permission errors
- **Safe traversal**: Doesn't follow symlinks or cross filesystem boundaries by default

## Quick Start

### Build

```bash
cargo build --release
```

### Basic Usage

Scan a directory:
```bash
./target/release/dux scan /path/to/directory
```

Get JSON output:
```bash
./target/release/dux scan /path/to/directory --json
```

Save and view snapshots:
```bash
./target/release/dux scan /large/dir --snapshot scan.parquet
./target/release/dux view --from-snapshot scan.parquet
```

## Documentation

- [Quickstart Guide](specs/001-disk-usage-cli/quickstart.md) - Detailed usage examples
- [Feature Specification](specs/001-disk-usage-cli/spec.md) - Complete feature requirements
- [Implementation Plan](specs/001-disk-usage-cli/plan.md) - Technical design and architecture

## Project Structure

```
rs-disk-usage/
├── src/
│   ├── lib.rs              # Public API and core types
│   ├── models/             # Data structures
│   ├── services/           # Core business logic
│   │   ├── traverse.rs     # Directory traversal
│   │   ├── aggregate.rs    # Size aggregation
│   │   ├── size.rs         # Size computation
│   │   └── format.rs       # Human-readable formatting
│   ├── io/                 # I/O operations
│   │   └── snapshot.rs     # Parquet snapshot handling
│   ├── cli/                # Command-line interface
│   └── bin/
│       └── dux.rs          # Main binary entry point
├── tests/
│   ├── unit/               # Unit tests
│   ├── integration/        # Integration tests
│   └── contract/           # API contract tests
└── specs/
    └── 001-disk-usage-cli/ # Feature specifications
```

## Development

### Prerequisites

- Rust 1.77 or later
- Cargo

### Running Tests

```bash
cargo test
```

### Code Quality

```bash
cargo clippy
cargo fmt
```

## License

See LICENSE file for details.

## Contributing

Contributions are welcome! Please read the specification documents in `specs/001-disk-usage-cli/` to understand the feature requirements and design decisions.
