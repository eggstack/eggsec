"""Comprehensive WebSocket correctness tests — Workstream 8 of Release 1/2 closure pass.

Validates the full provisional WebSocket lifecycle:
- DTO construction and serialization
- Session lifecycle (config, context manager, close-on-unconnected)
- Assessment config/result types
- Probe result types (ConnectionTest, InjectionTest, OriginTest, FuzzTest, Finding, Report)
- Scope enforcement
- Message type semantics
- Assessment finding severity levels and serialization
- Engine dispatch paths
"""

import json
import pytest

import eggsec

_has_websocket = eggsec.has_feature("websocket")

pytestmark = [
    pytest.mark.skipif(not _has_websocket, reason="websocket feature not compiled"),
    pytest.mark.websocket_correctness,
    pytest.mark.timeout(30),
]

# ---------------------------------------------------------------------------
# 1. DTO Construction and Serialization
# ---------------------------------------------------------------------------


class TestWebSocketSessionConfigConstruction:
    """WebSocketSessionConfigPy: construction, defaults, and validation."""

    def test_minimal_config(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://localhost:8080")
        assert c.url == "ws://localhost:8080"
        assert c.origin is None
        assert c.timeout_ms == 10000
        assert c.max_message_size == 1048576
        assert c.ping_interval_ms == 30000
        assert c.close_timeout_ms == 5000
        assert c.verify_tls is True
        assert c.subprotocols == []
        assert c.headers == []
        assert c.cookies == []

    def test_full_config(self):
        c = eggsec.WebSocketSessionConfigPy(
            url="wss://echo.websocket.org",
            headers=[("Authorization", "Bearer tok"), ("X-Custom", "val")],
            cookies=[("session", "abc123")],
            origin="https://example.com",
            subprotocols=["graphql-ws", "superchat"],
            timeout_ms=5000,
            max_message_size=2097152,
            ping_interval_ms=15000,
            close_timeout_ms=3000,
            verify_tls=False,
        )
        assert c.url == "wss://echo.websocket.org"
        assert c.origin == "https://example.com"
        assert c.subprotocols == ["graphql-ws", "superchat"]
        assert c.timeout_ms == 5000
        assert c.max_message_size == 2097152
        assert c.ping_interval_ms == 15000
        assert c.close_timeout_ms == 3000
        assert c.verify_tls is False
        assert len(c.headers) == 2
        assert len(c.cookies) == 1

    def test_headers_are_tuples(self):
        c = eggsec.WebSocketSessionConfigPy(
            url="ws://localhost",
            headers=[("Key", "Value")],
        )
        h = c.headers
        assert len(h) == 1
        assert h[0] == ("Key", "Value")

    def test_cookies_are_tuples(self):
        c = eggsec.WebSocketSessionConfigPy(
            url="ws://localhost",
            cookies=[("a", "1"), ("b", "2")],
        )
        ck = c.cookies
        assert len(ck) == 2
        assert ck[0] == ("a", "1")
        assert ck[1] == ("b", "2")

    def test_empty_url_raises(self):
        with pytest.raises(ValueError, match="url must not be empty"):
            eggsec.WebSocketSessionConfigPy(url="")

    def test_invalid_scheme_http_raises(self):
        with pytest.raises(ValueError, match="url must start with ws:// or wss://"):
            eggsec.WebSocketSessionConfigPy(url="http://example.com")

    def test_invalid_scheme_ftp_raises(self):
        with pytest.raises(ValueError, match="url must start with ws:// or wss://"):
            eggsec.WebSocketSessionConfigPy(url="ftp://example.com")

    def test_ws_scheme_accepted(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://example.com")
        assert c.url == "ws://example.com"

    def test_wss_scheme_accepted(self):
        c = eggsec.WebSocketSessionConfigPy(url="wss://example.com")
        assert c.url == "wss://example.com"

    def test_to_dict_roundtrip(self):
        c = eggsec.WebSocketSessionConfigPy(
            url="wss://echo.websocket.org",
            origin="https://example.com",
            subprotocols=["graphql-ws"],
            timeout_ms=5000,
            verify_tls=False,
        )
        d = c.to_dict()
        assert d["url"] == "wss://echo.websocket.org"
        assert d["origin"] == "https://example.com"
        assert d["subprotocols"] == ["graphql-ws"]
        assert d["timeout_ms"] == 5000
        assert d["verify_tls"] is False
        assert "headers" in d
        assert "cookies" in d
        assert "max_message_size" in d
        assert "ping_interval_ms" in d
        assert "close_timeout_ms" in d

    def test_to_json_roundtrip(self):
        c = eggsec.WebSocketSessionConfigPy(
            url="ws://localhost:9001",
            timeout_ms=7777,
        )
        j = c.to_json()
        parsed = json.loads(j)
        assert parsed["url"] == "ws://localhost:9001"
        assert parsed["timeout_ms"] == 7777

    def test_repr(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://localhost:8080")
        r = repr(c)
        assert "ws://localhost:8080" in r
        assert "timeout_ms=" in r
        assert "verify_tls=" in r

    def test_str(self):
        c = eggsec.WebSocketSessionConfigPy(url="wss://echo.websocket.org")
        s = str(c)
        assert "wss://echo.websocket.org" in s
        assert "timeout=" in s
        assert "tls=" in s


class TestWebSocketMessageConstruction:
    """WebSocketMessagePy: construction and type flags."""

    def test_text_message(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"hello world",
            is_text=True,
            is_binary=False,
            text_content="hello world",
            size=11,
        )
        assert msg.is_text is True
        assert msg.is_binary is False
        assert msg.is_ping is False
        assert msg.is_pong is False
        assert msg.text_content == "hello world"
        assert msg.size == 11
        assert msg.data == b"hello world"

    def test_text_to_text(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"hello",
            is_text=True,
            is_binary=False,
            text_content="hello",
            size=5,
        )
        assert msg.to_text() == "hello"

    def test_text_to_bytes(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"hello",
            is_text=True,
            is_binary=False,
            text_content="hello",
            size=5,
        )
        assert msg.to_bytes() == b"hello"

    def test_binary_message(self):
        raw = bytes([0x00, 0xFF, 0x42, 0xDE, 0xAD])
        msg = eggsec.WebSocketMessagePy(
            data=raw,
            is_text=False,
            is_binary=True,
            text_content=None,
            size=5,
        )
        assert msg.is_text is False
        assert msg.is_binary is True
        assert msg.is_ping is False
        assert msg.is_pong is False
        assert msg.text_content is None
        assert msg.to_bytes() == raw

    def test_binary_to_text_valid_utf8(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"valid utf8",
            is_text=False,
            is_binary=True,
            text_content="valid utf8",
            size=10,
        )
        assert msg.to_text() == "valid utf8"

    def test_binary_to_text_invalid_utf8(self):
        msg = eggsec.WebSocketMessagePy(
            data=bytes([0xFF, 0xFE]),
            is_text=False,
            is_binary=True,
            text_content=None,
            size=2,
        )
        assert msg.to_text() is None

    def test_ping_message(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"\x00\x01",
            is_text=False,
            is_binary=False,
            is_ping=True,
            is_pong=False,
            text_content=None,
            size=2,
        )
        assert msg.is_ping is True
        assert msg.is_pong is False
        assert msg.is_text is False
        assert msg.is_binary is False

    def test_pong_message(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"\x00\x01",
            is_text=False,
            is_binary=False,
            is_ping=False,
            is_pong=True,
            text_content=None,
            size=2,
        )
        assert msg.is_pong is True
        assert msg.is_ping is False

    def test_large_message(self):
        payload = b"A" * 65536
        msg = eggsec.WebSocketMessagePy(
            data=payload,
            is_text=True,
            is_binary=False,
            text_content="A" * 65536,
            size=65536,
        )
        assert msg.size == 65536
        assert len(msg.to_bytes()) == 65536

    def test_empty_message(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"",
            is_text=True,
            is_binary=False,
            text_content="",
            size=0,
        )
        assert msg.size == 0
        assert msg.to_text() == ""
        assert msg.to_bytes() == b""

    def test_to_dict(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"test",
            is_text=True,
            is_binary=False,
            text_content="test",
            size=4,
        )
        d = msg.to_dict()
        assert d["is_text"] is True
        assert d["is_binary"] is False
        assert d["is_ping"] is False
        assert d["is_pong"] is False
        assert d["text_content"] == "test"
        assert d["size"] == 4
        assert d["data"] == b"test"

    def test_to_json(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"json",
            is_text=True,
            is_binary=False,
            text_content="json",
            size=4,
        )
        j = msg.to_json()
        parsed = json.loads(j)
        assert parsed["is_text"] is True
        assert parsed["size"] == 4

    def test_repr_text(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"hi", is_text=True, is_binary=False,
            text_content="hi", size=2,
        )
        r = repr(msg)
        assert "text" in r
        assert "2" in r

    def test_repr_binary(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"\x00", is_text=False, is_binary=True,
            text_content=None, size=1,
        )
        r = repr(msg)
        assert "binary" in r

    def test_repr_ping(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"\x00", is_text=False, is_binary=False,
            is_ping=True, is_pong=False, text_content=None, size=1,
        )
        r = repr(msg)
        assert "ping" in r

    def test_repr_pong(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"\x00", is_text=False, is_binary=False,
            is_ping=False, is_pong=True, text_content=None, size=1,
        )
        r = repr(msg)
        assert "pong" in r

    def test_str_text(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"hello", is_text=True, is_binary=False,
            text_content="hello", size=5,
        )
        assert str(msg) == "hello"

    def test_str_binary(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"\x00\x01", is_text=False, is_binary=True,
            text_content=None, size=2,
        )
        assert "binary" in str(msg)

    def test_str_ping(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"\x00", is_text=False, is_binary=False,
            is_ping=True, is_pong=False, text_content=None, size=1,
        )
        assert "ping" in str(msg)


class TestWebSocketCloseInfoConstruction:
    """WebSocketCloseInfoPy: construction for clean and abnormal close."""

    def test_clean_close(self):
        ci = eggsec.WebSocketCloseInfoPy(
            code=1000, reason="normal closure", was_clean=True,
        )
        assert ci.code == 1000
        assert ci.reason == "normal closure"
        assert ci.was_clean is True

    def test_abnormal_close(self):
        ci = eggsec.WebSocketCloseInfoPy(
            code=1006, reason="abnormal closure", was_clean=False,
        )
        assert ci.code == 1006
        assert ci.reason == "abnormal closure"
        assert ci.was_clean is False

    def test_going_away(self):
        ci = eggsec.WebSocketCloseInfoPy(
            code=1001, reason="server going away", was_clean=True,
        )
        assert ci.code == 1001
        assert ci.was_clean is True

    def test_protocol_error(self):
        ci = eggsec.WebSocketCloseInfoPy(
            code=1002, reason="protocol error", was_clean=True,
        )
        assert ci.code == 1002

    def test_unsupported_data(self):
        ci = eggsec.WebSocketCloseInfoPy(
            code=1003, reason="unsupported data type", was_clean=True,
        )
        assert ci.code == 1003

    def test_no_status_received(self):
        ci = eggsec.WebSocketCloseInfoPy(
            code=1005, reason="no status received", was_clean=False,
        )
        assert ci.code == 1005
        assert ci.was_clean is False

    def test_policy_violation(self):
        ci = eggsec.WebSocketCloseInfoPy(
            code=1008, reason="policy violation", was_clean=True,
        )
        assert ci.code == 1008

    def test_empty_reason(self):
        ci = eggsec.WebSocketCloseInfoPy(
            code=1000, reason="", was_clean=True,
        )
        assert ci.reason == ""

    def test_to_dict(self):
        ci = eggsec.WebSocketCloseInfoPy(
            code=1000, reason="ok", was_clean=True,
        )
        d = ci.to_dict()
        assert d["code"] == 1000
        assert d["reason"] == "ok"
        assert d["was_clean"] is True

    def test_to_json(self):
        ci = eggsec.WebSocketCloseInfoPy(
            code=1001, reason="going away", was_clean=False,
        )
        j = ci.to_json()
        parsed = json.loads(j)
        assert parsed["code"] == 1001
        assert parsed["was_clean"] is False

    def test_repr_clean(self):
        ci = eggsec.WebSocketCloseInfoPy(
            code=1000, reason="ok", was_clean=True,
        )
        r = repr(ci)
        assert "1000" in r
        assert "was_clean=True" in r

    def test_repr_unclean(self):
        ci = eggsec.WebSocketCloseInfoPy(
            code=1006, reason="abnormal", was_clean=False,
        )
        r = repr(ci)
        assert "1006" in r
        assert "was_clean=False" in r

    def test_str_clean(self):
        ci = eggsec.WebSocketCloseInfoPy(
            code=1000, reason="ok", was_clean=True,
        )
        s = str(ci)
        assert "1000" in s
        assert "clean" in s

    def test_str_unclean(self):
        ci = eggsec.WebSocketCloseInfoPy(
            code=1006, reason="abnormal", was_clean=False,
        )
        s = str(ci)
        assert "unclean" in s


class TestWebSocketHandshakeConstruction:
    """WebSocketHandshakePy: construction with subprotocol and extensions."""

    def test_basic_handshake(self):
        hs = eggsec.WebSocketHandshakePy(
            url="ws://localhost:8080",
            status_code=101,
            headers=[("Upgrade", "websocket"), ("Connection", "Upgrade")],
            selected_subprotocol=None,
            selected_extensions=[],
            duration_ms=42.5,
        )
        assert hs.url == "ws://localhost:8080"
        assert hs.status_code == 101
        assert hs.selected_subprotocol is None
        assert hs.selected_extensions == []
        assert hs.duration_ms == 42.5
        assert len(hs.headers) == 2

    def test_handshake_with_subprotocol(self):
        hs = eggsec.WebSocketHandshakePy(
            url="wss://api.example.com/ws",
            status_code=101,
            headers=[],
            selected_subprotocol="graphql-ws",
            selected_extensions=["permessage-deflate"],
            duration_ms=120.3,
        )
        assert hs.selected_subprotocol == "graphql-ws"
        assert hs.selected_extensions == ["permessage-deflate"]

    def test_handshake_with_multiple_extensions(self):
        hs = eggsec.WebSocketHandshakePy(
            url="ws://localhost",
            status_code=101,
            headers=[],
            selected_subprotocol=None,
            selected_extensions=["permessage-deflate", "x-webkit-deflate-frame"],
            duration_ms=10.0,
        )
        assert len(hs.selected_extensions) == 2

    def test_headers_accessor(self):
        hs = eggsec.WebSocketHandshakePy(
            url="ws://localhost",
            status_code=101,
            headers=[("X-Request-Id", "abc-123")],
            selected_subprotocol=None,
            selected_extensions=[],
            duration_ms=5.0,
        )
        h = hs.headers
        assert len(h) == 1
        assert h[0] == ("X-Request-Id", "abc-123")

    def test_to_dict(self):
        hs = eggsec.WebSocketHandshakePy(
            url="ws://localhost",
            status_code=101,
            headers=[("Key", "Val")],
            selected_subprotocol="graphql-ws",
            selected_extensions=["ext1"],
            duration_ms=50.0,
        )
        d = hs.to_dict()
        assert d["url"] == "ws://localhost"
        assert d["status_code"] == 101
        assert d["selected_subprotocol"] == "graphql-ws"
        assert d["duration_ms"] == 50.0
        assert len(d["headers"]) == 1
        assert len(d["selected_extensions"]) == 1

    def test_to_json(self):
        hs = eggsec.WebSocketHandshakePy(
            url="ws://localhost",
            status_code=101,
            headers=[],
            selected_subprotocol=None,
            selected_extensions=[],
            duration_ms=10.0,
        )
        j = hs.to_json()
        parsed = json.loads(j)
        assert parsed["url"] == "ws://localhost"
        assert parsed["status_code"] == 101

    def test_repr(self):
        hs = eggsec.WebSocketHandshakePy(
            url="ws://localhost", status_code=101,
            headers=[], selected_subprotocol=None,
            selected_extensions=[], duration_ms=42.1,
        )
        r = repr(hs)
        assert "ws://localhost" in r
        assert "101" in r
        assert "42.1" in r

    def test_str(self):
        hs = eggsec.WebSocketHandshakePy(
            url="wss://echo.websocket.org", status_code=101,
            headers=[], selected_subprotocol=None,
            selected_extensions=[], duration_ms=88.5,
        )
        s = str(hs)
        assert "101" in s
        assert "wss://echo.websocket.org" in s
        assert "88.5ms" in s


class TestWebSocketFrameConstruction:
    """WebSocketFramePy: frame-level DTO."""

    def test_text_frame(self):
        f = eggsec.WebSocketFramePy(
            opcode=1, opcode_name="text",
            payload=b"hello", fin=True, masked=True,
        )
        assert f.opcode == 1
        assert f.opcode_name == "text"
        assert f.payload == b"hello"
        assert f.fin is True
        assert f.masked is True

    def test_binary_frame(self):
        f = eggsec.WebSocketFramePy(
            opcode=2, opcode_name="binary",
            payload=b"\x00\xFF", fin=True, masked=False,
        )
        assert f.opcode == 2
        assert f.masked is False

    def test_close_frame(self):
        f = eggsec.WebSocketFramePy(
            opcode=8, opcode_name="close",
            payload=b"", fin=True, masked=True,
        )
        assert f.opcode == 8
        assert f.fin is True

    def test_ping_frame(self):
        f = eggsec.WebSocketFramePy(
            opcode=9, opcode_name="ping",
            payload=b"\x01\x02", fin=True, masked=True,
        )
        assert f.opcode == 9
        assert f.opcode_name == "ping"

    def test_pong_frame(self):
        f = eggsec.WebSocketFramePy(
            opcode=10, opcode_name="pong",
            payload=b"\x01\x02", fin=True, masked=True,
        )
        assert f.opcode == 10

    def test_continuation_frame(self):
        f = eggsec.WebSocketFramePy(
            opcode=0, opcode_name="continuation",
            payload=b"part2", fin=False, masked=False,
        )
        assert f.opcode == 0
        assert f.fin is False

    def test_to_dict(self):
        f = eggsec.WebSocketFramePy(
            opcode=1, opcode_name="text",
            payload=b"hi", fin=True, masked=True,
        )
        d = f.to_dict()
        assert d["opcode"] == 1
        assert d["opcode_name"] == "text"
        assert d["fin"] is True
        assert d["masked"] is True

    def test_to_json(self):
        f = eggsec.WebSocketFramePy(
            opcode=1, opcode_name="text",
            payload=b"hi", fin=True, masked=True,
        )
        j = f.to_json()
        parsed = json.loads(j)
        assert parsed["opcode"] == 1
        assert parsed["opcode_name"] == "text"

    def test_repr(self):
        f = eggsec.WebSocketFramePy(
            opcode=1, opcode_name="text",
            payload=b"hello", fin=True, masked=True,
        )
        r = repr(f)
        assert "text" in r
        assert "fin=True" in r
        assert "5" in r

    def test_str(self):
        f = eggsec.WebSocketFramePy(
            opcode=2, opcode_name="binary",
            payload=b"\x00\x01\x02", fin=True, masked=False,
        )
        s = str(f)
        assert "binary" in s
        assert "3 bytes" in s


# ---------------------------------------------------------------------------
# 2. Session Lifecycle (no live WS server required)
# ---------------------------------------------------------------------------


class TestSessionLifecycle:
    """WebSocketSessionPy lifecycle: construction, properties, context manager."""

    def test_session_construction(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        assert s.url == "ws://127.0.0.1:1"
        assert s.is_closed is False

    def test_session_initial_is_closed_false(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        assert s.is_closed is False

    def test_session_url_property(self):
        c = eggsec.WebSocketSessionConfigPy(url="wss://echo.websocket.org")
        s = eggsec.WebSocketSessionPy(config=c)
        assert s.url == "wss://echo.websocket.org"

    def test_session_context_manager_enter_exit(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        with s as session:
            assert session is s
            assert session.is_closed is False
        assert s.is_closed is True

    def test_session_context_manager_exception_still_closes(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        with pytest.raises(RuntimeError):
            with s as session:
                raise RuntimeError("test exception")
        assert s.is_closed is True

    def test_session_close_on_unconnected(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        ci = s.close()
        assert ci.code == 1000
        assert ci.was_clean is True
        assert s.is_closed is True

    def test_session_double_close(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        ci1 = s.close()
        assert ci1.was_clean is True
        ci2 = s.close()
        assert ci2.code == 1000
        assert ci2.was_clean is True

    def test_session_custom_close_code(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        ci = s.close(code=1001, reason="going away")
        assert ci.code == 1001
        assert ci.reason == "going away"

    def test_session_repr(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        r = repr(s)
        assert "ws://127.0.0.1:1" in r
        assert "closed=" in r
        assert "sent=" in r
        assert "received=" in r

    def test_session_str_open(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        st = str(s)
        assert "open" in st

    def test_session_str_closed(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        s.close()
        st = str(s)
        assert "closed" in st

    def test_session_transcript(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        t = s.transcript
        assert t.total_bytes == 0

    def test_session_send_text_not_connected_raises(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        with pytest.raises(Exception, match="Not connected"):
            s.send_text("hello")

    def test_session_send_binary_not_connected_raises(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        with pytest.raises(Exception, match="Not connected"):
            s.send_binary(b"\x00")

    def test_session_recv_not_connected_raises(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        with pytest.raises(Exception, match="Not connected"):
            s.recv()

    def test_session_ping_not_connected_raises(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        with pytest.raises(Exception, match="Not connected"):
            s.ping()

    def test_session_recv_available_not_connected_raises(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        with pytest.raises(Exception, match="Not connected"):
            s.recv_available()

    def test_session_connect_to_refused_raises(self):
        c = eggsec.WebSocketSessionConfigPy(
            url="ws://127.0.0.1:1",
            timeout_ms=1000,
        )
        s = eggsec.WebSocketSessionPy(config=c)
        with pytest.raises(Exception):
            s.connect()


class TestAsyncSessionLifecycle:
    """AsyncWebSocketSessionPy: construction, properties, async context manager."""

    def test_async_session_construction(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.AsyncWebSocketSessionPy(config=c)
        assert s.url == "ws://127.0.0.1:1"
        assert s.is_closed is False

    def test_async_session_url_property(self):
        c = eggsec.WebSocketSessionConfigPy(url="wss://example.com")
        s = eggsec.AsyncWebSocketSessionPy(config=c)
        assert s.url == "wss://example.com"

    def test_async_session_repr(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.AsyncWebSocketSessionPy(config=c)
        r = repr(s)
        assert "ws://127.0.0.1:1" in r
        assert "closed=" in r

    def test_async_session_str_open(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.AsyncWebSocketSessionPy(config=c)
        st = str(s)
        assert "open" in st

    def test_async_session_str_closed(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.AsyncWebSocketSessionPy(config=c)
        s.async_close()
        st = str(s)
        assert "closed" in st


# ---------------------------------------------------------------------------
# 3. Assessment Types
# ---------------------------------------------------------------------------


class TestWebSocketAssessmentConfig:
    """WebSocketAssessmentConfigPy: construction and validation."""

    def test_minimal_config(self):
        c = eggsec.WebSocketAssessmentConfigPy(url="ws://localhost:8080/ws")
        assert c.url == "ws://localhost:8080/ws"
        assert c.timeout_ms == 30000
        assert c.test_connection is True
        assert c.test_origin_validation is True
        assert c.test_authentication is True
        assert c.test_subprotocol is True
        assert c.test_message_access is True
        assert c.test_close_behavior is True

    def test_selective_tests(self):
        c = eggsec.WebSocketAssessmentConfigPy(
            url="ws://localhost",
            timeout_ms=5000,
            test_connection=True,
            test_origin_validation=False,
            test_authentication=False,
            test_subprotocol=False,
            test_message_access=False,
            test_close_behavior=False,
        )
        assert c.test_connection is True
        assert c.test_origin_validation is False
        assert c.test_authentication is False
        assert c.test_subprotocol is False
        assert c.test_message_access is False
        assert c.test_close_behavior is False

    def test_empty_url_raises(self):
        with pytest.raises(ValueError, match="url must not be empty"):
            eggsec.WebSocketAssessmentConfigPy(url="")

    def test_to_dict(self):
        c = eggsec.WebSocketAssessmentConfigPy(
            url="ws://localhost",
            timeout_ms=5000,
        )
        d = c.to_dict()
        assert d["url"] == "ws://localhost"
        assert d["timeout_ms"] == 5000
        assert d["test_connection"] is True
        assert d["test_origin_validation"] is True

    def test_to_json(self):
        c = eggsec.WebSocketAssessmentConfigPy(url="ws://localhost")
        j = c.to_json()
        parsed = json.loads(j)
        assert parsed["url"] == "ws://localhost"
        assert parsed["timeout_ms"] == 30000

    def test_repr(self):
        c = eggsec.WebSocketAssessmentConfigPy(
            url="ws://localhost", timeout_ms=5000,
        )
        r = repr(c)
        assert "ws://localhost" in r
        assert "5000" in r


class TestWebSocketAssessmentResult:
    """WebSocketAssessmentResultPy: construction and serialization."""

    def _make_result(self, **overrides):
        """Helper to create a result with sensible defaults."""
        defaults = dict(
            target="ws://localhost:8080",
            handshake=None,
            origin_test=None,
            auth_test=None,
            subprotocol_test=None,
            message_test=None,
            close_test=None,
            findings=[],
            timing=eggsec.ConnectionTimingPy(total_ms=100.0),
        )
        defaults.update(overrides)
        return eggsec.WebSocketAssessmentResultPy(**defaults)

    def test_minimal_result(self):
        r = self._make_result()
        assert r.target == "ws://localhost:8080"
        assert r.handshake is None
        assert r.finding_count == 0

    def test_result_with_findings(self):
        finding = eggsec.WebSocketFindingPy(
            category="origin-validation",
            severity=eggsec.Severity.High,
            title="CSWSH risk",
            description="Server accepted malicious origin",
            recommendation="Validate Origin header",
        )
        r = self._make_result(findings=[finding])
        assert r.finding_count == 1
        assert r.findings[0].severity == eggsec.Severity.High

    def test_result_to_dict(self):
        r = self._make_result()
        d = r.to_dict()
        assert d["target"] == "ws://localhost:8080"
        assert d["finding_count"] == 0
        assert d["handshake"] is None
        assert "timing" in d

    def test_result_to_json(self):
        r = self._make_result()
        j = r.to_json()
        parsed = json.loads(j)
        assert parsed["target"] == "ws://localhost:8080"

    def test_result_repr(self):
        r = self._make_result()
        rep = repr(r)
        assert "ws://localhost:8080" in rep
        assert "findings=" in rep

    def test_result_str(self):
        r = self._make_result()
        s = str(r)
        assert "ws://localhost:8080" in s
        assert "0 findings" in s

    def test_result_with_handshake(self):
        hs = eggsec.WebSocketHandshakePy(
            url="ws://localhost",
            status_code=101,
            headers=[],
            selected_subprotocol=None,
            selected_extensions=[],
            duration_ms=50.0,
        )
        r = self._make_result(handshake=hs)
        assert r.handshake is not None
        assert r.handshake.status_code == 101


# ---------------------------------------------------------------------------
# 4. Probe Result Types
# ---------------------------------------------------------------------------


class TestWebSocketReport:
    """WebSocketReport (alias for WebSocketReportPy): construction and serialization.

    The probe result types (ConnectionTestResultPy, InjectionTestResultPy,
    OriginTestResultPy, FuzzTestResultPy) are not directly constructible from
    Python (no #[new]). We test the report type via the websocket_probe engine
    path, and verify the report's property accessors and serialization.
    """

    def test_report_type_exists(self):
        assert hasattr(eggsec, "WebSocketReport")

    def test_report_from_probe_unreachable(self):
        """websocket_probe returns a report even for unreachable targets."""
        if not hasattr(eggsec, "websocket_probe"):
            pytest.skip("websocket_probe not available")
        report = eggsec.websocket_probe("ws://127.0.0.1:1", timeout_secs=1)
        assert hasattr(report, "target")
        assert hasattr(report, "connection_test")
        assert hasattr(report, "injection_tests")
        assert hasattr(report, "origin_tests")
        assert hasattr(report, "fuzz_tests")
        assert hasattr(report, "findings")
        assert hasattr(report, "finding_count")

    def test_report_from_fuzz_unreachable(self):
        """websocket_fuzz returns a report even for unreachable targets."""
        if not hasattr(eggsec, "websocket_fuzz"):
            pytest.skip("websocket_fuzz not available")
        report = eggsec.websocket_fuzz("ws://127.0.0.1:1", timeout_secs=1)
        assert hasattr(report, "target")
        assert report.finding_count >= 0

    def test_report_to_dict(self):
        """Report to_dict produces expected structure."""
        if not hasattr(eggsec, "websocket_probe"):
            pytest.skip("websocket_probe not available")
        report = eggsec.websocket_probe("ws://127.0.0.1:1", timeout_secs=1)
        d = report.to_dict()
        assert "target" in d
        assert "finding_count" in d
        assert "connection_test" in d
        assert "injection_tests" in d
        assert "origin_tests" in d
        assert "fuzz_tests" in d
        assert "findings" in d

    def test_report_to_json(self):
        """Report to_json produces valid JSON."""
        if not hasattr(eggsec, "websocket_probe"):
            pytest.skip("websocket_probe not available")
        report = eggsec.websocket_probe("ws://127.0.0.1:1", timeout_secs=1)
        j = report.to_json()
        parsed = json.loads(j)
        assert "target" in parsed

    def test_report_repr(self):
        if not hasattr(eggsec, "websocket_probe"):
            pytest.skip("websocket_probe not available")
        report = eggsec.websocket_probe("ws://127.0.0.1:1", timeout_secs=1)
        r = repr(report)
        assert "WebSocketReport" in r

    def test_report_str(self):
        if not hasattr(eggsec, "websocket_probe"):
            pytest.skip("websocket_probe not available")
        report = eggsec.websocket_probe("ws://127.0.0.1:1", timeout_secs=1)
        s = str(report)
        assert "findings" in s

    def test_report_injection_tests_is_list(self):
        if not hasattr(eggsec, "websocket_probe"):
            pytest.skip("websocket_probe not available")
        report = eggsec.websocket_probe("ws://127.0.0.1:1", timeout_secs=1)
        assert isinstance(report.injection_tests, list)

    def test_report_origin_tests_is_list(self):
        if not hasattr(eggsec, "websocket_probe"):
            pytest.skip("websocket_probe not available")
        report = eggsec.websocket_probe("ws://127.0.0.1:1", timeout_secs=1)
        assert isinstance(report.origin_tests, list)

    def test_report_fuzz_tests_is_list(self):
        if not hasattr(eggsec, "websocket_probe"):
            pytest.skip("websocket_probe not available")
        report = eggsec.websocket_probe("ws://127.0.0.1:1", timeout_secs=1)
        assert isinstance(report.fuzz_tests, list)


# ---------------------------------------------------------------------------
# 5. Scope Enforcement
# ---------------------------------------------------------------------------


class TestScopeEnforcement:
    """Verify scope enforcement for websocket_assess and session operations."""

    def test_websocket_assess_unreachable_target(self):
        """websocket_assess with unreachable target returns findings, not crash."""
        result = eggsec.websocket_assess(
            "ws://127.0.0.1:1",
            timeout_ms=1000,
        )
        assert result.target == "ws://127.0.0.1:1"
        assert result.finding_count >= 1
        assert result.handshake is None

    def test_websocket_assess_returns_result_on_connection_failure(self):
        """Connection failure still produces a valid WebSocketAssessmentResultPy."""
        result = eggsec.websocket_assess(
            "ws://192.0.2.1:1",
            timeout_ms=500,
        )
        assert isinstance(result, eggsec.WebSocketAssessmentResultPy)
        assert result.target == "ws://192.0.2.1:1"

    def test_session_scope_independent(self):
        """WebSocketSessionPy does not take scope; it's a raw transport."""
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        s = eggsec.WebSocketSessionPy(config=c)
        assert s.url == "ws://127.0.0.1:1"

    def test_assessment_config_scope_independent(self):
        """WebSocketAssessmentConfigPy does not take scope parameter."""
        c = eggsec.WebSocketAssessmentConfigPy(url="ws://evil.example.com")
        assert c.url == "ws://evil.example.com"

    def test_engine_dispatch_scope_enforcement(self, sentinel_engine):
        """Engine with sentinel scope should enforce scope on dispatch."""
        op = eggsec.OperationRequest(
            "scan_ports",
            target="evil.example.org",
        )
        with pytest.raises(eggsec.EnforcementError):
            sentinel_engine.run(op)


# ---------------------------------------------------------------------------
# 6. Message Types (semantic verification)
# ---------------------------------------------------------------------------


class TestMessageTypes:
    """WebSocketMessagePy: verify type flag semantics and data access."""

    def test_text_message_flags(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"text data",
            is_text=True, is_binary=False,
            is_ping=False, is_pong=False,
            text_content="text data",
            size=9,
        )
        assert msg.is_text is True
        assert msg.is_binary is False
        assert msg.is_ping is False
        assert msg.is_pong is False
        assert msg.text_content is not None

    def test_binary_message_flags(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"\x00\xFF",
            is_text=False, is_binary=True,
            is_ping=False, is_pong=False,
            text_content=None,
            size=2,
        )
        assert msg.is_text is False
        assert msg.is_binary is True
        assert msg.is_ping is False
        assert msg.is_pong is False
        assert msg.text_content is None

    def test_ping_message_flags(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"",
            is_text=False, is_binary=False,
            is_ping=True, is_pong=False,
            text_content=None,
            size=0,
        )
        assert msg.is_ping is True
        assert msg.is_pong is False

    def test_pong_message_flags(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"",
            is_text=False, is_binary=False,
            is_ping=False, is_pong=True,
            text_content=None,
            size=0,
        )
        assert msg.is_pong is True
        assert msg.is_ping is False

    def test_text_to_text_returns_string(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"hello",
            is_text=True, is_binary=False,
            text_content="hello",
            size=5,
        )
        result = msg.to_text()
        assert isinstance(result, str)
        assert result == "hello"

    def test_binary_to_bytes_returns_bytes(self):
        raw = b"\xDE\xAD\xBE\xEF"
        msg = eggsec.WebSocketMessagePy(
            data=raw,
            is_text=False, is_binary=True,
            text_content=None,
            size=4,
        )
        result = msg.to_bytes()
        assert isinstance(result, bytes)
        assert result == raw

    def test_text_size_matches_data_length(self):
        text = "unicode test: \u00e9\u00e8\u00ea"
        msg = eggsec.WebSocketMessagePy(
            data=text.encode("utf-8"),
            is_text=True, is_binary=False,
            text_content=text,
            size=len(text.encode("utf-8")),
        )
        assert msg.size == len(msg.data)

    def test_binary_size_matches_data_length(self):
        data = bytes(range(256))
        msg = eggsec.WebSocketMessagePy(
            data=data,
            is_text=False, is_binary=True,
            text_content=None,
            size=256,
        )
        assert msg.size == 256

    def test_multibyte_utf8_text(self):
        text = "\u2603 \u2764 \U0001F600"
        data = text.encode("utf-8")
        msg = eggsec.WebSocketMessagePy(
            data=data,
            is_text=True, is_binary=False,
            text_content=text,
            size=len(data),
        )
        assert msg.to_text() == text
        assert msg.size == len(data)


# ---------------------------------------------------------------------------
# 7. Assessment Finding Types
# ---------------------------------------------------------------------------


class TestWebSocketFindingTypes:
    """WebSocketFindingPy: construction with all severity levels."""

    def _make_finding(self, severity):
        return eggsec.WebSocketFindingPy(
            category="test-category",
            severity=severity,
            title=f"Test finding [{severity}]",
            description="Test description",
            recommendation="Test recommendation",
        )

    def test_critical_finding(self):
        f = self._make_finding(eggsec.Severity.Critical)
        assert f.severity == eggsec.Severity.Critical
        assert f.category == "test-category"
        assert "Critical" in f.title

    def test_high_finding(self):
        f = self._make_finding(eggsec.Severity.High)
        assert f.severity == eggsec.Severity.High

    def test_medium_finding(self):
        f = self._make_finding(eggsec.Severity.Medium)
        assert f.severity == eggsec.Severity.Medium

    def test_low_finding(self):
        f = self._make_finding(eggsec.Severity.Low)
        assert f.severity == eggsec.Severity.Low

    def test_info_finding(self):
        f = self._make_finding(eggsec.Severity.Info)
        assert f.severity == eggsec.Severity.Info

    def test_finding_to_dict(self):
        f = eggsec.WebSocketFindingPy(
            category="origin-validation",
            severity=eggsec.Severity.High,
            title="CSWSH",
            description="Malicious origin accepted",
            recommendation="Validate Origin",
        )
        d = f.to_dict()
        assert d["category"] == "origin-validation"
        assert d["severity"] == "High"
        assert d["title"] == "CSWSH"
        assert d["description"] == "Malicious origin accepted"
        assert d["recommendation"] == "Validate Origin"

    def test_finding_to_json_roundtrip(self):
        f = eggsec.WebSocketFindingPy(
            category="close-behavior",
            severity=eggsec.Severity.Low,
            title="Unclean close",
            description="No close frame received",
            recommendation="Fix close handling",
        )
        j = f.to_json()
        parsed = json.loads(j)
        assert parsed["category"] == "close-behavior"
        assert parsed["severity"] == "Low"
        assert parsed["title"] == "Unclean close"

    def test_finding_repr(self):
        f = eggsec.WebSocketFindingPy(
            category="injection",
            severity=eggsec.Severity.Critical,
            title="XSS in WS",
            description="Reflected XSS",
            recommendation="Sanitize input",
        )
        r = repr(f)
        assert "injection" in r
        assert "Critical" in r
        assert "XSS in WS" in r

    def test_finding_str(self):
        f = eggsec.WebSocketFindingPy(
            category="auth",
            severity=eggsec.Severity.Medium,
            title="Missing auth",
            description="No authentication required",
            recommendation="Add auth",
        )
        s = str(f)
        assert "Medium" in s
        assert "auth" in s
        assert "Missing auth" in s

    def test_finding_fields_read_only(self):
        """Frozen pyclass — attributes should not be assignable."""
        f = self._make_finding(eggsec.Severity.Info)
        with pytest.raises(AttributeError):
            f.title = "modified"

    def test_finding_all_severity_levels_roundtrip(self):
        """All severity levels survive serialization roundtrip."""
        for sev in [
            eggsec.Severity.Critical,
            eggsec.Severity.High,
            eggsec.Severity.Medium,
            eggsec.Severity.Low,
            eggsec.Severity.Info,
        ]:
            f = self._make_finding(sev)
            j = f.to_json()
            d = f.to_dict()
            assert d["severity"] == str(sev)


# ---------------------------------------------------------------------------
# 8. Engine Dispatch
# ---------------------------------------------------------------------------


class TestEngineDispatch:
    """Engine.run with websocket-related operation requests."""

    def test_engine_operation_request_construction(self):
        """OperationRequest can be constructed for websocket operations."""
        req = eggsec.OperationRequest(
            "websocket_assess",
            target="ws://localhost:8080",
        )
        assert req.operation == "websocket_assess"
        assert req.target == "ws://localhost:8080"

    def test_engine_scan_ports_dispatch(self, sentinel_engine):
        """Basic engine dispatch for scan_ports works."""
        req = eggsec.OperationRequest(
            "scan_ports",
            target="sentinel.example.org",
        )
        result = sentinel_engine.run(req)
        assert isinstance(result, eggsec.OperationResult)

    def test_engine_out_of_scope_dispatch(self, sentinel_engine):
        """Engine dispatch with out-of-scope target raises EnforcementError."""
        req = eggsec.OperationRequest(
            "scan_ports",
            target="evil.example.org",
        )
        with pytest.raises(eggsec.EnforcementError):
            sentinel_engine.run(req)

    def test_async_engine_dispatch(self, sentinel_async_engine):
        """AsyncEngine dispatch returns a future-like result."""
        req = eggsec.OperationRequest(
            "scan_ports",
            target="sentinel.example.org",
        )
        future = sentinel_async_engine.run(req)
        assert future is not None


# ---------------------------------------------------------------------------
# 9. Feature Availability
# ---------------------------------------------------------------------------


class TestFeatureAvailability:
    """Verify websocket types and functions are available when feature compiled."""

    def test_websocket_session_config_available(self):
        assert hasattr(eggsec, "WebSocketSessionConfigPy")

    def test_websocket_session_available(self):
        assert hasattr(eggsec, "WebSocketSessionPy")

    def test_async_websocket_session_available(self):
        assert hasattr(eggsec, "AsyncWebSocketSessionPy")

    def test_websocket_message_available(self):
        assert hasattr(eggsec, "WebSocketMessagePy")

    def test_websocket_frame_available(self):
        assert hasattr(eggsec, "WebSocketFramePy")

    def test_websocket_close_info_available(self):
        assert hasattr(eggsec, "WebSocketCloseInfoPy")

    def test_websocket_handshake_available(self):
        assert hasattr(eggsec, "WebSocketHandshakePy")

    def test_websocket_assessment_config_available(self):
        assert hasattr(eggsec, "WebSocketAssessmentConfigPy")

    def test_websocket_assessment_result_available(self):
        assert hasattr(eggsec, "WebSocketAssessmentResultPy")

    def test_websocket_assess_function_available(self):
        assert hasattr(eggsec, "websocket_assess")

    def test_async_websocket_assess_function_available(self):
        assert hasattr(eggsec, "async_websocket_assess")

    def test_websocket_report_available(self):
        assert hasattr(eggsec, "WebSocketReport")

    def test_severity_available(self):
        assert hasattr(eggsec, "Severity")
        assert eggsec.Severity.Critical is not None
        assert eggsec.Severity.High is not None
        assert eggsec.Severity.Medium is not None
        assert eggsec.Severity.Low is not None
        assert eggsec.Severity.Info is not None


# ---------------------------------------------------------------------------
# 10. Serialization Consistency
# ---------------------------------------------------------------------------


class TestSerializationConsistency:
    """Verify to_dict and to_json produce consistent output across types."""

    def test_config_dict_json_consistency(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://localhost")
        d = c.to_dict()
        j = json.loads(c.to_json())
        assert d["url"] == j["url"]
        assert d["timeout_ms"] == j["timeout_ms"]
        assert d["verify_tls"] == j["verify_tls"]

    def test_close_info_dict_json_consistency(self):
        ci = eggsec.WebSocketCloseInfoPy(code=1000, reason="ok", was_clean=True)
        d = ci.to_dict()
        j = json.loads(ci.to_json())
        assert d["code"] == j["code"]
        assert d["reason"] == j["reason"]
        assert d["was_clean"] == j["was_clean"]

    def test_handshake_dict_json_consistency(self):
        hs = eggsec.WebSocketHandshakePy(
            url="ws://localhost", status_code=101,
            headers=[("Key", "Val")],
            selected_subprotocol="graphql-ws",
            selected_extensions=["ext1"],
            duration_ms=42.0,
        )
        d = hs.to_dict()
        j = json.loads(hs.to_json())
        assert d["url"] == j["url"]
        assert d["status_code"] == j["status_code"]
        assert d["selected_subprotocol"] == j["selected_subprotocol"]
        assert d["duration_ms"] == j["duration_ms"]

    def test_frame_dict_json_consistency(self):
        f = eggsec.WebSocketFramePy(
            opcode=1, opcode_name="text",
            payload=b"test", fin=True, masked=True,
        )
        d = f.to_dict()
        j = json.loads(f.to_json())
        assert d["opcode"] == j["opcode"]
        assert d["opcode_name"] == j["opcode_name"]
        assert d["fin"] == j["fin"]
        assert d["masked"] == j["masked"]

    def test_message_dict_json_consistency(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"test", is_text=True, is_binary=False,
            text_content="test", size=4,
        )
        d = msg.to_dict()
        j = json.loads(msg.to_json())
        assert d["is_text"] == j["is_text"]
        assert d["is_binary"] == j["is_binary"]
        assert d["size"] == j["size"]

    def test_finding_dict_json_consistency(self):
        f = eggsec.WebSocketFindingPy(
            category="test", severity=eggsec.Severity.High,
            title="t", description="d", recommendation="r",
        )
        d = f.to_dict()
        j = json.loads(f.to_json())
        assert d["category"] == j["category"]
        assert d["severity"] == j["severity"]
        assert d["title"] == j["title"]

    def test_assessment_config_dict_json_consistency(self):
        c = eggsec.WebSocketAssessmentConfigPy(url="ws://localhost")
        d = c.to_dict()
        j = json.loads(c.to_json())
        assert d["url"] == j["url"]
        assert d["timeout_ms"] == j["timeout_ms"]
        assert d["test_connection"] == j["test_connection"]

    def test_report_dict_json_consistency(self):
        if not hasattr(eggsec, "websocket_probe"):
            pytest.skip("websocket_probe not available")
        r = eggsec.websocket_probe("ws://127.0.0.1:1", timeout_secs=1)
        d = r.to_dict()
        j = json.loads(r.to_json())
        assert d["target"] == j["target"]
        assert d["finding_count"] == j["finding_count"]
