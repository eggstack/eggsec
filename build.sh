#!/bin/bash
set -e

echo "Building release..."
cargo build --release -p eggsec-cli

echo "Deleting old version..."
rm -f eggsec

echo "Copying release to root directory..."
cp target/release/eggsec ./eggsec

echo "Done! Binary available at ./eggsec"
