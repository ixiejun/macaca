//! **Catalog** layer — turns neutral configuration + dependency injection into ordered provider
//! chains (Builder + Abstract Factory + Decorator).  No transport protocols live here.

pub mod assemble;
pub mod constants;
pub mod descriptor;
pub mod health;
pub mod versioned;

pub use assemble::{assemble_context_providers, ProviderAssemblyEnvironment};
pub use descriptor::{list_builtin_family_descriptors, ProviderFamilyDescriptor};
pub use health::{ProviderHealthLedger, ProviderHealthSnapshot};
pub use versioned::VersionedContextProvider;
