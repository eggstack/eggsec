//! Guards the corpus/probe seam created by Phase G.
//!
//! `eggsec-payloads` owns the 34 pure-data payload modules. The 6 that generate
//! payloads by performing live `reqwest` probing stay in the engine, and the
//! engine owns the cross-variant caches that span all 40 variants.
//!
//! That split has one dangerous failure mode: if the engine's dispatch ever
//! stops resolving the 6 advanced types, `get_payloads` would return an empty
//! vector for them. No test inside `eggsec-payloads` could catch it, because
//! those modules do not live there — the crate correctly panics instead. So the
//! check belongs on the engine side, and that is what this file is.

use eggsec::fuzzer::payloads::{get_all_payloads_cached, get_payloads, PayloadType};

/// The 6 live-probe types must keep producing real payloads through the engine.
///
/// This is the specific regression the seam makes possible, so it is asserted
/// explicitly rather than left to aggregate count checks.
#[test]
fn advanced_probe_types_return_non_empty_payloads() {
    let advanced = [
        PayloadType::GraphQL,
        PayloadType::OAuth,
        PayloadType::Jwt,
        PayloadType::Idor,
        PayloadType::Ssti,
        PayloadType::Grpc,
    ];

    for pt in advanced {
        assert!(
            pt.is_advanced(),
            "{pt:?} is in the live-probe set but is_advanced() says otherwise; the corpus/probe \
             split and the is_advanced predicate must agree"
        );

        let payloads = get_payloads(pt);
        assert!(
            !payloads.is_empty(),
            "{pt:?} returned an EMPTY payload list — the engine stopped resolving an \
             engine-owned live-probe type (silent capability regression)"
        );
        for payload in &payloads {
            assert!(
                !payload.payload.trim().is_empty(),
                "{pt:?} produced a payload with an empty body"
            );
            assert_eq!(
                payload.payload_type, pt,
                "{pt:?} produced a payload tagged as another type"
            );
        }
    }
}

/// Every variant must resolve, and the cached union must match the per-type sum.
///
/// If the cache were built from fewer than all 40 variants, this fails. It is the
/// check that would catch the caches being moved into the corpus crate.
#[test]
fn every_payload_type_resolves_and_cache_matches_sum() {
    let all = PayloadType::all_variants();
    assert_eq!(
        all.len(),
        40,
        "PayloadType gained or lost a variant; update the advanced set and this test"
    );

    for pt in all {
        assert!(
            !get_payloads(*pt).is_empty(),
            "{pt:?} resolves to an empty payload list"
        );
    }

    let per_type_sum: usize = all.iter().map(|pt| get_payloads(*pt).len()).sum();
    let cached = get_all_payloads_cached();

    assert_eq!(
        cached.len(),
        per_type_sum,
        "cached union ({}) differs from the sum over all 40 types ({per_type_sum}); the cache is \
         not covering every variant",
        cached.len()
    );

    let mut distinct = std::collections::HashSet::new();
    for payload in cached {
        distinct.insert(payload.payload_type);
    }
    assert_eq!(
        distinct.len(),
        all.len(),
        "cached payloads represent {} of {} payload types",
        distinct.len(),
        all.len()
    );
}

/// `PayloadType::is_advanced` must remain exactly the corpus/probe split.
///
/// If this drifts, the engine's dispatch and the corpus crate's panic boundary
/// silently disagree, and library consumers filtering on `is_advanced()` get the
/// wrong answer.
#[test]
fn is_advanced_matches_the_probe_split() {
    let expected_advanced = [
        PayloadType::GraphQL,
        PayloadType::OAuth,
        PayloadType::Jwt,
        PayloadType::Idor,
        PayloadType::Ssti,
        PayloadType::Grpc,
    ];

    for pt in PayloadType::all_variants() {
        let should_be_advanced = expected_advanced.contains(pt);
        assert_eq!(
            pt.is_advanced(),
            should_be_advanced,
            "{pt:?}: is_advanced() disagrees with the engine-owned probe set"
        );
    }
}
