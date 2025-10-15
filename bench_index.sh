#!/bin/bash
# Benchmark script for mentat indexing performance
# Usage: ./bench_index.sh [label] [release|debug]

set -e

LABEL="${1:-benchmark}"
MODE="${2:-debug}"
LOG="bench_results.txt"

if [ "$MODE" = "release" ]; then
    BUILD_FLAG="--release"
    RUSTFLAGS="-C target-cpu=native"
    export RUSTFLAGS
else
    BUILD_FLAG=""
fi

echo "========================================" | tee -a "$LOG"
echo "Benchmark: $LABEL" | tee -a "$LOG"
echo "Date: $(date)" | tee -a "$LOG"
echo "========================================" | tee -a "$LOG"

# Clean previous index
rm -rf index/

# Run with time measurement
echo "Starting indexing (mode: $MODE)..." | tee -a "$LOG"
/usr/bin/time -v cargo run -p mentat $BUILD_FLAG -- index . 2>&1 | tee -a "bench_${LABEL}.log" | grep -E "(Elapsed|Maximum resident|index built)"

# Extract key metrics
ELAPSED=$(grep "Elapsed" "bench_${LABEL}.log" | tail -1 || echo "N/A")
MAXMEM=$(grep "Maximum resident" "bench_${LABEL}.log" | tail -1 || echo "N/A")

echo "Results:" | tee -a "$LOG"
echo "  $ELAPSED" | tee -a "$LOG"
echo "  $MAXMEM" | tee -a "$LOG"

# Count chunks indexed
CHUNKS=$(cargo run -p mentat -- build-hnsw 2>&1 | grep "Building HNSW" | awk '{print $5}')
echo "  Chunks indexed: $CHUNKS" | tee -a "$LOG"
echo "" | tee -a "$LOG"
