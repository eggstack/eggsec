use crate::packet::{hexdump, PacketInfo};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use tokio::sync::mpsc;

#[cfg(all(feature = "packet-inspection", unix))]
use pnet::datalink::{self, DataLinkReceiver, NetworkInterface};

pub struct PcapWriter {
    file: BufWriter<File>,
    snapshot_len: usize,
    pcap_hdr: [u8; 24],
}

impl PcapWriter {
    pub fn new(path: &str, snapshot_len: usize) -> Result<Self, std::io::Error> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        let magic: u32 = 0xa1b2c3d4;
        let version_major: u16 = 2;
        let version_minor: u16 = 4;
        let thiszone: i32 = 0;
        let sigfigs: u32 = 0;
        let snaplen: u32 = snapshot_len as u32;
        let network: u32 = 1;

        writer.write_all(&magic.to_le_bytes())?;
        writer.write_all(&version_major.to_le_bytes())?;
        writer.write_all(&version_minor.to_le_bytes())?;
        writer.write_all(&thiszone.to_le_bytes())?;
        writer.write_all(&sigfigs.to_le_bytes())?;
        writer.write_all(&snaplen.to_le_bytes())?;
        writer.write_all(&network.to_le_bytes())?;

        Ok(Self {
            file: writer,
            snapshot_len,
            pcap_hdr: [0; 24],
        })
    }

    pub fn write_packet(&mut self, data: &[u8]) -> std::io::Result<()> {
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        
        let len = data.len().min(self.snapshot_len);
        
        let pkt_hdr = [
            (ts.as_secs() as u32).to_le_bytes(),
            (ts.subsec_nanos() as u32).to_le_bytes(),
            (len as u32).to_le_bytes(),
            (data.len() as u32).to_le_bytes(),
        ].concat();
        
        self.file.write_all(&pkt_hdr)?;
        self.file.write_all(&data[..len])?;
        
        Ok(())
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub interface: String,
    pub filter: Option<String>,
    pub promiscuous: bool,
    pub snapshot_len: usize,
    pub timeout: Duration,
    pub max_packets: Option<usize>,
    pub save_to_file: Option<String>,
    pub validate_checksums: bool,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            interface: String::new(),
            filter: None,
            promiscuous: true,
            snapshot_len: 65535,
            timeout: Duration::from_secs(1),
            max_packets: None,
            save_to_file: None,
            validate_checksums: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureStats {
    pub packets_captured: usize,
    pub bytes_captured: usize,
    pub packets_dropped: usize,
    pub runtime_ms: u64,
}

pub struct PacketCapture {
    config: CaptureConfig,
    running: Arc<AtomicBool>,
    stats: CaptureStats,
}

impl PacketCapture {
    pub fn new(config: CaptureConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            stats: CaptureStats {
                packets_captured: 0,
                bytes_captured: 0,
                packets_dropped: 0,
                runtime_ms: 0,
            },
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn stats(&self) -> CaptureStats {
        self.stats.clone()
    }

    pub fn running(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    #[cfg(all(feature = "packet-inspection", unix))]
    pub async fn start(&mut self, sender: mpsc::Sender<PacketInfo>) -> Result<CaptureStats, CaptureError> {
        use std::time::Instant;
        use crossbeam::channel;

        if self.running.swap(true, Ordering::SeqCst) {
            return Err(CaptureError::AlreadyRunning);
        }

        let interface = self.get_interface()?;
        let mut rx = self.create_receiver(&interface)?;

        let pcap_path = self.config.save_to_file.clone();
        let mut pcap_writer = if let Some(ref path) = pcap_path {
            Some(PcapWriter::new(path, self.config.snapshot_len)?)
        } else {
            None
        };

        let start = Instant::now();
        let packets_received = Arc::new(AtomicUsize::new(0));
        let bytes_received = Arc::new(AtomicUsize::new(0));

        tracing::info!("Starting packet capture on interface: {}", self.config.interface);

        let (tx_packet, rx_packet) = channel::bounded::<Vec<u8>>(100);
        let running = self.running.clone();
        
        let _capture_thread = std::thread::spawn(move || {
            let mut receiver = rx;
            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                match receiver.next() {
                    Ok(packet) => {
                        if tx_packet.send(packet.to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        });

        loop {
            tokio::select! {
                biased;
                _ = tokio::time::sleep(Duration::from_millis(50)) => {
                    if !self.running.load(Ordering::SeqCst) {
                        break;
                    }
                }
                else => {}
            }
            
            match rx_packet.try_recv() {
                Ok(packet) => {
                    if let Some(ref mut writer) = pcap_writer {
                        let _ = writer.write_packet(&packet);
                    }
                    
                    let packet_info = Self::parse_packet_internal(&packet);
                    if sender.send(packet_info).await.is_err() {
                        break;
                    }

                    packets_received.fetch_add(1, Ordering::Relaxed);
                    bytes_received.fetch_add(packet.len(), Ordering::Relaxed);

                    if let Some(max) = self.config.max_packets {
                        if packets_received.load(Ordering::Relaxed) >= max {
                            break;
                        }
                    }
                }
                Err(crossbeam::channel::TryRecvError::Empty) => {
                    if !self.running.load(Ordering::SeqCst) {
                        break;
                    }
                }
                Err(crossbeam::channel::TryRecvError::Disconnected) => {
                    break;
                }
            }
        }

        self.running.store(false, Ordering::SeqCst);

        self.stats.packets_captured = packets_received.load(Ordering::Relaxed);
        self.stats.bytes_captured = bytes_received.load(Ordering::Relaxed);
        self.stats.runtime_ms = start.elapsed().as_millis() as u64;

        if let Some(ref mut writer) = pcap_writer {
            let _ = writer.flush();
        }

        tracing::info!(
            packets = self.stats.packets_captured,
            bytes = self.stats.bytes_captured,
            "Packet capture stopped"
        );

        Ok(self.stats.clone())
    }

    fn parse_packet_internal(data: &[u8]) -> PacketInfo {
        use crate::packet::parse::ParsedPacket;

        let timestamp = chrono::Utc::now();
        let hex = hexdump(data);

        let parsed = ParsedPacket::parse(data);

        PacketInfo {
            timestamp,
            ethernet: parsed.as_ref().and_then(|p| p.ethernet.clone()),
            ip: parsed.as_ref().and_then(|p| p.ip.clone()),
            transport: parsed.as_ref().and_then(|p| p.transport.clone()),
            app: parsed.as_ref().and_then(|p| p.app.clone()),
            raw_size: data.len(),
            hex_dump: hex,
        }
    }

    #[cfg(not(all(feature = "packet-inspection", unix)))]
    pub async fn start(&mut self, _sender: mpsc::Sender<PacketInfo>) -> Result<CaptureStats, CaptureError> {
        Err(CaptureError::RequiresRoot)
    }

    #[cfg(all(feature = "packet-inspection", unix))]
    fn get_interface(&self) -> Result<NetworkInterface, CaptureError> {
        let interfaces = datalink::interfaces();

        if self.config.interface.is_empty() {
            interfaces
                .into_iter()
                .find(|i| i.is_up() && !i.is_loopback() && !i.ips.is_empty())
                .ok_or(CaptureError::NoInterface)
        } else {
            interfaces
                .into_iter()
                .find(|i| i.name == self.config.interface)
                .ok_or(CaptureError::InterfaceNotFound(self.config.interface.clone()))
        }
    }

    #[cfg(all(feature = "packet-inspection", unix))]
    fn create_receiver(
        &self,
        interface: &NetworkInterface,
    ) -> Result<Box<dyn DataLinkReceiver>, CaptureError> {
        use pnet::datalink::Channel::Ethernet;

        let config = datalink::Config {
            read_timeout: Some(self.config.timeout),
            ..Default::default()
        };

        match datalink::channel(interface, config) {
            Ok(Ethernet(_tx, rx)) => Ok(rx),
            Ok(_) => Err(CaptureError::UnsupportedChannel),
            Err(e) => Err(CaptureError::ChannelError(e.to_string())),
        }
    }
}

#[cfg(all(feature = "packet-inspection", unix))]
pub fn list_interfaces() -> Vec<NetworkInterfaceInfo> {
    use pnet::datalink;

    datalink::interfaces()
        .into_iter()
        .map(|i| {
            let is_up = i.is_up();
            let is_loopback = i.is_loopback();
            NetworkInterfaceInfo {
                name: i.name,
                ips: i.ips.iter().map(|ip| ip.to_string()).collect(),
                mac: i.mac.map(|m| format!("{}", m)),
                is_up,
                is_loopback,
            }
        })
        .collect()
}

#[cfg(not(all(feature = "packet-inspection", unix)))]
pub fn list_interfaces() -> Vec<NetworkInterfaceInfo> {
    vec![]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceInfo {
    pub name: String,
    pub ips: Vec<String>,
    pub mac: Option<String>,
    pub is_up: bool,
    pub is_loopback: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("Capture already running")]
    AlreadyRunning,
    #[error("No suitable network interface found")]
    NoInterface,
    #[error("Interface not found: {0}")]
    InterfaceNotFound(String),
    #[error("Packet capture requires root privileges")]
    RequiresRoot,
    #[error("Unsupported channel type")]
    UnsupportedChannel,
    #[error("Failed to create channel: {0}")]
    ChannelError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub struct CaptureBuilder {
    config: CaptureConfig,
}

impl CaptureBuilder {
    pub fn new() -> Self {
        Self {
            config: CaptureConfig::default(),
        }
    }

    pub fn interface(mut self, interface: impl Into<String>) -> Self {
        self.config.interface = interface.into();
        self
    }

    pub fn filter(mut self, filter: impl Into<String>) -> Self {
        self.config.filter = Some(filter.into());
        self
    }

    pub fn promiscuous(mut self, promiscuous: bool) -> Self {
        self.config.promiscuous = promiscuous;
        self
    }

    pub fn snapshot_len(mut self, len: usize) -> Self {
        self.config.snapshot_len = len;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    pub fn max_packets(mut self, max: usize) -> Self {
        self.config.max_packets = Some(max);
        self
    }

    pub fn save_to_file(mut self, path: impl Into<String>) -> Self {
        self.config.save_to_file = Some(path.into());
        self
    }

    pub fn build(self) -> PacketCapture {
        PacketCapture::new(self.config)
    }
}

impl Default for CaptureBuilder {
    fn default() -> Self {
        Self::new()
    }
}
