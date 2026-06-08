//! Generic industrial tool-family provider catalog (Facade module tree).
//!
//! This module is the runtime-host Abstract Factory for the broad Tool Capability Plane.
//! It does not implement file, shell, browser, payment, or other domain behavior directly.
//! Instead it creates sanitized descriptor rows that point to owning service, MCP, plugin,
//! gateway, runtime-adapter, or unavailable-provider boundaries.  Planning can therefore
//! expose a complete generic tool surface while invocation still flows through
//! `service.tool` decorators and the owning service route selected by descriptor data.
//!
//! Module layout:
//! - `constants.rs` — stable service identifiers and required family table
//! - `family_catalog.rs` — `FamilySpec` catalog rows and ownership resolution
//! - `descriptor_builder.rs` — catalog row → `IndustrialToolDescriptor` Builder
//! - `inventory.rs` — sanitized governance inventory assembly
//! - `planning_factory.rs` — contributor, toolsets, and `ToolPlanningService` factory

mod constants;
mod descriptor_builder;
mod family_catalog;
mod inventory;
mod planning_factory;

pub use constants::REQUIRED_INDUSTRIAL_TOOL_FAMILIES;
pub use inventory::{
    industrial_tool_family_provider_inventory, IndustrialToolFamilyProviderInventory,
};
pub use planning_factory::{
    industrial_tool_family_provider_contributor, industrial_tool_family_toolsets,
    industrial_tool_planning_service,
};
