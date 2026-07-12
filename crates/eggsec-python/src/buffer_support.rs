use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyMemoryView};
use std::ffi::c_void;

/// Binary buffer with zero-copy support via PEP 3118 buffer protocol.
///
/// Wraps a `Vec<u8>` and exposes it to Python as a buffer object.
/// Supports `memoryview()`, `bytes()`, `hex()`, and direct buffer access.
#[pyclass(name = "BinaryBuffer")]
pub struct BinaryBufferPy {
    data: Vec<u8>,
    shape: isize,
}

#[pymethods]
impl BinaryBufferPy {
    #[new]
    fn new(data: Vec<u8>) -> Self {
        let shape = data.len() as isize;
        Self { data, shape }
    }

    /// Create from raw bytes.
    #[staticmethod]
    fn from_bytes(data: Vec<u8>) -> Self {
        let shape = data.len() as isize;
        Self { data, shape }
    }

    /// Create from hex string.
    #[staticmethod]
    fn from_hex(hex_str: &str) -> PyResult<Self> {
        let data = hex_decode(hex_str)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))?;
        let shape = data.len() as isize;
        Ok(Self { data, shape })
    }

    /// Number of bytes.
    fn __len__(&self) -> usize {
        self.data.len()
    }

    /// Return raw bytes.
    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new_bound(py, &self.data)
    }

    /// Return a memoryview over the buffer.
    fn memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        let bytes = PyBytes::new_bound(py, &self.data);
        PyMemoryView::from_bound(&bytes)
    }

    /// Hex-encoded representation of the data.
    fn hex(&self) -> String {
        hex_encode(&self.data)
    }

    /// Return a copy of the internal data as a Python bytes object.
    fn to_py_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new_bound(py, &self.data)
    }

    /// PEP 3118 buffer protocol: fill a Py_buffer for the caller.
    ///
    /// # Safety
    /// This function fills a raw `ffi::Py_buffer` and must follow PEP 3118 rules.
    unsafe fn __getbuffer__(
        slf: PyRef<'_, Self>,
        view: *mut pyo3::ffi::Py_buffer,
        _flags: i32,
    ) -> PyResult<()> {
        if view.is_null() {
            return Err(pyo3::exceptions::PyBufferError::new_err(
                "view is null",
            ));
        }

        let buf = &slf.data;
        let ptr = buf.as_ptr() as *const c_void;
        let len = buf.len() as isize;

        // Fill the buffer info
        (*view).obj = std::ptr::null_mut(); // PyO3 will set this
        (*view).buf = ptr as *mut c_void;
        (*view).len = len;
        (*view).readonly = 0; // mutable in principle, but we don't expose mutation
        (*view).itemsize = 1;
        (*view).format = b"b\0".as_ptr() as *mut _; // signed char
        (*view).ndim = 1;
        (*view).shape = std::ptr::addr_of!(slf.shape) as *mut _;
        (*view).strides = std::ptr::null_mut(); // contiguous
        (*view).suboffsets = std::ptr::null_mut();
        (*view).internal = std::ptr::null_mut();

        Ok(())
    }

    /// PEP 3118 buffer protocol: release the buffer.
    ///
    /// # Safety
    /// Called by Python when the buffer is no longer needed.
    unsafe fn __releasebuffer__(&self, _view: *mut pyo3::ffi::Py_buffer) {
        // Nothing to release — we don't allocate separate memory.
    }

    fn __repr__(&self) -> String {
        format!("BinaryBuffer(len={})", self.data.len())
    }

    fn __str__(&self) -> String {
        self.hex()
    }

    /// Equality check against another BinaryBuffer.
    fn __eq__(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl BinaryBufferPy {
    /// Crate-internal accessor for the raw data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Create from a slice, copying the data.
    pub fn from_slice(slice: &[u8]) -> Self {
        let shape = slice.len() as isize;
        Self {
            data: slice.to_vec(),
            shape,
        }
    }
}

const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

fn hex_encode(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 2);
    for &byte in data {
        s.push(HEX_CHARS[(byte >> 4) as usize] as char);
        s.push(HEX_CHARS[(byte & 0x0f) as usize] as char);
    }
    s
}

fn hex_decode(hex_str: &str) -> Result<Vec<u8>, String> {
    let hex_str = hex_str.trim();
    if hex_str.len() % 2 != 0 {
        return Err("Hex string must have even length".to_string());
    }
    let mut bytes = Vec::with_capacity(hex_str.len() / 2);
    for chunk in hex_str.as_bytes().chunks(2) {
        let high = from_hex_char(chunk[0])?;
        let low = from_hex_char(chunk[1])?;
        bytes.push((high << 4) | low);
    }
    Ok(bytes)
}

fn from_hex_char(c: u8) -> Result<u8, String> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(format!("Invalid hex character: {}", c as char)),
    }
}
