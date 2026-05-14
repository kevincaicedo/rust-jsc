#![allow(unused_imports)]

use rust_jsc::{finalize, JSValue};

#[finalize]
fn finalize(_data: JSValue) {}

fn main() {}
