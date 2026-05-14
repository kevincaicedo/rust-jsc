#![allow(unused_imports)]

use rust_jsc::{uncaught_exception_event_loop, JSContext, JSString};

#[uncaught_exception_event_loop]
fn uncaught_event_loop(_ctx: JSContext, _exception: JSString) {}

fn main() {}
