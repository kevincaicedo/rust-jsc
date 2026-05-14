#![allow(unused_imports)]

use rust_jsc::{inspector_pause_event_callback, JSContext, JSValue};

#[inspector_pause_event_callback]
fn pause(_ctx: JSContext, _event: JSValue) {}

fn main() {}
