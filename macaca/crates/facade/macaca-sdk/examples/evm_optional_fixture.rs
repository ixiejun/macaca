//! Example: build an optional EVM/DApp package descriptor.

fn main() {
    let descriptor = macaca_sdk::evm_optional_fixture();
    println!("{}", serde_json::to_string_pretty(&descriptor).unwrap());
}
