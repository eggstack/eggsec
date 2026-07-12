use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::path::PathBuf;

/// PathLike helper — accepts str or pathlib.Path.
///
/// Tries str extraction first, then falls back to `path.__fspath__()`.
pub fn resolve_path(path: &Bound<'_, PyAny>) -> PyResult<PathBuf> {
    // Try str first
    if let Ok(s) = path.extract::<String>() {
        return Ok(PathBuf::from(s));
    }
    // Try pathlib.Path via __fspath__()
    let fspath = path.getattr("__fspath__")?;
    let result = fspath.call0()?;
    let pystr = result.downcast::<pyo3::types::PyString>().map_err(|_| {
        pyo3::exceptions::PyTypeError::new_err("__fspath__() must return str or bytes")
    })?;
    let s = pystr
        .to_str()
        .map_err(|_| pyo3::exceptions::PyUnicodeDecodeError::new_err("path is not valid UTF-8"))?;
    Ok(PathBuf::from(s))
}

/// Convert a chrono DateTime<Utc> to a Python datetime.datetime.
pub fn py_datetime(dt: chrono::DateTime<chrono::Utc>) -> PyResult<PyObject> {
    Python::with_gil(|py| {
        let datetime_mod = py.import_bound("datetime")?;
        let utc = datetime_mod.getattr("timezone")?.getattr("utc")?;
        let timestamp = dt.timestamp() as f64 + dt.timestamp_subsec_micros() as f64 / 1_000_000.0;
        let naive = datetime_mod
            .getattr("datetime")?
            .call_method1("utcfromtimestamp", (timestamp,))?;
        let aware = datetime_mod.getattr("datetime")?.call1((
            naive.getattr("year")?,
            naive.getattr("month")?,
            naive.getattr("day")?,
            naive.getattr("hour")?,
            naive.getattr("minute")?,
            naive.getattr("second")?,
            naive.getattr("microsecond")?,
            utc,
        ))?;
        Ok(aware.into())
    })
}

/// Convert a Python object to a JSON-serializable serde_json::Value.
pub fn pyobj_to_json(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    use pyo3::types::PyNone;
    if obj.is_instance_of::<PyNone>() {
        return Ok(serde_json::Value::Null);
    }
    if let Ok(v) = obj.extract::<bool>() {
        return Ok(serde_json::Value::Bool(v));
    }
    if let Ok(v) = obj.extract::<i64>() {
        return Ok(serde_json::Value::Number(v.into()));
    }
    if let Ok(v) = obj.extract::<u64>() {
        return Ok(serde_json::Value::Number(v.into()));
    }
    if let Ok(v) = obj.extract::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(v) {
            return Ok(serde_json::Value::Number(n));
        }
    }
    if let Ok(v) = obj.extract::<String>() {
        return Ok(serde_json::Value::String(v));
    }
    if let Ok(d) = obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (key, value) in d.iter() {
            if let Ok(key_str) = key.extract::<String>() {
                map.insert(key_str, pyobj_to_json(py, &value)?);
            }
        }
        return Ok(serde_json::Value::Object(map));
    }
    if let Ok(list) = obj.downcast::<pyo3::types::PyList>() {
        let mut arr = Vec::new();
        for item in list.iter() {
            arr.push(pyobj_to_json(py, &item)?);
        }
        return Ok(serde_json::Value::Array(arr));
    }
    Ok(serde_json::Value::String(format!("{:?}", obj)))
}
