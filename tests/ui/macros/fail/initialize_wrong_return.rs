#![allow(unused_imports)]

use rust_jsc::{initialize, JSContext, JSObject};

#[initialize]
fn initialize(_ctx: JSContext, _object: JSObject) -> bool {
    true
}

fn main() {}
