//! Provider-neutral Autonomy Evolution Control Plane contracts.
//!
//! These modules define only stable command/result DTOs, service identifiers,
//! and bounded helper constructors that are safe to share across SDK, shells,
//! runtime-host adapters, and service providers. Provider strategies and source
//! mutation behavior remain outside `macaca-proto`.

mod live;
mod model;
mod os_code_proposal;
mod release;

pub use live::*;
pub use model::*;
pub use os_code_proposal::*;
pub use release::*;
