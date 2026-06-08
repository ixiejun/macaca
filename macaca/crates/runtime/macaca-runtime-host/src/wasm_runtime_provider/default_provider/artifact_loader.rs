//! Artifact reference loading for the default in-process WASM provider.
//!
//! Loading is intentionally filesystem-based and provider-neutral.  The helper
//! accepts only sanitized artifact refs from session requests and never inspects
//! guest bytecode semantics beyond reading bounded bytes from disk.

use std::fs;
use std::path::{Path, PathBuf};

use macaca_proto::{WasmRuntimeErrorKind, WasmRuntimeSessionRequest};

use super::super::errors::WasmRuntimeHostError;

/// Read WASM artifact bytes referenced by one validated session request.
pub(super) fn load_artifact_bytes(
    request: &WasmRuntimeSessionRequest,
) -> Result<Vec<u8>, WasmRuntimeHostError> {
    let path = artifact_path(request.artifact.as_str())?;
    fs::read(&path).map_err(|error| {
        WasmRuntimeHostError::new(
            WasmRuntimeErrorKind::ArtifactLoadFailed,
            format!("WASM artifact could not be loaded: {error}"),
        )
    })
}

/// Resolve a provider-neutral artifact reference into a local filesystem path.
pub(super) fn artifact_path(reference: &str) -> Result<PathBuf, WasmRuntimeHostError> {
    let trimmed = reference.trim();
    let path = trimmed.strip_prefix("file://").unwrap_or(trimmed);
    if path.is_empty() {
        return Err(WasmRuntimeHostError::new(
            WasmRuntimeErrorKind::ArtifactLoadFailed,
            "WASM artifact reference is empty",
        ));
    }
    let candidate = Path::new(path);
    if candidate.components().next().is_none() {
        return Err(WasmRuntimeHostError::new(
            WasmRuntimeErrorKind::ArtifactLoadFailed,
            "WASM artifact reference does not resolve to a file",
        ));
    }
    Ok(candidate.to_path_buf())
}

/// Return true when a host command targets guest export invocation semantics.
pub(super) fn is_wasm_export_invoke(command: &macaca_proto::ApplicationHostCommand) -> bool {
    command.metadata.contains_key("wasm.export")
        || matches!(
            &command.import,
            macaca_proto::ApplicationImport::Custom(value) if value == "macaca:wasm/invoke"
        )
}
