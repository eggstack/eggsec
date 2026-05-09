#!/usr/bin/env bash
set -euo pipefail

# Avoid pulling MacPorts arm64 libs into x86_64 builds.
export PKG_CONFIG_LIBDIR="${PKG_CONFIG_LIBDIR:-/usr/lib/pkgconfig:/usr/share/pkgconfig}"
export PKG_CONFIG_PATH="${PKG_CONFIG_PATH:-}"

if [ "$#" -eq 0 ]; then
  cargo test --test nse_tests -p slapper --features nse
else
  cargo "$@"
fi
