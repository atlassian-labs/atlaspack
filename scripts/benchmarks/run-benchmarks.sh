#!/bin/bash
# Run benchmarks on current branch with optional baseline comparison

set -e

GITHUB_WORKSPACE="${GITHUB_WORKSPACE:-.}"
BASELINE_OUTPUT_DIR="$GITHUB_WORKSPACE/packages/core/e2e-tests/benchmark-results"

echo "🔨 Building current branch..."
yarn build

echo "🏃 Running benchmarks..."
cd packages/core/e2e-tests

# Run benchmarks with baseline if available, otherwise without baseline
if [ -f "$BASELINE_OUTPUT_DIR/baseline-report.json" ]; then
  echo "🔄 Running benchmarks with baseline comparison..."
  yarn benchmark \
    --output="$BASELINE_OUTPUT_DIR" \
    --baseline="$BASELINE_OUTPUT_DIR/baseline-report.json" \
    --github-comment
else
  echo "🔄 Running benchmarks without baseline..."
  yarn benchmark \
    --output="$BASELINE_OUTPUT_DIR" \
    --github-comment
fi

cd "$GITHUB_WORKSPACE"
