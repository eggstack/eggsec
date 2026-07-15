"""Workstream 9: Packet parser, capture, and flow validation tests.

Tests construction, serialization, repr, edge cases, and lifecycle for
packet-inspection DTOs. All tests are feature-gated behind packet-inspection.
"""

import json
import pytest

import eggsec

# Feature gate — skip all tests if packet-inspection not compiled
_has_packet = hasattr(eggsec, "EthernetFrame")
pytestmark = pytest.mark.skipif(not _has_packet, reason="packet-inspection not compiled")


# ────────────────────────────────────────────────────────────────────
# Layer A: Pure DTO Construction
# ────────────────────────────────────────────────────────────────────


class TestEthernetFrameConstruction:
    def test_basic(self):
        frame = eggsec.EthernetFrame(
            src_mac="00:11:22:33:44:55",
            dst_mac="66:77:88:99:aa:bb",
            ether_type=0x0800,
            ether_type_name="IPv4",
        )
        assert frame.src_mac == "00:11:22:33:44:55"
        assert frame.dst_mac == "66:77:88:99:aa:bb"
        assert frame.ether_type == 0x0800
        assert frame.ether_type_name == "IPv4"
        assert frame.vlan_id is None
        assert frame.payload_len == 0

    def test_with_vlan(self):
        frame = eggsec.EthernetFrame(
            src_mac="aa:bb:cc:dd:ee:ff",
            dst_mac="11:22:33:44:55:66",
            ether_type=0x8100,
            ether_type_name="802.1Q",
            vlan_id=100,
            payload_len=1500,
        )
        assert frame.vlan_id == 100
        assert frame.payload_len == 1500

    def test_fields_frozen(self):
        frame = eggsec.EthernetFrame(
            src_mac="00:00:00:00:00:00",
            dst_mac="ff:ff:ff:ff:ff:ff",
            ether_type=0x0806,
            ether_type_name="ARP",
        )
        with pytest.raises(AttributeError):
            frame.src_mac = "xx"


class TestIpv4PacketConstruction:
    def test_basic(self):
        pkt = eggsec.Ipv4Packet(
            src_ip="192.168.1.1",
            dst_ip="10.0.0.1",
            protocol=6,
            protocol_name="TCP",
            ttl=64,
            tos=0,
            total_length=60,
            fragment_offset=0,
            flags=["DF"],
            header_checksum=None,
        )
        assert pkt.src_ip == "192.168.1.1"
        assert pkt.dst_ip == "10.0.0.1"
        assert pkt.protocol == 6
        assert pkt.protocol_name == "TCP"
        assert pkt.ttl == 64
        assert pkt.total_length == 60
        assert pkt.flags == ["DF"]
        assert pkt.header_checksum is None

    def test_with_checksum(self):
        pkt = eggsec.Ipv4Packet(
            src_ip="127.0.0.1",
            dst_ip="127.0.0.1",
            protocol=17,
            protocol_name="UDP",
            ttl=128,
            tos=0,
            total_length=40,
            fragment_offset=0,
            flags=[],
            header_checksum=0xABCD,
        )
        assert pkt.header_checksum == 0xABCD


class TestIpv6PacketConstruction:
    def test_basic(self):
        pkt = eggsec.Ipv6Packet(
            src_ip="fe80::1",
            dst_ip="fe80::2",
            next_header=59,
            next_header_name="No Next Header",
            hop_limit=64,
            payload_length=1280,
            flow_label=0,
            traffic_class=0,
        )
        assert pkt.src_ip == "fe80::1"
        assert pkt.dst_ip == "fe80::2"
        assert pkt.next_header == 59
        assert pkt.hop_limit == 64
        assert pkt.payload_length == 1280


class TestTcpSegmentConstruction:
    def test_syn(self):
        seg = eggsec.TcpSegment(
            src_port=54321,
            dst_port=443,
            seq_num=1000,
            ack_num=0,
            data_offset=5,
            flags=["SYN"],
            window_size=65535,
            urgent_pointer=0,
            options=["MSS:1460"],
            payload_len=0,
        )
        assert seg.src_port == 54321
        assert seg.dst_port == 443
        assert seg.seq_num == 1000
        assert seg.ack_num == 0
        assert seg.flags == ["SYN"]
        assert seg.window_size == 65535
        assert seg.options == ["MSS:1460"]

    def test_syn_ack(self):
        seg = eggsec.TcpSegment(
            src_port=80,
            dst_port=54321,
            seq_num=2000,
            ack_num=1001,
            data_offset=5,
            flags=["SYN", "ACK"],
            window_size=28960,
            urgent_pointer=0,
            options=[],
        )
        assert "SYN" in seg.flags
        assert "ACK" in seg.flags


class TestUdpDatagramConstruction:
    def test_basic(self):
        dg = eggsec.UdpDatagram(
            src_port=1234,
            dst_port=53,
            length=32,
            checksum=None,
            payload_len=12,
        )
        assert dg.src_port == 1234
        assert dg.dst_port == 53
        assert dg.length == 32
        assert dg.checksum is None
        assert dg.payload_len == 12

    def test_with_checksum(self):
        dg = eggsec.UdpDatagram(
            src_port=53,
            dst_port=1234,
            length=52,
            checksum=0x1234,
            payload_len=32,
        )
        assert dg.checksum == 0x1234


class TestIcmpPacketConstruction:
    def test_echo_request(self):
        pkt = eggsec.IcmpPacket(
            icmp_type=8,
            icmp_type_name="Echo Request",
            icmp_code=0,
            checksum=0,
            id=0x1234,
            sequence=1,
            payload_len=48,
        )
        assert pkt.icmp_type == 8
        assert pkt.icmp_type_name == "Echo Request"
        assert pkt.id == 0x1234
        assert pkt.sequence == 1

    def test_echo_reply_no_id(self):
        pkt = eggsec.IcmpPacket(
            icmp_type=0,
            icmp_type_name="Echo Reply",
            icmp_code=0,
        )
        assert pkt.id is None
        assert pkt.sequence is None


class TestDnsPacketConstruction:
    def test_query(self):
        dns = eggsec.DnsPacketPy(
            transaction_id=0x1234,
            is_response=False,
            op_code=0,
            recursion_desired=True,
            question_count=1,
            answer_count=0,
        )
        assert dns.transaction_id == 0x1234
        assert dns.is_response is False
        assert dns.recursion_desired is True
        assert dns.question_count == 1
        assert dns.answer_count == 0
        assert dns.response_code == 0

    def test_response(self):
        dns = eggsec.DnsPacketPy(
            transaction_id=0x1234,
            is_response=True,
            authoritative=True,
            response_code=0,
            question_count=1,
            answer_count=2,
        )
        assert dns.is_response is True
        assert dns.authoritative is True
        assert dns.answer_count == 2


class TestTlsRecordInfoConstruction:
    def test_client_hello(self):
        tls = eggsec.TlsRecordInfoPy(
            content_type="Handshake",
            version="TLS 1.3",
            record_length=256,
            handshake_type="ClientHello",
            cipher_suites=["TLS_AES_256_GCM_SHA384", "TLS_CHACHA20_POLY1305_SHA256"],
            extensions=["server_name", "supported_versions"],
            sni="example.com",
            alpn_protocols=["h2", "http/1.1"],
        )
        assert tls.content_type == "Handshake"
        assert tls.version == "TLS 1.3"
        assert tls.sni == "example.com"
        assert len(tls.cipher_suites) == 2
        assert "h2" in tls.alpn_protocols

    def test_no_optional_fields(self):
        tls = eggsec.TlsRecordInfoPy(
            content_type="ApplicationData",
            version="TLS 1.2",
            record_length=128,
        )
        assert tls.handshake_type is None
        assert tls.sni is None
        assert tls.cipher_suites == []
        assert tls.extensions == []


class TestPacketTimestampConstruction:
    def test_basic(self):
        ts = eggsec.PacketTimestampPy(
            seconds=1700000000,
            nanos=500000000,
            epoch_micros=1700000000500000,
        )
        assert ts.seconds == 1700000000
        assert ts.nanos == 500000000
        assert ts.epoch_micros == 1700000000500000

    def test_zero(self):
        ts = eggsec.PacketTimestampPy(seconds=0, nanos=0, epoch_micros=0)
        assert ts.seconds == 0
        assert ts.nanos == 0


class TestPacketArtifactConstruction:
    def test_basic(self):
        art = eggsec.PacketArtifactPy(
            packet_index=42,
            artifact_type="pcap",
            description="capture of DNS exchange",
            file_path="/tmp/capture.pcap",
            byte_offset=1024,
        )
        assert art.packet_index == 42
        assert art.artifact_type == "pcap"
        assert art.file_path == "/tmp/capture.pcap"
        assert art.byte_offset == 1024

    def test_minimal(self):
        art = eggsec.PacketArtifactPy(
            packet_index=0,
            artifact_type="raw",
        )
        assert art.file_path is None
        assert art.byte_offset is None
        assert art.description == ""


class TestFlowKeyConstruction:
    def test_basic(self):
        fk = eggsec.FlowKey(
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            src_port=12345,
            dst_port=80,
            protocol="TCP",
        )
        assert fk.src_ip == "10.0.0.1"
        assert fk.dst_ip == "10.0.0.2"
        assert fk.src_port == 12345
        assert fk.dst_port == 80
        assert fk.protocol == "TCP"


class TestFlowRecordConstruction:
    def test_readonly_attributes(self):
        fr = eggsec.FlowRecord
        assert hasattr(fr, "src_ip")
        assert hasattr(fr, "dst_ip")
        assert hasattr(fr, "packet_count")


# ────────────────────────────────────────────────────────────────────
# Layer A: Serialization
# ────────────────────────────────────────────────────────────────────

ALL_PACKET_TYPES = [
    (
        "EthernetFrame",
        lambda: eggsec.EthernetFrame(
            src_mac="00:11:22:33:44:55",
            dst_mac="66:77:88:99:aa:bb",
            ether_type=0x0800,
            ether_type_name="IPv4",
        ),
    ),
    (
        "Ipv4Packet",
        lambda: eggsec.Ipv4Packet(
            src_ip="127.0.0.1",
            dst_ip="127.0.0.1",
            protocol=6,
            protocol_name="TCP",
            ttl=64,
            tos=0,
            total_length=60,
            fragment_offset=0,
            flags=[],
        ),
    ),
    (
        "Ipv6Packet",
        lambda: eggsec.Ipv6Packet(
            src_ip="::1",
            dst_ip="::1",
            next_header=6,
            next_header_name="TCP",
            hop_limit=64,
            payload_length=60,
            flow_label=0,
            traffic_class=0,
        ),
    ),
    (
        "TcpSegment",
        lambda: eggsec.TcpSegment(
            src_port=12345,
            dst_port=80,
            seq_num=1000,
            ack_num=0,
            data_offset=5,
            flags=["SYN"],
            window_size=65535,
            urgent_pointer=0,
            options=[],
        ),
    ),
    (
        "UdpDatagram",
        lambda: eggsec.UdpDatagram(
            src_port=1234,
            dst_port=53,
            length=32,
        ),
    ),
    (
        "IcmpPacket",
        lambda: eggsec.IcmpPacket(
            icmp_type=8,
            icmp_type_name="Echo Request",
            icmp_code=0,
        ),
    ),
    (
        "DnsPacketPy",
        lambda: eggsec.DnsPacketPy(
            transaction_id=0x1234,
            is_response=False,
        ),
    ),
    (
        "TlsRecordInfoPy",
        lambda: eggsec.TlsRecordInfoPy(
            content_type="Handshake",
            version="TLS 1.3",
            record_length=256,
        ),
    ),
    (
        "PacketTimestampPy",
        lambda: eggsec.PacketTimestampPy(
            seconds=1700000000,
            nanos=500000000,
            epoch_micros=1700000000500000,
        ),
    ),
    (
        "PacketArtifactPy",
        lambda: eggsec.PacketArtifactPy(
            packet_index=0,
            artifact_type="pcap",
        ),
    ),
    (
        "FlowKey",
        lambda: eggsec.FlowKey(
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            src_port=80,
            dst_port=443,
            protocol="TCP",
        ),
    ),
    (
        "CaptureConfig",
        lambda: eggsec.CaptureConfig(interface="lo"),
    ),
    (
        "PacketInfo",
        lambda: eggsec.PacketInfo(timestamp="2024-01-01T00:00:00Z"),
    ),
    (
        "PacketFilter",
        lambda: eggsec.PacketFilter(protocol="tcp"),
    ),
    (
        "CaptureDropStats",
        lambda: eggsec.CaptureDropStats(
            dropped_by_policy=0,
            dropped_by_full_queue=0,
            dropped_by_error=0,
            total_dropped=0,
        ),
    ),
]


@pytest.fixture(params=ALL_PACKET_TYPES, ids=[t[0] for t in ALL_PACKET_TYPES])
def packet_obj(request):
    return request.param[1]()


class TestAllPacketTypesToDict:
    def test_to_dict_returns_dict(self, packet_obj):
        d = packet_obj.to_dict()
        assert isinstance(d, dict)

    def test_to_dict_non_empty(self, packet_obj):
        d = packet_obj.to_dict()
        assert len(d) > 0


class TestAllPacketTypesToJson:
    def test_to_json_returns_string(self, packet_obj):
        j = packet_obj.to_json()
        assert isinstance(j, str)

    def test_to_json_valid_json(self, packet_obj):
        j = packet_obj.to_json()
        parsed = json.loads(j)
        assert isinstance(parsed, dict)


class TestAllPacketTypesRepr:
    def test_repr_returns_string(self, packet_obj):
        r = repr(packet_obj)
        assert isinstance(r, str)
        assert len(r) > 0

    def test_repr_contains_class_name(self, packet_obj):
        r = repr(packet_obj)
        assert "(" in r


class TestAllPacketTypesToDictRoundtrip:
    def test_dict_keys_match(self, packet_obj):
        d = packet_obj.to_dict()
        for key in d:
            assert key, f"Empty key in dict: {d}"


# ────────────────────────────────────────────────────────────────────
# Layer A: Edge Cases
# ────────────────────────────────────────────────────────────────────


class TestEdgeCases:
    def test_empty_mac_address(self):
        frame = eggsec.EthernetFrame(
            src_mac="",
            dst_mac="",
            ether_type=0x0800,
            ether_type_name="IPv4",
        )
        assert frame.src_mac == ""
        assert frame.dst_mac == ""

    def test_zero_port(self):
        seg = eggsec.TcpSegment(
            src_port=0,
            dst_port=0,
            seq_num=0,
            ack_num=0,
            data_offset=5,
            flags=[],
            window_size=0,
            urgent_pointer=0,
            options=[],
        )
        assert seg.src_port == 0
        assert seg.dst_port == 0

    def test_max_port(self):
        seg = eggsec.TcpSegment(
            src_port=65535,
            dst_port=65535,
            seq_num=0xFFFFFFFF,
            ack_num=0xFFFFFFFF,
            data_offset=15,
            flags=[],
            window_size=0xFFFF,
            urgent_pointer=0,
            options=[],
        )
        assert seg.src_port == 65535
        assert seg.dst_port == 65535
        assert seg.seq_num == 0xFFFFFFFF

    def test_max_ttl(self):
        pkt = eggsec.Ipv4Packet(
            src_ip="0.0.0.0",
            dst_ip="255.255.255.255",
            protocol=0,
            protocol_name="HOPOPT",
            ttl=255,
            tos=255,
            total_length=0xFFFF,
            fragment_offset=0x1FFF,
            flags=[],
            header_checksum=0xFFFF,
        )
        assert pkt.ttl == 255
        assert pkt.tos == 255
        assert pkt.total_length == 0xFFFF

    def test_empty_flags_list(self):
        pkt = eggsec.Ipv4Packet(
            src_ip="0.0.0.0",
            dst_ip="0.0.0.0",
            protocol=6,
            protocol_name="TCP",
            ttl=64,
            tos=0,
            total_length=40,
            fragment_offset=0,
            flags=[],
        )
        assert pkt.flags == []

    def test_many_flags(self):
        pkt = eggsec.Ipv4Packet(
            src_ip="0.0.0.0",
            dst_ip="0.0.0.0",
            protocol=6,
            protocol_name="TCP",
            ttl=64,
            tos=0,
            total_length=40,
            fragment_offset=0,
            flags=["DF", "MF"],
        )
        assert len(pkt.flags) == 2

    def test_long_mac_address(self):
        frame = eggsec.EthernetFrame(
            src_mac="aa:bb:cc:dd:ee:ff:00:11:22:33",
            dst_mac="ff:ff:ff:ff:ff:ff:ff:ff:ff:ff",
            ether_type=0x0800,
            ether_type_name="IPv4",
        )
        assert len(frame.src_mac) > 17

    def test_capture_config_defaults(self):
        cfg = eggsec.CaptureConfig()
        assert cfg.interface == ""
        assert cfg.filter is None
        assert cfg.promiscuous is True
        assert cfg.snapshot_len == 65535
        assert cfg.validate_checksums is False

    def test_capture_config_full(self):
        cfg = eggsec.CaptureConfig(
            interface="eth0",
            filter="tcp port 80",
            promiscuous=False,
            snapshot_len=96,
            timeout_secs=5,
            max_packets=1000,
            save_to_file="/tmp/cap.pcap",
            validate_checksums=True,
        )
        assert cfg.interface == "eth0"
        assert cfg.filter == "tcp port 80"
        assert cfg.max_packets == 1000


# ────────────────────────────────────────────────────────────────────
# Layer B: Synthetic Stream
# ────────────────────────────────────────────────────────────────────


class TestPacketStream:
    def test_creation(self):
        info = eggsec.PacketInfo(timestamp="2024-01-01T00:00:00Z", protocol="TCP")
        pkt = eggsec.CapturedPacket(
            sequence=1,
            timestamp_ms=1700000000000,
            captured_len=64,
            original_len=64,
            info=info,
            raw_bytes=b"\x00" * 64,
        )
        stream = eggsec.PacketStreamPy([pkt])
        assert stream.len() == 1
        assert not stream.is_empty()

    def test_iteration(self):
        info = eggsec.PacketInfo(timestamp="2024-01-01T00:00:00Z")
        pkts = [
            eggsec.CapturedPacket(
                sequence=i,
                timestamp_ms=1700000000000 + i,
                captured_len=64,
                original_len=64,
                info=info,
                raw_bytes=b"\x00" * 64,
            )
            for i in range(5)
        ]
        stream = eggsec.PacketStreamPy(pkts)
        collected = list(stream)
        assert len(collected) == 5

    def test_next_returns_none_at_end(self):
        stream = eggsec.PacketStreamPy([])
        assert stream.next() is None

    def test_next_returns_packets(self):
        info = eggsec.PacketInfo(timestamp="2024-01-01T00:00:00Z")
        pkt = eggsec.CapturedPacket(
            sequence=0,
            timestamp_ms=0,
            captured_len=10,
            original_len=10,
            info=info,
            raw_bytes=b"\x00" * 10,
        )
        stream = eggsec.PacketStreamPy([pkt])
        result = stream.next()
        assert result is not None
        assert result.sequence == 0
        assert stream.next() is None

    def test_to_list(self):
        info = eggsec.PacketInfo(timestamp="2024-01-01T00:00:00Z")
        pkts = [
            eggsec.CapturedPacket(
                sequence=i,
                timestamp_ms=0,
                captured_len=10,
                original_len=10,
                info=info,
                raw_bytes=b"\x00" * 10,
            )
            for i in range(3)
        ]
        stream = eggsec.PacketStreamPy(pkts)
        as_list = stream.to_list()
        assert len(as_list) == 3


class TestFlowAggregator:
    def test_creation(self):
        agg = eggsec.FlowAggregator()
        assert agg.flow_count() == 0
        assert agg.total_packets() == 0

    def test_record_packet(self):
        agg = eggsec.FlowAggregator()
        agg.record_packet(
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            src_port=1234,
            dst_port=80,
            protocol="TCP",
            packet_size=100,
            timestamp_ms=1000,
        )
        assert agg.flow_count() == 1
        assert agg.total_packets() == 1
        assert agg.total_bytes() == 100

    def test_record_multiple_same_flow(self):
        agg = eggsec.FlowAggregator()
        for i in range(5):
            agg.record_packet(
                src_ip="10.0.0.1",
                dst_ip="10.0.0.2",
                src_port=1234,
                dst_port=80,
                protocol="TCP",
                packet_size=100,
                timestamp_ms=1000 + i,
            )
        assert agg.flow_count() == 1
        assert agg.total_packets() == 5
        assert agg.total_bytes() == 500

    def test_record_different_flows(self):
        agg = eggsec.FlowAggregator()
        agg.record_packet(
            src_ip="10.0.0.1", dst_ip="10.0.0.2",
            src_port=1234, dst_port=80, protocol="TCP",
            packet_size=100, timestamp_ms=1000,
        )
        agg.record_packet(
            src_ip="10.0.0.1", dst_ip="10.0.0.3",
            src_port=5678, dst_port=443, protocol="TCP",
            packet_size=200, timestamp_ms=2000,
        )
        assert agg.flow_count() == 2

    def test_eviction(self):
        agg = eggsec.FlowAggregator(max_flows=2)
        for i in range(3):
            agg.record_packet(
                src_ip=f"10.0.0.{i}",
                dst_ip="10.0.0.100",
                src_port=1000 + i,
                dst_port=80,
                protocol="TCP",
                packet_size=64,
                timestamp_ms=i * 1000,
            )
        assert agg.flow_count() == 2
        assert agg.eviction_count() >= 1

    def test_tcp_flags_tracking(self):
        agg = eggsec.FlowAggregator()
        agg.record_packet(
            src_ip="10.0.0.1", dst_ip="10.0.0.2",
            src_port=1234, dst_port=80, protocol="TCP",
            packet_size=64, timestamp_ms=1000,
            tcp_flags=["SYN"],
        )
        agg.record_packet(
            src_ip="10.0.0.1", dst_ip="10.0.0.2",
            src_port=1234, dst_port=80, protocol="TCP",
            packet_size=64, timestamp_ms=2000,
            tcp_flags=["ACK"],
        )
        flows = agg.get_flows()
        assert len(flows) == 1
        assert "SYN" in flows[0].tcp_flags_seen
        assert "ACK" in flows[0].tcp_flags_seen


class TestCapturedPacketRawBytes:
    def test_raw_bytes_accessible(self):
        info = eggsec.PacketInfo(timestamp="2024-01-01T00:00:00Z")
        raw = b"\x00\x11\x22\x33\x44\x55" + b"\x00" * 58
        pkt = eggsec.CapturedPacket(
            sequence=0,
            timestamp_ms=0,
            captured_len=64,
            original_len=64,
            info=info,
            raw_bytes=raw,
        )
        rb = pkt.raw_bytes()
        assert len(rb) == 64
        assert rb[:6] == [0, 17, 34, 51, 68, 85]


# ────────────────────────────────────────────────────────────────────
# Layer C/D: Capture Lifecycle (xfail — requires platform support)
# ────────────────────────────────────────────────────────────────────


class TestCaptureSessionLifecycle:
    def test_capture_session_loopback(self):
        pytest.skip("Requires root and live capture interface")

    def test_sync_capture_session_construction(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.SyncCaptureSessionPy(cfg)
        assert not session.is_running
        assert not session.is_closed

    def test_sync_capture_session_start_stop(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.SyncCaptureSessionPy(cfg)
        session.start()
        assert session.is_running
        stats = session.stop()
        assert not session.is_running
        assert session.is_closed
        assert isinstance(stats, eggsec.CaptureStats)

    def test_sync_capture_double_start_errors(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.SyncCaptureSessionPy(cfg)
        session.start()
        with pytest.raises(ValueError, match="already running"):
            session.start()
        session.stop()

    def test_sync_capture_stop_when_not_running_errors(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.SyncCaptureSessionPy(cfg)
        with pytest.raises(ValueError, match="not running"):
            session.stop()

    def test_sync_capture_start_after_close_errors(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.SyncCaptureSessionPy(cfg)
        session.start()
        session.stop()
        with pytest.raises(ValueError, match="closed"):
            session.start()

    def test_sync_capture_context_manager(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        with eggsec.SyncCaptureSessionPy(cfg) as session:
            session.start()
            assert session.is_running
        assert session.is_closed

    def test_async_capture_session_construction(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.AsyncCaptureSession(cfg)
        assert not session.is_running
        assert not session.is_closed
        assert session.interface == "lo"

    def test_async_capture_session_start_stop(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.AsyncCaptureSession(cfg)
        session.start()
        assert session.is_running
        stats = session.stop()
        assert not session.is_running
        assert session.is_closed
        assert isinstance(stats, eggsec.CaptureStats)

    def test_async_capture_context_manager(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        with eggsec.AsyncCaptureSession(cfg) as session:
            session.start()
            assert session.is_running
        assert session.is_closed


# ────────────────────────────────────────────────────────────────────
# Layer A: BackpressurePolicy enum
# ────────────────────────────────────────────────────────────────────


class TestBackpressurePolicy:
    def test_all_variants(self):
        for name in ("Block", "DropOldest", "DropNewest", "ArtifactOnly"):
            policy = getattr(eggsec.BackpressurePolicy, name)
            assert repr(policy).startswith("BackpressurePolicy.")

    def test_to_dict(self):
        d = eggsec.BackpressurePolicy.DropOldest.to_dict()
        assert isinstance(d, dict)
        assert "policy" in d

    def test_to_json(self):
        j = eggsec.BackpressurePolicy.Block.to_json()
        assert isinstance(j, str)
        assert len(j) > 0


# ────────────────────────────────────────────────────────────────────
# Layer A: LiveCaptureResult / NetworkInterfaceInfo
# ────────────────────────────────────────────────────────────────────


class TestLiveCaptureResultConstruction:
    def test_class_exists(self):
        assert hasattr(eggsec.LiveCaptureResult, "interface")
        assert hasattr(eggsec.LiveCaptureResult, "packets_captured")


class TestNetworkInterfaceInfo:
    def test_class_exists(self):
        assert hasattr(eggsec.NetworkInterfaceInfo, "name")
        assert hasattr(eggsec.NetworkInterfaceInfo, "ips")


# ────────────────────────────────────────────────────────────────────
# Layer A: TracerouteConfig / TracerouteHop / TracerouteResult
# ────────────────────────────────────────────────────────────────────


class TestTracerouteConfig:
    def test_basic(self):
        cfg = eggsec.TracerouteConfig(target="8.8.8.8")
        assert cfg.target == "8.8.8.8"
        assert cfg.max_hops == 30
        assert cfg.timeout_secs == 3
        assert cfg.use_icmp is False

    def test_full(self):
        cfg = eggsec.TracerouteConfig(
            target="example.com",
            max_hops=15,
            timeout_secs=5,
            max_retries=1,
            first_ttl=3,
            port=33434,
            use_icmp=True,
            packet_size=128,
            resolve_names=False,
        )
        assert cfg.max_hops == 15
        assert cfg.use_icmp is True
        assert cfg.resolve_names is False


class TestTracerouteHop:
    def test_class_exists(self):
        assert hasattr(eggsec.TracerouteHop, "hop")
        assert hasattr(eggsec.TracerouteHop, "rtt_ms")
        assert hasattr(eggsec.TracerouteHop, "is_final")


class TestTracerouteResult:
    def test_class_exists(self):
        assert hasattr(eggsec.TracerouteResult, "target")
        assert hasattr(eggsec.TracerouteResult, "success")


# ────────────────────────────────────────────────────────────────────
# Layer A: Probe types
# ────────────────────────────────────────────────────────────────────


class TestIcmpProbeTypes:
    def test_config(self):
        cfg = eggsec.IcmpProbeConfig(target="127.0.0.1")
        assert cfg.target == "127.0.0.1"
        assert cfg.count == 4
        assert cfg.timeout_ms == 5000

    def test_reply(self):
        reply = eggsec.IcmpProbeReply(seq=0, rtt_ms=1.5, ttl=64, bytes=64)
        assert reply.seq == 0
        assert reply.rtt_ms == 1.5

    def test_result_construction(self):
        result = eggsec.IcmpProbeResult(
            target="127.0.0.1",
            reachable=True,
            replies=[],
            packets_sent=4,
            packets_received=4,
            packet_loss_pct=0.0,
        )
        assert result.reachable is True
        assert result.packet_loss_pct == 0.0


class TestTcpProbeTypes:
    def test_config(self):
        cfg = eggsec.TcpProbeConfig(target="127.0.0.1", port=80)
        assert cfg.target == "127.0.0.1"
        assert cfg.port == 80

    def test_result(self):
        result = eggsec.TcpProbeResult(
            target="127.0.0.1",
            port=80,
            state="open",
        )
        assert result.state == "open"


class TestUdpReachabilityTypes:
    def test_config(self):
        cfg = eggsec.UdpReachabilityConfigPy(host="127.0.0.1", port=53)
        assert cfg.host == "127.0.0.1"
        assert cfg.port == 53

    def test_result(self):
        result = eggsec.UdpReachabilityResultPy(
            reachable=False,
            attempts=1,
            responses_received=0,
        )
        assert result.reachable is False
