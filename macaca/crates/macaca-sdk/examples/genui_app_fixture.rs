//! Example: build a traced GenUI application package descriptor.

fn main() {
    let descriptor = macaca_sdk::genui_app_fixture();
    println!("{}", serde_json::to_string_pretty(&descriptor).unwrap());
}
