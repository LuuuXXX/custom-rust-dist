#!/usr/bin/env bash

set -e

# windows
export target="x86_64-pc-windows-msvc"
command="cargo dev vendor && cargo dev dist"

# install nodejs



echo "Execiting cargo dev vendor"
cargo dev vendor

echo "Execiting cargo dev dist"
cargo dev dist