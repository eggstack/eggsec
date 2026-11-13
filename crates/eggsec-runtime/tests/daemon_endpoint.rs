//! The canonical daemon endpoint through the crate's **public** surface.
//!
//! The unit tests beside the constant live in
//! `crates/eggsec-runtime/src/daemon_endpoint.rs`; they are the one place
//! that pins the constant's value and the environment variable's name. These
//! tests assert the same precedence rule through the re-exports the other
//! surfaces depend on, so that a re-export cannot rot silently.

use eggsec_runtime::{
    daemon_socket_path_from_env, resolve_daemon_socket_path, DEFAULT_DAEMON_SOCKET_PATH,
};

#[test]
fn an_absent_variable_resolves_to_the_built_in_default() {
    assert_eq!(resolve_daemon_socket_path(None), DEFAULT_DAEMON_SOCKET_PATH);
}

#[test]
fn an_empty_value_resolves_to_the_built_in_default() {
    assert_eq!(
        resolve_daemon_socket_path(Some("")),
        DEFAULT_DAEMON_SOCKET_PATH
    );
}

#[test]
fn a_whitespace_only_value_resolves_to_the_built_in_default() {
    assert_eq!(
        resolve_daemon_socket_path(Some("   \t ")),
        DEFAULT_DAEMON_SOCKET_PATH
    );
}

#[test]
fn an_explicit_value_resolves_verbatim() {
    assert_eq!(
        resolve_daemon_socket_path(Some("/run/user/1000/eggsec.sock")),
        "/run/user/1000/eggsec.sock"
    );
}

#[test]
fn an_explicit_value_is_trimmed() {
    assert_eq!(
        resolve_daemon_socket_path(Some(" /run/user/1000/eggsec.sock ")),
        "/run/user/1000/eggsec.sock"
    );
}

#[test]
fn the_environment_reader_agrees_with_the_resolver() {
    // The reader applies the resolver to the process environment. The test
    // process does not set the variable, so the two must agree on the
    // built-in default; the environment-dependent paths are covered by the
    // daemon's subprocess test.
    if std::env::var_os(eggsec_runtime::DAEMON_SOCKET_ENV_VAR).is_none() {
        assert_eq!(
            daemon_socket_path_from_env(),
            DEFAULT_DAEMON_SOCKET_PATH.to_owned()
        );
    }
}
