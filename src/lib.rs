
pub use rust_jsc_sys as sys;

pub mod bindings {
    pub use rust_jsc_sys::*;
}

/// A JavaScript class.
///
/// The best way to create a class is by using [`JSClass::builder`].
pub struct JSClass {
    ctx: sys::JSContextRef,
    raw: sys::JSClassRef,
}