//! Generic Application Platform fixtures built with `macaca-sdk`.
//!
//! These examples are data-only.  They demonstrate how application authors can
//! describe different application shapes without constructing Macaca runtimes,
//! kernels, Web state, or provider implementations.

use macaca_proto::{
    ApplicationCommerceDeclaration, ApplicationPluginDependency, ApplicationRuntimeProfile,
    PackageRuntimeKind,
};
use macaca_sdk::{AbilityKit, ApplicationContractTestKit, ApplicationKit};

fn main() {
    let declarative = ApplicationKit::manifest(
        "application.fixture.declarative",
        "developer.fixture",
        "Declarative Fixture",
        "1.0.0",
    )
    .ability(
        AbilityKit::agent("ability.fixture.agent")
            .activation("session", "agent.fixture")
            .permission("trace.emit", "Agent emits trace events")
            .build(),
    )
    .permission("trace.emit", "Application emits trace events")
    .build();

    let genui = ApplicationKit::manifest(
        "application.fixture.genui",
        "developer.fixture",
        "GenUI Fixture",
        "1.0.0",
    )
    .ability(
        AbilityKit::ui("ability.fixture.ui")
            .activation("ui", "surface.fixture")
            .ui_surface("surface.fixture", "schema.fixture.genui")
            .permission("ui.render", "UI ability renders GenUI surfaces")
            .build(),
    )
    .genui("surface.fixture")
    .build();

    let headless = ApplicationKit::manifest(
        "application.fixture.headless",
        "developer.fixture",
        "Headless Fixture",
        "1.0.0",
    )
    .ability(
        AbilityKit::headless("ability.fixture.headless")
            .activation("scheduled", "job.fixture")
            .permission("trace.emit", "Headless ability emits trace events")
            .build(),
    )
    .build();

    let plugin_enhanced = ApplicationKit::manifest(
        "application.fixture.plugin",
        "developer.fixture",
        "Plugin Fixture",
        "1.0.0",
    )
    .ability(AbilityKit::agent("ability.fixture.plugin_agent").build())
    .plugin_dependency(ApplicationPluginDependency::required(
        "plugin.fixture.capability",
        "Application imports a generic plugin capability",
    ))
    .build();

    let store_entitled = ApplicationKit::manifest(
        "application.fixture.store",
        "developer.fixture",
        "Store Fixture",
        "1.0.0",
    )
    .ability(AbilityKit::agent("ability.fixture.store_agent").build())
    .commerce(ApplicationCommerceDeclaration {
        license: "subscription".into(),
        store_required: true,
        metering: vec!["application.launch".into()],
        metadata: Default::default(),
    })
    .build();

    let wasm = ApplicationKit::manifest(
        "application.fixture.wasm",
        "developer.fixture",
        "WASM Fixture",
        "1.0.0",
    )
    .runtime(PackageRuntimeKind::WasmComponent, "1")
    .ability(
        AbilityKit::wasm(
            "ability.fixture.wasm",
            macaca_proto::ApplicationAbilityKind::Agent,
        )
        .activation("wasm_export", "app:start")
        .permission(
            "trace.emit",
            "WASM guest emits trace events through host ABI",
        )
        .build(),
    )
    .build();

    for manifest in [
        declarative,
        genui,
        headless,
        plugin_enhanced,
        store_entitled,
        wasm,
    ] {
        let report = ApplicationContractTestKit.validate_manifest(&manifest);
        assert!(
            report.is_success(),
            "fixture {} should satisfy application contracts: {report:?}",
            manifest.package_id
        );
    }

    // Keep the import used in this example visible for future WASM-specific
    // scaffold expansion without constructing any runtime.
    let _profile = ApplicationRuntimeProfile::new(PackageRuntimeKind::WasmComponent, "1");
}
