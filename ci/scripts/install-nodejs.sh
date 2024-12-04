#!/bin/bash

mkdir /opt/nodejs

curl -fsSL https://unofficial-builds.nodejs.org/download/release/v18.19.0/node-v18.19.0-linux-x64-glibc-217.tar.gz -o /tmp/node-v18.19.0-linux-x64-glibc-217.tar.gz

tar -xzf /tmp/node-v18.19.0-linux-x64-glibc-217.tar.gz -C /opt/nodejs --strip-components=1

rm /tmp/node-v18.19.0-linux-x64-glibc-217.tar.gz