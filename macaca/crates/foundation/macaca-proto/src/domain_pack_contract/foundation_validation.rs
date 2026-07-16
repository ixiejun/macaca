//! Shared structural validation for foundation pack DTOs.
//!
//! These helpers deliberately validate only trace-safe references. Provider
//! dispatch, policy decisions, resource reservations, and approvals remain in
//! the runtime because a protocol crate must not perform side effects.

pub(crate) fn bounded_reference(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && !value.chars().any(char::is_control)
        && !value.contains("://")
}

pub(crate) fn opaque_artifact_reference(value: &str) -> bool {
    bounded_reference(value, 256)
        && !value.contains('\n')
        && !value.contains('\r')
        && !value.contains("BEGIN ")
}

pub(crate) fn secret_store_reference(value: &str) -> bool {
    bounded_reference(value, 256)
        && matches!(
            value.split_once(':').map(|(prefix, _)| prefix),
            Some("secret" | "vault" | "kms")
        )
}

pub(crate) fn bounded_page_size(value: u32, maximum: u32) -> bool {
    value > 0 && value <= maximum
}
