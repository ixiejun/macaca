use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use super::allowlist;

/// A stable Route C architecture layer.
///
/// Layers intentionally model ownership, not implementation detail. For
/// example, `macaca-llm` and `macaca-memory` are currently ordinary crates, but
/// Route C treats them as replaceable service providers because their concrete
/// backends must be swappable without changing kernel or presentation code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Layer {
    Proto,
    Kernel,
    IpcServiceBus,
    ServiceContract,
    ServiceProvider,
    RuntimeHost,
    ApplicationFramework,
    PresentationShell,
    OptionalModule,
    IntegrationTest,
}

impl Layer {
    /// Returns the stable diagnostic label used by docs and test failures.
    fn as_str(self) -> &'static str {
        match self {
            Self::Proto => "proto",
            Self::Kernel => "kernel",
            Self::IpcServiceBus => "ipc-service-bus",
            Self::ServiceContract => "service-contract",
            Self::ServiceProvider => "service-provider",
            Self::RuntimeHost => "runtime-host",
            Self::ApplicationFramework => "application-framework",
            Self::PresentationShell => "presentation-shell",
            Self::OptionalModule => "optional-module",
            Self::IntegrationTest => "integration-test",
        }
    }
}

/// A direct dependency edge between two workspace crates.
///
/// The gate only evaluates direct workspace edges because S0 is a boundary audit
/// phase, not a full source scanner. Later phases can add symbol-level scans for
/// provider construction, but direct Cargo edges are already enough to stop new
/// macro-kernel coupling from entering silently.
#[derive(Clone, Debug)]
struct DependencyEdge {
    from: String,
    to: String,
}

/// Cargo dependency kind as exposed by `cargo metadata`.
///
/// The Route C boundary gate audits production dependency ownership. Dev-only
/// edges are allowed to point at fixtures and provider crates because tests
/// often need concrete doubles to exercise contracts. Keeping kind explicit in
/// the parsed edge model makes that policy auditable and prevents future test
/// dependencies from being mistaken for production architecture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DependencyKind {
    Normal,
    Build,
    Dev,
}

impl DependencyKind {
    /// Convert Cargo's nullable dependency `kind` field into a stable enum.
    ///
    /// `cargo metadata` represents normal dependencies with `null`, while
    /// build and dev dependencies use string values. Unknown strings fail fast
    /// so a future Cargo format or target-specific edge cannot silently bypass
    /// the executable architecture specification.
    fn from_metadata(kind: &Value) -> Self {
        match kind.as_str() {
            None => Self::Normal,
            Some("build") => Self::Build,
            Some("dev") => Self::Dev,
            Some(other) => panic!("unsupported cargo dependency kind: {other}"),
        }
    }

    /// Return whether this edge participates in production dependency rules.
    fn is_production(self) -> bool {
        matches!(self, Self::Normal | Self::Build)
    }
}

/// A forbidden-edge specification.
///
/// `matches` is a function pointer to keep rule evaluation extensible without a
/// giant nested conditional block. Future Route C phases can add new
/// specifications while preserving the same visitor and diagnostic pipeline.
struct BoundaryRule {
    id: &'static str,
    rationale: &'static str,
    replacement: &'static str,
    matches: fn(&ClassifiedEdge<'_>) -> bool,
}

/// An edge with source and target layers already resolved.
///
/// Classification is intentionally a separate stage. Unknown crates fail before
/// rule evaluation so every new workspace crate must declare its architectural
/// ownership through OpenSpec rather than inheriting a permissive default.
struct ClassifiedEdge<'a> {
    from: &'a str,
    to: &'a str,
    from_layer: Layer,
    to_layer: Layer,
}

/// A temporary migration exception for known Route C debt.
///
/// The allowlist is duplicated in `macaca/docs/macaca-os-serviceization-allowlist.md`
/// for human review. The test-local copy makes the gate deterministic and avoids
/// markdown parsing becoming the source of truth for executable architecture.
#[derive(Clone, Copy)]
pub struct AllowlistEntry {
    pub rule_id: &'static str,
    pub from: &'static str,
    pub to: &'static str,
    pub owner_track: &'static str,
    pub current_caller: &'static str,
    pub target_phase: &'static str,
    pub replacement: &'static str,
    pub validation_command: &'static str,
}

impl AllowlistEntry {
    /// Builds one migration-debt entry with all fields needed for deterministic
    /// rule matching and audit logging.
    ///
    /// The extra metadata is intentionally part of the executable gate instead
    /// of markdown-only documentation. It keeps each exception traceable to an
    /// owner track, current caller evidence, target replacement, expiry phase,
    /// and validation command before any future worker can add or retain debt.
    pub const fn new(
        rule_id: &'static str,
        from: &'static str,
        to: &'static str,
        owner_track: &'static str,
        current_caller: &'static str,
        target_phase: &'static str,
        replacement: &'static str,
        validation_command: &'static str,
    ) -> Self {
        Self {
            rule_id,
            from,
            to,
            owner_track,
            current_caller,
            target_phase,
            replacement,
            validation_command,
        }
    }
}

/// A rendered violation that can be sorted for deterministic diagnostics.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
struct Violation {
    rule_id: &'static str,
    from: String,
    to: String,
    from_layer: &'static str,
    to_layer: &'static str,
    rationale: &'static str,
    replacement: &'static str,
}

fn workspace_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate Macaca workspace root from CARGO_MANIFEST_DIR")
}

fn classify_crate(crate_name: &str) -> Option<Layer> {
    match crate_name {
        "macaca-proto" => Some(Layer::Proto),
        "macaca-kernel" => Some(Layer::Kernel),
        "macaca-ipc" => Some(Layer::IpcServiceBus),
        "macaca-context" => Some(Layer::ServiceContract),
        "macaca-llm"
        | "macaca-memory"
        | "macaca-task"
        | "macaca-driver"
        | "macaca-skill"
        | "macaca-gateway"
        | "macaca-persist"
        | "macaca-tools"
        | "macaca-scheduler"
        | "macaca-heartbeat"
        | "macaca-scheduled-agent-task" => Some(Layer::ServiceProvider),
        "macaca-runtime" | "macaca-runtime-host" => Some(Layer::RuntimeHost),
        "macaca-agent" | "macaca-framework" | "macaca-app" | "macaca-sdk" => {
            Some(Layer::ApplicationFramework)
        }
        "macaca-web" | "macaca-cli" => Some(Layer::PresentationShell),
        "macaca-integration-tests" => Some(Layer::IntegrationTest),
        _ => None,
    }
}

fn boundary_rules() -> Vec<BoundaryRule> {
    vec![
        BoundaryRule {
            id: "kernel-no-provider-deps",
            rationale: "Kernel must own system invariants and facades, not replaceable provider implementations.",
            replacement: "route through Service Registry, ServiceRuntime, or a narrow system service contract",
            matches: |edge| edge.from == "macaca-kernel" && edge.to_layer == Layer::ServiceProvider,
        },
        BoundaryRule {
            id: "presentation-no-provider-construction-hub",
            rationale: "Presentation shells must adapt user input and render state, not construct provider graphs.",
            replacement: "route through macaca-sdk SystemFacade or service clients",
            matches: |edge| {
                edge.from_layer == Layer::PresentationShell && edge.to_layer == Layer::ServiceProvider
            },
        },
        BoundaryRule {
            id: "cli-no-web-internals",
            rationale: "CLI must remain a command shell and must not become coupled to Web internals.",
            replacement: "share shell behavior through macaca-sdk SystemFacade or a service inspector facade",
            matches: |edge| edge.from == "macaca-cli" && edge.to == "macaca-web",
        },
        BoundaryRule {
            id: "optional-not-base-required",
            rationale: "Optional modules must not become required dependencies of the base Agent OS.",
            replacement: "register optional capabilities through module descriptors and return structured unavailable states",
            matches: |edge| {
                edge.to_layer == Layer::OptionalModule
                    && matches!(
                        edge.from_layer,
                        Layer::Kernel
                            | Layer::IpcServiceBus
                            | Layer::RuntimeHost
                            | Layer::ApplicationFramework
                            | Layer::PresentationShell
                    )
            },
        },
        BoundaryRule {
            id: "service-provider-no-presentation",
            rationale: "Service providers must never depend on presentation shells because that reverses ownership.",
            replacement: "emit service events or trace records and let Web/CLI subscribe through shell adapters",
            matches: |edge| {
                edge.from_layer == Layer::ServiceProvider
                    && edge.to_layer == Layer::PresentationShell
            },
        },
    ]
}

fn run_cargo_metadata() -> Value {
    eprintln!(
        "route_c_dependency_gate event=metadata_start workspace={}",
        workspace_root().display()
    );
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!(
        "route_c_dependency_gate event=metadata_complete bytes={}",
        output.stdout.len()
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata should emit valid JSON")
}

fn workspace_dependency_edges(metadata: &Value) -> Vec<DependencyEdge> {
    let workspace_members: BTreeSet<String> = metadata["workspace_members"]
        .as_array()
        .expect("workspace_members should be an array")
        .iter()
        .map(|id| {
            id.as_str()
                .expect("workspace member id should be a string")
                .to_owned()
        })
        .collect();

    let packages = metadata["packages"]
        .as_array()
        .expect("packages should be an array");
    let workspace_names: BTreeSet<String> = packages
        .iter()
        .filter(|package| workspace_members.contains(package["id"].as_str().unwrap_or_default()))
        .map(|package| {
            package["name"]
                .as_str()
                .expect("package name should be a string")
                .to_owned()
        })
        .collect();

    let mut edges = Vec::new();
    for package in packages {
        let package_id = package["id"]
            .as_str()
            .expect("package id should be a string");
        if !workspace_members.contains(package_id) {
            continue;
        }
        let from = package["name"]
            .as_str()
            .expect("package name should be a string");
        for dependency in package["dependencies"]
            .as_array()
            .expect("dependencies should be an array")
        {
            let to = dependency["name"]
                .as_str()
                .expect("dependency name should be a string");
            let kind = DependencyKind::from_metadata(&dependency["kind"]);
            if !kind.is_production() {
                eprintln!(
                    "route_c_dependency_gate event=skip_dev_edge from={} to={}",
                    from, to
                );
                continue;
            }
            if workspace_names.contains(to) {
                edges.push(DependencyEdge {
                    from: from.to_owned(),
                    to: to.to_owned(),
                });
            }
        }
    }
    edges.sort_by(|left, right| (&left.from, &left.to).cmp(&(&right.from, &right.to)));
    edges
}

fn assert_every_workspace_crate_is_classified(metadata: &Value) {
    let workspace_members: BTreeSet<&str> = metadata["workspace_members"]
        .as_array()
        .expect("workspace_members should be an array")
        .iter()
        .map(|id| id.as_str().expect("workspace member id should be a string"))
        .collect();
    let mut unknown = Vec::new();
    for package in metadata["packages"]
        .as_array()
        .expect("packages should be an array")
    {
        let package_id = package["id"]
            .as_str()
            .expect("package id should be a string");
        if !workspace_members.contains(package_id) {
            continue;
        }
        let crate_name = package["name"]
            .as_str()
            .expect("package name should be a string");
        if classify_crate(crate_name).is_none() {
            unknown.push(crate_name.to_owned());
        }
    }
    unknown.sort();
    assert!(
        unknown.is_empty(),
        "Route C dependency gate found unclassified workspace crates: {unknown:?}. \
         Classify each crate through OpenSpec before relying on it in dependency boundaries."
    );
}

fn evaluate_edges(edges: &[DependencyEdge]) -> Vec<Violation> {
    let rules = boundary_rules();
    let allowed: BTreeMap<(&str, &str, &str), AllowlistEntry> = allowlist::entries()
        .into_iter()
        .map(|entry| ((entry.rule_id, entry.from, entry.to), entry))
        .collect();
    let mut violations = Vec::new();

    for edge in edges {
        let classified = ClassifiedEdge {
            from: &edge.from,
            to: &edge.to,
            from_layer: classify_crate(&edge.from).expect("source crate should be classified"),
            to_layer: classify_crate(&edge.to).expect("target crate should be classified"),
        };
        for rule in &rules {
            if !(rule.matches)(&classified) {
                continue;
            }
            if let Some(entry) = allowed.get(&(rule.id, classified.from, classified.to)) {
                eprintln!(
                    "route_c_dependency_gate event=allowlisted rule={} from={} to={} owner_track={} current_caller={} phase={} replacement={} validation_command={}",
                    entry.rule_id,
                    entry.from,
                    entry.to,
                    entry.owner_track,
                    entry.current_caller,
                    entry.target_phase,
                    entry.replacement,
                    entry.validation_command
                );
                continue;
            }
            violations.push(Violation {
                rule_id: rule.id,
                from: classified.from.to_owned(),
                to: classified.to.to_owned(),
                from_layer: classified.from_layer.as_str(),
                to_layer: classified.to_layer.as_str(),
                rationale: rule.rationale,
                replacement: rule.replacement,
            });
        }
    }

    violations.sort();
    violations
}

fn render_violations(violations: &[Violation]) -> String {
    violations
        .iter()
        .map(|violation| {
            format!(
                "\nrule={}\nfrom={} ({})\nto={} ({})\nrationale={}\nreplacement={}\nprocess=New exceptions require OpenSpec plus macaca/docs/macaca-os-serviceization-allowlist.md update.\n",
                violation.rule_id,
                violation.from,
                violation.from_layer,
                violation.to,
                violation.to_layer,
                violation.rationale,
                violation.replacement
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

pub fn assert_route_c_dependency_boundaries() {
    let metadata = run_cargo_metadata();
    assert_every_workspace_crate_is_classified(&metadata);

    let edges = workspace_dependency_edges(&metadata);
    eprintln!(
        "route_c_dependency_gate event=edge_visit_complete edges={}",
        edges.len()
    );
    let violations = evaluate_edges(&edges);
    assert!(
        violations.is_empty(),
        "Route C dependency boundary violations were found:{}",
        render_violations(&violations)
    );
}
