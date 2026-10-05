//! Payload corpora — engine-side dispatch.
//!
//! Phase G split this module in two along a seam that already existed in the
//! source. The 34 pure-data payload modules live in the leaf crate
//! `eggsec-payloads`; the 6 that generate payloads by performing live `reqwest`
//! probing — [`graphql`], [`grpc`], [`idor`], [`jwt`], [`oauth`], [`ssti`] —
//! stay here, because probing is an engine concern.
//!
//! This file owns the **union**: the dispatch across all 40 [`PayloadType`]
//! variants, and the cross-variant caches that span them. Those caches cannot
//! live in `eggsec-payloads`, because building them needs the 6 probe variants
//! this crate owns. See the crate docs for why that matters.
//!
//! Both the corpus types and the cache accessors are re-exported below, so
//! `eggsec::fuzzer::payloads::*` and `eggsec::fuzzer::{PayloadType, Payload,
//! get_payloads, …}` keep resolving exactly as before. The re-exports are a
//! **permanent** facade, not transitional scaffolding.

// The 6 live-probe payload types. These take a `&reqwest::Client` and perform
// asynchronous probing, so they cannot move into a crate with no network
// surface. `eggsec-payloads` panics rather than stubbing them — see
// `engine_owned_probe_payloads` there.
pub mod graphql;
pub mod grpc;
pub mod idor;
pub mod jwt;
pub mod oauth;
pub mod ssti;

// Corpus types, re-exported from the leaf crate. Everything below refers to
// these; no consumer import changes as a result of the extraction.
pub use eggsec_payloads::{Payload, PayloadType, Severity};

/// Everything else from the corpus crate: the 34 data modules, `macros`, and
/// `PayloadType::{is_advanced, all_variants}`.
pub use eggsec_payloads::*;

use std::sync::LazyLock;

/// Cross-variant caches.
///
/// **Laziness is load-bearing.** These stay `LazyLock` and stay in the engine:
/// a process that never asks for a payload never pays to build one. Materializing
/// all 40 variants eagerly at startup would regress every binary, including the
/// TUI.
static PAYLOAD_CACHE: LazyLock<rustc_hash::FxHashMap<PayloadType, Vec<Payload>>> =
    LazyLock::new(|| {
        let mut map = rustc_hash::FxHashMap::default();
        for pt in PayloadType::all_variants() {
            map.insert(*pt, get_payloads(*pt));
        }
        map
    });

static ALL_PAYLOADS_CACHE: LazyLock<Vec<Payload>> =
    LazyLock::new(|| PAYLOAD_CACHE.values().flatten().cloned().collect());

/// Build the payloads for one [`PayloadType`].
///
/// Dispatches the 6 advanced types to the engine-owned live-probe modules and
/// everything else to `eggsec-payloads`. All 40 variants resolve to real
/// payloads here; none return an empty vector by omission.
pub fn get_payloads(payload_type: PayloadType) -> Vec<Payload> {
    match payload_type {
        // Engine-owned live-probe modules.
        PayloadType::GraphQL => graphql::get_payloads(),
        PayloadType::OAuth => oauth::get_payloads(),
        PayloadType::Jwt => jwt::get_payloads(),
        PayloadType::Idor => idor::get_payloads(),
        PayloadType::Ssti => ssti::get_payloads(),
        PayloadType::Grpc => grpc::get_payloads(),
        // Everything else is corpus data.
        other => eggsec_payloads::get_payloads(other),
    }
}

pub fn get_payloads_cached(payload_type: PayloadType) -> &'static Vec<Payload> {
    PAYLOAD_CACHE.get(&payload_type).unwrap_or_else(|| {
        static EMPTY: LazyLock<Vec<Payload>> = LazyLock::new(Vec::new);
        &EMPTY
    })
}

pub fn get_all_payloads_cached() -> &'static Vec<Payload> {
    &ALL_PAYLOADS_CACHE
}
