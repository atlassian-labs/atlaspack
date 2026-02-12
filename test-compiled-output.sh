#!/bin/bash

set -e

# Script to test the compiled CSS-in-JS transformer output by comparing
# SWC (new) vs Babel (legacy) transformer results.
#
# This script runs a single Atlaspack build with both transformers emitting output:
# - SWC transformer emits <file>.swc.js (but doesn't modify the asset)
# - Babel transformer emits <file>.babel.js (processes the original code)
#
# Usage:
#   ./test-compiled-output.sh [--no-build] [--cwd <directory>] [--pattern <glob>] [--compare] [--cleanup]
#
# Options:
#   --no-build        Skip building atlaspack (use existing build)
#   --cwd <directory> Working directory for the test (default: atlassian-frontend-monorepo/jira)
#   --pattern <glob>  Only process files matching pattern (default: all compiled files)
#   --compare         After building, run comparison of .swc.js and .babel.js files
#   --cleanup         Remove generated .swc.js and .babel.js files after comparison
#   --swc-only        Only emit SWC output (single pass, SWC transformer applies changes)
#   --babel-only      Only emit Babel output (single pass, disable SWC transformer)

# Avoid Nx sqlite cache issues in Cursor agent environments.
export NX_DAEMON=false
export NX_SKIP_NX_CACHE=true
export NX_DISABLE_DB=true
export NX_DB_CACHE=false
export NX_CACHE_DIRECTORY="/tmp"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CWD="${SCRIPT_DIR}/atlassian-frontend-monorepo/jira"
BUILD=true
COMPARE=false
CLEANUP=false
SWC_ONLY=false
BABEL_ONLY=false
PATTERN=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-build)
            BUILD=false
            shift
            ;;
        --cwd)
            CWD="$2"
            shift 2
            ;;
        --cwd=*)
            CWD="${1#*=}"
            shift
            ;;
        --pattern)
            PATTERN="$2"
            shift 2
            ;;
        --pattern=*)
            PATTERN="${1#*=}"
            shift
            ;;
        --compare)
            COMPARE=true
            shift
            ;;
        --cleanup)
            CLEANUP=true
            shift
            ;;
        --swc-only)
            SWC_ONLY=true
            shift
            ;;
        --babel-only)
            BABEL_ONLY=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "=== Compiled CSS-in-JS Transformer Output Test ==="
echo "Working directory: $CWD"
echo "Build atlaspack: $BUILD"
echo "Compare outputs: $COMPARE"
echo "Cleanup after: $CLEANUP"
echo ""

# Build atlaspack if needed
if [ "$BUILD" = true ]; then
    echo "Building atlaspack..."
    cd "$SCRIPT_DIR"
    yarn build-native-release && yarn build
    echo "Build complete."
    echo ""
fi

# Ensure CWD exists
if [ ! -d "$CWD" ]; then
    echo "Error: Directory $CWD does not exist!"
    exit 1
fi

cd "$CWD"

# Use nvm if available
if command -v nvm &> /dev/null; then
    nvm use 2>/dev/null || true
fi

# Apply patch to @compiled/parcel-transformer to disable extract config
# Patch file is stored in atlaspack-beta/patches/ for reuse
PARCEL_TRANSFORMER_PATCH="$SCRIPT_DIR/patches/@compiled+parcel-transformer+0.19.0.patch"
if [ -f "$PARCEL_TRANSFORMER_PATCH" ]; then
    echo "Applying @compiled/parcel-transformer patch..."
    # Check if patch is already applied by looking for EXTRACT_DISABLED
    if ! grep -q "EXTRACT_DISABLED" node_modules/@compiled/parcel-transformer/dist/index.js 2>/dev/null; then
        patch -p0 -N < "$PARCEL_TRANSFORMER_PATCH" || echo "Patch may already be applied or failed"
    else
        echo "Patch already applied."
    fi
fi

# Clean previous build artifacts
echo "Cleaning previous build artifacts..."
rm -rf .parcel-cache/ build/

if [ "$SWC_ONLY" = true ]; then
    # SWC-only mode: enable SWC transformer normally (it will apply changes)
    echo ""
    echo "=== Running SWC-only Pass ==="
    echo "This will generate .swc.js files and apply SWC transformation."
    
    ATLASPACK_COMPILED_CSS_IN_JS_TRANSFORMER=true \
    ATLASPACK_EMIT_SWC_OUTPUT=true \
    ATLASPACK_ENABLE_TOKENS_TRANSFORMER=true \
    ATLASPACK_CORE_TOKENS_AND_COMPILED_CSS_IN_JS_TRANSFORM=false \
    ATLASPACK_COMPILED_EXTRACT_DISABLED=true \
    yarn build:local --minimal 2>&1 | tee build.log || true
    
    SWC_COUNT=$(find . -name "*.swc.js" 2>/dev/null | wc -l | tr -d ' ')
    echo "Generated $SWC_COUNT .swc.js files"

elif [ "$BABEL_ONLY" = true ]; then
    # Babel-only mode: disable SWC transformer, enable Babel output
    echo ""
    echo "=== Running Babel-only Pass ==="
    echo "This will generate .babel.js files using Babel transformer."
    
    ATLASPACK_COMPILED_CSS_IN_JS_TRANSFORMER=false \
    ATLASPACK_EMIT_BABEL_OUTPUT=true \
    ATLASPACK_ENABLE_TOKENS_TRANSFORMER=true \
    ATLASPACK_COMPILED_EXTRACT_DISABLED=true \
    yarn build:local --minimal 2>&1 | tee build.log || true
    
    BABEL_COUNT=$(find . -name "*.babel.js" 2>/dev/null | wc -l | tr -d ' ')
    echo "Generated $BABEL_COUNT .babel.js files"

else
    # Combined mode: run single build with both transformers emitting output
    # SWC transformer runs first, emits .swc.js, but returns original asset unchanged
    # Babel transformer then processes the original code and emits .babel.js
    echo ""
    echo "=== Running Combined Pass (SWC + Babel) ==="
    echo "This will generate both .swc.js and .babel.js files in a single build."
    echo "SWC transformer emits output but doesn't modify the asset."
    echo "Babel transformer processes the original code."
    
    ATLASPACK_COMPILED_CSS_IN_JS_TRANSFORMER=true \
    ATLASPACK_EMIT_SWC_OUTPUT=true \
    ATLASPACK_EMIT_BABEL_OUTPUT=true \
    ATLASPACK_ENABLE_TOKENS_TRANSFORMER=true \
    ATLASPACK_CORE_TOKENS_AND_COMPILED_CSS_IN_JS_TRANSFORM=false \
    ATLASPACK_COMPILED_EXTRACT_DISABLED=true \
    yarn build:local --minimal 2>&1 | tee build.log || true
    
    echo ""
    echo "Build complete."
    
    # Count generated files
    SWC_COUNT=$(find . -name "*.swc.js" 2>/dev/null | wc -l | tr -d ' ')
    BABEL_COUNT=$(find . -name "*.babel.js" 2>/dev/null | wc -l | tr -d ' ')
    echo "Generated $SWC_COUNT .swc.js files"
    echo "Generated $BABEL_COUNT .babel.js files"
fi

# Compare outputs if requested
if [ "$COMPARE" = true ]; then
    echo ""
    echo "=== Comparing Outputs ==="
    
    node "$SCRIPT_DIR/compare-compiled-output.js" --cwd "$CWD" ${PATTERN:+--pattern "$PATTERN"}
fi

# Cleanup if requested
if [ "$CLEANUP" = true ]; then
    echo ""
    echo "=== Cleaning Up ==="
    find . -name "*.swc.js" -delete 2>/dev/null || true
    find . -name "*.babel.js" -delete 2>/dev/null || true
    find . -name "*.diff" -delete 2>/dev/null || true
    echo "Cleaned up generated files."
fi

echo ""
echo "=== Done ==="
