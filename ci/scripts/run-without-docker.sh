#!/usr/bin/env bash

set -e

# windows
export TARGET="x86_64-pc-windows-msvc"

# 安装 Rust
sh ci/scripts/install-rust.sh

# 安装 nodejs 18.x
NODE_VERSION="v18.0.0"
NODE_URL="https://nodejs.org/dist/$NODE_VERSION/node-$NODE_VERSION-win-x64.zip"
USER_NODEJS_PATH="/c/Users/$(whoami)/nodejs"
DOWNLOAD_PATH="$USER_NODEJS_PATH/node-$NODE_VERSION-x64.zip"
mkdir -p "$USER_NODEJS_PATH"
curl -L -o "$DOWNLOAD_PATH" "$NODE_URL"
unzip "$DOWNLOAD_PATH" -d "$USER_NODEJS_PATH"
rm "$DOWNLOAD_PATH"
NODE_BIN_PATH="$USER_NODEJS_PATH/node-$NODE_VERSION-x64"
export PATH="$NODE_BIN_PATH:$PATH"

# 安装 pnpm
npm set strict-ssl false && npm install -g pnpm

# 安装 Tauri CLI
cargo install tauri-cli@1.6.4

echo "Execiting cargo dev vendor"
cargo dev vendor

echo "Execiting cargo dev dist"
cargo dev dist