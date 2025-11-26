#!/bin/bash
# Check for performance regressions in benchmark results

set -e

GITHUB_WORKSPACE="${GITHUB_WORKSPACE:-.}"
RESULTS_FILE="$GITHUB_WORKSPACE/packages/core/e2e-tests/benchmark-results/github-comment.md"

if [ -f "$RESULTS_FILE" ]; then
  if grep -q "🔴 Regression" "$RESULTS_FILE"; then
    echo "❌ Performance regressions detected!"
    cat "$RESULTS_FILE"
  else
    echo "✅ No performance regressions detected"
    cat "$RESULTS_FILE"
  fi
else
  echo "ℹ️ No benchmark results found - skipping regression check"
fi
