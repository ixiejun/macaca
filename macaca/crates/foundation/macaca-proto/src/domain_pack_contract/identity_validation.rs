//! Trace-safe structural validation shared by Identity pack contracts.
//!
//! This module enforces only provider-neutral protocol admission. Runtime
//! decorators remain responsible for permission, entitlement, approval,
//! resource reservation, audit emission, and concrete provider dispatch.

pub(crate) fn bounded_identity_reference(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && !value.chars().any(char::is_control)
        && !value.contains("://")
}

pub(crate) fn opaque_identity_artifact(value: &str) -> bool {
    bounded_identity_reference(value, 256)
        && !value.contains("BEGIN ")
        && !value.contains("token=")
        && !value.contains("password=")
}

pub(crate) fn bounded_identity_hash(value: &str) -> bool {
    bounded_identity_reference(value, 256)
        && !value.contains('@')
        && !value.contains('=')
        && !value.contains(':')
}

pub(crate) fn bounded_identity_page(value: Option<u32>, maximum: u32) -> bool {
    value.is_none_or(|page_size| page_size > 0 && page_size <= maximum)
}
