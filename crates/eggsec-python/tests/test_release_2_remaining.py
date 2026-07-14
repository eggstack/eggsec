"""
Release 2 remaining acceptance criteria tests.

Covers WS7 (capture lifecycle), WS8 (layer DTOs), WS9 (active probes),
WS10 (evidence→finding), WS11 (network events), WS13 (fixtures), WS14 (benchmarks).
"""

import json
import time

import eggsec
import pytest


# ── WS8: Packet layer DTOs ──────────────────────────────────────────

class TestPacketLayerDTOs:
    """Test structured packet layer types."""

    def test_ethernet_frame_construction(self):
        frame = eggsec.EthernetFrame(
            src_mac="00:11:22:33:44:55",
            dst_mac="aa:bb:cc:dd:ee:ff",
            ether_type=0x0800,
            ether_type_name="IPv4",
            vlan_id=None,
            payload_len=1500,
        )
        assert frame.src_mac == "00:11:22:33:44:55"
        assert frame.dst_mac == "aa:bb:cc:dd:ee:ff"
        assert frame.ether_type == 0x0800
        assert frame.ether_type_name == "IPv4"
        assert frame.vlan_id is None
        assert frame.payload_len == 1500

    def test_ethernet_frame_with_vlan(self):
        frame = eggsec.EthernetFrame(
            src_mac="00:11:22:33:44:55",
            dst_mac="aa:bb:cc:dd:ee:ff",
            ether_type=0x8100,
            ether_type_name="VLAN",
            vlan_id=100,
            payload_len=1500,
        )
        assert frame.vlan_id == 100

    def test_ethernet_frame_to_dict(self):
        frame = eggsec.EthernetFrame(
            src_mac="00:11:22:33:44:55",
            dst_mac="aa:bb:cc:dd:ee:ff",
            ether_type=0x0800,
            ether_type_name="IPv4",
            vlan_id=None,
            payload_len=1500,
        )
        d = frame.to_dict()
        assert d["src_mac"] == "00:11:22:33:44:55"
        assert d["ether_type"] == 0x0800

    def test_ipv4_packet_construction(self):
        pkt = eggsec.Ipv4Packet(
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            protocol=6,
            protocol_name="TCP",
            ttl=64,
            tos=0,
            total_length=60,
            fragment_offset=0,
            flags=["SYN"],
            header_checksum=0x1234,
        )
        assert pkt.src_ip == "10.0.0.1"
        assert pkt.dst_ip == "10.0.0.2"
        assert pkt.protocol == 6
        assert pkt.protocol_name == "TCP"
        assert pkt.flags == ["SYN"]

    def test_ipv4_packet_to_json(self):
        pkt = eggsec.Ipv4Packet(
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            protocol=17,
            protocol_name="UDP",
            ttl=128,
            tos=0,
            total_length=40,
            fragment_offset=0,
            flags=[],
            header_checksum=None,
        )
        j = json.loads(pkt.to_json())
        assert j["src_ip"] == "10.0.0.1"
        assert j["protocol_name"] == "UDP"

    def test_ipv6_packet_construction(self):
        pkt = eggsec.Ipv6Packet(
            src_ip="fe80::1",
            dst_ip="fe80::2",
            next_header=6,
            next_header_name="TCP",
            hop_limit=64,
            payload_length=40,
            flow_label=0,
            traffic_class=0,
        )
        assert pkt.src_ip == "fe80::1"
        assert pkt.hop_limit == 64

    def test_tcp_segment_construction(self):
        seg = eggsec.TcpSegment(
            src_port=12345,
            dst_port=80,
            seq_num=1000,
            ack_num=2000,
            data_offset=5,
            flags=["SYN", "ACK"],
            window_size=65535,
            urgent_pointer=0,
            options=["MSS 1460"],
            payload_len=0,
        )
        assert seg.src_port == 12345
        assert seg.dst_port == 80
        assert "SYN" in seg.flags
        assert seg.options == ["MSS 1460"]

    def test_udp_datagram_construction(self):
        dg = eggsec.UdpDatagram(
            src_port=5353,
            dst_port=53,
            length=40,
            checksum=None,
            payload_len=32,
        )
        assert dg.src_port == 5353
        assert dg.dst_port == 53
        assert dg.length == 40

    def test_icmp_packet_construction(self):
        pkt = eggsec.IcmpPacket(
            icmp_type=8,
            icmp_type_name="Echo Request",
            icmp_code=0,
            checksum=None,
            id=1234,
            sequence=1,
            payload_len=64,
        )
        assert pkt.icmp_type == 8
        assert pkt.icmp_type_name == "Echo Request"
        assert pkt.id == 1234
        assert pkt.sequence == 1

    def test_flow_key_construction(self):
        key = eggsec.FlowKey(
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            src_port=12345,
            dst_port=80,
            protocol="TCP",
        )
        assert key.src_ip == "10.0.0.1"
        assert key.protocol == "TCP"

    def test_flow_key_to_dict(self):
        key = eggsec.FlowKey(
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            src_port=12345,
            dst_port=80,
            protocol="TCP",
        )
        d = key.to_dict()
        assert d["src_port"] == 12345
        assert d["dst_port"] == 80


# ── WS8: Flow aggregator ────────────────────────────────────────────

class TestFlowAggregator:
    """Test bounded flow aggregation."""

    def test_construction(self):
        agg = eggsec.FlowAggregator(max_flows=100)
        assert agg.flow_count() == 0
        assert agg.eviction_count() == 0

    def test_record_packet(self):
        agg = eggsec.FlowAggregator(max_flows=100)
        agg.record_packet(
            src_ip="10.0.0.1", dst_ip="10.0.0.2",
            src_port=12345, dst_port=80,
            protocol="TCP", packet_size=60,
            timestamp_ms=1000, tcp_flags=["SYN"],
        )
        assert agg.flow_count() == 1
        assert agg.total_packets() == 1
        assert agg.total_bytes() == 60

    def test_record_multiple_packets_same_flow(self):
        agg = eggsec.FlowAggregator(max_flows=100)
        for i in range(5):
            agg.record_packet(
                src_ip="10.0.0.1", dst_ip="10.0.0.2",
                src_port=12345, dst_port=80,
                protocol="TCP", packet_size=100,
                timestamp_ms=1000 + i,
                tcp_flags=None,
            )
        assert agg.flow_count() == 1
        assert agg.total_packets() == 5
        assert agg.total_bytes() == 500

    def test_eviction(self):
        agg = eggsec.FlowAggregator(max_flows=3)
        for i in range(5):
            agg.record_packet(
                src_ip=f"10.0.0.{i}", dst_ip="10.0.0.2",
                src_port=12345, dst_port=80,
                protocol="TCP", packet_size=60,
                timestamp_ms=1000 + i,
                tcp_flags=None,
            )
        assert agg.flow_count() == 3
        assert agg.eviction_count() == 2

    def test_get_flows(self):
        agg = eggsec.FlowAggregator(max_flows=100)
        agg.record_packet(
            src_ip="10.0.0.1", dst_ip="10.0.0.2",
            src_port=12345, dst_port=80,
            protocol="TCP", packet_size=60,
            timestamp_ms=1000,
            tcp_flags=["SYN"],
        )
        flows = agg.get_flows()
        assert len(flows) == 1
        assert flows[0].src_ip == "10.0.0.1"
        assert "SYN" in flows[0].tcp_flags_seen

    def test_to_dict(self):
        agg = eggsec.FlowAggregator(max_flows=100)
        agg.record_packet(
            src_ip="10.0.0.1", dst_ip="10.0.0.2",
            src_port=80, dst_port=12345,
            protocol="TCP", packet_size=1200,
            timestamp_ms=1000, tcp_flags=None,
        )
        d = agg.to_dict()
        assert d["flow_count"] == 1
        assert d["max_flows"] == 100
        assert d["total_packets"] == 1


# ── WS7: Capture session lifecycle ──────────────────────────────────

class TestCaptureSession:
    """Test managed capture lifecycle."""

    def test_capture_drop_stats(self):
        stats = eggsec.CaptureDropStats(
            dropped_by_policy=5,
            dropped_by_full_queue=10,
            dropped_by_error=2,
            total_dropped=17,
        )
        assert stats.total_dropped == 17
        d = stats.to_dict()
        assert d["dropped_by_full_queue"] == 10

    def test_async_capture_session_construction(self):
        config = eggsec.CaptureConfig(interface="lo")
        session = eggsec.AsyncCaptureSession(config, queue_size=500)
        assert not session.is_running
        assert not session.is_closed
        assert session.interface == "lo"
        assert session.queue_size == 500

    def test_async_capture_session_start_stop(self):
        config = eggsec.CaptureConfig(interface="lo")
        session = eggsec.AsyncCaptureSession(config)
        session.start()
        assert session.is_running
        stats = session.stop()
        assert not session.is_running
        assert session.is_closed
        assert stats.packets_captured == 0

    def test_async_capture_session_context_manager(self):
        config = eggsec.CaptureConfig(interface="lo")
        with eggsec.AsyncCaptureSession(config) as session:
            session.start()
            assert session.is_running
        assert session.is_closed

    def test_captured_packet_construction(self):
        info = eggsec.PacketInfo(
            timestamp="2026-01-01T00:00:00Z",
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            protocol="TCP",
            src_port=12345,
            dst_port=80,
            size=60,
            summary="TCP SYN 12345->80",
        )
        pkt = eggsec.CapturedPacket(
            sequence=1,
            timestamp_ms=1000,
            captured_len=60,
            original_len=60,
            info=info,
            raw_bytes=bytes(60),
        )
        assert pkt.sequence == 1
        assert pkt.captured_len == 60
        assert len(pkt.raw_bytes()) == 60


# ── WS9: Active probes ──────────────────────────────────────────────

class TestActiveProbes:
    """Test ICMP echo and TCP SYN probe types."""

    def test_icmp_probe_config(self):
        config = eggsec.IcmpProbeConfig(
            target="127.0.0.1",
            count=3,
            timeout_ms=2000,
            packet_size=32,
            ttl=32,
        )
        assert config.target == "127.0.0.1"
        assert config.count == 3
        assert config.timeout_ms == 2000

    def test_icmp_probe_result(self):
        reply = eggsec.IcmpProbeReply(seq=0, rtt_ms=1.5, ttl=64, bytes=64)
        result = eggsec.IcmpProbeResult(
            target="127.0.0.1",
            resolved_address="127.0.0.1",
            reachable=True,
            replies=[reply],
            packets_sent=1,
            packets_received=1,
            min_rtt_ms=1.5,
            max_rtt_ms=1.5,
            avg_rtt_ms=1.5,
            packet_loss_pct=0.0,
            error=None,
        )
        assert result.reachable
        assert result.packet_loss_pct == 0.0
        assert len(result.replies) == 1

    def test_tcp_probe_config(self):
        config = eggsec.TcpProbeConfig(
            target="127.0.0.1",
            port=80,
            timeout_ms=3000,
        )
        assert config.target == "127.0.0.1"
        assert config.port == 80

    def test_tcp_probe_result(self):
        result = eggsec.TcpProbeResult(
            target="127.0.0.1",
            port=80,
            state="open",
            rtt_ms=1.2,
            ttl=64,
            window_size=65535,
            error=None,
        )
        assert result.state == "open"
        assert result.rtt_ms == 1.2

    def test_tcp_probe_result_closed(self):
        result = eggsec.TcpProbeResult(
            target="127.0.0.1",
            port=1,
            state="closed",
            rtt_ms=0.5,
            ttl=None,
            window_size=None,
            error=None,
        )
        assert result.state == "closed"


# ── WS10: Evidence → Finding conversion ─────────────────────────────

class TestEvidenceToFinding:
    """Test evidence-to-finding conversion."""

    def test_conversion(self):
        evidence = eggsec.NetworkEvidencePy(
            operation="tls_probe",
            target="example.com",
            timing=eggsec.ConnectionTimingPy(
                dns_resolution_ms=10.0,
                tcp_connect_ms=20.0,
                tls_handshake_ms=30.0,
                first_byte_ms=40.0,
                total_ms=50.0,
                connection_reused=False,
            ),
        )
        finding = eggsec.evidence_to_finding(
            evidence,
            title="TLS issue detected",
            description="Server supports weak cipher",
            severity="medium",
        )
        assert finding.title == "TLS issue detected"
        assert finding.severity == "medium"
        assert finding.source_tool == "eggsec"
        assert finding.source_module == "network"
        assert len(finding.evidence) == 1
        assert finding.affected_asset.identifier == "example.com"

    def test_conversion_with_all_severities(self):
        evidence = eggsec.NetworkEvidencePy(operation="dns_query", target="example.com")
        for sev in ["critical", "high", "medium", "low", "info"]:
            finding = eggsec.evidence_to_finding(evidence, "test", "desc", sev)
            assert finding.severity == sev


# ── WS11: Network events ───────────────────────────────────────────

class TestNetworkEvents:
    """Test network-specific event types."""

    def test_resolution_event(self):
        evt = eggsec.ResolutionEvent(
            target="example.com",
            status="completed",
            resolved_address="93.184.216.34",
            resolution_time_ms=15.5,
        )
        assert evt.target == "example.com"
        assert evt.status == "completed"
        d = evt.to_dict()
        assert d["resolved_address"] == "93.184.216.34"

    def test_connection_event(self):
        evt = eggsec.ConnectionEvent(
            target="example.com",
            port=443,
            status="connected",
            rtt_ms=25.0,
        )
        assert evt.port == 443
        assert evt.rtt_ms == 25.0

    def test_probe_event(self):
        evt = eggsec.ProbeEvent(
            probe_type="tcp_syn",
            target="10.0.0.1:80",
            success=True,
            rtt_ms=5.0,
        )
        assert evt.probe_type == "tcp_syn"
        assert evt.success

    def test_websocket_message_event(self):
        evt = eggsec.WebSocketMessageEvent(
            url="ws://example.com/ws",
            direction="recv",
            message_type="text",
            size=1024,
        )
        assert evt.direction == "recv"
        assert evt.size == 1024

    def test_capture_stats_event(self):
        evt = eggsec.CaptureStatsEvent(
            interface="eth0",
            packets_captured=1000,
            packets_dropped=5,
            bytes_captured=64000,
            runtime_ms=10000,
        )
        assert evt.packets_captured == 1000
        assert evt.packets_dropped == 5


# ── WS14: Performance and resource budgets ──────────────────────────

class TestPerformanceBudgets:
    """Test that performance budgets are met."""

    def test_flow_aggregator_throughput(self):
        agg = eggsec.FlowAggregator(max_flows=10000)
        start = time.monotonic()
        for i in range(10000):
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
        # 10k flow inserts should complete in < 1 second
        assert elapsed < 1.0, f"Flow aggregation took {elapsed:.3f}s (>1s budget)"
        assert agg.flow_count() == 10000

    def test_flow_aggregator_memory_bound(self):
        agg = eggsec.FlowAggregator(max_flows=100)
        for i in range(1000):
            agg.record_packet(
                src_ip=f"10.0.{i // 256}.{i % 256}",
                dst_ip="10.0.0.1",
                src_port=12345,
                dst_port=80,
                protocol="TCP",
                packet_size=60,
                timestamp_ms=i,
                tcp_flags=None,
            )
        # Should be bounded at 100, not 1000
        assert agg.flow_count() <= 100
        assert agg.eviction_count() > 0

    def test_layer_dto_serialization_throughput(self):
        start = time.monotonic()
        for _ in range(10000):
            frame = eggsec.EthernetFrame(
                src_mac="00:11:22:33:44:55",
                dst_mac="aa:bb:cc:dd:ee:ff",
                ether_type=0x0800,
                ether_type_name="IPv4",
                vlan_id=None,
                payload_len=1500,
            )
            _ = frame.to_dict()
            _ = frame.to_json()
        elapsed = time.monotonic() - start
        # 10k serializations should complete in < 2 seconds
        assert elapsed < 2.0, f"DTO serialization took {elapsed:.3f}s (>2s budget)"

    def test_event_creation_throughput(self):
        start = time.monotonic()
        for i in range(10000):
            evt = eggsec.ConnectionEvent(
                target="example.com",
                port=443,
                status="connected",
                rtt_ms=10.0,
            )
            _ = evt.to_dict()
        elapsed = time.monotonic() - start
        # 10k event creations should complete in < 1 second
        assert elapsed < 1.0, f"Event creation took {elapsed:.3f}s (>1s budget)"
