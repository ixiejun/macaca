use super::foundation_filesystem::{
    FilesystemContentRef, FilesystemListDirectoryCommand, FilesystemPathRef,
    FilesystemReadFileCommand, FilesystemRootRef, FilesystemWriteFileCommand,
};
use super::foundation_validation::{
    bounded_page_size, bounded_reference, opaque_artifact_reference,
};

impl FilesystemRootRef {
    /// Validate logical roots without revealing or accepting host filesystem paths.
    pub fn is_bounded_reference(&self) -> bool {
        bounded_reference(&self.root_id, 160) && bounded_reference(&self.root_kind, 96)
    }
}

impl FilesystemPathRef {
    /// Reject traversal, absolute paths, and raw host-path encodings before dispatch.
    pub fn is_safe_relative_path(&self) -> bool {
        self.root.is_bounded_reference()
            && !self.relative_path.is_empty()
            && self.relative_path.len() <= 512
            && !self.relative_path.starts_with('/')
            && !self.relative_path.contains('\\')
            && self
                .relative_path
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != "..")
    }
}

impl FilesystemContentRef {
    /// Require file contents to remain opaque provider-owned artifacts.
    pub fn is_safe_reference(&self) -> bool {
        opaque_artifact_reference(&self.content_ref)
            && self
                .encoding
                .as_deref()
                .is_none_or(|value| bounded_reference(value, 64))
            && self
                .expected_hash
                .as_deref()
                .is_none_or(|value| bounded_reference(value, 256))
    }
}

impl FilesystemReadFileCommand {
    /// Require exactly one safe read target and a provider-independent byte cap.
    pub fn is_bounded_request(&self, max_bytes: u64) -> bool {
        self.path.is_some() != self.handle.is_some()
            && self
                .path
                .as_ref()
                .is_none_or(FilesystemPathRef::is_safe_relative_path)
            && self.max_bytes > 0
            && self.max_bytes <= max_bytes
    }
}

impl FilesystemListDirectoryCommand {
    /// Bound directory pagination and recursive traversal intent before policy evaluation.
    pub fn is_bounded_request(&self, max_entries: u32, recursive_allowed: bool) -> bool {
        self.path.is_safe_relative_path()
            && (!self.recursive || recursive_allowed)
            && bounded_page_size(self.page_size, max_entries)
            && self
                .cursor
                .as_deref()
                .is_none_or(|value| bounded_reference(value, 256))
    }
}

impl FilesystemWriteFileCommand {
    /// Validate write metadata; approval and resource reservation remain runtime concerns.
    pub fn has_safe_preconditions(&self) -> bool {
        self.path.is_safe_relative_path() && self.content.is_safe_reference()
    }
}
