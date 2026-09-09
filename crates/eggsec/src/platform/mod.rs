//! Platform capability and prerequisite detection (Phase F).
//!
//! Centralizes environment checks for platform-sensitive domains so tests and
//! `eggsec doctor` report the same facts. All detection is best-effort and
//! read-only: it never requires privileges, never transmits, and never
//! mutates namespaces, interfaces, or devices.
//!
//! A skipped test must state which prerequisite is absent; call
//! [`skip_reason_for`] from test harnesses to produce that message.

mod prereqs;

pub use prereqs::{
    all_domain_ids, capabilities_summary, current_arch, current_kernel, current_os, has_binary,
    has_cap_net_admin, has_cap_net_raw, is_root, list_interfaces, report_for, skip_reason_for,
    DomainPrerequisites, PlatformReport, PrereqStatus, Prerequisite,
};
