use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

/// Paginated result iterator — fetches results in pages.
///
/// Wraps a pre-materialized list and provides page-based access
/// and standard Python iteration protocol.
#[pyclass(name = "PaginatedResults")]
pub struct PaginatedResultsPy {
    items: Vec<PyObject>,
    page_size: usize,
    total: usize,
    current_index: usize,
}

#[pymethods]
impl PaginatedResultsPy {
    #[new]
    #[pyo3(signature = (items, page_size=100))]
    fn new(items: Vec<PyObject>, page_size: usize) -> Self {
        let total = items.len();
        Self {
            items,
            page_size: if page_size == 0 { 100 } else { page_size },
            total,
            current_index: 0,
        }
    }

    /// Total number of items across all pages.
    fn __len__(&self) -> usize {
        self.total
    }

    /// Total number of pages.
    fn total_pages(&self) -> usize {
        if self.total == 0 {
            0
        } else {
            (self.total + self.page_size - 1) / self.page_size
        }
    }

    /// Get a specific page (0-indexed).
    fn get_page(&self, page: usize, py: Python) -> PyResult<PyObject> {
        let start = page * self.page_size;
        if start >= self.total {
            return Ok(PyList::empty_bound(py).into());
        }
        let end = std::cmp::min(start + self.page_size, self.total);
        let list = PyList::empty_bound(py);
        for item in &self.items[start..end] {
            list.append(item.clone_ref(py))?;
        }
        Ok(list.into())
    }

    /// Get a specific page as a dict with metadata.
    fn get_page_info(&self, page: usize, py: Python) -> PyResult<PyObject> {
        let start = page * self.page_size;
        let total_pages = self.total_pages();
        let actual_end = std::cmp::min(start + self.page_size, self.total);

        let page_items: PyObject = if start < self.total {
            let list = PyList::empty_bound(py);
            for item in &self.items[start..actual_end] {
                list.append(item.clone_ref(py))?;
            }
            list.into_any().unbind()
        } else {
            PyList::empty_bound(py).into_any().unbind()
        };

        let dict = PyDict::new_bound(py);
        dict.set_item("items", page_items)?;
        dict.set_item("page", page)?;
        dict.set_item("page_size", self.page_size)?;
        dict.set_item("total_pages", total_pages)?;
        dict.set_item("total_items", self.total)?;
        dict.set_item("has_next", page + 1 < total_pages)?;
        dict.set_item("has_prev", page > 0)?;
        Ok(dict.into())
    }

    /// Python iterator protocol: return self.
    fn __iter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Python iterator protocol: next item.
    fn __next__<'py>(
        mut slf: PyRefMut<'py, Self>,
        py: Python<'py>,
    ) -> PyResult<Option<PyObject>> {
        if slf.current_index >= slf.items.len() {
            return Ok(None);
        }
        let item = slf.items[slf.current_index].clone_ref(py);
        slf.current_index += 1;
        Ok(Some(item))
    }

    /// Reset the iterator position to the beginning.
    fn reset(&mut self) {
        self.current_index = 0;
    }

    /// Collect all items into a Python list.
    fn to_list(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for item in &self.items {
            list.append(item.clone_ref(py))?;
        }
        Ok(list.into())
    }

    /// Get item count (method alias for __len__).
    fn count(&self) -> usize {
        self.total
    }

    fn __repr__(&self) -> String {
        format!(
            "PaginatedResults(total={}, page_size={}, pages={})",
            self.total,
            self.page_size,
            self.total_pages()
        )
    }
}

/// Batch converter — converts Rust items to Python dicts in batches.
///
/// Takes a list of items that implement `ToPyObject` and returns
/// them grouped into batches for efficient processing.
pub fn batch_to_dicts<'py, T: ToPyObject>(
    py: Python<'py>,
    items: &[T],
    batch_size: usize,
) -> Vec<Vec<Bound<'py, PyAny>>> {
    if batch_size == 0 {
        return vec![items.iter().map(|i| i.to_object(py).into_bound(py)).collect()];
    }
    items
        .chunks(batch_size)
        .map(|chunk| chunk.iter().map(|i| i.to_object(py).into_bound(py)).collect())
        .collect()
}
