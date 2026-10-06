//! Guards the engine's payload-corpus facade.
//!
//! `eggsec-payloads` owns all 40 payload modules and resolves every
//! `PayloadType` variant itself. The engine re-exports that API permanently
//! (`eggsec::fuzzer::payloads`), and also keeps the six **probers** —
//! `GraphQLFuzzer`, `OAuthFuzzer`, `JwtFuzzer`, `IdorFuzzer`, `SstiFuzzer`,
//! `GrpcFuzzer` — which hold a `reqwest::Client` and cannot live in a crate
//! with no network surface.
//!
//! So there are two independent things to keep true, and this file checks the
//! engine side of both:
//!
//! 1. The facade keeps re-exporting the corpus API. If someone drops
//!    `pub use eggsec_payloads::*`, every engine consumer breaks while
//!    `eggsec-payloads` still passes its own tests. Nothing inside the corpus
//!    crate can catch that, so the check belongs here.
//! 2. The six probers keep their payload strings. Each prober module re-exports
//!    the same-named builder from the corpus, so a regression in that seam
//!    silently changes what the probers send.
//!
//! (`eggsec-payloads` has its own mirror of the all-40 invariant in its `lib`
//! tests. This file is the engine-side half, not a duplicate of it.)

use eggsec::fuzzer::payloads::{get_all_payloads_cached, get_payloads, PayloadType};

/// The six prober types must keep producing real payloads through the engine.
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
            "{pt:?} is in the prober set but is_advanced() says otherwise; the prober set and \
             the is_advanced predicate must agree"
        );

        let payloads = get_payloads(pt);
        assert!(
            !payloads.is_empty(),
            "{pt:?} returned an EMPTY payload list — the facade stopped resolving a prober type \
             (silent capability regression)"
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

/// Every variant must resolve through the facade, and the cached union must
/// match the per-type sum.
///
/// The caches now live in `eggsec-payloads`, re-exported here. If the engine
/// facade ever stops re-exporting them, this fails while the corpus crate stays
/// green — which is exactly the failure the facade exists to make visible.
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

/// `PayloadType::is_advanced` must remain exactly the prober set.
///
/// `is_advanced` selects an **execution strategy** in the engine's fuzzer
/// (`eggsec::fuzzer::engine`): these six run through their probers, everything
/// else through the generic payload-batch runner. It is not a statement about
/// where payloads live — all 40 resolve in `eggsec-payloads` either way. If
/// this set drifts from the probers actually implemented in the engine, the
/// fuzzer routes a type to a prober that does not exist, or skips a prober
/// entirely.
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
            "{pt:?}: is_advanced() disagrees with the engine's prober set"
        );
    }
}
