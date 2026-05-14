#![allow(unused_imports)]

use rust_jsc::{module_import_meta, JSContext, JSStringProtected, JSValue};

#[module_import_meta]
fn import_meta(
    _ctx: JSContext,
    _key: JSValue,
    _fetcher: JSValue,
) -> JSStringProtected {
    JSStringProtected::from("meta")
}

fn main() {}
