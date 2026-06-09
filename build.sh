#!/bin/bash
set -e

echo "Building release..."
cargo build --release

echo "Deleting old version../"
rm eggsec

echo "Copying release to root directory..."
cp target/release/eggsec ./eggsec

echo "Done! Binary available at ./eggsec"
