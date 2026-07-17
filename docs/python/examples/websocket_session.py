#!/usr/bin/env python3
"""WebSocket session lifecycle and assessment.

Spins up a minimal WebSocket echo server using the built-in Python
library, then uses eggsec's `WebSocketSession` to connect, send/receive
messages, and run a security assessment probe. All against loopback.

Requirements:
    - eggsec (default features)
    - EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 environment variable

Usage:
    EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 python3 docs/python/examples/websocket_session.py
"""

import asyncio
import hashlib
import os
import socket
import struct
import threading
import time


LISTEN_PORT = 19880


def _minimal_ws_server():
    """Minimal RFC 6455 echo server (no external dependencies)."""
    srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("127.0.0.1", LISTEN_PORT))
    srv.listen(1)
    srv.settimeout(5)

    def _handle(conn):
        try:
            data = conn.recv(4096)
            if not data:
                return
            # Parse WebSocket handshake
            text = data.decode("utf-8", errors="replace")
            if "Upgrade: websocket" not in text and "Upgrade: WebSocket" not in text:
                return
            key = None
            for line in text.split("\r\n"):
                if line.lower().startswith("sec-websocket-key:"):
                    key = line.split(":", 1)[1].strip()
                    break
            if not key:
                return
            accept = hashlib.sha1(
                (key + "258EAFA5-E914-47DA-95CA-5AB5DC65C740").encode()
            ).digest()
            import base64
            resp = (
                "HTTP/1.1 101 Switching Protocols\r\n"
                "Upgrade: websocket\r\n"
                "Connection: Upgrade\r\n"
                f"Sec-WebSocket-Accept: {base64.b64encode(accept).decode()}\r\n"
                "\r\n"
            )
            conn.sendall(resp.encode())

            # Echo loop: read frames, send them back
            while True:
                frame = conn.recv(4096)
                if not frame or len(frame) < 2:
                    break
                opcode = frame[0] & 0x0F
                if opcode == 0x8:  # close
                    break
                if opcode == 0x9:  # ping -> pong
                    conn.sendall(bytes([0x8A, 0x02]) + frame[2:4])
                    continue
                # Simple echo
                masked = bool(frame[1] & 0x80)
                length = frame[1] & 0x7F
                offset = 2
                if length == 126:
                    length = struct.unpack("!H", frame[2:4])[0]
                    offset = 4
                elif length == 127:
                    length = struct.unpack("!Q", frame[2:10])[0]
                    offset = 10
                if masked:
                    mask = frame[offset:offset + 4]
                    offset += 4
                payload = frame[offset:offset + length]
                if masked:
                    payload = bytes(b ^ mask[i % 4] for i, b in enumerate(payload))
                # Send unmasked echo
                header = bytes([0x81])
                if length < 126:
                    header += bytes([length])
                elif length < 65536:
                    header += bytes([126]) + struct.pack("!H", length)
                else:
                    header += bytes([127]) + struct.pack("!Q", length)
                conn.sendall(header + payload)
        except Exception:
            pass
        finally:
            conn.close()

    def _serve():
        try:
            while True:
                try:
                    conn, _ = srv.accept()
                    threading.Thread(target=_handle, args=(conn,), daemon=True).start()
                except socket.timeout:
                    continue
        except Exception:
            pass

    thread = threading.Thread(target=_serve, daemon=True)
    thread.start()
    time.sleep(0.3)
    return srv


def demo_session():
    """Connect, send messages, receive echo, close."""
    from eggsec.net import WebSocketSession, WebSocketSessionConfig

    url = f"ws://127.0.0.1:{LISTEN_PORT}"
    config = WebSocketSessionConfig(
        url=url,
        timeout_ms=5000,
        ping_interval_ms=None,  # disable auto-ping for test
    )

    with WebSocketSession(config) as ws:
        handshake = ws.connect()
        print(f"Connected: status={handshake.status_code}, duration={handshake.duration_ms:.1f}ms")

        ws.send_text("hello websocket")
        msg = ws.recv()
        print(f"  echo: {msg.text_content!r} (text={msg.is_text}, size={msg.size})")

        ws.send_text("second message")
        msg2 = ws.recv()
        print(f"  echo: {msg2.text_content!r}")

        ws.send_binary(b"\x00\x01\x02\x03")
        msg3 = ws.recv()
        print(f"  binary echo: {msg3.to_bytes()!r} (binary={msg3.is_binary})")

        close_info = ws.close(code=1000, reason="done")
        print(f"  closed: code={close_info.code}, clean={close_info.was_clean}")


def demo_assessment():
    """Run a WebSocket security assessment probe."""
    from eggsec.websocket import websocket_probe

    url = f"ws://127.0.0.1:{LISTEN_PORT}"
    report = websocket_probe(url, timeout_secs=5)
    print(f"\nAssessment of {report.target}:")
    if report.connection_test:
        print(f"  connected={report.connection_test.connected}")
    print(f"  injection_tests={len(report.injection_tests)}")
    print(f"  origin_tests={len(report.origin_tests)}")
    print(f"  fuzz_tests={len(report.fuzz_tests)}")
    print(f"  findings={len(report.findings)}")
    for f in report.findings:
        print(f"    [{f.severity}] {f.title}: {f.description[:60]}")


def main():
    server = _minimal_ws_server()
    try:
        demo_session()
        demo_assessment()
    finally:
        server.close()


if __name__ == "__main__":
    main()
