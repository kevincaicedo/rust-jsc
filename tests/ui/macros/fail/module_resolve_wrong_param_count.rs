#![allow(unused_imports)]

use rust_jsc::{module_resolve, JSContext, JSStringProtected, JSValue};

#[module_resolve]
fn resolve(_ctx: JSContext, key: JSValue, _referrer: JSValue) -> JSStringProtected {
    JSStringProtected::from(key.as_string().unwrap().to_string())
}

fn main() {}
