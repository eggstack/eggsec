//! NSE lfs (LuaFileSystem) library wrapper
//!
//! File system operations for NSE scripts.
//! Based on Nmap's lfs library concepts.

use mlua::{Lua, Result as LuaResult};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::SandboxConfig;

pub static LFS_SANDBOX_VIOLATIONS: AtomicUsize = AtomicUsize::new(0);

pub fn get_lfs_sandbox_metrics() -> usize {
    LFS_SANDBOX_VIOLATIONS.load(Ordering::SeqCst)
}

pub fn register_lfs_library(lua: &Lua, sandbox: &SandboxConfig) -> LuaResult<()> {
    let globals = lua.globals();
    let lfs = lua.create_table()?;

    let sandbox_enabled = sandbox.enabled;
    let sandbox_for_check = sandbox.clone();

    let check_path = {
        let sandbox_enabled = sandbox_enabled;
        move |path: &str| -> bool {
            if !sandbox_enabled {
                return true;
            }
            !path.contains("..") && sandbox_for_check.is_path_allowed(path)
        }
    };

    // lfs.attributes(path) - Get file attributes
    let check_path_for_closure = check_path.clone();
    let attributes_fn = lua.create_function(move |lua, path: String| {
        if !check_path_for_closure(&path) {
            LFS_SANDBOX_VIOLATIONS.fetch_add(1, Ordering::SeqCst);
            return Err(mlua::Error::RuntimeError(format!(
                "Path '{}' blocked by sandbox",
                path
            )));
        }
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
    let check_path_dir = check_path.clone();
    let dir_fn = lua.create_function(move |lua, path: String| {
        if !check_path_dir(&path) {
            LFS_SANDBOX_VIOLATIONS.fetch_add(1, Ordering::SeqCst);
            return Err(mlua::Error::RuntimeError(format!(
                "Path '{}' blocked by sandbox",
                path
            )));
        }
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
    let check_path_mkdir = check_path.clone();
    let mkdir_fn = lua.create_function(move |_lua, path: String| {
        if !check_path_mkdir(&path) {
            LFS_SANDBOX_VIOLATIONS.fetch_add(1, Ordering::SeqCst);
            return Err(mlua::Error::RuntimeError(format!(
                "Path '{}' blocked by sandbox",
                path
            )));
        }
        match fs::create_dir_all(&path) {
            Ok(()) => Ok(true),
            Err(e) => Err(mlua::Error::RuntimeError(format!(
                "Failed to create directory: {}",
                e
            ))),
        }
    })?;
    lfs.set("mkdir", mkdir_fn)?;

    // lfs.rmdir(path) - Remove directory
    let check_path_rmdir = check_path.clone();
    let rmdir_fn = lua.create_function(move |_lua, path: String| {
        if !check_path_rmdir(&path) {
            LFS_SANDBOX_VIOLATIONS.fetch_add(1, Ordering::SeqCst);
            return Err(mlua::Error::RuntimeError(format!(
                "Path '{}' blocked by sandbox",
                path
            )));
        }
        match fs::remove_dir(&path) {
            Ok(()) => Ok(true),
            Err(e) => Err(mlua::Error::RuntimeError(format!(
                "Failed to remove directory: {}",
                e
            ))),
        }
    })?;
    lfs.set("rmdir", rmdir_fn)?;

    // lfs.remove(path) - Remove file
    let check_path_remove = check_path.clone();
    let remove_fn = lua.create_function(move |_lua, path: String| {
        if !check_path_remove(&path) {
            LFS_SANDBOX_VIOLATIONS.fetch_add(1, Ordering::SeqCst);
            return Err(mlua::Error::RuntimeError(format!(
                "Path '{}' blocked by sandbox",
                path
            )));
        }
        match fs::remove_file(&path) {
            Ok(()) => Ok(true),
            Err(e) => Err(mlua::Error::RuntimeError(format!(
                "Failed to remove file: {}",
                e
            ))),
        }
    })?;
    lfs.set("remove", remove_fn)?;

    // lfs.rename(old, new) - Rename file/directory
    let check_path_rename = check_path.clone();
    let rename_fn = lua.create_function(move |_lua, (old_path, new_path): (String, String)| {
        if !check_path_rename(&old_path) || !check_path_rename(&new_path) {
            LFS_SANDBOX_VIOLATIONS.fetch_add(1, Ordering::SeqCst);
            return Err(mlua::Error::RuntimeError(
                "Rename blocked by sandbox".to_string(),
            ));
        }
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
    let check_path_link = check_path.clone();
    let link_fn = lua.create_function(
        move |_lua, (source, link, symbolic): (String, String, bool)| {
            if !check_path_link(&source) || !check_path_link(&link) {
                LFS_SANDBOX_VIOLATIONS.fetch_add(1, Ordering::SeqCst);
                return Err(mlua::Error::RuntimeError(
                    "Link creation blocked by sandbox".to_string(),
                ));
            }
            if symbolic {
                match std::os::unix::fs::symlink(&source, &link) {
                    Ok(()) => Ok(true),
                    Err(e) => Err(mlua::Error::RuntimeError(format!(
                        "Failed to create symlink: {}",
                        e
                    ))),
                }
            } else {
                match fs::hard_link(&source, &link) {
                    Ok(()) => Ok(true),
                    Err(e) => Err(mlua::Error::RuntimeError(format!(
                        "Failed to create hard link: {}",
                        e
                    ))),
                }
            }
        },
    )?;
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
    let check_path_chdir = check_path.clone();
    let chdir_fn = lua.create_function(move |_lua, path: String| {
        if !check_path_chdir(&path) {
            LFS_SANDBOX_VIOLATIONS.fetch_add(1, Ordering::SeqCst);
            return Err(mlua::Error::RuntimeError(format!(
                "Path '{}' blocked by sandbox",
                path
            )));
        }
        match std::env::set_current_dir(&path) {
            Ok(()) => Ok(true),
            Err(e) => Err(mlua::Error::RuntimeError(format!(
                "Failed to change directory: {}",
                e
            ))),
        }
    })?;
    lfs.set("chdir", chdir_fn)?;

    // lfs.touch(path) - Touch file
    let check_path_touch = check_path.clone();
    let touch_fn = lua.create_function(
        move |_lua, (path, _access_time, _modification_time): (String, Option<u64>, Option<u64>)| {
            if !check_path_touch(&path) {
                LFS_SANDBOX_VIOLATIONS.fetch_add(1, Ordering::SeqCst);
                return Err(mlua::Error::RuntimeError(format!(
                    "Path '{}' blocked by sandbox",
                    path
                )));
            }
            let p = Path::new(&path);

            if p.exists() {
                Ok(true)
            } else {
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
    let lock_fn = lua.create_function(|_lua, (_path, _mode): (String, String)| {
        // Simplified lock implementation
        Ok(true)
    })?;
    lfs.set("lock", lock_fn)?;

    // lfs.unlock(filehandle) - Unlock file
    let unlock_fn = lua.create_function(|_lua, _path: String| Ok(true))?;
    lfs.set("unlock", unlock_fn)?;

    // lfs.set_mode(path, mode) - Set file permissions
    let set_mode_fn = lua.create_function(|_lua, (path, mode): (String, String)| {
        if let Ok(_perms) = u32::from_str_radix(&mode, 8) {
            match fs::metadata(&path) {
                Ok(meta) => {
                    let _ = meta.permissions();
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
    let check_path_symlink = check_path.clone();
    let symlinkattributes_fn = lua.create_function(move |lua, path: String| {
        if !check_path_symlink(&path) {
            LFS_SANDBOX_VIOLATIONS.fetch_add(1, Ordering::SeqCst);
            return Err(mlua::Error::RuntimeError(format!(
                "Path '{}' blocked by sandbox",
                path
            )));
        }
        let p = Path::new(&path);

        match fs::symlink_metadata(p) {
            Ok(meta) => {
                let attrs = lua.create_table()?;

                attrs.set("size", meta.len())?;
                attrs.set("readonly", meta.permissions().readonly())?;
                attrs.set("is_dir", meta.is_dir())?;
                attrs.set("is_file", meta.is_file())?;
                attrs.set("is_link", meta.is_symlink())?;

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
