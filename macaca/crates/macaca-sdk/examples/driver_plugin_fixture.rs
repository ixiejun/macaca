//! Example: build a generic driver plugin package descriptor.

fn main() {
    let descriptor = macaca_sdk::driver_plugin_fixture();
    println!("{}", serde_json::to_string_pretty(&descriptor).unwrap());
}
