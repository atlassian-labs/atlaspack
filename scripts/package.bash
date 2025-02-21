set -e

PATH_SCRIPT="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
PATH_ROOT=$(dirname $PATH_SCRIPT)

cd $PATH_ROOT/..

rm -rf atlaspack-bin

tar \
  --exclude './atlaspack/benchmarks' \
  --exclude './atlaspack/.parcel-cache' \
  --exclude './atlaspack/.git' \
  --exclude './atlaspack/target' \
  --exclude './atlaspack/packages/core/integration-tests' \
  --exclude './atlaspack/packages/examples' \
  --exclude './atlaspack/packages/migrations' \
  --exclude './atlaspack/packages/docs' \
  --exclude './atlaspack/packages/flow-libs' \
  --exclude './atlaspack/packages/flow-typed' \
  --exclude './atlaspack/packages/patches' \
  --exclude './atlaspack/packages/dev/repl' \
  --exclude './atlaspack/packages/utils/atlaspackforvscode' \
  --exclude './atlaspack/crates' \
  -J -cvf \
    ./atlaspack.tar.xz ./atlaspack

# ll ./atlaspack-bin
# tar -czf ./atlaspack.tar.gz ./atlaspack-bin
du -sh --apparent-size ./atlaspack.tar.xz


          tar \
            --exclude './atlaspack/benchmarks' \
            --exclude './atlaspack/.parcel-cache' \
            --exclude './atlaspack/.git' \
            --exclude './atlaspack/target' \
            --exclude './atlaspack/packages/core/integration-tests' \
            --exclude './atlaspack/packages/examples' \
            --exclude './atlaspack/packages/migrations' \
            --exclude './atlaspack/packages/docs' \
            --exclude './atlaspack/packages/flow-libs' \
            --exclude './atlaspack/packages/flow-typed' \
            --exclude './atlaspack/packages/patches' \
            --exclude './atlaspack/packages/dev/repl' \
            --exclude './atlaspack/packages/utils/atlaspackforvscode' \
            --exclude './atlaspack/crates' \
            -J -cf \
              ./atlaspack-${{ matrix.config.os }}-${{ matrix.config.arch }}.tar.xz ./atlaspack
