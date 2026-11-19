fn main() {
    // `tauri_build::build()` reads metadata that the `tauri` crate's own build
    // script publishes. When the `shell` feature is off — the
    // `--no-default-features` profile that `make check` runs — the `tauri`
    // crate is not in the graph, that metadata does not exist, and the call
    // panics. The shell feature is the only thing that brings `tauri` in, so
    // gating on it keeps both profiles building.
    if std::env::var_os("CARGO_FEATURE_SHELL").is_some() {
        tauri_build::build()
    }
}
