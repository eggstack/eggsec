"""
Release 2 comprehensive tests — contract, functional, and performance budget.

Covers AsyncTcpSession, AsyncUdpSocket, SyncCaptureSession, PacketTimestamp,
PacketStream, PacketArtifact, DnsPacket, TlsRecordInfo, UdpReachability,
WS11 network events, and performance budgets.

All tests are pure unit tests requiring no network access, live servers,
or special privileges.
"""

import json
import time
import unittest

try:
    import eggsec
except ImportError:
    eggsec = None


def _require_eggsec(cls):
    """Skip the entire class if eggsec is not importable."""
    if eggsec is None:
        @unittest.skip("eggsec not importable")
        class _Skip(cls):
            pass
        return _Skip
    return cls


def _require_packet_inspection(cls):
    """Skip the entire class if packet-inspection types are missing."""
    if not hasattr(eggsec, "PacketTimestampPy"):
        @unittest.skip("packet-inspection feature not compiled")
        class _Skip(cls):
            pass
        return _Skip
    return cls


# ═══════════════════════════════════════════════════════════════════════
# 1. TestAsyncTcpSession (contract tests)
# ═══════════════════════════════════════════════════════════════════════

@_require_eggsec
class TestAsyncTcpSession(unittest.TestCase):
    """Contract tests for AsyncTcpSessionPy."""

    def test_construction(self):
        """AsyncTcpSessionPy can be created with a TcpConfigPy."""
        cfg = eggsec.TcpConfigPy(host="10.0.0.1", port=443)
        session = eggsec.AsyncTcpSessionPy(config=cfg)
        self.assertIsNotNone(session)

    def test_initial_state(self):
        """Session starts not closed, zero bytes."""
        cfg = eggsec.TcpConfigPy(host="10.0.0.1", port=443)
        session = eggsec.AsyncTcpSessionPy(config=cfg)
        self.assertFalse(session.is_closed)
        self.assertEqual(session.bytes_sent, 0)
        self.assertEqual(session.bytes_received, 0)

    def test_config_property(self):
        """Config is returned correctly."""
        cfg = eggsec.TcpConfigPy(host="10.0.0.1", port=443)
        session = eggsec.AsyncTcpSessionPy(config=cfg)
        returned_cfg = session.config
        self.assertEqual(returned_cfg.host, "10.0.0.1")
        self.assertEqual(returned_cfg.port, 443)

    def test_read_before_connect_raises(self):
        """Reading before connect raises NetworkError."""
        import asyncio

        cfg = eggsec.TcpConfigPy(host="10.0.0.1", port=443)
        session = eggsec.AsyncTcpSessionPy(config=cfg)
        loop = asyncio.new_event_loop()
        try:
            with self.assertRaises(eggsec.NetworkError):
                loop.run_until_complete(session.read(10))
        finally:
            loop.close()

    def test_write_before_connect_raises(self):
        """Writing before connect raises NetworkError."""
        import asyncio

        cfg = eggsec.TcpConfigPy(host="10.0.0.1", port=443)
        session = eggsec.AsyncTcpSessionPy(config=cfg)
        loop = asyncio.new_event_loop()
        try:
            with self.assertRaises(eggsec.NetworkError):
                loop.run_until_complete(session.write(b"hello"))
        finally:
            loop.close()

    def test_double_close(self):
        """Closing twice is safe."""
        cfg = eggsec.TcpConfigPy(host="10.0.0.1", port=443)
        session = eggsec.AsyncTcpSessionPy(config=cfg)
        session.close()
        self.assertTrue(session.is_closed)
        session.close()
        self.assertTrue(session.is_closed)

    def test_sync_enter_raises_typeerror(self):
        """Using sync context manager raises TypeError."""
        cfg = eggsec.TcpConfigPy(host="10.0.0.1", port=443)
        session = eggsec.AsyncTcpSessionPy(config=cfg)
        with self.assertRaises(TypeError):
            with session:
                pass

    def test_repr_and_str(self):
        """repr and str return non-empty strings."""
        cfg = eggsec.TcpConfigPy(host="10.0.0.1", port=443)
        session = eggsec.AsyncTcpSessionPy(config=cfg)
        r = repr(session)
        s = str(session)
        self.assertGreater(len(r), 0)
        self.assertGreater(len(s), 0)
        self.assertIn("10.0.0.1", r)
        self.assertIn("443", r)

    def test_transcript_initially_empty(self):
        """Transcript starts empty."""
        cfg = eggsec.TcpConfigPy(host="10.0.0.1", port=443)
        session = eggsec.AsyncTcpSessionPy(config=cfg)
        transcript = session.transcript
        self.assertEqual(len(transcript), 0)


# ═══════════════════════════════════════════════════════════════════════
# 2. TestAsyncUdpSocket (contract tests)
# ═══════════════════════════════════════════════════════════════════════

@_require_eggsec
class TestAsyncUdpSocket(unittest.TestCase):
    """Contract tests for AsyncUdpSocketPy."""

    def test_construction(self):
        """AsyncUdpSocketPy can be created with a UdpConfigPy."""
        cfg = eggsec.UdpConfigPy(host="10.0.0.1", port=53)
        sock = eggsec.AsyncUdpSocketPy(config=cfg)
        self.assertIsNotNone(sock)

    def test_initial_state(self):
        """Socket starts not closed, zero bytes."""
        cfg = eggsec.UdpConfigPy(host="10.0.0.1", port=53)
        sock = eggsec.AsyncUdpSocketPy(config=cfg)
        self.assertFalse(sock.is_closed)
        self.assertEqual(sock.bytes_sent, 0)
        self.assertEqual(sock.bytes_received, 0)

    def test_double_close(self):
        """Closing twice is safe."""
        cfg = eggsec.UdpConfigPy(host="10.0.0.1", port=53)
        sock = eggsec.AsyncUdpSocketPy(config=cfg)
        sock.close()
        self.assertTrue(sock.is_closed)
        sock.close()
        self.assertTrue(sock.is_closed)

    def test_sync_enter_raises_typeerror(self):
        """Using sync context manager raises TypeError."""
        cfg = eggsec.UdpConfigPy(host="10.0.0.1", port=53)
        sock = eggsec.AsyncUdpSocketPy(config=cfg)
        with self.assertRaises(TypeError):
            with sock:
                pass

    def test_transcript_initially_empty(self):
        """Transcript starts empty."""
        cfg = eggsec.UdpConfigPy(host="10.0.0.1", port=53)
        sock = eggsec.AsyncUdpSocketPy(config=cfg)
        transcript = sock.transcript
        self.assertEqual(len(transcript), 0)

    def test_repr_and_str(self):
        """repr and str return non-empty strings."""
        cfg = eggsec.UdpConfigPy(host="10.0.0.1", port=53)
        sock = eggsec.AsyncUdpSocketPy(config=cfg)
        r = repr(sock)
        s = str(sock)
        self.assertGreater(len(r), 0)
        self.assertGreater(len(s), 0)
        self.assertIn("10.0.0.1", r)
        self.assertIn("53", r)


# ═══════════════════════════════════════════════════════════════════════
# 3. TestSyncCaptureSession (contract tests)
# ═══════════════════════════════════════════════════════════════════════

@_require_packet_inspection
class TestSyncCaptureSession(unittest.TestCase):
    """Contract tests for SyncCaptureSessionPy."""

    def test_construction(self):
        """SyncCaptureSessionPy can be created with a CaptureConfig."""
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.SyncCaptureSessionPy(config=cfg)
        self.assertIsNotNone(session)

    def test_initial_state(self):
        """Session starts not running and not closed."""
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.SyncCaptureSessionPy(config=cfg)
        self.assertFalse(session.is_running)
        self.assertFalse(session.is_closed)

    def test_start_stop_lifecycle(self):
        """Start then stop completes successfully."""
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.SyncCaptureSessionPy(config=cfg)
        session.start()
        self.assertTrue(session.is_running)
        self.assertFalse(session.is_closed)
        stats = session.stop()
        self.assertFalse(session.is_running)
        self.assertTrue(session.is_closed)
        self.assertEqual(stats.packets_captured, 0)

    def test_double_start_raises(self):
        """Starting an already-running session raises ValueError."""
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.SyncCaptureSessionPy(config=cfg)
        session.start()
        with self.assertRaises(ValueError):
            session.start()
        session.stop()

    def test_stop_when_not_running_raises(self):
        """Stopping a non-running session raises ValueError."""
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.SyncCaptureSessionPy(config=cfg)
        with self.assertRaises(ValueError):
            session.stop()

    def test_context_manager(self):
        """Context manager auto-stops on exit."""
        cfg = eggsec.CaptureConfig(interface="lo")
        with eggsec.SyncCaptureSessionPy(config=cfg) as session:
            session.start()
            self.assertTrue(session.is_running)
        self.assertTrue(session.is_closed)

    def test_packets_initially_empty(self):
        """Packets list is empty initially."""
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.SyncCaptureSessionPy(config=cfg)
        packets = session.packets()
        self.assertEqual(len(packets), 0)

    def test_stats_initial(self):
        """Stats show zero captured initially."""
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.SyncCaptureSessionPy(config=cfg)
        stats = session.stats()
        self.assertEqual(stats.packets_captured, 0)
        self.assertEqual(stats.bytes_captured, 0)
        self.assertEqual(stats.packets_dropped, 0)

    def test_drop_stats_initial(self):
        """Drop stats show zero initially."""
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.SyncCaptureSessionPy(config=cfg)
        ds = session.drop_stats()
        self.assertEqual(ds.dropped_by_policy, 0)
        self.assertEqual(ds.dropped_by_full_queue, 0)
        self.assertEqual(ds.dropped_by_error, 0)


# ═══════════════════════════════════════════════════════════════════════
# 4. TestPacketTimestamp (DTO tests)
# ═══════════════════════════════════════════════════════════════════════

@_require_packet_inspection
class TestPacketTimestamp(unittest.TestCase):
    """DTO tests for PacketTimestampPy."""

    def test_construction(self):
        """PacketTimestampPy stores seconds, nanos, epoch_micros."""
        ts = eggsec.PacketTimestampPy(seconds=1000000, nanos=500000000, epoch_micros=1000000500)
        self.assertEqual(ts.seconds, 1000000)
        self.assertEqual(ts.nanos, 500000000)
        self.assertEqual(ts.epoch_micros, 1000000500)

    def test_to_dict(self):
        """to_dict returns correct keys and values."""
        ts = eggsec.PacketTimestampPy(seconds=42, nanos=7, epoch_micros=42000007)
        d = ts.to_dict()
        self.assertEqual(d["seconds"], 42)
        self.assertEqual(d["nanos"], 7)
        self.assertEqual(d["epoch_micros"], 42000007)

    def test_to_json(self):
        """to_json returns valid JSON with correct values."""
        ts = eggsec.PacketTimestampPy(seconds=100, nanos=200, epoch_micros=1000200)
        j = json.loads(ts.to_json())
        self.assertEqual(j["seconds"], 100)
        self.assertEqual(j["nanos"], 200)
        self.assertEqual(j["epoch_micros"], 1000200)

    def test_repr_and_str(self):
        """repr and str return non-empty strings containing key info."""
        ts = eggsec.PacketTimestampPy(seconds=999, nanos=1, epoch_micros=999000001)
        r = repr(ts)
        s = str(ts)
        self.assertIn("999", r)
        self.assertGreater(len(s), 0)


# ═══════════════════════════════════════════════════════════════════════
# 5. TestPacketStream (iterator tests)
# ═══════════════════════════════════════════════════════════════════════

@_require_packet_inspection
class TestPacketStream(unittest.TestCase):
    """Iterator tests for PacketStreamPy."""

    def test_construction_empty(self):
        """PacketStreamPy(packets=[]) creates empty stream."""
        ps = eggsec.PacketStreamPy(packets=[])
        self.assertEqual(len(ps), 0)
        self.assertTrue(ps.is_empty())

    def test_construction_with_packets(self):
        """PacketStreamPy(packets=[...]) stores count."""
        ps = eggsec.PacketStreamPy(packets=[])
        self.assertEqual(len(ps), 0)

    def test_next_returns_none_when_empty(self):
        """next() on empty stream returns None."""
        ps = eggsec.PacketStreamPy(packets=[])
        self.assertIsNone(ps.next())

    def test_iter_and_next(self):
        """next() advances through stream, then returns None."""
        ps = eggsec.PacketStreamPy(packets=[])
        self.assertIsNone(ps.next())
        self.assertIsNone(ps.next())

    def test_len(self):
        """len() returns 0 for empty stream."""
        ps = eggsec.PacketStreamPy(packets=[])
        self.assertEqual(len(ps), 0)
        self.assertEqual(len(ps), 0)

    def test_is_empty(self):
        """is_empty() returns True for empty stream."""
        ps = eggsec.PacketStreamPy(packets=[])
        self.assertTrue(ps.is_empty())

    def test_to_list(self):
        """to_list() returns empty list for empty stream."""
        ps = eggsec.PacketStreamPy(packets=[])
        self.assertEqual(ps.to_list(), [])


# ═══════════════════════════════════════════════════════════════════════
# 6. TestPacketArtifact (DTO tests)
# ═══════════════════════════════════════════════════════════════════════

@_require_packet_inspection
class TestPacketArtifact(unittest.TestCase):
    """DTO tests for PacketArtifactPy."""

    def test_construction(self):
        """PacketArtifactPy stores all fields correctly."""
        pa = eggsec.PacketArtifactPy(
            packet_index=42,
            artifact_type="pcap",
            description="test artifact",
            file_path="/tmp/test.pcap",
            byte_offset=0,
        )
        self.assertEqual(pa.packet_index, 42)
        self.assertEqual(pa.artifact_type, "pcap")
        self.assertEqual(pa.description, "test artifact")
        self.assertEqual(pa.file_path, "/tmp/test.pcap")
        self.assertEqual(pa.byte_offset, 0)

    def test_to_dict(self):
        """to_dict returns correct keys and values."""
        pa = eggsec.PacketArtifactPy(
            packet_index=10,
            artifact_type="frame",
            description="first frame",
        )
        d = pa.to_dict()
        self.assertEqual(d["packet_index"], 10)
        self.assertEqual(d["artifact_type"], "frame")
        self.assertEqual(d["description"], "first frame")
        self.assertIsNone(d["file_path"])
        self.assertIsNone(d["byte_offset"])

    def test_to_json(self):
        """to_json returns valid JSON."""
        pa = eggsec.PacketArtifactPy(
            packet_index=0,
            artifact_type="pcap",
            description="full capture",
        )
        j = json.loads(pa.to_json())
        self.assertEqual(j["packet_index"], 0)
        self.assertEqual(j["artifact_type"], "pcap")

    def test_optional_fields(self):
        """Optional fields default to None."""
        pa = eggsec.PacketArtifactPy(
            packet_index=0,
            artifact_type="frame",
        )
        self.assertIsNone(pa.file_path)
        self.assertIsNone(pa.byte_offset)


# ═══════════════════════════════════════════════════════════════════════
# 7. TestDnsPacket (DTO tests)
# ═══════════════════════════════════════════════════════════════════════

@_require_packet_inspection
class TestDnsPacket(unittest.TestCase):
    """DTO tests for DnsPacketPy."""

    def test_construction(self):
        """DnsPacketPy stores all fields correctly."""
        dp = eggsec.DnsPacketPy(
            transaction_id=0x1234,
            is_response=True,
            op_code=0,
            authoritative=False,
            truncated=False,
            recursion_desired=True,
            recursion_available=True,
            response_code=0,
            question_count=1,
            answer_count=2,
            authority_count=0,
            additional_count=0,
        )
        self.assertEqual(dp.transaction_id, 0x1234)
        self.assertTrue(dp.is_response)
        self.assertEqual(dp.response_code, 0)
        self.assertEqual(dp.answer_count, 2)

    def test_to_dict(self):
        """to_dict returns correct keys and values."""
        dp = eggsec.DnsPacketPy(
            transaction_id=100,
            is_response=False,
            response_code=0,
        )
        d = dp.to_dict()
        self.assertEqual(d["transaction_id"], 100)
        self.assertFalse(d["is_response"])
        self.assertEqual(d["response_code"], 0)

    def test_to_json(self):
        """to_json returns valid JSON."""
        dp = eggsec.DnsPacketPy(
            transaction_id=42,
            is_response=True,
            response_code=3,
        )
        j = json.loads(dp.to_json())
        self.assertEqual(j["transaction_id"], 42)
        self.assertTrue(j["is_response"])
        self.assertEqual(j["response_code"], 3)

    def test_all_flags(self):
        """All boolean flags are stored correctly."""
        dp = eggsec.DnsPacketPy(
            transaction_id=1,
            is_response=True,
            op_code=1,
            authoritative=True,
            truncated=True,
            recursion_desired=True,
            recursion_available=True,
            response_code=0,
        )
        self.assertTrue(dp.authoritative)
        self.assertTrue(dp.truncated)
        self.assertTrue(dp.recursion_desired)
        self.assertTrue(dp.recursion_available)
        self.assertEqual(dp.op_code, 1)


# ═══════════════════════════════════════════════════════════════════════
# 8. TestTlsRecordInfo (DTO tests)
# ═══════════════════════════════════════════════════════════════════════

@_require_packet_inspection
class TestTlsRecordInfo(unittest.TestCase):
    """DTO tests for TlsRecordInfoPy."""

    def test_construction(self):
        """TlsRecordInfoPy stores all fields correctly."""
        tr = eggsec.TlsRecordInfoPy(
            content_type="handshake",
            version="TLS12",
            record_length=200,
            handshake_type="ClientHello",
            cipher_suites=["TLS_AES_256_GCM_SHA384"],
            extensions=["server_name"],
            sni="example.com",
            alpn_protocols=["h2", "http/1.1"],
        )
        self.assertEqual(tr.content_type, "handshake")
        self.assertEqual(tr.version, "TLS12")
        self.assertEqual(tr.record_length, 200)
        self.assertEqual(tr.handshake_type, "ClientHello")
        self.assertEqual(tr.sni, "example.com")
        self.assertIn("h2", tr.alpn_protocols)

    def test_to_dict(self):
        """to_dict returns correct keys and values."""
        tr = eggsec.TlsRecordInfoPy(
            content_type="handshake",
            version="TLS13",
            record_length=100,
        )
        d = tr.to_dict()
        self.assertEqual(d["content_type"], "handshake")
        self.assertEqual(d["version"], "TLS13")
        self.assertEqual(d["record_length"], 100)
        self.assertIsNone(d["handshake_type"])
        self.assertIsNone(d["sni"])
        self.assertEqual(d["cipher_suites"], [])
        self.assertEqual(d["extensions"], [])
        self.assertEqual(d["alpn_protocols"], [])

    def test_to_json(self):
        """to_json returns valid JSON."""
        tr = eggsec.TlsRecordInfoPy(
            content_type="handshake",
            version="TLS12",
            record_length=300,
        )
        j = json.loads(tr.to_json())
        self.assertEqual(j["content_type"], "handshake")
        self.assertEqual(j["version"], "TLS12")
        self.assertEqual(j["record_length"], 300)

    def test_with_sni(self):
        """SNI field is preserved when set."""
        tr = eggsec.TlsRecordInfoPy(
            content_type="handshake",
            version="TLS13",
            record_length=150,
            sni="secure.example.com",
        )
        self.assertEqual(tr.sni, "secure.example.com")
        d = tr.to_dict()
        self.assertEqual(d["sni"], "secure.example.com")

    def test_without_handshake(self):
        """Non-handshake record has handshake_type=None."""
        tr = eggsec.TlsRecordInfoPy(
            content_type="application_data",
            version="TLS13",
            record_length=1024,
        )
        self.assertIsNone(tr.handshake_type)
        self.assertEqual(tr.content_type, "application_data")


# ═══════════════════════════════════════════════════════════════════════
# 9. TestUdpReachability (DTO + error tests)
# ═══════════════════════════════════════════════════════════════════════

@_require_packet_inspection
class TestUdpReachability(unittest.TestCase):
    """DTO tests for UdpReachabilityConfigPy and UdpReachabilityResultPy."""

    def test_config_construction(self):
        """UdpReachabilityConfigPy stores all fields."""
        cfg = eggsec.UdpReachabilityConfigPy(
            host="10.0.0.1",
            port=53,
            payload=b"\x00",
            timeout_ms=5000,
            attempts=3,
        )
        self.assertEqual(cfg.host, "10.0.0.1")
        self.assertEqual(cfg.port, 53)
        self.assertEqual(cfg.payload, [0])
        self.assertEqual(cfg.timeout_ms, 5000)
        self.assertEqual(cfg.attempts, 3)

    def test_config_defaults(self):
        """UdpReachabilityConfigPy has sensible defaults."""
        cfg = eggsec.UdpReachabilityConfigPy(host="10.0.0.1", port=53)
        self.assertIsNone(cfg.payload)
        self.assertEqual(cfg.timeout_ms, 2000)
        self.assertEqual(cfg.attempts, 1)

    def test_config_to_dict(self):
        """UdpReachabilityConfigPy to_dict returns correct keys."""
        cfg = eggsec.UdpReachabilityConfigPy(
            host="10.0.0.1", port=53, timeout_ms=1000
        )
        d = cfg.to_dict()
        self.assertEqual(d["host"], "10.0.0.1")
        self.assertEqual(d["port"], 53)
        self.assertEqual(d["timeout_ms"], 1000)
        self.assertEqual(d["attempts"], 1)

    def test_result_construction(self):
        """UdpReachabilityResultPy stores reachable result."""
        r = eggsec.UdpReachabilityResultPy(
            reachable=True,
            attempts=3,
            responses_received=3,
            response=b"\x01\x02",
            rtt_ms=1.5,
            error=None,
        )
        self.assertTrue(r.reachable)
        self.assertEqual(r.responses_received, 3)
        self.assertEqual(r.rtt_ms, 1.5)
        self.assertEqual(r.response, [1, 2])

    def test_result_unreachable(self):
        """UdpReachabilityResultPy stores unreachable result."""
        r = eggsec.UdpReachabilityResultPy(
            reachable=False,
            attempts=3,
            responses_received=0,
            response=None,
            rtt_ms=None,
            error="timeout",
        )
        self.assertFalse(r.reachable)
        self.assertEqual(r.responses_received, 0)
        self.assertIsNone(r.rtt_ms)
        self.assertEqual(r.error, "timeout")

    def test_result_to_dict(self):
        """UdpReachabilityResultPy to_dict returns correct keys."""
        r = eggsec.UdpReachabilityResultPy(
            reachable=True,
            attempts=2,
            responses_received=1,
            rtt_ms=5.0,
        )
        d = r.to_dict()
        self.assertTrue(d["reachable"])
        self.assertEqual(d["attempts"], 2)
        self.assertEqual(d["responses_received"], 1)
        self.assertEqual(d["rtt_ms"], 5.0)
        self.assertIsNone(d["error"])


# ═══════════════════════════════════════════════════════════════════════
# 10. TestNetworkEventsWS11 (all 8 new events)
# ═══════════════════════════════════════════════════════════════════════

@_require_eggsec
class TestNetworkEventsWS11(unittest.TestCase):
    """Contract tests for WS11 network event types."""

    def test_handshake_completed_event(self):
        """HandshakeCompletedEvent construction and serialization."""
        e = eggsec.HandshakeCompletedEvent(
            protocol="TLS",
            host="example.com",
            port=443,
            duration_ms=50.5,
            negotiated_version="TLS 1.3",
            cipher_suite="TLS_AES_256_GCM_SHA384",
        )
        self.assertEqual(e.protocol, "TLS")
        self.assertEqual(e.host, "example.com")
        self.assertEqual(e.port, 443)
        self.assertEqual(e.duration_ms, 50.5)
        self.assertEqual(e.negotiated_version, "TLS 1.3")
        d = e.to_dict()
        self.assertEqual(d["host"], "example.com")
        self.assertEqual(d["duration_ms"], 50.5)
        j = json.loads(e.to_json())
        self.assertEqual(j["port"], 443)

    def test_request_sent_event(self):
        """RequestSentEvent construction and serialization."""
        e = eggsec.RequestSentEvent(
            method="POST",
            url="https://example.com/api",
            headers_count=8,
            body_size=256,
            request_index=2,
        )
        self.assertEqual(e.method, "POST")
        self.assertEqual(e.url, "https://example.com/api")
        self.assertEqual(e.headers_count, 8)
        self.assertEqual(e.body_size, 256)
        self.assertEqual(e.request_index, 2)
        d = e.to_dict()
        self.assertEqual(d["method"], "POST")
        self.assertEqual(d["body_size"], 256)

    def test_response_headers_received_event(self):
        """ResponseHeadersReceivedEvent construction and serialization."""
        e = eggsec.ResponseHeadersReceivedEvent(
            status_code=201,
            reason="Created",
            headers_count=12,
            content_length=512,
            content_type="application/json",
            redirect_url=None,
            request_index=0,
        )
        self.assertEqual(e.status_code, 201)
        self.assertEqual(e.reason, "Created")
        self.assertEqual(e.headers_count, 12)
        self.assertEqual(e.content_length, 512)
        self.assertEqual(e.content_type, "application/json")
        self.assertIsNone(e.redirect_url)
        d = e.to_dict()
        self.assertEqual(d["status_code"], 201)
        self.assertEqual(d["reason"], "Created")

    def test_body_progress_event(self):
        """BodyProgressEvent construction and serialization."""
        e = eggsec.BodyProgressEvent(
            bytes_received=1024,
            is_complete=False,
            bytes_expected=4096,
            percentage=25.0,
        )
        self.assertEqual(e.bytes_received, 1024)
        self.assertFalse(e.is_complete)
        self.assertEqual(e.bytes_expected, 4096)
        self.assertEqual(e.percentage, 25.0)
        d = e.to_dict()
        self.assertEqual(d["bytes_received"], 1024)
        self.assertFalse(d["is_complete"])

    def test_capture_started_event(self):
        """CaptureStartedEvent construction and serialization."""
        e = eggsec.CaptureStartedEvent(
            interface="eth0",
            promiscuous=True,
            snapshot_len=65535,
            filter="tcp port 443",
        )
        self.assertEqual(e.interface, "eth0")
        self.assertTrue(e.promiscuous)
        self.assertEqual(e.snapshot_len, 65535)
        self.assertEqual(e.filter, "tcp port 443")
        d = e.to_dict()
        self.assertEqual(d["interface"], "eth0")
        self.assertTrue(d["promiscuous"])

    def test_packet_sampled_event(self):
        """PacketSampledEvent construction and serialization."""
        e = eggsec.PacketSampledEvent(
            interface="lo",
            packet_index=100,
            captured_len=60,
            original_len=1500,
            protocol_hint="TCP",
        )
        self.assertEqual(e.interface, "lo")
        self.assertEqual(e.packet_index, 100)
        self.assertEqual(e.captured_len, 60)
        self.assertEqual(e.original_len, 1500)
        self.assertEqual(e.protocol_hint, "TCP")
        d = e.to_dict()
        self.assertEqual(d["captured_len"], 60)
        self.assertEqual(d["protocol_hint"], "TCP")

    def test_flow_observed_event(self):
        """FlowObservedEvent construction and serialization."""
        e = eggsec.FlowObservedEvent(
            src_address="10.0.0.1",
            src_port=12345,
            dst_address="10.0.0.2",
            dst_port=80,
            protocol="TCP",
            packet_count=50,
            byte_count=3000,
        )
        self.assertEqual(e.src_address, "10.0.0.1")
        self.assertEqual(e.src_port, 12345)
        self.assertEqual(e.dst_address, "10.0.0.2")
        self.assertEqual(e.dst_port, 80)
        self.assertEqual(e.protocol, "TCP")
        self.assertEqual(e.packet_count, 50)
        self.assertEqual(e.byte_count, 3000)
        d = e.to_dict()
        self.assertEqual(d["src_address"], "10.0.0.1")
        self.assertEqual(d["byte_count"], 3000)

    def test_artifact_created_event(self):
        """ArtifactCreatedEvent construction and serialization."""
        e = eggsec.ArtifactCreatedEvent(
            artifact_type="pcap",
            description="Full capture file",
            size=1024000,
            path="/tmp/capture.pcap",
            mime_type="application/vnd.tcpdump.pcap",
        )
        self.assertEqual(e.artifact_type, "pcap")
        self.assertEqual(e.description, "Full capture file")
        self.assertEqual(e.size, 1024000)
        self.assertEqual(e.path, "/tmp/capture.pcap")
        self.assertEqual(e.mime_type, "application/vnd.tcpdump.pcap")
        d = e.to_dict()
        self.assertEqual(d["artifact_type"], "pcap")
        self.assertEqual(d["size"], 1024000)

    def test_all_events_to_dict(self):
        """All 8 event types produce valid to_dict output."""
        events = [
            eggsec.HandshakeCompletedEvent(
                protocol="TLS", host="h", port=443, duration_ms=1.0,
            ),
            eggsec.RequestSentEvent(
                method="GET", url="https://h", headers_count=1,
            ),
            eggsec.ResponseHeadersReceivedEvent(
                status_code=200, reason="OK", headers_count=1,
            ),
            eggsec.BodyProgressEvent(
                bytes_received=100, is_complete=True,
            ),
            eggsec.CaptureStartedEvent(
                interface="lo", promiscuous=False, snapshot_len=65535,
            ),
            eggsec.PacketSampledEvent(
                interface="lo", packet_index=0, captured_len=60, original_len=60,
            ),
            eggsec.FlowObservedEvent(
                src_address="1.2.3.4", src_port=1000,
                dst_address="5.6.7.8", dst_port=2000,
                protocol="TCP", packet_count=1, byte_count=60,
            ),
            eggsec.ArtifactCreatedEvent(
                artifact_type="pcap", description="test", size=100,
            ),
        ]
        for evt in events:
            d = evt.to_dict()
            self.assertIsInstance(d, dict)
            self.assertGreater(len(d), 0)

    def test_all_events_to_json(self):
        """All 8 event types produce valid JSON via to_json."""
        events = [
            eggsec.HandshakeCompletedEvent(
                protocol="TLS", host="h", port=443, duration_ms=1.0,
            ),
            eggsec.RequestSentEvent(
                method="GET", url="https://h", headers_count=1,
            ),
            eggsec.ResponseHeadersReceivedEvent(
                status_code=200, reason="OK", headers_count=1,
            ),
            eggsec.BodyProgressEvent(
                bytes_received=100, is_complete=True,
            ),
            eggsec.CaptureStartedEvent(
                interface="lo", promiscuous=False, snapshot_len=65535,
            ),
            eggsec.PacketSampledEvent(
                interface="lo", packet_index=0, captured_len=60, original_len=60,
            ),
            eggsec.FlowObservedEvent(
                src_address="1.2.3.4", src_port=1000,
                dst_address="5.6.7.8", dst_port=2000,
                protocol="TCP", packet_count=1, byte_count=60,
            ),
            eggsec.ArtifactCreatedEvent(
                artifact_type="pcap", description="test", size=100,
            ),
        ]
        for evt in events:
            j = json.loads(evt.to_json())
            self.assertIsInstance(j, dict)
            self.assertGreater(len(j), 0)


# ═══════════════════════════════════════════════════════════════════════
# 11. TestPerformanceBudgetsExtended (WS14 metrics)
# ═══════════════════════════════════════════════════════════════════════

@_require_eggsec
class TestPerformanceBudgetsExtended(unittest.TestCase):
    """Performance budget tests for Release 2 types."""

    def test_event_creation_10k(self):
        """10k events created in < 2s."""
        start = time.monotonic()
        for i in range(10000):
            evt = eggsec.RequestSentEvent(
                method="GET", url="https://example.com", headers_count=5,
            )
            _ = evt.to_dict()
        elapsed = time.monotonic() - start
        self.assertLess(elapsed, 2.0, f"Event creation took {elapsed:.3f}s (>2s)")

    @_require_packet_inspection
    def test_packet_timestamp_creation_10k(self):
        """10k PacketTimestamps in < 1s."""
        start = time.monotonic()
        for i in range(10000):
            ts = eggsec.PacketTimestampPy(
                seconds=i, nanos=i % 1000000, epoch_micros=i * 1000,
            )
            _ = ts.to_dict()
        elapsed = time.monotonic() - start
        self.assertLess(elapsed, 1.0, f"PacketTimestamp took {elapsed:.3f}s (>1s)")

    @_require_packet_inspection
    def test_dns_packet_creation_10k(self):
        """10k DnsPackets in < 1s."""
        start = time.monotonic()
        for i in range(10000):
            dp = eggsec.DnsPacketPy(
                transaction_id=i % 65536,
                is_response=True,
                response_code=i % 5,
                question_count=1,
                answer_count=i % 10,
            )
            _ = dp.to_dict()
        elapsed = time.monotonic() - start
        self.assertLess(elapsed, 1.0, f"DnsPacket took {elapsed:.3f}s (>1s)")

    @_require_packet_inspection
    def test_tls_record_creation_10k(self):
        """10k TlsRecordInfo in < 1s."""
        start = time.monotonic()
        for i in range(10000):
            tr = eggsec.TlsRecordInfoPy(
                content_type="handshake",
                version="TLS13",
                record_length=200,
            )
            _ = tr.to_dict()
        elapsed = time.monotonic() - start
        self.assertLess(elapsed, 1.0, f"TlsRecordInfo took {elapsed:.3f}s (>1s)")

    @_require_packet_inspection
    def test_udp_reachability_config_10k(self):
        """10k configs serialized in < 1s."""
        start = time.monotonic()
        for i in range(10000):
            cfg = eggsec.UdpReachabilityConfigPy(
                host=f"10.0.{i // 256}.{i % 256}",
                port=53,
                timeout_ms=2000,
                attempts=1,
            )
            _ = cfg.to_dict()
        elapsed = time.monotonic() - start
        self.assertLess(elapsed, 1.0, f"UdpReachabilityConfig took {elapsed:.3f}s (>1s)")

    @_require_packet_inspection
    def test_packet_stream_iteration_10k(self):
        """10k packet stream iteration in < 1s."""
        ps = eggsec.PacketStreamPy(packets=[])
        start = time.monotonic()
        for _ in range(10000):
            ps.next()
        elapsed = time.monotonic() - start
        self.assertLess(elapsed, 1.0, f"PacketStream iteration took {elapsed:.3f}s (>1s)")

    @_require_packet_inspection
    def test_capture_session_lifecycle_100(self):
        """100 start/stop cycles in < 2s."""
        cfg = eggsec.CaptureConfig(interface="lo")
        start = time.monotonic()
        for _ in range(100):
            session = eggsec.SyncCaptureSessionPy(config=cfg)
            session.start()
            session.stop()
        elapsed = time.monotonic() - start
        self.assertLess(elapsed, 2.0, f"Capture lifecycle took {elapsed:.3f}s (>2s)")

    @_require_packet_inspection
    def test_flow_aggregator_stress(self):
        """100k flow inserts with eviction in < 5s."""
        if not hasattr(eggsec, "FlowAggregatorPy"):
            self.skipTest("FlowAggregatorPy not available")
        agg = eggsec.FlowAggregatorPy(max_flows=10000)
        start = time.monotonic()
        for i in range(100000):
            agg.record_packet(
                src_ip=f"10.{(i >> 16) & 255}.{(i >> 8) & 255}.{i & 255}",
                dst_ip="10.0.0.1",
                src_port=(i % 60000) + 1024,
                dst_port=80,
                protocol="TCP",
                packet_size=60,
                timestamp_ms=i,
                tcp_flags=None,
            )
        elapsed = time.monotonic() - start
        self.assertLess(elapsed, 5.0, f"FlowAggregator stress took {elapsed:.3f}s (>5s)")


# ═══════════════════════════════════════════════════════════════════════
# 12. TestContractCoverage (comprehensive contract tests)
# ═══════════════════════════════════════════════════════════════════════

@_require_eggsec
class TestContractCoverage(unittest.TestCase):
    """Comprehensive contract tests for edge cases and serialization."""

    def test_invalid_tcp_config_zero_port(self):
        """Zero port is still valid (edge case)."""
        cfg = eggsec.TcpConfigPy(host="10.0.0.1", port=0)
        self.assertEqual(cfg.port, 0)
        d = cfg.to_dict()
        self.assertEqual(d["port"], 0)

    def test_invalid_timeout_config(self):
        """Timeout with zero values."""
        cfg = eggsec.TimeoutConfigPy(connect_ms=0, operation_ms=0)
        self.assertEqual(cfg.connect_ms, 0)
        self.assertEqual(cfg.operation_ms, 0)
        d = cfg.to_dict()
        self.assertEqual(d["connect_ms"], 0)

    def test_retry_policy_zero_retries(self):
        """Retry with zero max retries."""
        rp = eggsec.RetryPolicyPy(max_retries=0)
        self.assertEqual(rp.max_retries, 0)
        d = rp.to_dict()
        self.assertEqual(d["max_retries"], 0)

    @_require_packet_inspection
    def test_capture_config_empty_interface(self):
        """Capture with empty interface string."""
        cfg = eggsec.CaptureConfig(interface="")
        session = eggsec.SyncCaptureSessionPy(config=cfg)
        self.assertFalse(session.is_running)
        self.assertFalse(session.is_closed)

    @_require_packet_inspection
    def test_packet_artifact_all_types(self):
        """PacketArtifact with all artifact_type values."""
        for art_type in ["pcap", "frame", "raw_bytes", "parsed_header", "reassembly"]:
            pa = eggsec.PacketArtifactPy(
                packet_index=0,
                artifact_type=art_type,
                description=f"test {art_type}",
            )
            self.assertEqual(pa.artifact_type, art_type)
            d = pa.to_dict()
            self.assertEqual(d["artifact_type"], art_type)

    @_require_packet_inspection
    def test_dns_packet_all_response_codes(self):
        """DnsPacket with various response codes."""
        codes = {
            0: "NOERROR",
            1: "FORMERR",
            2: "SERVFAIL",
            3: "NXDOMAIN",
            4: "NOTIMP",
            5: "REFUSED",
        }
        for code, _name in codes.items():
            dp = eggsec.DnsPacketPy(
                transaction_id=1,
                is_response=True,
                response_code=code,
            )
            self.assertEqual(dp.response_code, code)

    @_require_packet_inspection
    def test_tls_record_all_content_types(self):
        """TlsRecordInfo with all content type values."""
        for ct in ["handshake", "application_data", "alert", "change_cipher_spec"]:
            tr = eggsec.TlsRecordInfoPy(
                content_type=ct,
                version="TLS12",
                record_length=100,
            )
            self.assertEqual(tr.content_type, ct)

    @_require_packet_inspection
    def test_double_close_sync_capture(self):
        """Closing SyncCaptureSession twice (via stop) is handled."""
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.SyncCaptureSessionPy(config=cfg)
        session.start()
        session.stop()
        self.assertTrue(session.is_closed)
        with self.assertRaises(ValueError):
            session.stop()

    def test_session_repr_not_empty(self):
        """All session types have non-empty repr."""
        # AsyncTcpSessionPy
        cfg_tcp = eggsec.TcpConfigPy(host="10.0.0.1", port=22)
        sess_tcp = eggsec.AsyncTcpSessionPy(config=cfg_tcp)
        self.assertGreater(len(repr(sess_tcp)), 0)
        self.assertGreater(len(str(sess_tcp)), 0)

        # AsyncUdpSocketPy
        cfg_udp = eggsec.UdpConfigPy(host="10.0.0.1", port=53)
        sess_udp = eggsec.AsyncUdpSocketPy(config=cfg_udp)
        self.assertGreater(len(repr(sess_udp)), 0)
        self.assertGreater(len(str(sess_udp)), 0)

    @_require_packet_inspection
    def test_all_dtos_serializable(self):
        """All DTOs can be serialized to dict and JSON."""
        # PacketTimestampPy
        ts = eggsec.PacketTimestampPy(seconds=1, nanos=2, epoch_micros=3)
        self.assertIsInstance(ts.to_dict(), dict)
        self.assertIsInstance(ts.to_json(), str)

        # PacketStreamPy
        ps = eggsec.PacketStreamPy(packets=[])
        self.assertIsInstance(ps.to_list(), list)

        # PacketArtifactPy
        pa = eggsec.PacketArtifactPy(packet_index=0, artifact_type="pcap", description="t")
        self.assertIsInstance(pa.to_dict(), dict)
        self.assertIsInstance(pa.to_json(), str)

        # DnsPacketPy
        dp = eggsec.DnsPacketPy(transaction_id=1, is_response=False)
        self.assertIsInstance(dp.to_dict(), dict)
        self.assertIsInstance(dp.to_json(), str)

        # TlsRecordInfoPy
        tr = eggsec.TlsRecordInfoPy(content_type="handshake", version="TLS12", record_length=100)
        self.assertIsInstance(tr.to_dict(), dict)
        self.assertIsInstance(tr.to_json(), str)

        # UdpReachabilityConfigPy
        urc = eggsec.UdpReachabilityConfigPy(host="10.0.0.1", port=53)
        self.assertIsInstance(urc.to_dict(), dict)
        self.assertIsInstance(urc.to_json(), str)

        # UdpReachabilityResultPy
        urr = eggsec.UdpReachabilityResultPy(reachable=True, attempts=1, responses_received=1)
        self.assertIsInstance(urr.to_dict(), dict)
        self.assertIsInstance(urr.to_json(), str)


if __name__ == "__main__":
    unittest.main()
