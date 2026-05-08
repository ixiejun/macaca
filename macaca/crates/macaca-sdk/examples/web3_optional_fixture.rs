//! Example: build an optional Web3 application package descriptor.

fn main() {
    let descriptor = macaca_sdk::web3_optional_fixture();
    println!("{}", serde_json::to_string_pretty(&descriptor).unwrap());
}
