#!/bin/bash
set -e

# Default to LTS
NODE_VERSION="$1"
if [ "$NODE_VERSION" = "" ]; then
  NODE_VERSION="22"
fi 

# Default to home directory
OUT_DIR="$2"
if [ "$OUT_DIR" = "" ]; then
  OUT_DIR="$HOME/.local/nodejs"
fi 

NODE_VERSION=$(curl -sSL https://nodejs.org/download/release/ |  sed -E 's/<a.*>(v.*\..*\.[0-9]+\/)<\/a>.*/\1/g' |  grep "^v" | sed -E "s/v(.*)\//\1/g" | sort -u -k 1,1n -k 2,2n -k 3,3n -t . | grep "^${NODE_VERSION}" | tail -n1)

URL=""
ARCH=""
OS=""

case $(uname -m) in
  x86_64 | x86-64 | x64 | amd64)
    ARCH="amd64"
  ;;
  aarch64 | arm64)
    ARCH="arm64"
  ;;
esac

case $(uname -s) in
  Darwin)
    OS="macos"
  ;;
  Linux)
    OS="linux"
  ;;
esac

case "$OS-$ARCH" in
  linux-amd64)
    URL=https://nodejs.org/download/release/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-x64.tar.gz
  ;;
  linux-arm64)
    URL=https://nodejs.org/download/release/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-arm64.tar.gz
  ;;
  macos-amd64)
    URL=https://nodejs.org/download/release/v${NODE_VERSION}/node-v${NODE_VERSION}-darwin-x64.tar.gz
  ;;
  macos-arm64)
    URL=https://nodejs.org/download/release/v${NODE_VERSION}/node-v${NODE_VERSION}-darwin-arm64.tar.gz
  ;;
esac

if [ "$URL" == "" ]; then
  echo "Cannot find installer for Nodejs"
  exit 1
fi

export PATH="${HOME}/.local/nodejs/bin:$PATH"
export PATH="${HOME}/.local/nodejs/prefix/bin:$PATH"
export NPM_CONFIG_PREFIX=$HOME/.local/nodejs/prefix

echo "${HOME}/.local/nodejs/bin" >> $GITHUB_PATH
echo "${HOME}/.local/nodejs/prefix/bin" >> $GITHUB_PATH
echo "NPM_CONFIG_PREFIX=${NPM_CONFIG_PREFIX}" >> $GITHUB_ENV

mkdir -p $HOME/.local/nodejs
mkdir -p $HOME/.local/nodejs/prefix
mkdir -p $HOME/.local/nodejs/cache
mkdir -p $HOME/.local/nodejs/pnpm-store

echo $URL
curl -s -L --url $URL | tar -xzf - -C $HOME/.local/nodejs --strip-components=1
npm install -g pnpm npm

npm -v
node -v