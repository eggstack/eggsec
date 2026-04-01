pub(crate) const PACKET_ABOUT: &str = "Packet inspection and analysis tools

Provides tools for live packet capture, packet crafting, hexdump view,
header inspection, and traceroute functionality.
NOTE: Live packet capture requires building with --features packet-inspection
Requires root/sudo for live packet capture.

Examples:
  slapper packet capture -i eth0
  slapper packet capture -i eth0 --filter tcp --max 100
  slapper packet send --tcp --dst example.com:80 --flags SYN
  slapper packet dump capture.pcap
  slapper packet traceroute example.com
  slapper packet interfaces";

#[derive(clap::Args)]
pub struct PacketArgs {
    #[command(subcommand)]
    pub command: PacketSubcommand,
}

#[derive(clap::Subcommand)]
pub enum PacketSubcommand {
    #[command(about = "Capture packets from network interface")]
    Capture(PacketCaptureArgs),
    #[command(about = "Craft and send custom packets")]
    Send(PacketSendArgs),
    #[command(about = "Hexdump a pcap file or packet data")]
    Dump(PacketDumpArgs),
    #[command(about = "Trace network route to target")]
    Traceroute(PacketTracerouteArgs),
    #[command(about = "List available network interfaces")]
    Interfaces,
}

#[derive(clap::Args)]
pub struct PacketCaptureArgs {
    #[arg(short = 'i', long, help = "Network interface name")]
    pub interface: Option<String>,
    #[arg(long, help = "BPF filter expression (e.g., 'tcp port 80')")]
    pub filter: Option<String>,
    #[arg(long, default_value = "100", help = "Maximum packets to capture")]
    pub max: Option<usize>,
    #[arg(long, help = "Output file for pcap")]
    pub output: Option<String>,
    #[arg(long, help = "Promiscuous mode")]
    pub promiscuous: bool,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
}

#[derive(clap::Args)]
pub struct PacketSendArgs {
    #[arg(help = "Target host")]
    pub target: String,
    #[arg(long, help = "Source IP address")]
    pub src_ip: Option<String>,
    #[arg(long, help = "Source port")]
    pub src_port: Option<u16>,
    #[arg(long, help = "Destination port")]
    pub dst_port: Option<u16>,
    #[arg(long, help = "TCP flags (syn,ack,rst,fin,psh,urg)")]
    pub flags: Option<String>,
    #[arg(long, help = "Use ICMP instead of TCP/UDP")]
    pub icmp: bool,
    #[arg(long, help = "UDP mode")]
    pub udp: bool,
    #[arg(long, help = "Packet payload (hex string)")]
    pub payload: Option<String>,
    #[arg(long, help = "TTL/Hop limit")]
    pub ttl: Option<u8>,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct PacketDumpArgs {
    #[arg(help = "File to dump (pcap or raw packet data)")]
    pub file: String,
    #[arg(long, help = "Number of packets to show")]
    pub count: Option<usize>,
    #[arg(long, help = "Show only packet at index")]
    pub index: Option<usize>,
    #[arg(long, help = "Show hexdump only")]
    pub hex_only: bool,
    #[arg(long, help = "Show parsed headers only")]
    pub headers_only: bool,
    #[arg(long, help = "Bytes per line")]
    pub bytes_per_line: Option<usize>,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct PacketTracerouteArgs {
    #[arg(help = "Target host")]
    pub target: String,
    #[arg(long, default_value = "30", help = "Maximum hops")]
    pub max_hops: u8,
    #[arg(long, default_value = "3", help = "Number of probes per hop")]
    pub probes: u8,
    #[arg(long, help = "Use ICMP Echo Request (requires root/sudo)")]
    pub icmp: bool,
    #[arg(long, help = "Use UDP probes (default, no root required)")]
    pub udp: bool,
    #[arg(long, help = "Timeout in seconds")]
    pub timeout: Option<u64>,
    #[arg(long, help = "First TTL")]
    pub first_ttl: Option<u8>,
    #[arg(long, help = "Run probes in parallel")]
    pub parallel: bool,
    #[arg(long, help = "Disable reverse DNS lookup")]
    pub no_resolve: bool,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
}
