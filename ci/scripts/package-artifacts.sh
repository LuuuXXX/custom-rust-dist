#!/bin/bash

set -euo pipefail
IFS=$'\n\t'

if [ -n "$CI_JOB_NAME" ]; then
    echo "[CI_JOB_NAME=$CI_JOB_NAME]"
fi

if [[ "$CI_JOB_NAME" == *msvc* ]]; then
    echo "windows"
else
    echo "unix"
fi