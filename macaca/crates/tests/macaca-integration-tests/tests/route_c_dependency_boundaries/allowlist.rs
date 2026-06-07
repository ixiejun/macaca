use super::gate::AllowlistEntry;

/// Returns the current Route C migration-debt snapshot used by the executable
/// dependency gate.
///
/// This table deliberately mirrors
/// `macaca/docs/macaca-os-serviceization-allowlist.md`. The Rust copy is the
/// deterministic execution input; the markdown copy is the human governance
/// memento. Adding a row here without the corresponding OpenSpec and document
/// update would hide architectural debt, so the gate diagnostics explicitly tell
/// maintainers to update both surfaces.
pub fn entries() -> Vec<AllowlistEntry> {
    vec![]
}
