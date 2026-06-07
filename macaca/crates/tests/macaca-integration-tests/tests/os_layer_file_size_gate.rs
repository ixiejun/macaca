//! OS-layer file-size boundary gate (VC-filesize).
//!
//! Enforces the Agent OS constitution: production sources under `crates/**/src`
//! must not exceed 500 lines unless explicitly recorded as migration debt in
//! `os_layer_file_size_gate/allowlist.rs`.

#[path = "os_layer_file_size_gate/allowlist.rs"]
mod allowlist;
#[path = "os_layer_file_size_gate/gate.rs"]
mod gate;

#[test]
fn os_layer_file_size_gate_reject_unallowlisted_oversized_sources() {
    gate::assert_os_layer_file_size_boundaries();
}
