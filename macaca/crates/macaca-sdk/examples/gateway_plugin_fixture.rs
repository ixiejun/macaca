//! Example: build a generic gateway plugin package descriptor.

fn main() {
    let descriptor = macaca_sdk::gateway_plugin_fixture();
    println!("{}", serde_json::to_string_pretty(&descriptor).unwrap());
}
