#!/bin/bash
# Coverage report generation script

set -e

echo "==============================="
echo "Test Coverage Report Generator"
echo "==============================="
echo ""

# Generate terminal coverage report
echo "Generating coverage summary..."
cargo llvm-cov --all-features --workspace

echo ""
echo "==============================="
echo ""

# Generate HTML report
echo "Generating HTML coverage report..."
cargo llvm-cov --html --all-features --workspace

echo ""
echo "HTML report generated at: target/llvm-cov/html/index.html"
echo ""

# Generate lcov format for CI
echo "Generating lcov format for CI..."
cargo llvm-cov --lcov --output-path coverage.lcov --all-features --workspace

echo ""
echo "Coverage reports generated successfully!"
echo ""
echo "To view HTML report, open: target/llvm-cov/html/index.html"
echo "Or run: cargo llvm-cov --html --open"
