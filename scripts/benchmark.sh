#!/bin/bash
# Benchmark script for performance validation
# Measures scan and view operation performance

set -e

# Configuration
BENCH_DIR="${BENCH_DIR:-/tmp/dux-benchmark}"
SNAPSHOT_FILE="${SNAPSHOT_FILE:-/tmp/bench-snapshot.parquet}"
BINARY="${BINARY:-./target/release/dux}"
FILE_COUNT="${FILE_COUNT:-10000}"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "===================================="
echo "rs-disk-usage Performance Benchmark"
echo "===================================="
echo ""

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo -e "${RED}Error: Binary not found at $BINARY${NC}"
    echo "Build with: cargo build --release"
    exit 1
fi

# Create benchmark fixture if it doesn't exist
if [ ! -d "$BENCH_DIR" ] || [ "$(find "$BENCH_DIR" -type f | wc -l)" -lt "$FILE_COUNT" ]; then
    echo -e "${YELLOW}Creating benchmark fixture with $FILE_COUNT files...${NC}"
    rm -rf "$BENCH_DIR"
    mkdir -p "$BENCH_DIR"
    
    # Create directory structure
    for i in $(seq 1 100); do
        mkdir -p "$BENCH_DIR/dir$i/subdir$((i % 10))"
    done
    
    # Create files
    for i in $(seq 1 "$FILE_COUNT"); do
        dir_num=$((i % 100 + 1))
        subdir_num=$((dir_num % 10))
        echo "Test file $i content" > "$BENCH_DIR/dir$dir_num/subdir$subdir_num/file$i.txt"
    done
    
    echo -e "${GREEN}Fixture created: $BENCH_DIR${NC}"
    echo ""
fi

echo "Benchmark Configuration:"
echo "  Binary: $BINARY"
echo "  Test directory: $BENCH_DIR"
echo "  File count: $(find "$BENCH_DIR" -type f | wc -l)"
echo "  Directory count: $(find "$BENCH_DIR" -type d | wc -l)"
echo "  Snapshot: $SNAPSHOT_FILE"
echo ""

# Function to measure time (cross-platform)
measure_time() {
    local cmd="$1"
    local start=$(date +%s%N 2>/dev/null || date +%s)
    eval "$cmd"
    local end=$(date +%s%N 2>/dev/null || date +%s)
    
    # Calculate duration (handle both nanoseconds and seconds)
    if [ ${#start} -gt 10 ]; then
        # Nanoseconds available
        local duration=$(( (end - start) / 1000000 ))
        echo "$duration"
    else
        # Only seconds available (macOS)
        local duration=$(( (end - start) * 1000 ))
        echo "$duration"
    fi
}

# Benchmark: Scan operation
echo -e "${YELLOW}Running scan benchmark...${NC}"
rm -f "$SNAPSHOT_FILE"
SCAN_TIME=$(measure_time "$BINARY scan '$BENCH_DIR' --snapshot '$SNAPSHOT_FILE' > /dev/null 2>&1")
echo -e "${GREEN}Scan completed in ${SCAN_TIME}ms${NC}"

# Verify snapshot was created
if [ ! -f "$SNAPSHOT_FILE" ]; then
    echo -e "${RED}Error: Snapshot file not created${NC}"
    exit 1
fi

SNAPSHOT_SIZE=$(du -h "$SNAPSHOT_FILE" | cut -f1)
echo "Snapshot size: $SNAPSHOT_SIZE"
echo ""

# Benchmark: View operation (top 100)
echo -e "${YELLOW}Running view benchmark (top 100)...${NC}"
VIEW_TIME=$(measure_time "$BINARY view '$SNAPSHOT_FILE' --top 100 > /dev/null 2>&1")
echo -e "${GREEN}View completed in ${VIEW_TIME}ms${NC}"
echo ""

# Benchmark: View with path filter (drill-down)
if [ -d "$BENCH_DIR/dir1" ]; then
    echo -e "${YELLOW}Running view with path filter benchmark...${NC}"
    DRILL_TIME=$(measure_time "$BINARY view '$SNAPSHOT_FILE' --path '$BENCH_DIR/dir1' > /dev/null 2>&1")
    echo -e "${GREEN}View with path filter completed in ${DRILL_TIME}ms${NC}"
    echo ""
fi

# Summary
echo "===================================="
echo "Benchmark Summary"
echo "===================================="
echo "Scan time:  ${SCAN_TIME}ms"
echo "View time:  ${VIEW_TIME}ms"
if [ -n "$DRILL_TIME" ]; then
    echo "Drill time: ${DRILL_TIME}ms"
fi
echo ""

# Save results to file
RESULTS_FILE="benchmark-results.txt"
{
    echo "# Benchmark Results - $(date)"
    echo "Binary: $BINARY"
    echo "Files: $(find "$BENCH_DIR" -type f | wc -l)"
    echo "Dirs: $(find "$BENCH_DIR" -type d | wc -l)"
    echo "Scan: ${SCAN_TIME}ms"
    echo "View: ${VIEW_TIME}ms"
    [ -n "$DRILL_TIME" ] && echo "Drill: ${DRILL_TIME}ms"
    echo ""
} >> "$RESULTS_FILE"

echo -e "${GREEN}Results appended to $RESULTS_FILE${NC}"
echo ""
echo "To compare with optimized build:"
echo "  1. Run: cargo build --release"
echo "  2. Run: ./scripts/benchmark.sh"
echo "  3. Compare results in $RESULTS_FILE"
