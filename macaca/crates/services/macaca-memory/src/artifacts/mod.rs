//! Public artifact facade for memory governance outputs.
//!
//! The concrete artifact models live in `governance::artifacts` because they
//! are produced by governance and knowledge compilation. This top-level module
//! gives callers a stable import path that matches the `artifacts/` module
//! planned by OpenSpec without creating another crate.

pub use crate::governance::{
    ArtifactContent, ArtifactKind, MemoryArtifact, MemoryArtifactList, MemoryArtifactScope,
};
