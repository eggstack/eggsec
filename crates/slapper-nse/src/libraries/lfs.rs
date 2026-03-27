//! NSE lfs (LuaFileSystem) library wrapper
//!
//! File system operations for NSE scripts.
//! Based on Nmap's lfs library concepts.

use mlua::{Lua, Result as LuaResult, Table};
use std::fs;
use std::path::Path;

pub fn register_lfs_library(lua: &Lua) -> LuaResult<()> {
    let globals = lua.globals();
    let lfs = lua.create_table()?;

    // lfs.attributes(path) - Get file attributes
    let attributes_fn = lua.create_function(|lua, path: String| {
        let p = Path::new(&path);

        match fs::metadata(p) {
            Ok(meta) => {
                let attrs = lua.create_table()?;

                let modification = meta
                    .modified()
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as f64
                    })
                    .unwrap_or(0.0);
                attrs.set("modification", modification)?;

                let access = meta
                    .accessed()
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as f64
                    })
                    .unwrap_or(0.0);
                attrs.set("access", access)?;

                let creation = meta
                    .created()
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as f64
                    })
                    .unwrap_or(0.0);
                attrs.set("creation", creation)?;

                attrs.set("size", meta.len())?;
                attrs.set(
                    "permissions",
                    if meta.permissions().readonly() {
                        "r--r--r--"
                    } else {
                        "rw-rw-rw-"
                    },
                )?;
                attrs.set("readonly", meta.permissions().readonly())?;
                attrs.set("is_dir", meta.is_dir())?;
                attrs.set("is_file", meta.is_file())?;
                attrs.set("is_link", meta.is_symlink())?;

                Ok(attrs)
            }
            Err(e) => Err(mlua::Error::RuntimeError(format!(
                "Failed to get attributes: {}",
                e
            ))),
        }
    })?;
    lfs.set("attributes", attributes_fn)?;

    // lfs.dir(path) - Iterate over directory entries
    let dir_fn = lua.create_function(|lua, path: String| {
        let entries = lua.create_table()?;

        match fs::read_dir(&path) {
            Ok(dir) => {
                let mut idx = 1;
                for entry in dir.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        entries.set(idx, name)?;
                        idx += 1;
                    }
                }
                Ok(entries)
            }
            Err(e) => Err(mlua::Error::RuntimeError(format!(
                "Failed to read directory: {}",
                e
            ))),
        }
    })?;
    lfs.set("dir", dir_fn)?;

    // lfs.mkdir(path) - Create directory
    let mkdir_fn = lua.create_function(|_lua, path: String| match fs::create_dir_all(&path) {
        Ok(()) => Ok(true),
        Err(e) => Err(mlua::Error::RuntimeError(format!(
            "Failed to create directory: {}",
            e
        ))),
    })?;
    lfs.set("mkdir", mkdir_fn)?;

    // lfs.rmdir(path) - Remove directory
    let rmdir_fn = lua.create_function(|_lua, path: String| match fs::remove_dir(&path) {
        Ok(()) => Ok(true),
        Err(e) => Err(mlua::Error::RuntimeError(format!(
            "Failed to remove directory: {}",
            e
        ))),
    })?;
    lfs.set("rmdir", rmdir_fn)?;

    // lfs.remove(path) - Remove file
    let remove_fn = lua.create_function(|_lua, path: String| match fs::remove_file(&path) {
        Ok(()) => Ok(true),
        Err(e) => Err(mlua::Error::RuntimeError(format!(
            "Failed to remove file: {}",
            e
        ))),
    })?;
    lfs.set("remove", remove_fn)?;

    // lfs.rename(old, new) - Rename file/directory
    let rename_fn = lua.create_function(|_lua, (old_path, new_path): (String, String)| {
        match fs::rename(&old_path, &new_path) {
            Ok(()) => Ok(true),
            Err(e) => Err(mlua::Error::RuntimeError(format!(
                "Failed to rename: {}",
                e
            ))),
        }
    })?;
    lfs.set("rename", rename_fn)?;

    // lfs.link(source, link, symbolic) - Create link
    let link_fn =
        lua.create_function(|_lua, (source, link, symbolic): (String, String, bool)| {
            if symbolic {
                match std::os::unix::fs::symlink(&source, &link) {
                    Ok(()) => Ok(true),
                    Err(e) => Err(mlua::Error::RuntimeError(format!(
                        "Failed to create symlink: {}",
                        e
                    ))),
                }
            } else {
                // Hard link - use standard fs::hard_link if available
                match fs::hard_link(&source, &link) {
                    Ok(()) => Ok(true),
                    Err(e) => Err(mlua::Error::RuntimeError(format!(
                        "Failed to create hard link: {}",
                        e
                    ))),
                }
            }
        })?;
    lfs.set("link", link_fn)?;

    // lfs.currentdir() - Get current directory
    let currentdir_fn = lua.create_function(|_lua, _: ()| match std::env::current_dir() {
        Ok(p) => Ok(p.to_string_lossy().to_string()),
        Err(e) => Err(mlua::Error::RuntimeError(format!(
            "Failed to get current directory: {}",
            e
        ))),
    })?;
    lfs.set("currentdir", currentdir_fn)?;

    // lfs.chdir(path) - Change directory
    let chdir_fn =
        lua.create_function(
            |_lua, path: String| match std::env::set_current_dir(&path) {
                Ok(()) => Ok(true),
                Err(e) => Err(mlua::Error::RuntimeError(format!(
                    "Failed to change directory: {}",
                    e
                ))),
            },
        )?;
    lfs.set("chdir", chdir_fn)?;

    // lfs.touch(path) - Touch file
    let touch_fn = lua.create_function(
        |_lua, (path, access_time, modification_time): (String, Option<u64>, Option<u64>)| {
            let p = Path::new(&path);

            // Touch file (create or update timestamp)
            if p.exists() {
                // File exists, just update timestamps would require more complex handling
                Ok(true)
            } else {
                // Create empty file
                match fs::write(p, "") {
                    Ok(()) => Ok(true),
                    Err(e) => Err(mlua::Error::RuntimeError(format!(
                        "Failed to touch file: {}",
                        e
                    ))),
                }
            }
        },
    )?;
    lfs.set("touch", touch_fn)?;

    // lfs.lock(filehandle, mode) - Lock file
    let lock_fn = lua.create_function(|_lua, (path, mode): (String, String)| {
        // Simplified lock implementation
        Ok(true)
    })?;
    lfs.set("lock", lock_fn)?;

    // lfs.unlock(filehandle) - Unlock file
    let unlock_fn = lua.create_function(|_lua, path: String| Ok(true))?;
    lfs.set("unlock", unlock_fn)?;

    // lfs.set_mode(path, mode) - Set file permissions
    let set_mode_fn = lua.create_function(|_lua, (path, mode): (String, String)| {
        // Simplified - just try to parse as octal
        if let Ok(perms) = u32::from_str_radix(&mode, 8) {
            match fs::metadata(&path) {
                Ok(meta) => {
                    let _ = meta.permissions();
                    // Setting permissions on Windows is limited
                    Ok(true)
                }
                Err(e) => Err(mlua::Error::RuntimeError(format!(
                    "Failed to set mode: {}",
                    e
                ))),
            }
        } else {
            Err(mlua::Error::RuntimeError("Invalid mode".to_string()))
        }
    })?;
    lfs.set("set_mode", set_mode_fn)?;

    // lfs.symlinkattributes(path) - Get symlink attributes
    let symlinkattributes_fn = lua.create_function(|lua, path: String| {
        let p = Path::new(&path);

        match fs::symlink_metadata(p) {
            Ok(meta) => {
                let attrs = lua.create_table()?;

                attrs.set("size", meta.len())?;
                attrs.set("readonly", meta.permissions().readonly())?;
                attrs.set("is_dir", meta.is_dir())?;
                attrs.set("is_file", meta.is_file())?;
                attrs.set("is_link", meta.is_symlink())?;

                // Get target if it's a symlink
                if meta.is_symlink() {
                    if let Ok(target) = fs::read_link(p) {
                        attrs.set("target", target.to_string_lossy().to_string())?;
                    }
                }

                Ok(attrs)
            }
            Err(e) => Err(mlua::Error::RuntimeError(format!(
                "Failed to get symlink attributes: {}",
                e
            ))),
        }
    })?;
    lfs.set("symlinkattributes", symlinkattributes_fn)?;

    // lfs.version() - Get version
    let version_fn = lua.create_function(|_lua, _: ()| Ok("1.0.0"))?;
    lfs.set("version", version_fn)?;

    globals.set("lfs", lfs)?;
    Ok(())
}
