//! The `eggsec-gui` binary.
//!
//! The binary is a thin shell around [`eggsec_gui::run`]. All the behaviour
//! lives in the library so it can be unit-tested without a window.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = eggsec_gui::run() {
        eprintln!("eggsec-gui: {error}");
        std::process::exit(1);
    }
}
