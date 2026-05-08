//! Example: build a generic YAML application package descriptor.

fn main() {
    let descriptor = macaca_sdk::yaml_app_fixture();
    println!("{}", serde_json::to_string_pretty(&descriptor).unwrap());
}
