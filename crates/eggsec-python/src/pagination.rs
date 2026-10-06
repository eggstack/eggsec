//! Shared limit/offset pagination for the Python-facing repository stores.
//!
//! Every paginated `#[pymethods]` entry point takes `limit: u64, offset: u64`
//! straight from Python. Slicing a `Vec` with those needs care, because both
//! values are attacker-controlled and unbounded.
//!
//! # Why this exists
//!
//! Six call sites had the same open-coded arithmetic:
//!
//! ```ignore
//! let start = offset as usize;
//! let end = start + limit as usize;   // <-- overflows
//! if start >= results.len() { return Ok(Vec::new()); }
//! results.drain(..start);
//! results.truncate((end - start) as usize);
//! ```
//!
//! `start + limit as usize` is evaluated **before** the bounds guard, so
//! `list_artifacts(limit=2**63, offset=2**63)` overflows `usize` in a debug
//! build — a panic on a Python-supplied value, surfaced to the caller as a
//! `PanicException` rather than a normal `PyResult` error. In a release build
//! the sum wraps instead of panicking, which silently changes the window.
//!
//! Note also that `end - start` is, by construction, exactly `limit`, so the
//! whole `end` binding bought nothing but the overflow.
//!
//! # The contract
//!
//! - An `offset` past the end yields an empty result, not a panic.
//! - A `limit` larger than what remains yields what remains, not a panic.
//! - Values too large for `usize` saturate rather than wrap.

/// Apply `offset`/`limit` pagination to `items`, in place.
///
/// Saturating on both arithmetic steps means no input can overflow; a
/// pathological `offset` simply produces an empty list and a pathological
/// `limit` simply returns everything from `offset` onward, which is what the
/// caller asked for.
pub(crate) fn paginate<T>(items: &mut Vec<T>, limit: u64, offset: u64) {
    let start = usize::try_from(offset).unwrap_or(usize::MAX);
    if start >= items.len() {
        items.clear();
        return;
    }
    items.drain(..start);
    // `usize::try_from(limit)` on a 32-bit target can fail for a u64 limit;
    // saturating to MAX means "keep everything that is left", which is correct.
    let keep = usize::try_from(limit).unwrap_or(usize::MAX);
    items.truncate(keep);
}

#[cfg(test)]
mod tests {
    use super::paginate;

    fn paged(limit: u64, offset: u64) -> Vec<u32> {
        let mut items: Vec<u32> = (1..=5).collect();
        paginate(&mut items, limit, offset);
        items
    }

    #[test]
    fn normal_window() {
        assert_eq!(paged(2, 0), vec![1, 2]);
        assert_eq!(paged(2, 1), vec![2, 3]);
        assert_eq!(paged(2, 3), vec![4, 5]);
    }

    #[test]
    fn limit_beyond_end_returns_the_remainder() {
        assert_eq!(paged(100, 0), vec![1, 2, 3, 4, 5]);
        assert_eq!(paged(100, 4), vec![5]);
    }

    #[test]
    fn offset_at_or_past_end_is_empty() {
        assert_eq!(paged(2, 5), Vec::<u32>::new());
        assert_eq!(paged(2, 6), Vec::<u32>::new());
        assert_eq!(paged(2, u64::MAX), Vec::<u32>::new());
    }

    #[test]
    fn zero_limit_returns_nothing() {
        assert_eq!(paged(0, 0), Vec::<u32>::new());
        assert_eq!(paged(0, 2), Vec::<u32>::new());
    }

    /// The regression this helper exists for: `start + limit` overflowing
    /// `usize` before the bounds check.
    #[test]
    fn huge_limit_and_offset_do_not_overflow() {
        let two_63 = 1u64 << 63;
        // Used to panic with "attempt to add with overflow" in a debug build.
        let _ = paged(two_63, two_63);
        // Also individually extreme, with a non-empty backing list.
        let _ = paged(u64::MAX, 0);
        let _ = paged(u64::MAX, 1);
        let _ = paged(1, u64::MAX - 1);
        // And the combination that overflows while offset is in range.
        assert_eq!(paged(u64::MAX, 0), vec![1, 2, 3, 4, 5]);
        assert_eq!(paged(u64::MAX, 4), vec![5]);
    }

    #[test]
    fn empty_input_stays_empty() {
        let mut items: Vec<u32> = Vec::new();
        paginate(&mut items, 10, 0);
        assert!(items.is_empty());
        paginate(&mut items, u64::MAX, u64::MAX);
        assert!(items.is_empty());
    }
}
