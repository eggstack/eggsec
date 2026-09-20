#!/usr/bin/env python3
"""TUI PTY regression smoke (Phase B, Workstream 6).

Runs the real `eggsec` binary inside a pseudo-terminal with a deterministic
size and proves the single-writer contract from the outside:

- case `warning-suppressed`: a malformed `--config` triggers the
  `Failed to load TUI config` tracing warning that previously leaked onto the
  terminal. The PTY byte stream must NOT contain the warning text as raw
  out-of-band output, must show alternate-screen entry/exit (restoration
  evidence), and the child must exit 0 for this recoverable-warning case.
- case `daemon-attach-failure`: `--runtime daemon` against a nonexistent
  socket forces the guarded-body attach path (Tokio runtime construction +
  attach inside the terminal-session guard). The child keeps running with an
  in-frame per-tab error; after the normal quit input it must restore the
  terminal (alternate-screen exit present) and exit 0.

Platform scope: Unix/Linux only (uses stdlib `pty`). On Windows (or where
`pty`/`termios` are unavailable) the script reports SKIP and exits 0 — a
named platform skip, never a compile/test failure. No Rust runtime
dependency is added; this stays a specialist/deep-checks script so normal
unit tests never depend on terminal timing.
"""

import argparse
import fcntl
import os
import pty
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time

ALT_ENTER = b"\x1b[?1049h"
ALT_LEAVE = b"\x1b[?1049l"
QUIT_INPUT = b"q"
STARTUP_WAIT_SECS = 2.5
EXIT_TIMEOUT_SECS = 20.0
READ_CHUNK = 65536


def run_in_pty(argv, env, rows=30, cols=100):
    """Spawn argv attached to a PTY slave; return (exit_code, captured_bytes)."""
    master, slave = pty.openpty()
    # Deterministic terminal size before spawn.
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
    proc = subprocess.Popen(
        argv,
        stdin=slave,
        stdout=slave,
        stderr=slave,
        env=env,
        close_fds=True,
    )
    os.close(slave)
    captured = bytearray()
    # Allow at least one redraw, then send the normal quit input.
    deadline = time.time() + STARTUP_WAIT_SECS
    quit_sent = False
    exit_code = None
    # Non-blocking drain.
    flags = fcntl.fcntl(master, fcntl.F_GETFL)
    fcntl.fcntl(master, fcntl.F_SETFL, flags | os.O_NONBLOCK)
    end = time.time() + STARTUP_WAIT_SECS + EXIT_TIMEOUT_SECS
    while time.time() < end:
        if not quit_sent and time.time() >= deadline:
            try:
                os.write(master, QUIT_INPUT)
            except OSError:
                pass
            quit_sent = True
        ready, _, _ = select.select([master], [], [], 0.25)
        if ready:
            try:
                chunk = os.read(master, READ_CHUNK)
            except OSError:
                break
            if not chunk:
                break
            captured += chunk
        if quit_sent and proc.poll() is not None:
            exit_code = proc.returncode
            break
    if exit_code is None:
        proc.kill()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            pass
        exit_code = proc.returncode
    # Final drain after exit.
    while True:
        ready, _, _ = select.select([master], [], [], 0.2)
        if not ready:
            break
        try:
            chunk = os.read(master, READ_CHUNK)
        except OSError:
            break
        if not chunk:
            break
        captured += chunk
    os.close(master)
    return exit_code, bytes(captured)


def check_case(name, argv, env, forbidden, expect_exit=0):
    code, stream = run_in_pty(argv, env)
    failures = []
    if code != expect_exit:
        failures.append(f"exit code {code} != expected {expect_exit}")
    for text in forbidden:
        if text in stream:
            failures.append(f"leaked out-of-band text: {text!r}")
    if ALT_ENTER not in stream:
        failures.append("missing alternate-screen entry (no restoration scope)")
    if ALT_LEAVE not in stream:
        failures.append("missing alternate-screen exit (terminal not restored)")
    if failures:
        print(f"FAIL: {name}")
        for failure in failures:
            print(f"  - {failure}")
        return False
    print(f"PASS: {name} (exit={code}, {len(stream)} PTY bytes)")
    return True


def main():
    parser = argparse.ArgumentParser(description="TUI PTY regression smoke")
    parser.add_argument("--binary", required=True, help="path to eggsec binary")
    args = parser.parse_args()

    if sys.platform == "win32":
        print("SKIP: PTY smoke is Unix/Linux-scoped (win32 has no stdlib pty)")
        return 0
    if not os.path.isfile(args.binary) or not os.access(args.binary, os.X_OK):
        print(f"FAIL: binary not found or not executable: {args.binary}")
        return 1

    base_env = dict(os.environ)
    base_env["TERM"] = "xterm-256color"
    # Keep the default filter so the representative warning is eligible;
    # the no-console TUI policy must suppress it, not the filter.
    base_env.pop("RUST_LOG", None)

    ok = True
    with tempfile.TemporaryDirectory(prefix="eggsec-pty-") as tmp:
        bad_config = os.path.join(tmp, "bad.toml")
        with open(bad_config, "w", encoding="utf-8") as handle:
            handle.write("this is = = not valid toml [[[\n")
        ok &= check_case(
            "warning-suppressed",
            [args.binary, "--config", bad_config],
            base_env,
            forbidden=[b"Failed to load TUI config"],
        )

        dead_socket = os.path.join(tmp, "nonexistent.sock")
        ok &= check_case(
            "daemon-attach-failure",
            [
                args.binary,
                "--runtime",
                "daemon",
                "--socket",
                dead_socket,
            ],
            base_env,
            forbidden=[b"panicked", b"Failed to restore terminal"],
        )

    print("PTY smoke: " + ("ALL PASS" if ok else "FAILURES PRESENT"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
