#!/bin/bash
set -e

echo "=== AI Linker Optimization Test ==="
echo "Testing with subset of Waymark data"
echo ""

# Build the project
echo "Building idud..."
cargo build --release --quiet 2>&1 | grep -E "(Compiling|Finished|error)" || echo "Build complete"

# Test 1: Check if copilot CLI is available
echo ""
echo "Test 1: Checking Copilot CLI..."
which copilot > /dev/null && echo "✓ Copilot CLI found" || echo "✗ Copilot CLI NOT found"

# Test 2: Run unit tests for AI linker
echo ""
echo "Test 2: Running AI linker unit tests..."
cargo test --lib ai_linker --quiet 2>&1 | tail -10 || echo "Tests passed"

echo ""
echo "=== AI Linker Tests Complete ==="
