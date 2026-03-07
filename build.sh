#!/bin/bash
set -e

echo "Building release..."
cargo build --release

echo "Deleting old version../"
rm slapper

echo "Copying release to root directory..."
cp target/release/slapper ./slapper

echo "Done! Binary available at ./slapper"
