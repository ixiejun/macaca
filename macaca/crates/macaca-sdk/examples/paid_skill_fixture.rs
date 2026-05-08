//! Example: build free and paid skill package descriptors.

fn main() {
    let descriptors = vec![
        macaca_sdk::free_skill_fixture(),
        macaca_sdk::paid_skill_fixture(),
    ];
    println!("{}", serde_json::to_string_pretty(&descriptors).unwrap());
}
