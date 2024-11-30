#!/usr/bin/env bash

set -e

# windows
export target="x86_64-pc-windows-msvc"
command="cargo dev vendor && cargo dev dist"

# install nodejs
# mkdir -p $HOME/node
# curl -sL https://nodejs.org/dist/v16.9.0/node-v16.9-linux-x64.tar.xz | tar -xJ -C $HOME/node --strip-components=1
# export PATH="$HOME/node/bin:$PATH"

echo "Execiting ${command}"
sh -x -c "$command" 
