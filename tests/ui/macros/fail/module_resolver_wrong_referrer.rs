#![allow(unused_imports)]

use rust_jsc::{module_resolver, JSContext, JSResult};

#[module_resolver]
fn resolve(
    _ctx: JSContext,
    specifier: String,
    _referrer: String,
) -> JSResult<Option<String>> {
    Ok(Some(specifier))
}

fn main() {}
