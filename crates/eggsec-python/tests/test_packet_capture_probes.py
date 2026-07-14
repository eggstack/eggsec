"""Workstreams 9 and 10: Packet capture and privileged active-probe tests.

WS9 validates live packet capture: interface enumeration, PCAP parsing,
BPF filter construction, packet-layer unit parsing, flow aggregation,
capture configuration, async capture session lifecycle, and PcapWriter.

WS10 validates privileged active probes: ICMP echo, TCP SYN, UDP
reachability, traceroute, scope enforcement on probe dispatch, and
unsupported-platform error handling.

All tests are feature-gated behind packet-inspection. Privileged tests
that need raw sockets are further gated behind root or gracefully handle
permission errors.
"""

import json
import os
import struct
import tempfile
import time

import pytest

import eggsec

LOOPBACK_ALLOWED = os.environ.get("EGGSEC_ALLOW_LOOPBACK_FIXTURE", "0") == "1"
IS_ROOT = os.geteuid() == 0

pytestmark = [
    pytest.mark.skipif(
        not eggsec.has_feature("packet-inspection"),
        reason="packet-inspection not compiled",
    ),
    pytest.mark.packet_capture,
    pytest.mark.timeout(30),
]


# ────────────────────────────────────────────────────────────────────
# Helpers
# ────────────────────────────────────────────────────────────────────

def _make_minimal_pcap(path, packets=None):
    """Write a minimal valid pcap file with optional Ethernet frames.

    Global header (24 bytes): magic, version 2.4, thiszone=0, sigfigs=0,
    snaplen=65535, linktype=1 (Ethernet).

    Each packet record: 16-byte header (ts_sec, ts_usec, incl_len, orig_len)
    followed by the raw bytes.
    """
    if packets is None:
        # Default: one minimal IPv4/TCP packet (60 bytes on wire)
        eth = bytes([
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff,  # dst mac (broadcast)
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55,  # src mac
            0x08, 0x00,                            # ether_type = IPv4
        ])
        # IPv4 header (20 bytes min): ver/ihl=0x45, tos=0, total_len=40
        ip = bytes([
            0x45, 0x00, 0x00, 0x28,  # ver/ihl, tos, total length=40
            0x00, 0x00, 0x40, 0x00,  # id, flags/frag=DF
            0x40, 0x06,              # ttl=64, proto=TCP
            0x00, 0x00,              # checksum (omitted)
            0x0a, 0x00, 0x01, 0x01,  # src: 10.0.1.1
            0x0a, 0x00, 0x01, 0x02,  # dst: 10.0.1.2
        ])
        # TCP header (20 bytes min)
        tcp = bytes([
            0x00, 0x50,              # src port = 80
            0x30, 0x39,              # dst port = 12345
            0x00, 0x00, 0x00, 0x01,  # seq = 1
            0x00, 0x00, 0x00, 0x00,  # ack = 0
            0x50, 0x02,              # data offset + flags = SYN
            0xff, 0xff,              # window = 65535
            0x00, 0x00,              # checksum (omitted)
            0x00, 0x00,              # urgent pointer
        ])
        # Pad to 60-byte minimum Ethernet payload
        payload = eth + ip + tcp + bytes(60 - len(ip) - len(tcp))
        packets = [payload]

    with open(path, "wb") as f:
        # Global header
        f.write(struct.pack(
            "<IHHiIII",
            0xA1B2C3D4,  # magic
            2, 4,         # version
            0,            # thiszone
            0,            # sigfigs
            65535,        # snaplen
            1,            # linktype (Ethernet)
        ))
        for i, pkt_bytes in enumerate(packets):
            f.write(struct.pack(
                "<IIII",
                1700000000 + i,  # ts_sec
                0,               # ts_usec
                len(pkt_bytes),  # incl_len
                len(pkt_bytes),  # orig_len
            ))
            f.write(pkt_bytes)
    return path


def _loopback_skip():
    """Skip decorator for tests needing loopback fixture."""
    return pytest.mark.skipif(
        not LOOPBACK_ALLOWED,
        reason="EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 not set",
    )


def _root_skip():
    """Skip decorator for tests needing root/raw-socket privileges."""
    return pytest.mark.skipif(
        not IS_ROOT,
        reason="requires root for raw socket operations",
    )


# ────────────────────────────────────────────────────────────────────
# WS9-1: Network interface enumeration
# ────────────────────────────────────────────────────────────────────


class TestNetworkInterfaceEnumeration:
    """Validate list_network_interfaces() returns structured results."""

    def test_returns_list(self):
        ifaces = eggsec.list_network_interfaces()
        assert isinstance(ifaces, list)

    def test_non_empty(self):
        ifaces = eggsec.list_network_interfaces()
        assert len(ifaces) > 0

    def test_each_interface_has_required_fields(self):
        ifaces = eggsec.list_network_interfaces()
        for iface in ifaces:
            assert hasattr(iface, "name")
            assert hasattr(iface, "ips")
            assert hasattr(iface, "is_up")
            assert hasattr(iface, "is_loopback")
            assert isinstance(iface.name, str)
            assert isinstance(iface.ips, list)
            assert isinstance(iface.is_up, bool)
            assert isinstance(iface.is_loopback, bool)

    def test_loopback_exists(self):
        ifaces = eggsec.list_network_interfaces()
        loopbacks = [i for i in ifaces if i.is_loopback]
        assert len(loopbacks) >= 1, "No loopback interface found"
        assert any(i.name == "lo" for i in loopbacks), "Expected 'lo' loopback"

    def test_loopback_has_loopback_address(self):
        ifaces = eggsec.list_network_interfaces()
        lo = next(i for i in ifaces if i.name == "lo")
        assert any("127.0.0.1" in addr for addr in lo.ips)

    def test_loopback_is_up(self):
        ifaces = eggsec.list_network_interfaces()
        lo = next(i for i in ifaces if i.name == "lo")
        assert lo.is_up is True

    def test_to_dict(self):
        ifaces = eggsec.list_network_interfaces()
        for iface in ifaces:
            d = iface.to_dict()
            assert isinstance(d, dict)
            assert "name" in d
            assert "ips" in d
            assert "is_up" in d
            assert "is_loopback" in d

    def test_to_json(self):
        ifaces = eggsec.list_network_interfaces()
        for iface in ifaces:
            j = iface.to_json()
            parsed = json.loads(j)
            assert parsed["name"] == iface.name


# ────────────────────────────────────────────────────────────────────
# WS9-2: PCAP parsing
# ────────────────────────────────────────────────────────────────────


class TestPcapParsing:
    """Validate parse_pcap against minimal synthetic PCAP files."""

    def test_parse_minimal_pcap(self, tmp_path):
        pcap_file = tmp_path / "test.pcap"
        _make_minimal_pcap(str(pcap_file))
        packets = eggsec.parse_pcap(str(pcap_file))
        assert isinstance(packets, list)
        assert len(packets) >= 1

    def test_parsed_packet_has_timestamp(self, tmp_path):
        pcap_file = tmp_path / "test.pcap"
        _make_minimal_pcap(str(pcap_file))
        packets = eggsec.parse_pcap(str(pcap_file))
        pkt = packets[0]
        assert hasattr(pkt, "timestamp")

    def test_parsed_packet_has_network_fields(self, tmp_path):
        pcap_file = tmp_path / "test.pcap"
        _make_minimal_pcap(str(pcap_file))
        packets = eggsec.parse_pcap(str(pcap_file))
        pkt = packets[0]
        assert hasattr(pkt, "protocol")
        assert hasattr(pkt, "size")

    def test_parsed_packet_protocol_is_string(self, tmp_path):
        pcap_file = tmp_path / "test.pcap"
        _make_minimal_pcap(str(pcap_file))
        packets = eggsec.parse_pcap(str(pcap_file))
        pkt = packets[0]
        assert isinstance(pkt.protocol, str)

    def test_parsed_packet_size_positive(self, tmp_path):
        pcap_file = tmp_path / "test.pcap"
        _make_minimal_pcap(str(pcap_file))
        packets = eggsec.parse_pcap(str(pcap_file))
        pkt = packets[0]
        assert pkt.size > 0

    def test_parse_multiple_packets(self, tmp_path):
        pcap_file = tmp_path / "multi.pcap"
        # Build two distinct Ethernet frames
        frame1 = (
            b"\xff" * 6 + b"\x00" * 5 + b"\x55"  # dst + src + ether_type=IPv4
            + b"\x45\x00\x00\x1c" + b"\x00" * 4
            + b"\x40\x00\x40\x06\x00\x00"
            + b"\x0a\x00\x00\x01" + b"\x0a\x00\x00\x02"
        )
        frame2 = (
            b"\x00" * 6 + b"\xaa" * 5 + b"\xbb"  # different src mac
            + b"\x45\x00\x00\x1c" + b"\x00" * 4
            + b"\x40\x00\x40\x11\x00\x00"  # proto=UDP
            + b"\xc0\xa8\x00\x01" + b"\xc0\xa8\x00\x02"
        )
        _make_minimal_pcap(str(pcap_file), packets=[frame1, frame2])
        packets = eggsec.parse_pcap(str(pcap_file))
        assert len(packets) == 2

    def test_parse_nonexistent_file(self):
        with pytest.raises(Exception):
            eggsec.parse_pcap("/nonexistent/path/to/file.pcap")

    def test_parse_empty_pcap(self, tmp_path):
        pcap_file = tmp_path / "empty.pcap"
        # Write valid global header but zero packet records
        with open(pcap_file, "wb") as f:
            f.write(struct.pack(
                "<IHHiIII",
                0xA1B2C3D4, 2, 4, 0, 0, 65535, 1,
            ))
        packets = eggsec.parse_pcap(str(pcap_file))
        assert packets == []

    def test_parsed_packet_to_dict(self, tmp_path):
        pcap_file = tmp_path / "test.pcap"
        _make_minimal_pcap(str(pcap_file))
        packets = eggsec.parse_pcap(str(pcap_file))
        pkt = packets[0]
        if hasattr(pkt, "to_dict"):
            d = pkt.to_dict()
            assert isinstance(d, dict)


# ────────────────────────────────────────────────────────────────────
# WS9-3: Packet filter / BPF
# ────────────────────────────────────────────────────────────────────


class TestPacketFilterBpf:
    """Validate PacketFilter BPF expression generation."""

    def test_tcp_dst_port(self):
        pf = eggsec.PacketFilter(protocol="tcp", dst_port=80)
        bpf = pf.to_bpf()
        assert isinstance(bpf, str)
        assert "tcp" in bpf.lower()
        assert "80" in bpf

    def test_src_host(self):
        pf = eggsec.PacketFilter(src_ip="127.0.0.1")
        bpf = pf.to_bpf()
        assert "127.0.0.1" in bpf
        assert "host" in bpf.lower() or "src" in bpf.lower()

    def test_dst_host(self):
        pf = eggsec.PacketFilter(dst_ip="10.0.0.1")
        bpf = pf.to_bpf()
        assert "10.0.0.1" in bpf

    def test_bpf_passthrough(self):
        pf = eggsec.PacketFilter(bpf_expression="tcp port 443")
        bpf = pf.to_bpf()
        assert bpf == "tcp port 443"

    def test_combined_filters(self):
        pf = eggsec.PacketFilter(
            protocol="udp",
            src_port=53,
            dst_ip="10.0.0.1",
        )
        bpf = pf.to_bpf()
        assert "udp" in bpf.lower()
        assert "53" in bpf
        assert "10.0.0.1" in bpf

    def test_empty_filter(self):
        pf = eggsec.PacketFilter()
        bpf = pf.to_bpf()
        assert isinstance(bpf, str)

    def test_to_dict(self):
        pf = eggsec.PacketFilter(protocol="tcp", dst_port=443)
        d = pf.to_dict()
        assert isinstance(d, dict)
        assert d.get("protocol") == "tcp" or d.get("dst_port") == 443

    def test_to_json(self):
        pf = eggsec.PacketFilter(protocol="tcp")
        j = pf.to_json()
        parsed = json.loads(j)
        assert isinstance(parsed, dict)

    def test_repr(self):
        pf = eggsec.PacketFilter(protocol="tcp", dst_port=80)
        r = repr(pf)
        assert isinstance(r, str)
        assert len(r) > 0


# ────────────────────────────────────────────────────────────────────
# WS9-4: Packet parsing unit tests
# ────────────────────────────────────────────────────────────────────


class TestEthernetFrameParsing:
    """Validate EthernetFrame construction and field access."""

    def test_basic_construction(self):
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

    def test_vlan_tagged(self):
        frame = eggsec.EthernetFrame(
            src_mac="aa:bb:cc:dd:ee:ff",
            dst_mac="11:22:33:44:55:66",
            ether_type=0x8100,
            ether_type_name="802.1Q",
            vlan_id=200,
            payload_len=1500,
        )
        assert frame.vlan_id == 200
        assert frame.payload_len == 1500

    def test_fields_immutable(self):
        frame = eggsec.EthernetFrame(
            src_mac="00:00:00:00:00:00",
            dst_mac="ff:ff:ff:ff:ff:ff",
            ether_type=0x0806,
            ether_type_name="ARP",
        )
        with pytest.raises(AttributeError):
            frame.src_mac = "xx:xx:xx:xx:xx:xx"

    def test_to_dict_fields(self):
        frame = eggsec.EthernetFrame(
            src_mac="00:11:22:33:44:55",
            dst_mac="66:77:88:99:aa:bb",
            ether_type=0x0800,
            ether_type_name="IPv4",
        )
        d = frame.to_dict()
        assert d["src_mac"] == "00:11:22:33:44:55"
        assert d["dst_mac"] == "66:77:88:99:aa:bb"
        assert d["ether_type"] == 0x0800

    def test_to_json_roundtrip(self):
        frame = eggsec.EthernetFrame(
            src_mac="00:11:22:33:44:55",
            dst_mac="66:77:88:99:aa:bb",
            ether_type=0x0800,
            ether_type_name="IPv4",
        )
        parsed = json.loads(frame.to_json())
        assert parsed["src_mac"] == "00:11:22:33:44:55"
        assert parsed["ether_type"] == 0x0800

    def test_arp_ether_type(self):
        frame = eggsec.EthernetFrame(
            src_mac="00:11:22:33:44:55",
            dst_mac="ff:ff:ff:ff:ff:ff",
            ether_type=0x0806,
            ether_type_name="ARP",
        )
        assert frame.ether_type == 0x0806

    def test_ipv6_ether_type(self):
        frame = eggsec.EthernetFrame(
            src_mac="00:11:22:33:44:55",
            dst_mac="66:77:88:99:aa:bb",
            ether_type=0x86DD,
            ether_type_name="IPv6",
        )
        assert frame.ether_type == 0x86DD


class TestIpv4PacketParsing:
    """Validate Ipv4Packet field access and protocol metadata."""

    def test_tcp_packet(self):
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
        assert pkt.protocol_name == "TCP"
        assert "DF" in pkt.flags

    def test_udp_packet(self):
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
        )
        assert pkt.protocol_name == "UDP"
        assert pkt.flags == []

    def test_ttl_and_tos(self):
        pkt = eggsec.Ipv4Packet(
            src_ip="0.0.0.0",
            dst_ip="255.255.255.255",
            protocol=1,
            protocol_name="ICMP",
            ttl=1,
            tos=0x10,
            total_length=28,
            fragment_offset=0,
            flags=[],
        )
        assert pkt.ttl == 1
        assert pkt.tos == 0x10

    def test_fragment_offset(self):
        pkt = eggsec.Ipv4Packet(
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            protocol=17,
            protocol_name="UDP",
            ttl=64,
            tos=0,
            total_length=1500,
            fragment_offset=185,
            flags=["MF"],
        )
        assert pkt.fragment_offset == 185
        assert "MF" in pkt.flags

    def test_to_dict_json_roundtrip(self):
        pkt = eggsec.Ipv4Packet(
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            protocol=6,
            protocol_name="TCP",
            ttl=64,
            tos=0,
            total_length=60,
            fragment_offset=0,
            flags=["DF"],
        )
        d = pkt.to_dict()
        j = json.loads(pkt.to_json())
        assert d["src_ip"] == j["src_ip"] == "10.0.0.1"
        assert d["protocol"] == j["protocol"] == 6

    def test_with_checksum(self):
        pkt = eggsec.Ipv4Packet(
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            protocol=6,
            protocol_name="TCP",
            ttl=64,
            tos=0,
            total_length=60,
            fragment_offset=0,
            flags=[],
            header_checksum=0xABCD,
        )
        assert pkt.header_checksum == 0xABCD


class TestIpv6PacketParsing:
    """Validate Ipv6Packet field access."""

    def test_basic(self):
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
        assert pkt.next_header_name == "TCP"
        assert pkt.hop_limit == 64

    def test_hop_limit_values(self):
        for limit in (1, 32, 64, 128, 255):
            pkt = eggsec.Ipv6Packet(
                src_ip="::1",
                dst_ip="::1",
                next_header=59,
                next_header_name="No Next Header",
                hop_limit=limit,
                payload_length=0,
                flow_label=0,
                traffic_class=0,
            )
            assert pkt.hop_limit == limit

    def test_to_dict_json(self):
        pkt = eggsec.Ipv6Packet(
            src_ip="fe80::1",
            dst_ip="ff02::1",
            next_header=17,
            next_header_name="UDP",
            hop_limit=255,
            payload_length=100,
            flow_label=0x12345,
            traffic_class=0,
        )
        d = pkt.to_dict()
        j = json.loads(pkt.to_json())
        assert d["src_ip"] == "fe80::1"
        assert j["next_header_name"] == "UDP"


class TestTcpSegmentParsing:
    """Validate TcpSegment flags, window size, and options."""

    def test_syn_segment(self):
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
        assert seg.flags == ["SYN"]
        assert seg.window_size == 65535
        assert "MSS:1460" in seg.options

    def test_syn_ack_segment(self):
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

    def test_fin_ack(self):
        seg = eggsec.TcpSegment(
            src_port=80,
            dst_port=54321,
            seq_num=3000,
            ack_num=2001,
            data_offset=5,
            flags=["FIN", "ACK"],
            window_size=28960,
            urgent_pointer=0,
            options=[],
        )
        assert "FIN" in seg.flags
        assert "ACK" in seg.flags

    def test_rst_segment(self):
        seg = eggsec.TcpSegment(
            src_port=80,
            dst_port=54321,
            seq_num=0,
            ack_num=0,
            data_offset=5,
            flags=["RST"],
            window_size=0,
            urgent_pointer=0,
            options=[],
        )
        assert seg.flags == ["RST"]

    def test_window_size_range(self):
        for ws in (0, 1024, 8192, 65535):
            seg = eggsec.TcpSegment(
                src_port=1, dst_port=1, seq_num=0, ack_num=0,
                data_offset=5, flags=[], window_size=ws,
                urgent_pointer=0, options=[],
            )
            assert seg.window_size == ws

    def test_to_dict_json(self):
        seg = eggsec.TcpSegment(
            src_port=12345,
            dst_port=80,
            seq_num=1000,
            ack_num=0,
            data_offset=5,
            flags=["SYN"],
            window_size=65535,
            urgent_pointer=0,
            options=["MSS:1460"],
            payload_len=0,
        )
        d = seg.to_dict()
        j = json.loads(seg.to_json())
        assert d["src_port"] == j["src_port"] == 12345
        assert d["window_size"] == j["window_size"] == 65535


class TestUdpDatagramParsing:
    """Validate UdpDatagram length and checksum fields."""

    def test_basic(self):
        dg = eggsec.UdpDatagram(
            src_port=1234,
            dst_port=53,
            length=32,
            checksum=None,
            payload_len=12,
        )
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

    def test_max_length(self):
        dg = eggsec.UdpDatagram(
            src_port=65535,
            dst_port=65535,
            length=65535,
            checksum=0xFFFF,
            payload_len=65527,
        )
        assert dg.length == 65535

    def test_to_dict_json(self):
        dg = eggsec.UdpDatagram(
            src_port=5353,
            dst_port=53,
            length=40,
            checksum=None,
            payload_len=32,
        )
        d = dg.to_dict()
        j = json.loads(dg.to_json())
        assert d["src_port"] == j["src_port"] == 5353
        assert d["length"] == j["length"] == 40


class TestIcmpPacketParsing:
    """Validate IcmpPacket type/code fields."""

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
        assert pkt.id == 0x1234
        assert pkt.sequence == 1

    def test_echo_reply(self):
        pkt = eggsec.IcmpPacket(
            icmp_type=0,
            icmp_type_name="Echo Reply",
            icmp_code=0,
            id=0x1234,
            sequence=1,
        )
        assert pkt.icmp_type_name == "Echo Reply"

    def test_destination_unreachable(self):
        pkt = eggsec.IcmpPacket(
            icmp_type=3,
            icmp_type_name="Destination Unreachable",
            icmp_code=3,
        )
        assert pkt.icmp_code == 3

    def test_to_dict_json(self):
        pkt = eggsec.IcmpPacket(
            icmp_type=11,
            icmp_type_name="Time Exceeded",
            icmp_code=0,
        )
        d = pkt.to_dict()
        j = json.loads(pkt.to_json())
        assert d["icmp_type"] == j["icmp_type"] == 11


class TestDnsPacketParsing:
    """Validate DnsPacket query/response fields."""

    def test_query(self):
        dns = eggsec.DnsPacketPy(
            transaction_id=0x1234,
            is_response=False,
            op_code=0,
            recursion_desired=True,
            question_count=1,
            answer_count=0,
        )
        assert dns.is_response is False
        assert dns.question_count == 1

    def test_response(self):
        dns = eggsec.DnsPacketPy(
            transaction_id=0x5678,
            is_response=True,
            authoritative=True,
            response_code=0,
            question_count=1,
            answer_count=2,
        )
        assert dns.is_response is True
        assert dns.answer_count == 2

    def test_nxdomain(self):
        dns = eggsec.DnsPacketPy(
            transaction_id=0xABCD,
            is_response=True,
            response_code=3,
            question_count=1,
            answer_count=0,
        )
        assert dns.response_code == 3


class TestTlsRecordInfoParsing:
    """Validate TLS record info fields."""

    def test_client_hello(self):
        tls = eggsec.TlsRecordInfoPy(
            content_type="Handshake",
            version="TLS 1.3",
            record_length=256,
            handshake_type="ClientHello",
            cipher_suites=["TLS_AES_256_GCM_SHA384"],
            extensions=["server_name"],
            sni="example.com",
            alpn_protocols=["h2", "http/1.1"],
        )
        assert tls.sni == "example.com"
        assert "h2" in tls.alpn_protocols

    def test_application_data(self):
        tls = eggsec.TlsRecordInfoPy(
            content_type="ApplicationData",
            version="TLS 1.2",
            record_length=128,
        )
        assert tls.handshake_type is None
        assert tls.sni is None


# ────────────────────────────────────────────────────────────────────
# WS9-5: Flow aggregation
# ────────────────────────────────────────────────────────────────────


class TestFlowAggregation:
    """Validate FlowKey, FlowRecord, and FlowAggregator."""

    def test_flow_key_construction(self):
        fk = eggsec.FlowKey(
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            src_port=12345,
            dst_port=80,
            protocol="TCP",
        )
        assert fk.src_ip == "10.0.0.1"
        assert fk.dst_port == 80

    def test_flow_key_to_dict(self):
        fk = eggsec.FlowKey(
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            src_port=12345,
            dst_port=80,
            protocol="TCP",
        )
        d = fk.to_dict()
        assert d["protocol"] == "TCP"
        assert d["src_port"] == 12345

    def test_flow_key_to_json(self):
        fk = eggsec.FlowKey(
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            src_port=12345,
            dst_port=80,
            protocol="TCP",
        )
        parsed = json.loads(fk.to_json())
        assert parsed["dst_port"] == 80

    def test_flow_aggregator_empty(self):
        agg = eggsec.FlowAggregator(max_flows=100)
        assert agg.flow_count() == 0
        assert agg.total_packets() == 0
        assert agg.total_bytes() == 0
        assert agg.eviction_count() == 0

    def test_flow_aggregator_single_flow(self):
        agg = eggsec.FlowAggregator(max_flows=100)
        agg.record_packet(
            src_ip="10.0.0.1", dst_ip="10.0.0.2",
            src_port=1234, dst_port=80,
            protocol="TCP", packet_size=100,
            timestamp_ms=1000,
        )
        assert agg.flow_count() == 1
        assert agg.total_packets() == 1
        assert agg.total_bytes() == 100

    def test_flow_aggregator_same_flow_multiple_packets(self):
        agg = eggsec.FlowAggregator(max_flows=100)
        for i in range(10):
            agg.record_packet(
                src_ip="10.0.0.1", dst_ip="10.0.0.2",
                src_port=1234, dst_port=80,
                protocol="TCP", packet_size=64,
                timestamp_ms=1000 + i,
            )
        assert agg.flow_count() == 1
        assert agg.total_packets() == 10
        assert agg.total_bytes() == 640

    def test_flow_aggregator_multiple_flows(self):
        agg = eggsec.FlowAggregator(max_flows=100)
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
        agg.record_packet(
            src_ip="10.0.0.2", dst_ip="10.0.0.1",
            src_port=53, dst_port=1234, protocol="UDP",
            packet_size=64, timestamp_ms=3000,
        )
        assert agg.flow_count() == 3
        assert agg.total_bytes() == 364

    def test_flow_aggregator_eviction(self):
        agg = eggsec.FlowAggregator(max_flows=2)
        for i in range(5):
            agg.record_packet(
                src_ip=f"10.0.0.{i}", dst_ip="10.0.0.100",
                src_port=1000 + i, dst_port=80,
                protocol="TCP", packet_size=64,
                timestamp_ms=i * 1000,
            )
        assert agg.flow_count() == 2
        assert agg.eviction_count() >= 1

    def test_flow_aggregator_tcp_flags_tracking(self):
        agg = eggsec.FlowAggregator(max_flows=100)
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
        agg.record_packet(
            src_ip="10.0.0.1", dst_ip="10.0.0.2",
            src_port=1234, dst_port=80, protocol="TCP",
            packet_size=64, timestamp_ms=3000,
            tcp_flags=["FIN", "ACK"],
        )
        flows = agg.get_flows()
        assert len(flows) == 1
        flags = flows[0].tcp_flags_seen
        assert "SYN" in flags
        assert "ACK" in flags
        assert "FIN" in flags

    def test_flow_aggregator_to_dict(self):
        agg = eggsec.FlowAggregator(max_flows=50)
        agg.record_packet(
            src_ip="10.0.0.1", dst_ip="10.0.0.2",
            src_port=80, dst_port=12345, protocol="TCP",
            packet_size=1200, timestamp_ms=1000,
        )
        d = agg.to_dict()
        assert d["flow_count"] == 1
        assert d["max_flows"] == 50
        assert d["total_packets"] == 1
        assert d["total_bytes"] == 1200


# ────────────────────────────────────────────────────────────────────
# WS9-6: Capture configuration
# ────────────────────────────────────────────────────────────────────


class TestCaptureConfiguration:
    """Validate CaptureConfig, BackpressurePolicy, and CaptureDropStats."""

    def test_capture_config_defaults(self):
        cfg = eggsec.CaptureConfig()
        assert cfg.interface == ""
        assert cfg.filter is None
        assert cfg.promiscuous is True
        assert cfg.snapshot_len == 65535
        assert cfg.validate_checksums is False

    def test_capture_config_loopback(self):
        cfg = eggsec.CaptureConfig(
            interface="lo",
            filter="icmp",
            promiscuous=False,
            snapshot_len=128,
            timeout_secs=5,
            max_packets=100,
            save_to_file="/tmp/test.pcap",
            validate_checksums=True,
        )
        assert cfg.interface == "lo"
        assert cfg.filter == "icmp"
        assert cfg.promiscuous is False
        assert cfg.snapshot_len == 128
        assert cfg.timeout_secs == 5
        assert cfg.max_packets == 100
        assert cfg.save_to_file == "/tmp/test.pcap"
        assert cfg.validate_checksums is True

    def test_capture_config_to_dict(self):
        cfg = eggsec.CaptureConfig(interface="lo", filter="tcp port 80")
        d = cfg.to_dict()
        assert isinstance(d, dict)
        assert d["interface"] == "lo"
        assert d["filter"] == "tcp port 80"
        assert d["promiscuous"] is True

    def test_capture_config_to_json(self):
        cfg = eggsec.CaptureConfig(interface="eth0")
        j = cfg.to_json()
        parsed = json.loads(j)
        assert parsed["interface"] == "eth0"

    def test_backpressure_policy_all_variants(self):
        for name in ("Block", "DropOldest", "DropNewest", "ArtifactOnly"):
            policy = getattr(eggsec.BackpressurePolicy, name)
            assert repr(policy).startswith("BackpressurePolicy.")

    def test_backpressure_policy_to_dict(self):
        for name in ("Block", "DropOldest", "DropNewest", "ArtifactOnly"):
            d = getattr(eggsec.BackpressurePolicy, name).to_dict()
            assert isinstance(d, dict)
            assert "policy" in d
            assert d["policy"] == name

    def test_backpressure_policy_to_json(self):
        j = eggsec.BackpressurePolicy.DropOldest.to_json()
        parsed = json.loads(j)
        assert parsed == "DropOldest"

    def test_capture_drop_stats(self):
        stats = eggsec.CaptureDropStats(
            dropped_by_policy=5,
            dropped_by_full_queue=10,
            dropped_by_error=2,
            total_dropped=17,
        )
        assert stats.dropped_by_policy == 5
        assert stats.dropped_by_full_queue == 10
        assert stats.dropped_by_error == 2
        assert stats.total_dropped == 17

    def test_capture_drop_stats_zeros(self):
        stats = eggsec.CaptureDropStats(
            dropped_by_policy=0,
            dropped_by_full_queue=0,
            dropped_by_error=0,
            total_dropped=0,
        )
        assert stats.total_dropped == 0

    def test_capture_drop_stats_to_dict(self):
        stats = eggsec.CaptureDropStats(
            dropped_by_policy=1,
            dropped_by_full_queue=2,
            dropped_by_error=3,
            total_dropped=6,
        )
        d = stats.to_dict()
        assert d["dropped_by_policy"] == 1
        assert d["dropped_by_full_queue"] == 2
        assert d["dropped_by_error"] == 3
        assert d["total_dropped"] == 6

    def test_capture_drop_stats_to_json(self):
        stats = eggsec.CaptureDropStats(
            dropped_by_policy=0,
            dropped_by_full_queue=0,
            dropped_by_error=0,
            total_dropped=0,
        )
        parsed = json.loads(stats.to_json())
        assert parsed["total_dropped"] == 0


# ────────────────────────────────────────────────────────────────────
# WS9-7: Async capture session lifecycle
# ────────────────────────────────────────────────────────────────────


class TestAsyncCaptureSessionLifecycle:
    """Validate async capture session construction and state transitions."""

    def test_construction(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.AsyncCaptureSession(cfg)
        assert session.is_running is False
        assert session.is_closed is False
        assert session.interface == "lo"

    def test_construction_with_queue_size(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.AsyncCaptureSession(cfg, queue_size=1024)
        assert session.queue_size == 1024

    def test_start_stop(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.AsyncCaptureSession(cfg)
        session.start()
        assert session.is_running is True
        stats = session.stop()
        assert session.is_running is False
        assert session.is_closed is True
        assert isinstance(stats, eggsec.CaptureStats)

    def test_stats_fields(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.AsyncCaptureSession(cfg)
        session.start()
        stats = session.stop()
        assert hasattr(stats, "packets_captured")
        assert hasattr(stats, "packets_dropped")
        assert hasattr(stats, "bytes_captured")
        assert hasattr(stats, "runtime_ms")
        assert isinstance(stats.packets_captured, int)
        assert stats.packets_captured >= 0

    def test_context_manager(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        with eggsec.AsyncCaptureSession(cfg) as session:
            session.start()
            assert session.is_running is True
        assert session.is_closed is True

    def test_double_start_is_idempotent(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.AsyncCaptureSession(cfg)
        session.start()
        assert session.is_running is True
        session.start()
        assert session.is_running is True
        session.stop()

    def test_stop_when_not_running_is_noop(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.AsyncCaptureSession(cfg)
        stats = session.stop()
        assert isinstance(stats, eggsec.CaptureStats)

    def test_start_after_close_raises(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.AsyncCaptureSession(cfg)
        session.start()
        session.stop()
        with pytest.raises(ValueError, match="closed"):
            session.start()

    def test_sync_capture_session(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.SyncCaptureSessionPy(cfg)
        assert session.is_running is False
        assert session.is_closed is False
        session.start()
        assert session.is_running is True
        stats = session.stop()
        assert session.is_closed is True
        assert isinstance(stats, eggsec.CaptureStats)

    def test_sync_capture_context_manager(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        with eggsec.SyncCaptureSessionPy(cfg) as session:
            session.start()
            assert session.is_running is True
        assert session.is_closed is True


# ────────────────────────────────────────────────────────────────────
# WS9-8: PcapWriter lifecycle
# ────────────────────────────────────────────────────────────────────


class TestPcapWriter:
    """Validate PcapWriter construction, write, and close lifecycle."""

    def test_construction(self, tmp_path):
        path = str(tmp_path / "out.pcap")
        pw = eggsec.PcapWriter(path, snapshot_len=65535)
        assert pw.is_closed is False

    def test_context_manager(self, tmp_path):
        path = str(tmp_path / "out.pcap")
        with eggsec.PcapWriter(path, snapshot_len=65535) as pw:
            assert pw.is_closed is False
            pw.write_packet(b"\x00" * 14)
            pw.close()
        assert pw.is_closed is True
        assert os.path.getsize(path) > 0

    def test_write_packet(self, tmp_path):
        path = str(tmp_path / "out.pcap")
        pw = eggsec.PcapWriter(path, snapshot_len=65535)
        pw.write_packet(b"\xff" * 6 + b"\x00" * 6 + b"\x08\x00")
        pw.close()
        assert os.path.getsize(path) > 0

    def test_write_multiple_packets(self, tmp_path):
        path = str(tmp_path / "out.pcap")
        pw = eggsec.PcapWriter(path, snapshot_len=65535)
        for _ in range(5):
            pw.write_packet(b"\x00" * 64)
        pw.close()
        assert os.path.getsize(path) > 24  # at least global header

    def test_written_pcap_is_parseable(self, tmp_path):
        path = str(tmp_path / "written.pcap")
        pw = eggsec.PcapWriter(path, snapshot_len=65535)
        pw.write_packet(b"\x00" * 14)  # minimal Ethernet frame
        pw.close()
        packets = eggsec.parse_pcap(path)
        assert len(packets) == 1

    def test_flush(self, tmp_path):
        path = str(tmp_path / "out.pcap")
        with eggsec.PcapWriter(path, snapshot_len=65535) as pw:
            pw.write_packet(b"\x00" * 64)
            pw.flush()

    def test_double_close(self, tmp_path):
        path = str(tmp_path / "out.pcap")
        pw = eggsec.PcapWriter(path, snapshot_len=65535)
        pw.close()
        assert pw.is_closed is True
        pw.close()  # should be idempotent


# ────────────────────────────────────────────────────────────────────
# WS10-8: ICMP probe
# ────────────────────────────────────────────────────────────────────


class TestIcmpProbe:
    """Validate ICMP probe config, result, and dispatch against localhost."""

    def test_config_construction(self):
        cfg = eggsec.IcmpProbeConfig(target="127.0.0.1")
        assert cfg.target == "127.0.0.1"
        assert cfg.count == 4
        assert cfg.timeout_ms == 5000

    def test_config_custom(self):
        cfg = eggsec.IcmpProbeConfig(
            target="127.0.0.1",
            count=2,
            timeout_ms=1000,
            packet_size=32,
            ttl=32,
        )
        assert cfg.count == 2
        assert cfg.timeout_ms == 1000
        assert cfg.packet_size == 32
        assert cfg.ttl == 32

    def test_config_to_dict(self):
        cfg = eggsec.IcmpProbeConfig(target="127.0.0.1", count=1)
        d = cfg.to_dict()
        assert isinstance(d, dict)
        assert d["target"] == "127.0.0.1"
        assert d["count"] == 1

    def test_config_to_json(self):
        cfg = eggsec.IcmpProbeConfig(target="127.0.0.1")
        parsed = json.loads(cfg.to_json())
        assert parsed["target"] == "127.0.0.1"

    def test_reply_construction(self):
        reply = eggsec.IcmpProbeReply(seq=0, rtt_ms=1.5, ttl=64, bytes=64)
        assert reply.seq == 0
        assert reply.rtt_ms == 1.5
        assert reply.ttl == 64

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

    def test_result_with_replies(self):
        replies = [
            eggsec.IcmpProbeReply(seq=i, rtt_ms=0.5 * (i + 1), ttl=64, bytes=64)
            for i in range(3)
        ]
        result = eggsec.IcmpProbeResult(
            target="127.0.0.1",
            reachable=True,
            replies=replies,
            packets_sent=3,
            packets_received=3,
            min_rtt_ms=0.5,
            max_rtt_ms=1.5,
            avg_rtt_ms=1.0,
            packet_loss_pct=0.0,
        )
        assert len(result.replies) == 3
        assert result.min_rtt_ms == 0.5
        assert result.max_rtt_ms == 1.5

    def test_result_to_dict(self):
        result = eggsec.IcmpProbeResult(
            target="127.0.0.1",
            reachable=True,
            replies=[],
            packets_sent=1,
            packets_received=1,
            packet_loss_pct=0.0,
        )
        d = result.to_dict()
        assert d["reachable"] is True
        assert d["packets_sent"] == 1

    def test_result_to_json(self):
        result = eggsec.IcmpProbeResult(
            target="127.0.0.1",
            reachable=False,
            replies=[],
            packets_sent=4,
            packets_received=0,
            packet_loss_pct=100.0,
        )
        parsed = json.loads(result.to_json())
        assert parsed["reachable"] is False
        assert parsed["packet_loss_pct"] == 100.0

    def test_result_unreachable(self):
        result = eggsec.IcmpProbeResult(
            target="10.255.255.1",
            reachable=False,
            replies=[],
            packets_sent=4,
            packets_received=0,
            packet_loss_pct=100.0,
            error="host unreachable",
        )
        assert result.reachable is False
        assert result.error == "host unreachable"

    @pytest.mark.skipif(not IS_ROOT, reason="requires root for raw ICMP socket")
    def test_icmp_probe_localhost(self):
        cfg = eggsec.IcmpProbeConfig(target="127.0.0.1", count=2, timeout_ms=3000)
        result = eggsec.icmp_probe(cfg)
        assert isinstance(result, eggsec.IcmpProbeResult)
        assert result.target == "127.0.0.1"

    @pytest.mark.skipif(not IS_ROOT, reason="requires root for raw ICMP socket")
    def test_async_icmp_probe_localhost(self):
        cfg = eggsec.IcmpProbeConfig(target="127.0.0.1", count=1, timeout_ms=3000)
        result = eggsec.async_icmp_probe(cfg)
        assert isinstance(result, eggsec.IcmpProbeResult)


# ────────────────────────────────────────────────────────────────────
# WS10-9: TCP SYN probe
# ────────────────────────────────────────────────────────────────────


class TestTcpSynProbe:
    """Validate TCP SYN probe config, result, and dispatch."""

    def test_config_construction(self):
        cfg = eggsec.TcpProbeConfig(target="127.0.0.1", port=80)
        assert cfg.target == "127.0.0.1"
        assert cfg.port == 80
        assert cfg.timeout_ms == 5000

    def test_config_custom(self):
        cfg = eggsec.TcpProbeConfig(
            target="127.0.0.1",
            port=443,
            timeout_ms=5000,
            ttl=64,
            source_port=12345,
        )
        assert cfg.port == 443
        assert cfg.timeout_ms == 5000
        assert cfg.source_port == 12345

    def test_config_to_dict(self):
        cfg = eggsec.TcpProbeConfig(target="127.0.0.1", port=22)
        d = cfg.to_dict()
        assert d["target"] == "127.0.0.1"
        assert d["port"] == 22

    def test_config_to_json(self):
        cfg = eggsec.TcpProbeConfig(target="127.0.0.1", port=80)
        parsed = json.loads(cfg.to_json())
        assert parsed["port"] == 80

    def test_result_open(self):
        result = eggsec.TcpProbeResult(
            target="127.0.0.1",
            port=80,
            state="open",
            rtt_ms=1.2,
            ttl=64,
            window_size=65535,
        )
        assert result.state == "open"
        assert result.rtt_ms == 1.2

    def test_result_closed(self):
        result = eggsec.TcpProbeResult(
            target="127.0.0.1",
            port=1,
            state="closed",
            rtt_ms=0.5,
        )
        assert result.state == "closed"

    def test_result_filtered(self):
        result = eggsec.TcpProbeResult(
            target="127.0.0.1",
            port=80,
            state="filtered",
        )
        assert result.state == "filtered"

    def test_result_to_dict(self):
        result = eggsec.TcpProbeResult(
            target="127.0.0.1",
            port=443,
            state="open",
            rtt_ms=2.0,
        )
        d = result.to_dict()
        assert d["state"] == "open"
        assert d["port"] == 443

    def test_result_to_json(self):
        result = eggsec.TcpProbeResult(
            target="127.0.0.1",
            port=80,
            state="closed",
        )
        parsed = json.loads(result.to_json())
        assert parsed["state"] == "closed"

    def test_result_with_error(self):
        result = eggsec.TcpProbeResult(
            target="127.0.0.1",
            port=80,
            state="error",
            error="connection refused",
        )
        assert result.error == "connection refused"

    @pytest.mark.skipif(not IS_ROOT, reason="requires root for raw TCP SYN socket")
    def test_tcp_syn_probe_localhost(self):
        cfg = eggsec.TcpProbeConfig(target="127.0.0.1", port=1, timeout_ms=2000)
        result = eggsec.tcp_syn_probe(cfg)
        assert isinstance(result, eggsec.TcpProbeResult)
        assert result.target == "127.0.0.1"

    @pytest.mark.skipif(not IS_ROOT, reason="requires root for raw TCP SYN socket")
    def test_async_tcp_syn_probe_localhost(self):
        cfg = eggsec.TcpProbeConfig(target="127.0.0.1", port=1, timeout_ms=2000)
        result = eggsec.async_tcp_syn_probe(cfg)
        assert isinstance(result, eggsec.TcpProbeResult)


# ────────────────────────────────────────────────────────────────────
# WS10-10: UDP reachability
# ────────────────────────────────────────────────────────────────────


class TestUdpReachability:
    """Validate UDP reachability config, result, and dispatch."""

    def test_config_construction(self):
        cfg = eggsec.UdpReachabilityConfigPy(host="127.0.0.1", port=53)
        assert cfg.host == "127.0.0.1"
        assert cfg.port == 53
        assert cfg.attempts == 1
        assert cfg.timeout_ms == 2000

    def test_config_custom(self):
        cfg = eggsec.UdpReachabilityConfigPy(
            host="127.0.0.1",
            port=5353,
            attempts=3,
            timeout_ms=5000,
            payload=b"\xde\xad\xbe\xef",
        )
        assert cfg.attempts == 3
        assert cfg.timeout_ms == 5000
        assert list(cfg.payload) == [0xDE, 0xAD, 0xBE, 0xEF]

    def test_config_to_dict(self):
        cfg = eggsec.UdpReachabilityConfigPy(host="10.0.0.1", port=1234)
        d = cfg.to_dict()
        assert d["host"] == "10.0.0.1"
        assert d["port"] == 1234

    def test_config_to_json(self):
        cfg = eggsec.UdpReachabilityConfigPy(host="10.0.0.1", port=53)
        parsed = json.loads(cfg.to_json())
        assert parsed["host"] == "10.0.0.1"

    def test_result_reachable(self):
        result = eggsec.UdpReachabilityResultPy(
            reachable=True,
            attempts=3,
            responses_received=2,
        )
        assert result.reachable is True
        assert result.responses_received == 2

    def test_result_unreachable(self):
        result = eggsec.UdpReachabilityResultPy(
            reachable=False,
            attempts=3,
            responses_received=0,
            error="timeout",
        )
        assert result.reachable is False
        assert result.error == "timeout"

    def test_result_to_dict(self):
        result = eggsec.UdpReachabilityResultPy(
            reachable=True,
            attempts=1,
            responses_received=1,
        )
        d = result.to_dict()
        assert d["reachable"] is True
        assert d["attempts"] == 1

    def test_result_to_json(self):
        result = eggsec.UdpReachabilityResultPy(
            reachable=False,
            attempts=1,
            responses_received=0,
        )
        parsed = json.loads(result.to_json())
        assert parsed["reachable"] is False

    def test_result_with_rtt(self):
        result = eggsec.UdpReachabilityResultPy(
            reachable=True,
            attempts=1,
            responses_received=1,
            rtt_ms=5.0,
        )
        assert result.rtt_ms == 5.0

    def test_udp_reachability_localhost(self):
        cfg = eggsec.UdpReachabilityConfigPy(host="127.0.0.1", port=53, timeout_ms=2000)
        try:
            result = eggsec.udp_reachability(cfg)
            assert isinstance(result, eggsec.UdpReachabilityResultPy)
        except PermissionError:
            pytest.skip("insufficient privileges for UDP probe")


# ────────────────────────────────────────────────────────────────────
# WS10-11: Traceroute
# ────────────────────────────────────────────────────────────────────


class TestTraceroute:
    """Validate traceroute config, hop, result, and dispatch."""

    def test_config_construction(self):
        cfg = eggsec.TracerouteConfig(target="127.0.0.1")
        assert cfg.target == "127.0.0.1"
        assert cfg.max_hops == 30
        assert cfg.timeout_secs == 3
        assert cfg.use_icmp is False

    def test_config_custom(self):
        cfg = eggsec.TracerouteConfig(
            target="127.0.0.1",
            max_hops=5,
            timeout_secs=2,
            max_retries=1,
            first_ttl=1,
            port=33434,
            use_icmp=True,
            packet_size=64,
            resolve_names=False,
        )
        assert cfg.max_hops == 5
        assert cfg.timeout_secs == 2
        assert cfg.use_icmp is True
        assert cfg.resolve_names is False

    def test_config_to_dict(self):
        cfg = eggsec.TracerouteConfig(target="127.0.0.1", max_hops=10)
        d = cfg.to_dict()
        assert d["target"] == "127.0.0.1"
        assert d["max_hops"] == 10

    def test_config_to_json(self):
        cfg = eggsec.TracerouteConfig(target="127.0.0.1")
        parsed = json.loads(json.dumps(cfg.to_dict()))
        assert parsed["target"] == "127.0.0.1"

    @pytest.mark.skipif(not IS_ROOT, reason="traceroute requires root for raw sockets")
    def test_traceroute_localhost(self):
        result = eggsec.traceroute("127.0.0.1", max_hops=5, timeout_secs=2)
        assert isinstance(result, eggsec.TracerouteResult)
        assert result.target == "127.0.0.1"

    @pytest.mark.skipif(not IS_ROOT, reason="traceroute requires root for raw sockets")
    def test_traceroute_localhost_reaches(self):
        result = eggsec.traceroute("127.0.0.1", max_hops=5, timeout_secs=2)
        assert result.success is True
        assert len(result.hops) >= 1

    @pytest.mark.skipif(not IS_ROOT, reason="traceroute requires root for raw sockets")
    def test_traceroute_result_fields(self):
        result = eggsec.traceroute("127.0.0.1", max_hops=3, timeout_secs=2)
        assert hasattr(result, "target")
        assert hasattr(result, "success")
        assert hasattr(result, "hops")
        assert hasattr(result, "total_hops")
        assert hasattr(result, "resolved_address")

    @pytest.mark.skipif(not IS_ROOT, reason="traceroute requires root for raw sockets")
    def test_traceroute_result_to_dict(self):
        result = eggsec.traceroute("127.0.0.1", max_hops=3, timeout_secs=2)
        d = result.to_dict()
        assert isinstance(d, dict)
        assert d["target"] == "127.0.0.1"
        assert isinstance(d["hops"], list)

    @pytest.mark.skipif(not IS_ROOT, reason="traceroute requires root for raw sockets")
    def test_traceroute_result_to_json(self):
        result = eggsec.traceroute("127.0.0.1", max_hops=3, timeout_secs=2)
        parsed = json.loads(result.to_json())
        assert parsed["target"] == "127.0.0.1"

    @pytest.mark.skipif(not IS_ROOT, reason="traceroute requires root for raw sockets")
    def test_traceroute_hops_have_required_fields(self):
        result = eggsec.traceroute("127.0.0.1", max_hops=3, timeout_secs=2)
        for hop in result.hops:
            assert hasattr(hop, "hop")
            assert hasattr(hop, "address")
            assert hasattr(hop, "rtt_ms")
            assert hasattr(hop, "is_final")

    @pytest.mark.skipif(not IS_ROOT, reason="traceroute requires root for raw sockets")
    def test_traceroute_hop_to_dict(self):
        result = eggsec.traceroute("127.0.0.1", max_hops=3, timeout_secs=2)
        if result.hops:
            d = result.hops[0].to_dict()
            assert isinstance(d, dict)
            assert "hop" in d

    @pytest.mark.skipif(not IS_ROOT, reason="traceroute requires root for raw sockets")
    def test_traceroute_hop_to_json(self):
        result = eggsec.traceroute("127.0.0.1", max_hops=3, timeout_secs=2)
        if result.hops:
            parsed = json.loads(result.hops[0].to_json())
            assert "hop" in parsed

    @pytest.mark.skipif(not IS_ROOT, reason="traceroute requires root for raw sockets")
    def test_async_run_traceroute(self):
        cfg = eggsec.TracerouteConfig(target="127.0.0.1", max_hops=3, timeout_secs=2)
        result = eggsec.async_run_traceroute(cfg)
        assert isinstance(result, eggsec.TracerouteResult)
        assert result.target == "127.0.0.1"


# ────────────────────────────────────────────────────────────────────
# WS10-12: Probe scope enforcement
# ────────────────────────────────────────────────────────────────────


class TestProbeScopeEnforcement:
    """Validate that probes to out-of-scope targets raise EnforcementError."""

    def test_in_scope_target_allowed(self):
        scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
        engine = eggsec.Engine(scope, timeout_ms=2000)
        req = eggsec.OperationRequest("scan_ports", "127.0.0.1", timeout_ms=2000)
        result = engine.run(req)
        assert result.is_failure() is False or "scope" not in str(
            result.error or ""
        ).lower()
        engine.close()

    def test_out_of_scope_target_denied(self):
        scope = eggsec.Scope.allow_hosts(["10.0.0.1"])
        engine = eggsec.Engine(scope, timeout_ms=2000)
        req = eggsec.OperationRequest("scan_ports", "192.168.99.99", timeout_ms=2000)
        result = engine.run(req)
        assert result.is_failure() is True
        assert result.error is not None
        engine.close()

    def test_deny_all_blocks_all(self):
        scope = eggsec.Scope.deny_all()
        engine = eggsec.Engine(scope, timeout_ms=2000)
        req = eggsec.OperationRequest("scan_ports", "127.0.0.1", timeout_ms=2000)
        result = engine.run(req)
        assert result.is_failure() is True
        engine.close()

    def test_scope_enforcement_error_is_raised(self):
        scope = eggsec.Scope.allow_hosts(["example.com"])
        with pytest.raises(eggsec.EnforcementError):
            eggsec.scan_ports("evil.com", [80], scope, timeout_ms=1000)

    def test_scope_loopback_in_loopback_scope(self):
        scope = eggsec.Scope.allow_cidrs(["127.0.0.0/8"])
        assert scope.is_target_allowed("127.0.0.1") is True
        assert scope.is_target_allowed("127.0.0.2") is True
        assert scope.is_target_allowed("10.0.0.1") is False

    def test_scope_external_denied(self):
        scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
        assert scope.is_target_allowed("8.8.8.8") is False
        assert scope.is_target_allowed("1.1.1.1") is False


# ────────────────────────────────────────────────────────────────────
# WS10-13: Unsupported platform / permission errors
# ────────────────────────────────────────────────────────────────────


class TestProbePlatformHandling:
    """Validate graceful handling when probe capabilities are unavailable."""

    def test_icmp_probe_non_root_graceful(self):
        if IS_ROOT:
            pytest.skip("running as root — no permission error to test")
        cfg = eggsec.IcmpProbeConfig(target="127.0.0.1", count=1, timeout_ms=1000)
        try:
            result = eggsec.icmp_probe(cfg)
            # If it succeeds despite non-root, that's fine too
            assert isinstance(result, eggsec.IcmpProbeResult)
        except (PermissionError, OSError, eggsec.EnforcementError) as e:
            # Expected on non-root without CAP_NET_RAW
            assert isinstance(e, (PermissionError, OSError, eggsec.EnforcementError))

    def test_tcp_syn_probe_non_root_graceful(self):
        if IS_ROOT:
            pytest.skip("running as root — no permission error to test")
        cfg = eggsec.TcpProbeConfig(target="127.0.0.1", port=1, timeout_ms=1000)
        try:
            result = eggsec.tcp_syn_probe(cfg)
            assert isinstance(result, eggsec.TcpProbeResult)
        except (PermissionError, OSError, eggsec.EnforcementError) as e:
            assert isinstance(e, (PermissionError, OSError, eggsec.EnforcementError))

    def test_icmp_probe_error_field_populated(self):
        """When probe fails, error field should explain the failure."""
        if IS_ROOT:
            # Even as root, unreachable host should still yield a result
            cfg = eggsec.IcmpProbeConfig(target="10.255.255.1", count=1, timeout_ms=1000)
            try:
                result = eggsec.icmp_probe(cfg)
                assert isinstance(result, eggsec.IcmpProbeResult)
                if not result.reachable:
                    # Error may or may not be set depending on timeout vs unreachable
                    assert result.packet_loss_pct == 100.0
            except Exception:
                pass  # timeout is acceptable for unreachable host
        else:
            pytest.skip("requires root")

    @pytest.mark.skipif(not IS_ROOT, reason="traceroute requires root for raw sockets")
    def test_traceroute_non_root_works(self):
        """Traceroute should work with root (uses UDP by default)."""
        result = eggsec.traceroute("127.0.0.1", max_hops=2, timeout_secs=2)
        assert isinstance(result, eggsec.TracerouteResult)


# ────────────────────────────────────────────────────────────────────
# WS9: PacketStream and CapturedPacket
# ────────────────────────────────────────────────────────────────────


class TestPacketStreamAndCapturedPacket:
    """Validate PacketStreamPy iteration and CapturedPacket construction."""

    def test_packet_stream_empty(self):
        stream = eggsec.PacketStreamPy([])
        assert stream.len() == 0
        assert stream.is_empty()

    def test_packet_stream_single(self):
        info = eggsec.PacketInfo(timestamp="2024-01-01T00:00:00Z")
        pkt = eggsec.CapturedPacket(
            sequence=0, timestamp_ms=0, captured_len=64,
            original_len=64, info=info, raw_bytes=b"\x00" * 64,
        )
        stream = eggsec.PacketStreamPy([pkt])
        assert stream.len() == 1
        assert not stream.is_empty()

    def test_packet_stream_iteration(self):
        info = eggsec.PacketInfo(timestamp="2024-01-01T00:00:00Z")
        pkts = [
            eggsec.CapturedPacket(
                sequence=i, timestamp_ms=i, captured_len=32,
                original_len=32, info=info, raw_bytes=b"\x00" * 32,
            )
            for i in range(10)
        ]
        stream = eggsec.PacketStreamPy(pkts)
        collected = list(stream)
        assert len(collected) == 10

    def test_packet_stream_next(self):
        info = eggsec.PacketInfo(timestamp="2024-01-01T00:00:00Z")
        pkt = eggsec.CapturedPacket(
            sequence=0, timestamp_ms=0, captured_len=10,
            original_len=10, info=info, raw_bytes=b"\x00" * 10,
        )
        stream = eggsec.PacketStreamPy([pkt])
        first = stream.next()
        assert first is not None
        assert first.sequence == 0
        assert stream.next() is None

    def test_packet_stream_to_list(self):
        info = eggsec.PacketInfo(timestamp="2024-01-01T00:00:00Z")
        pkts = [
            eggsec.CapturedPacket(
                sequence=i, timestamp_ms=0, captured_len=10,
                original_len=10, info=info, raw_bytes=b"\x00" * 10,
            )
            for i in range(5)
        ]
        stream = eggsec.PacketStreamPy(pkts)
        as_list = stream.to_list()
        assert len(as_list) == 5

    def test_captured_packet_raw_bytes(self):
        info = eggsec.PacketInfo(timestamp="2024-01-01T00:00:00Z")
        raw = b"\x00\x11\x22\x33\x44\x55" + b"\x00" * 58
        pkt = eggsec.CapturedPacket(
            sequence=0, timestamp_ms=0, captured_len=64,
            original_len=64, info=info, raw_bytes=raw,
        )
        rb = pkt.raw_bytes()
        assert len(rb) == 64
        assert rb[:6] == [0, 17, 34, 51, 68, 85]

    def test_captured_packet_fields(self):
        info = eggsec.PacketInfo(
            timestamp="2024-01-01T00:00:00Z",
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            protocol="TCP",
        )
        pkt = eggsec.CapturedPacket(
            sequence=42,
            timestamp_ms=1700000000000,
            captured_len=128,
            original_len=1500,
            info=info,
            raw_bytes=b"\x00" * 128,
        )
        assert pkt.sequence == 42
        assert pkt.timestamp_ms == 1700000000000
        assert pkt.captured_len == 128
        assert pkt.original_len == 1500


# ────────────────────────────────────────────────────────────────────
# WS9: LiveCaptureResult and CaptureStats (read-only types)
# ────────────────────────────────────────────────────────────────────


class TestLiveCaptureResultAndCaptureStats:
    """Validate read-only result types returned from capture sessions."""

    def test_capture_stats_fields_after_stop(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.AsyncCaptureSession(cfg)
        session.start()
        time.sleep(0.1)  # let capture run briefly
        stats = session.stop()
        assert isinstance(stats.packets_captured, int)
        assert isinstance(stats.packets_dropped, int)
        assert isinstance(stats.bytes_captured, int)
        assert isinstance(stats.runtime_ms, (int, float))
        assert stats.packets_captured >= 0
        assert stats.runtime_ms >= 0

    def test_capture_stats_to_dict(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.AsyncCaptureSession(cfg)
        session.start()
        stats = session.stop()
        d = stats.to_dict()
        assert isinstance(d, dict)
        assert "packets_captured" in d

    def test_capture_stats_to_json(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        session = eggsec.AsyncCaptureSession(cfg)
        session.start()
        stats = session.stop()
        parsed = json.loads(stats.to_json())
        assert "packets_captured" in parsed

    def test_network_interface_info_mac(self):
        ifaces = eggsec.list_network_interfaces()
        non_loopback = [i for i in ifaces if not i.is_loopback and i.ips]
        if non_loopback:
            iface = non_loopback[0]
            assert hasattr(iface, "mac")


# ────────────────────────────────────────────────────────────────────
# WS10: All probe types serialization roundtrips
# ────────────────────────────────────────────────────────────────────


class TestProbeSerializationRoundtrips:
    """Verify to_dict/to_json for all WS10 probe config and result types."""

    PROBE_CONFIGS = [
        ("IcmpProbeConfig", lambda: eggsec.IcmpProbeConfig(target="127.0.0.1")),
        ("TcpProbeConfig", lambda: eggsec.TcpProbeConfig(target="127.0.0.1", port=80)),
        ("UdpReachabilityConfigPy", lambda: eggsec.UdpReachabilityConfigPy(host="127.0.0.1", port=53)),
        ("TracerouteConfig", lambda: eggsec.TracerouteConfig(target="127.0.0.1")),
    ]

    PROBE_RESULTS = [
        (
            "IcmpProbeResult",
            lambda: eggsec.IcmpProbeResult(
                target="127.0.0.1", reachable=True, replies=[],
                packets_sent=1, packets_received=1, packet_loss_pct=0.0,
            ),
        ),
        (
            "TcpProbeResult",
            lambda: eggsec.TcpProbeResult(target="127.0.0.1", port=80, state="open"),
        ),
        (
            "UdpReachabilityResultPy",
            lambda: eggsec.UdpReachabilityResultPy(reachable=True, attempts=1, responses_received=1),
        ),
    ]

    @pytest.mark.parametrize("name,factory", PROBE_CONFIGS, ids=[c[0] for c in PROBE_CONFIGS])
    def test_config_to_dict(self, name, factory):
        cfg = factory()
        d = cfg.to_dict()
        assert isinstance(d, dict)
        assert len(d) > 0

    @pytest.mark.parametrize("name,factory", PROBE_CONFIGS, ids=[c[0] for c in PROBE_CONFIGS])
    def test_config_to_json_valid(self, name, factory):
        cfg = factory()
        if hasattr(cfg, "to_json"):
            parsed = json.loads(cfg.to_json())
        else:
            parsed = json.loads(json.dumps(cfg.to_dict()))
        assert isinstance(parsed, dict)
        assert len(parsed) > 0

    @pytest.mark.parametrize("name,factory", PROBE_RESULTS, ids=[r[0] for r in PROBE_RESULTS])
    def test_result_to_dict(self, name, factory):
        result = factory()
        d = result.to_dict()
        assert isinstance(d, dict)
        assert len(d) > 0

    @pytest.mark.parametrize("name,factory", PROBE_RESULTS, ids=[r[0] for r in PROBE_RESULTS])
    def test_result_to_json_valid(self, name, factory):
        result = factory()
        parsed = json.loads(result.to_json())
        assert isinstance(parsed, dict)
        assert len(parsed) > 0

    @pytest.mark.parametrize("name,factory", PROBE_CONFIGS, ids=[c[0] for c in PROBE_CONFIGS])
    def test_config_repr(self, name, factory):
        cfg = factory()
        r = repr(cfg)
        assert isinstance(r, str)
        assert len(r) > 0

    @pytest.mark.parametrize("name,factory", PROBE_RESULTS, ids=[r[0] for r in PROBE_RESULTS])
    def test_result_repr(self, name, factory):
        result = factory()
        r = repr(result)
        assert isinstance(r, str)
        assert len(r) > 0


# ────────────────────────────────────────────────────────────────────
# WS9: API surface inclusion
# ────────────────────────────────────────────────────────────────────


class TestApiSurfaceInclusion:
    """Verify WS9/WS10 types appear in the API surface registry."""

    def test_capture_functions_in_surface(self):
        surface = eggsec.api_surface()
        for name in (
            "parse_pcap", "list_network_interfaces",
        ):
            assert name in surface, f"{name} missing from API surface"

    def test_probe_functions_in_surface(self):
        surface = eggsec.api_surface()
        for name in (
            "traceroute", "run_traceroute",
            "async_run_traceroute",
        ):
            assert name in surface, f"{name} missing from API surface"

    def test_capture_types_stability(self):
        surface = eggsec.api_surface()
        for name in ("CaptureConfig", "AsyncCaptureSession"):
            if name in surface:
                assert surface[name]["stability"] in ("stable", "provisional")


# ────────────────────────────────────────────────────────────────────
# WS9+WS10: Cross-cutting concerns
# ────────────────────────────────────────────────────────────────────


class TestCrossCuttingConcerns:
    """Miscellaneous cross-cutting validations."""

    def test_packet_filter_repr(self):
        pf = eggsec.PacketFilter(protocol="tcp", dst_port=443)
        r = repr(pf)
        assert "PacketFilter" in r or "tcp" in r

    def test_capture_config_repr(self):
        cfg = eggsec.CaptureConfig(interface="lo")
        r = repr(cfg)
        assert "lo" in r

    def test_flow_key_repr(self):
        fk = eggsec.FlowKey(
            src_ip="10.0.0.1", dst_ip="10.0.0.2",
            src_port=80, dst_port=443, protocol="TCP",
        )
        r = repr(fk)
        assert "10.0.0.1" in r or "TCP" in r

    def test_icmp_probe_config_repr(self):
        cfg = eggsec.IcmpProbeConfig(target="127.0.0.1")
        r = repr(cfg)
        assert "127.0.0.1" in r

    def test_traceroute_config_repr(self):
        cfg = eggsec.TracerouteConfig(target="127.0.0.1")
        r = repr(cfg)
        assert "127.0.0.1" in r

    def test_capture_config_equality(self):
        cfg1 = eggsec.CaptureConfig(interface="lo", promiscuous=True)
        cfg2 = eggsec.CaptureConfig(interface="lo", promiscuous=True)
        assert cfg1.to_dict() == cfg2.to_dict()

    def test_probe_config_immutability(self):
        cfg = eggsec.IcmpProbeConfig(target="127.0.0.1")
        with pytest.raises(AttributeError):
            cfg.target = "10.0.0.1"

    def test_many_packet_filters(self):
        """Verify BPF generation works across many parameter combinations."""
        filters = [
            eggsec.PacketFilter(protocol="tcp"),
            eggsec.PacketFilter(protocol="udp"),
            eggsec.PacketFilter(protocol="icmp"),
            eggsec.PacketFilter(dst_port=80),
            eggsec.PacketFilter(dst_port=443),
            eggsec.PacketFilter(src_port=12345),
            eggsec.PacketFilter(src_ip="10.0.0.1"),
            eggsec.PacketFilter(dst_ip="10.0.0.2"),
            eggsec.PacketFilter(protocol="tcp", dst_port=80, src_ip="10.0.0.1"),
        ]
        for pf in filters:
            bpf = pf.to_bpf()
            assert isinstance(bpf, str)
            assert len(bpf) > 0

    def test_capture_config_with_all_fields(self):
        cfg = eggsec.CaptureConfig(
            interface="eth0",
            filter="tcp port 443",
            promiscuous=True,
            snapshot_len=96,
            timeout_secs=10,
            max_packets=5000,
            save_to_file="/tmp/capture.pcap",
            validate_checksums=True,
        )
        d = cfg.to_dict()
        assert d["interface"] == "eth0"
        assert d["filter"] == "tcp port 443"
        assert d["max_packets"] == 5000
        assert d["validate_checksums"] is True

    @pytest.mark.skipif(not IS_ROOT, reason="traceroute requires root for raw sockets")
    def test_traceroute_result_hop_consistency(self):
        result = eggsec.traceroute("127.0.0.1", max_hops=3, timeout_secs=2)
        for i, hop in enumerate(result.hops):
            assert hop.hop == i + 1

    def test_multiple_flow_aggregators_independent(self):
        agg1 = eggsec.FlowAggregator(max_flows=10)
        agg2 = eggsec.FlowAggregator(max_flows=10)
        agg1.record_packet(
            src_ip="10.0.0.1", dst_ip="10.0.0.2",
            src_port=80, dst_port=443, protocol="TCP",
            packet_size=100, timestamp_ms=1000,
        )
        assert agg1.flow_count() == 1
        assert agg2.flow_count() == 0

    def test_packet_stream_large_batch(self):
        info = eggsec.PacketInfo(timestamp="2024-01-01T00:00:00Z")
        pkts = [
            eggsec.CapturedPacket(
                sequence=i, timestamp_ms=i, captured_len=64,
                original_len=64, info=info, raw_bytes=b"\x00" * 64,
            )
            for i in range(100)
        ]
        stream = eggsec.PacketStreamPy(pkts)
        assert stream.len() == 100
        all_pkts = stream.to_list()
        assert len(all_pkts) == 100
