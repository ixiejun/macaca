//! Concatenated skill-self-evolution observer sources for static contract tests.

/// Returns all observer Rust sources concatenated for static scans.
pub(crate) fn skill_self_evolution_observer_module_sources() -> String {
    [
        include_str!("mod.rs"),
        include_str!("types.rs"),
        include_str!("observer.rs"),
        include_str!("projection.rs"),
        include_str!("forwarder.rs"),
        include_str!("proposal_builder.rs"),
        include_str!("semantic_signal.rs"),
    ]
    .join("\n")
}
