//! Application Platform certification contracts.
//!
//! Certification is a provider-neutral admission and ecosystem-readiness layer.
//! It inspects Manifest v1 data, ability descriptors, service declarations,
//! plugin dependencies, commerce metadata, UI declarations, and optional ABI
//! metadata without executing application code.
//!
//! The module is intentionally split into DTO and rule modules so each file
//! stays small, reviewable, and aligned with the project-wide 500-line limit.
//! `kit` owns the Facade + Visitor + Specification logic, while `types` owns
//! Memento-style reports and data-only certification inputs.

mod kit;
mod types;
mod wasm_admission;
mod wasm_supply_chain;

pub use kit::ApplicationCertificationKit;
pub use types::{
    ApplicationCertificationContext, ApplicationCertificationDiagnostic,
    ApplicationCertificationFixture, ApplicationCertificationReport,
    ApplicationCertificationStatus,
};
pub use wasm_admission::{
    WasmPackageAdmissionReport, WasmPackageAdmissionSpec, WasmPackageAdmissionStatus,
};
pub use wasm_supply_chain::WasmSupplyChainAdmissionSpec;
