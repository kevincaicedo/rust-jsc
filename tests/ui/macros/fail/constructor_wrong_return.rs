#![allow(unused_imports)]

use rust_jsc::{constructor, JSContext, JSObject};

struct NotConvertible;

#[constructor]
fn constructor(_ctx: JSContext, _constructor: JSObject) -> NotConvertible {
    NotConvertible
}

fn main() {}
