//! Example: build a WASM metadata-only package descriptor.

fn main() {
    let descriptor = macaca_sdk::wasm_stub_app_fixture();
    println!("{}", serde_json::to_string_pretty(&descriptor).unwrap());
}
