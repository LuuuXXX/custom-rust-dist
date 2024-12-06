#!/bin/bash

NODE_VERSION="v18.19.0"

if [ "$(uname -s)" = "Linux" ]; then
    ARCH=$(uname -m)
    mkdir /opt/nodejs
    if [ "$ARCH" == "aarch64" ]; then
        # NODE_URL="https://unofficial-builds.nodejs.org/download/release/$NODE_VERSION/node-$NODE_VERSION-linux-armv6l.tar.gz"
        # DOWNLOAD_PATH="/tmp/node-$NODE_VERSION-linux-armv6l.tar.gz"
        NODE_URL="https://nodejs.org/dist/$NODE_VERSION/node-$NODE_VERSION-linux-arm64.tar.gz"
        DOWNLOAD_PATH="/tmp/node-$NODE_VERSION-linux-arm64.tar.gz"
    else
        NODE_URL="https://nodejs.org/dist/$NODE_VERSION/node-$NODE_VERSION-linux-x64.tar.gz"
        DOWNLOAD_PATH="/tmp/node-$NODE_VERSION-linux-x64.tar.gz"
    fi
    echo "Downloading $NODE_URL"
    curl -L -o "$DOWNLOAD_PATH" "$NODE_URL"
    tar -xzf "$DOWNLOAD_PATH" -C /opt/nodejs --strip-components=1
    rm "$DOWNLOAD_PATH"
else
    NODE_URL="https://nodejs.org/dist/$NODE_VERSION/node-$NODE_VERSION-win-x64.zip"
    USER_NODEJS_PATH="/c/Users/$(whoami)/nodejs"
    DOWNLOAD_PATH="$USER_NODEJS_PATH/node-$NODE_VERSION-x64.zip"
    mkdir -p "$USER_NODEJS_PATH"
    curl -L -o "$DOWNLOAD_PATH" "$NODE_URL"
    unzip "$DOWNLOAD_PATH" -d "$USER_NODEJS_PATH"
    rm "$DOWNLOAD_PATH"
    NODE_BIN_PATH="$USER_NODEJS_PATH/node-$NODE_VERSION-x64"
    # export PATH="$NODE_BIN_PATH:$PATH"
    CURRENT_PATH=$(powershell -Command "[System.Environment]::GetEnvironmentVariable('PATH', 'User')")
    NEW_PATH="$CURRENT_PATH;$NODE_BIN_PATH"
    powershell -Command "[System.Environment]::SetEnvironmentVariable('PATH', '$NEW_PATH', 'User')"
fi

