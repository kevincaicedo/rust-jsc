#![allow(unused_imports)]

use rust_jsc::{module_evaluate, JSContext, JSStringProtected, JSValue};

#[module_evaluate]
fn evaluate(_ctx: JSContext, _key: JSValue) -> JSStringProtected {
    JSStringProtected::from("done")
}

fn main() {}
