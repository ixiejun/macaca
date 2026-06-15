//! System service descriptor adapter for the driver layer.
//!
//! The canonical descriptor is part of the provider-neutral protocol in
//! `macaca-proto`. This module re-exports it so driver-provider internals keep
//! a local path without making SDK callers depend on the driver crate.

pub use macaca_proto::driver_service_descriptor;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_descriptor_exports_contract_shape() {
        let descriptor = driver_service_descriptor();
        assert_eq!(descriptor.service_type.as_str(), "driver");
        assert_eq!(descriptor.capabilities.len(), 5);
        assert!(descriptor
            .required_permissions
            .contains(&"driver.execute".into()));
    }
}
