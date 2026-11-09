//! zip crate 9.x helpers: entry names are validated and returned as `Result`.

use crate::{MobileError, Result};
use std::io::{Read, Seek};
use zip::read::ZipFile;

pub(crate) fn entry_name<R: Read + Seek>(entry: &ZipFile<'_, R>) -> Result<String> {
    entry
        .name()
        .map(|name| name.into_owned())
        .map_err(|e| MobileError::Validation(format!("invalid zip entry name: {}", e)))
}

pub(crate) fn entry_name_ok<R: Read + Seek>(entry: &ZipFile<'_, R>) -> Option<String> {
    entry.name().ok().map(|name| name.into_owned())
}
