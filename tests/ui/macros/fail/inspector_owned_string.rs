use rust_jsc::inspector_callback;

#[inspector_callback]
fn inspector(_message: String) {}

fn main() {}
