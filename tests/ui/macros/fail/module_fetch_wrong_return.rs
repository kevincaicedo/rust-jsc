#![allow(unused_imports)]

use rust_jsc::{module_fetch, JSContext, JSValue};

#[module_fetch]
fn fetch(
    ctx: JSContext,
    _key: JSValue,
    _attributes: JSValue,
    _fetcher: JSValue,
) -> JSValue {
    JSValue::undefined(&ctx)
}

fn main() {}
