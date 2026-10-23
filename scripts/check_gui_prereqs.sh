#!/usr/bin/env bash
#
# Probe whether the desktop GUI's Tauri shell profile can be built on this
# host.
#
# WHY THIS EXISTS. The Makefile's GUI gate treats an absent webview
# prerequisite as a SKIP (Makefile:21-30), which is the right shape for a
# fast CI loop but the wrong shape for a qualification answer: a skip says
# "we did not look", and milestone 007's matrix needs a cell that says
# "we looked, and the answer is no". This probe therefore never prints
# "skipped". It reports BUILDABLE or NOT BUILDABLE, and when the answer is
# no it names exactly what is missing and what to install.
#
# The probe deliberately does not install anything and does not run a cargo
# build. Installing to find out is not a probe, and a full release build
# takes minutes; the question "can this host build the shell?" is answered
# by the presence of the SDKs the shell links against.
#
# SCOPE. These are the TAURI SHELL's prerequisites only. The daemon's
# engine-feature prerequisites are a different list with a different owner
# (see docs/DAEMON_FEATURE_PROFILE.md); they are not restated here, because
# two lists that both claim to describe "the prerequisites" drift.
#
# EXIT STATUS.
#   0  BUILDABLE on this host.
#   1  NOT BUILDABLE; the missing prerequisites are named on stdout.
#   2  OUT OF SCOPE: the platform is excluded by ADR-0010.
set -uo pipefail

# `set -e` is deliberately NOT used: the probe collects every finding so
# that one run names every missing prerequisite, rather than making the
# operator iterate one missing package at a time.

OS="$(uname -s)"
MISSING=()
NOTES=()

note() { NOTES+=("$1"); }

missing() { MISSING+=("$1"); }

require_cmd() {
  local cmd="$1" label="$2"
  if command -v "$cmd" >/dev/null 2>&1; then
    echo "  ok       $label"
  else
    echo "  MISSING  $label"
    missing "$label"
  fi
}

require_pkgconfig() {
  local module="$1" label="$2"
  if ! command -v pkg-config >/dev/null 2>&1; then
    echo "  MISSING  $label (pkg-config is absent)"
    missing "$label"
    return
  fi
  if pkg-config --exists "$module" 2>/dev/null; then
    echo "  ok       $label"
  else
    echo "  MISSING  $label"
    missing "$label"
  fi
}

echo "Desktop GUI shell-profile prerequisite probe"
echo "Host:      $(uname -s) $(uname -r) $(uname -m)"
echo ""

case "$OS" in
  Linux)
    echo "Linux checks (the Tauri v2 webview SDK chain):"
    require_cmd cargo "cargo (Rust toolchain)"
    require_cmd rustc "rustc (Rust toolchain)"
    require_cmd pkg-config "pkg-config"
    require_cmd cc "a C compiler (cc)"
    require_pkgconfig webkit2gtk-4.1 "webkit2gtk-4.1 (Tauri v2 webview)"
    require_pkgconfig gtk+-3.0 "gtk+-3.0"
    if pkg-config --exists libayatana-appindicator3-0.1 2>/dev/null; then
      echo "  ok       libayatana-appindicator3-0.1"
    elif pkg-config --exists appindicator3-0.1 2>/dev/null; then
      echo "  ok       appindicator3-0.1 (the pre-ayatana name)"
      note "appindicator3-0.1 is present but libayatana-appindicator3-0.1 is the name Tauri v2 documents; install libayatana-appindicator3-dev."
    else
      echo "  MISSING  libayatana-appindicator3-0.1"
      missing "libayatana-appindicator3-0.1"
    fi
    require_pkgconfig librsvg-2.0 "librsvg-2.0"
    # webkit2gtk-4.0 alone satisfies the Makefile gate but not the Tauri v2
    # build, so it is called out rather than left to surprise the operator.
    if ! pkg-config --exists webkit2gtk-4.1 2>/dev/null \
      && pkg-config --exists webkit2gtk-4.0 2>/dev/null; then
      note "webkit2gtk-4.0 is present but webkit2gtk-4.1 is absent: the Makefile gate accepts 4.0, the Tauri v2 build requires 4.1."
    fi
    # patchelf is used by the Tauri bundler when it rewrites binaries. The
    # bundler is inactive here, so this is informational, not build-blocking.
    if command -v patchelf >/dev/null 2>&1; then
      echo "  ok       patchelf (bundler-only; not required while bundle.active is false)"
    else
      echo "  --       patchelf absent (bundler-only; not required while bundle.active is false)"
    fi
    echo ""
    echo "Install with the distribution's package manager; see docs/GUI_BUILD.md"
    echo "for the per-distribution package names."
    ;;
  Darwin)
    echo "macOS checks (the webview ships with the OS as WKWebView):"
    require_cmd cargo "cargo (Rust toolchain)"
    require_cmd rustc "rustc (Rust toolchain)"
    if command -v xcode-select >/dev/null 2>&1 && xcode-select -p >/dev/null 2>&1; then
      echo "  ok       Xcode command line tools ($(xcode-select -p))"
    else
      echo "  MISSING  Xcode command line tools"
      missing "Xcode command line tools (run: xcode-select --install)"
    fi
    if command -v clang >/dev/null 2>&1; then
      echo "  ok       clang"
    else
      echo "  --       clang absent (the Xcode toolchain provides it)"
    fi
    echo ""
    echo "No system packages are needed on macOS: the webview ships with the OS."
    ;;
  MINGW*|MSYS*|CYGWIN*|Windows_NT)
    echo "Windows: OUT OF SCOPE for the desktop GUI (ADR-0010)."
    echo ""
    echo "The GUI's unconditional dependency eggsec-frontend-client imports"
    echo "tokio::net::UnixStream, so the GUI does not compile for Windows."
    echo "See docs/GUI_PLATFORM_QUALIFICATION.md and the known limitations in"
    echo "docs/GUI_DISTRIBUTION_HANDOFF.md."
    exit 2
    ;;
  *)
    echo "Unsupported host for the desktop GUI shell profile: $OS"
    echo "See docs/GUI_PLATFORM_QUALIFICATION.md for the supported platform scope."
    exit 2
    ;;
esac

echo ""
for note in "${NOTES[@]:-}"; do
  [ -n "$note" ] && echo "note: $note"
done

echo ""
if [ "${#MISSING[@]}" -eq 0 ]; then
  echo "VERDICT: BUILDABLE - the desktop GUI shell profile can be built on this host."
  exit 0
fi

echo "VERDICT: NOT BUILDABLE - the desktop GUI shell profile cannot be built on this host."
echo "Missing prerequisites:"
for item in "${MISSING[@]}"; do
  echo "  - $item"
done
exit 1
