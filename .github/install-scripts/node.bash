#!/usr/bin/env bash

set -e

NODE_VERSION="$1"
if [ "$NODE_VERSION" = "" ]; then
  if [ -f ".nvmrc" ]; then
    # Use nvmrc
    NODE_VERSION=$(cat .nvmrc)
  else
    # Get latest
    NODE_VERSION="v$(curl -sSL https://nodejs.org/download/release/ |  sed -E 's/<a.*>(v.*\..*\.[0-9]+\/)<\/a>.*/\1/g' |  grep "^v" | sed -E "s/v(.*)\//\1/g" | sort -u -k 1,1n -k 2,2n -k 3,3n -t . | grep "^${NODE_VERSION}" | tail -n1)"
  fi
fi

install_dir="$HOME/.local/nodejs"

# Create Directories
mkdir -p $install_dir

# Setup Environment
export PATH="$install_dir/bin:$PATH"
echo "$install_dir/bin" >> $GITHUB_PATH

# Download & Install
arch=""
platform=""

case $(uname -m) in
  x86_64 | x86-64 | x64 | amd64)
    arch="amd64"
  ;;
  aarch64 | arm64)
    arch="arm64"
  ;;
esac

case $(uname -s) in
  Darwin)
    platform="macos"
  ;;
  Linux)
    platform="linux"
  ;;
  MSYS_NT*)
    platform="windows"
  ;;
esac

echo "Installing $platform-$arch"

case "$platform-$arch" in
  linux-amd64)
    URL=https://nodejs.org/download/release/${NODE_VERSION}/node-${NODE_VERSION}-linux-x64.tar.gz
  ;;
  linux-arm64)
    URL=https://nodejs.org/download/release/${NODE_VERSION}/node-${NODE_VERSION}-linux-arm64.tar.gz
  ;;
  macos-amd64)
    URL=https://nodejs.org/download/release/${NODE_VERSION}/node-${NODE_VERSION}-darwin-x64.tar.gz
  ;;
  macos-arm64)
    URL=https://nodejs.org/download/release/${NODE_VERSION}/node-${NODE_VERSION}-darwin-arm64.tar.gz
  ;;
esac

if [ "$URL" = "" ]; then
  echo No platform matches
  exit 1
fi

echo "Downloading $URL"

curl -s -L --url $URL | tar -xzf - -C "$install_dir" --strip-components=1
npm install -g yarn

echo "Node: $(node -v)"
echo "Npm: $(npm -v)"
echo "Yarn: $(yarn -v)"
