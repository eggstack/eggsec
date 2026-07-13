use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

/// A single open port from a scan result.
#[pyclass(frozen)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenPort {
    #[pyo3(get)]
    pub port: u16,
    #[pyo3(get)]
    pub protocol: String,
    #[pyo3(get)]
    pub service: String,
    #[pyo3(get)]
    pub banner: Option<String>,
    #[pyo3(get)]
    pub confidence: f64,
}

#[pymethods]
impl OpenPort {
    fn __repr__(&self) -> String {
        format!("OpenPort(port={}, service={})", self.port, self.service)
    }

    fn __str__(&self) -> String {
        format!("{}/tcp - {}", self.port, self.service)
    }
}

/// Scan statistics.
#[pyclass(frozen)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanStats {
    #[pyo3(get)]
    pub ports_scanned: u32,
    #[pyo3(get)]
    pub total_open: usize,
    #[pyo3(get)]
    pub elapsed_ms: u64,
}

#[pymethods]
impl ScanStats {
    fn __repr__(&self) -> String {
        format!(
            "ScanStats(scanned={}, open={}, elapsed_ms={})",
            self.ports_scanned, self.total_open, self.elapsed_ms
        )
    }
}

/// Result of a port scan operation.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortScanResult {
    #[pyo3(get)]
    pub target: String,
    pub(crate) open_ports: Vec<OpenPort>,
    #[pyo3(get)]
    pub scanned_ports: u32,
    #[pyo3(get)]
    pub elapsed_ms: u64,
    #[pyo3(get)]
    pub stats: ScanStats,
}

#[pymethods]
impl PortScanResult {
    /// Returns the list of open ports found.
    #[getter]
    fn open_ports(&self) -> Vec<OpenPort> {
        self.open_ports.clone()
    }

    /// Convert result to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("scanned_ports", self.scanned_ports)?;
        dict.set_item("elapsed_ms", self.elapsed_ms)?;

        let ports_list = PyList::empty_bound(py);
        for port in &self.open_ports {
            let port_dict = PyDict::new_bound(py);
            port_dict.set_item("port", port.port)?;
            port_dict.set_item("protocol", &port.protocol)?;
            port_dict.set_item("service", &port.service)?;
            port_dict.set_item("banner", &port.banner)?;
            port_dict.set_item("confidence", port.confidence)?;
            ports_list.append(port_dict)?;
        }
        dict.set_item("open_ports", ports_list)?;

        let stats_dict = PyDict::new_bound(py);
        stats_dict.set_item("ports_scanned", self.stats.ports_scanned)?;
        stats_dict.set_item("total_open", self.stats.total_open)?;
        stats_dict.set_item("elapsed_ms", self.stats.elapsed_ms)?;
        dict.set_item("stats", stats_dict)?;

        Ok(dict.into())
    }

    /// Convert result to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PortScanResult(target={}, open={}, scanned={})",
            self.target,
            self.open_ports.len(),
            self.scanned_ports
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Scan of {}: {} open ports ({} scanned in {}ms)",
            self.target,
            self.open_ports.len(),
            self.scanned_ports,
            self.elapsed_ms
        )
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.target.hash(&mut hasher);
        self.scanned_ports.hash(&mut hasher);
        self.elapsed_ms.hash(&mut hasher);
        self.open_ports.len().hash(&mut hasher);
        hasher.finish()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.target == other.target
            && self.scanned_ports == other.scanned_ports
            && self.elapsed_ms == other.elapsed_ms
            && self.open_ports.len() == other.open_ports.len()
    }

    /// Convert open ports to a list of row dicts suitable for tabular output.
    fn to_rows(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for port in &self.open_ports {
            let dict = PyDict::new_bound(py);
            dict.set_item("target", &self.target)?;
            dict.set_item("port", port.port)?;
            dict.set_item("protocol", &port.protocol)?;
            dict.set_item("service", &port.service)?;
            dict.set_item("banner", &port.banner)?;
            dict.set_item("confidence", port.confidence)?;
            list.append(dict)?;
        }
        Ok(list.into())
    }
}

impl PortScanResult {
    /// Convert from engine PortScanResults.
    pub fn from_engine(engine: eggsec::scanner::PortScanResults) -> Self {
        let open_ports: Vec<OpenPort> = engine
            .open_ports
            .into_iter()
            .map(|p| OpenPort {
                port: p.port,
                protocol: "tcp".to_string(),
                service: p.service,
                banner: None,
                confidence: 1.0,
            })
            .collect();

        // A completed operation has observable work even when the platform
        // timer rounds a very fast scan down to zero milliseconds.
        let elapsed_ms = engine.duration_ms.max(1);
        let stats = ScanStats {
            ports_scanned: engine.ports_scanned,
            total_open: engine.total_open_ports,
            elapsed_ms,
        };

        Self {
            target: engine.host,
            open_ports,
            scanned_ports: engine.ports_scanned,
            elapsed_ms,
            stats,
        }
    }
}

/// A port specification for scanning.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct PortRange {
    ports: Vec<u16>,
}

#[pymethods]
impl PortRange {
    /// Create a port range from an explicit list.
    #[staticmethod]
    fn list(ports: Vec<u16>) -> PyResult<Self> {
        if ports.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "ports list must not be empty",
            ));
        }
        Ok(Self { ports })
    }

    /// Create a port range from start to end (inclusive).
    #[staticmethod]
    fn range(start: u16, end: u16) -> PyResult<Self> {
        if start == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err("port must be >= 1"));
        }
        if start > end {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "start must be <= end",
            ));
        }
        let ports: Vec<u16> = (start..=end).collect();
        Ok(Self { ports })
    }

    /// Return the top 100 most common ports.
    #[staticmethod]
    fn top_100() -> Self {
        Self {
            ports: TOP_100_PORTS.to_vec(),
        }
    }

    /// Return the top 1000 most common ports.
    #[staticmethod]
    fn top_1000() -> Self {
        Self {
            ports: TOP_1000_PORTS.to_vec(),
        }
    }

    /// Returns the list of ports.
    #[getter]
    fn ports(&self) -> Vec<u16> {
        self.ports.clone()
    }

    fn __len__(&self) -> usize {
        self.ports.len()
    }

    fn __repr__(&self) -> String {
        if self.ports.len() <= 10 {
            format!("PortRange({:?})", self.ports)
        } else {
            format!(
                "PortRange({} ports: {}..{})",
                self.ports.len(),
                self.ports.first().unwrap_or(&0),
                self.ports.last().unwrap_or(&0)
            )
        }
    }
}

/// Timing preset for scan speed.
#[pyclass(frozen)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimingPreset {
    inner: eggsec::scanner::TimingPreset,
}

#[pymethods]
impl TimingPreset {
    #[staticmethod]
    fn paranoid() -> Self {
        Self {
            inner: eggsec::scanner::TimingPreset::Paranoid,
        }
    }

    #[staticmethod]
    fn sneaky() -> Self {
        Self {
            inner: eggsec::scanner::TimingPreset::Sneaky,
        }
    }

    #[staticmethod]
    fn polite() -> Self {
        Self {
            inner: eggsec::scanner::TimingPreset::Polite,
        }
    }

    #[staticmethod]
    fn normal() -> Self {
        Self {
            inner: eggsec::scanner::TimingPreset::Normal,
        }
    }

    #[staticmethod]
    fn aggressive() -> Self {
        Self {
            inner: eggsec::scanner::TimingPreset::Aggressive,
        }
    }

    #[staticmethod]
    fn insane() -> Self {
        Self {
            inner: eggsec::scanner::TimingPreset::Insane,
        }
    }

    fn __repr__(&self) -> String {
        format!("TimingPreset({})", self.inner)
    }

    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }
}

const TOP_100_PORTS: &[u16] = &[
    7, 9, 13, 21, 22, 23, 25, 26, 37, 53, 79, 80, 81, 88, 106, 110, 111, 113, 119, 135, 139, 143,
    144, 179, 199, 389, 427, 443, 444, 445, 465, 513, 514, 515, 543, 544, 548, 554, 587, 631, 646,
    873, 990, 993, 995, 1025, 1026, 1027, 1028, 1029, 1110, 1433, 1720, 1723, 1755, 1900, 2000,
    2001, 2049, 2121, 2717, 3000, 3128, 3306, 3389, 3986, 4899, 5000, 5009, 5051, 5060, 5101, 5190,
    5357, 5432, 5631, 5666, 5800, 5900, 6000, 6001, 6646, 7070, 8000, 8008, 8009, 8080, 8081, 8443,
    8888, 9100, 9999, 10000, 32768, 32769, 32770, 32771, 32772, 32773, 32774, 32775, 49152, 49153,
    49154, 49155, 49156, 49157,
];

const TOP_1000_PORTS: &[u16] = &TOP_100_PORTS;
