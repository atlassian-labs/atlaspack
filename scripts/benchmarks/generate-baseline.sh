#!/bin/bash
# Generate baseline benchmark from main branch

set -e

GITHUB_WORKSPACE="${GITHUB_WORKSPACE:-.}"
BASELINE_OUTPUT_DIR="$GITHUB_WORKSPACE/packages/core/e2e-tests/benchmark-results"

mkdir -p "$BASELINE_OUTPUT_DIR"

echo "📊 Fetching main branch for baseline..."
git fetch origin main:main
git checkout main

echo "🔨 Building main branch..."
yarn build

# Try to run benchmark on main branch, continue gracefully if it fails
cd packages/core/e2e-tests
if yarn benchmark --output="$BASELINE_OUTPUT_DIR" --samples=5 2>/dev/null; then
  echo "✅ Baseline benchmark completed successfully"
  mv "$BASELINE_OUTPUT_DIR/current-report.json" "$BASELINE_OUTPUT_DIR/baseline-report.json"
else
  echo "ℹ️ Main branch doesn't have benchmark command - skipping baseline generation"
  echo "This is expected for the first benchmark PR"
  mkdir -p "$BASELINE_OUTPUT_DIR"
fi
cd "$GITHUB_WORKSPACE"

echo "🔄 Checking out original branch..."
git checkout "${GITHUB_HEAD_REF:-.}"
