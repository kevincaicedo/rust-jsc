#![allow(unused_imports)]

use rust_jsc::{uncaught_exception, JSContext, JSString, JSValue};

#[uncaught_exception]
fn uncaught(_ctx: JSContext, _filename: JSString, _exception: JSValue) -> bool {
    true
}

fn main() {}
