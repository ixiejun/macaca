//! Kernel microkernel dependency purity gate (VC-kernel-purity / P5 §6.1.5).
//!
//! Verifies `macaca-kernel` direct workspace dependencies are limited to
//! foundation protocol contracts (`macaca-proto`, `macaca-ipc`).

#[path = "kernel_purity_gate/gate.rs"]
mod gate;

#[test]
fn kernel_purity_gate_rejects_forbidden_workspace_dependencies() {
    gate::assert_kernel_workspace_dependency_purity();
}
