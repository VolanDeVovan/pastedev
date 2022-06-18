#!/usr/bin/env bash

set -e 

cwd=$(pwd)

# Build web
cd ${cwd}/web
yarn install && yarn build

# Build backend
cd ${cwd}
cargo build --release

# Copy web artifacts
cp -r ${cwd}/web/dist ${cwd}/static