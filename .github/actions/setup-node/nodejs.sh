#!/bin/sh
set -e

SCRIPT_DIR=`dirname $0 | while read a; do cd $a && pwd && break; done`

# Detect Nodejs Version
VERSION="$NODE_VERSION"

# Find .nvmrc
if [ "$VERSION" = "" ]; then
  CUR="$SCRIPT_DIR"
  while true; do
    if [ -f "$CUR/.nvmrc" ]; then
      VERSION="$(cat "$CUR/.nvmrc")"
      break
    fi
    if [ -f "$CUR/.nodejs_version" ]; then
      VERSION="$(cat "$CUR/.nodejs_version")"
      break
    fi
    if [ -d "$CUR/.git" ]; then
      break
    fi
    NEXT=$(dirname $CUR)
    if [ "$NEXT" = "$CUR" ]; then
      break
    fi
    CUR="$NEXT"
  done
fi

if [ "$VERSION" = "" ]; then
  VERSION=$(curl -sSL https://nodejs.org/download/release/ |  sed -E 's/<a.*>(v.*\..*\.[0-9]+\/)<\/a>.*/\1/g' |  grep "^v" | sed -E "s/v(.*)\//\1/g" | sort -u -k 1,1n -k 2,2n -k 3,3n -t . | grep "^${VERSION}" | tail -n1)
fi

OUT_DIR="$OUT_DIR"
if [ "$OUT_DIR" = "" ]; then
  OUT_DIR="$HOME/.local/nodejs"
fi

>&2 echo VERSION: $VERSION
>&2 echo OUT_DIR: $OUT_DIR

ARCH=""
case "$(uname -m)" in
  x86_64 | x86-64 | x64 | amd64) ARCH="amd64";;
  aarch64 | arm64) ARCH="arm64";;
  *) ARCH="";;
esac

OS=""
case "$(uname -s)" in
  Darwin) OS="macos";;
  Linux) OS="linux";;
  MINGW64_NT* | Windows_NT) OS="windows";;
  *) OS="";;
esac

>&2 echo ARCH: $ARCH
>&2 echo OS: $OS

URL=""
case "$OS-$ARCH" in
  linux-amd64) URL="https://nodejs.org/download/release/v${VERSION}/node-v${VERSION}-linux-x64.tar.gz";;
  linux-arm64) URL="https://nodejs.org/download/release/v${VERSION}/node-v${VERSION}-linux-arm64.tar.gz";;
  macos-amd64) URL="https://nodejs.org/download/release/v${VERSION}/node-v${VERSION}-darwin-x64.tar.gz";;
  macos-arm64) URL="https://nodejs.org/download/release/v${VERSION}/node-v${VERSION}-darwin-arm64.tar.gz";;
  windows-arm64) URL="https://nodejs.org/download/release/v${VERSION}/node-v${VERSION}-win-arm64.zip";;
  windows-amd64) URL="https://nodejs.org/download/release/v${VERSION}/node-v${VERSION}-win-x64.zip";;
esac

if [ "$URL" = "" ]; then
  >&2 echo "Cannot find archive"
  exit 1
fi

>&2 echo URL: $URL
>&2 echo

OUT_DIR_INSTALL="${OUT_DIR}/${VERSION}"
PATH_PREFIX="${OUT_DIR_INSTALL}/prefix"
PATH_PREFIX_BIN="${OUT_DIR_INSTALL}/prefix/bin"

if [ "$OS" = "windows" ]; then
  PATH_BIN="${OUT_DIR_INSTALL}"
else
  PATH_BIN="${OUT_DIR_INSTALL}/bin"
fi

echo "export PATH=\"${PATH_BIN}:${PATH_PREFIX_BIN}:\$PATH\""
echo "export NPM_CONFIG_PREFIX=\"${PATH_PREFIX}\""

# Github CI
if ! [ "$GITHUB_PATH" = "" ]; then
  echo $PATH_BIN >> $GITHUB_PATH
  echo $PATH_PREFIX_BIN >> $GITHUB_PATH
fi

if ! [ "$GITHUB_ENV" = "" ]; then
  echo "NPM_CONFIG_PREFIX=${PATH_PREFIX}" >> $GITHUB_ENV
fi

rm -rf $OUT_DIR_INSTALL
mkdir -p $OUT_DIR_INSTALL
mkdir -p $OUT_DIR_INSTALL/prefix
mkdir -p $OUT_DIR_INSTALL/cache

if [ -z "${URL##*.tar.gz}" ]; then
  curl -s -L --url $URL | tar -xzf - -C $OUT_DIR_INSTALL --strip-components=1
fi

if [ -z "${URL##*.zip}" ]; then
  mkdir -p $OUT_DIR_INSTALL/tmp
  curl -s -L -o $OUT_DIR_INSTALL/tmp/nodejs.zip --url $URL
  unzip -qq $OUT_DIR_INSTALL/tmp/nodejs.zip -d $OUT_DIR_INSTALL/tmp
  mv $OUT_DIR_INSTALL/tmp/node-* $OUT_DIR_INSTALL/tmp/node
  mv $OUT_DIR_INSTALL/tmp/node/* $OUT_DIR_INSTALL
  rm -rf $OUT_DIR_INSTALL/tmp
fi
