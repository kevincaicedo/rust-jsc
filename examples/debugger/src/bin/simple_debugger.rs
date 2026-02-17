//! Simple Debugger Example (Thread-based)
//!
//! This example demonstrates the minimal setup required to:
//! 1. Initialize a JSContext with inspector support on a dedicated thread.
//! 2. Set up type-safe inspector handlers (pause, tick, resume).
//! 3. Validate pausing via:
//!    - `Debugger.pause` (pause at next opportunity), and
//!    - `debugger;` statement (pause at debugger statement).
//! 4. Resume execution programmatically from the host (on pause event).
//!
//! Usage:
//!   cargo run --manifest-path examples/debugger/Cargo.toml --bin simple_debugger -- <path/to/module.js>
//!
//! If no path is provided, defaults to:
//!   examples/debugger/scripts/simple_debugger.js

use rust_jsc::context::InspectorPauseEvent;
use rust_jsc::{
    callback, JSContext, JSFunction, JSObject, JSResult, JSValue,
    PropertyDescriptorBuilder,
};
use rust_jsc_macros::{inspector_callback, inspector_pause_event_callback};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

/// Shared state passed to the inspector callbacks.
/// This struct is owned by the JS thread and passed as the context data to callbacks.
struct DebuggerState {
    // We store the context here so callbacks can use it to send messages.
    // Since everything runs on one thread, we don't need Arc/Mutex for the context itself,
    // but the callbacks receive `&mut DebuggerState`.
    ctx: Arc<JSContext>,
    paused: bool,
    // Synchronization to notify the main thread (for demonstration purposes)
    sync: Arc<(Mutex<bool>, Condvar)>,
}

/// Callback for protocol messages (JSON) sent by the inspector backend.
#[inspector_callback]
fn on_inspector_message(message: &str) {
    println!("[Inspector Protocol] {}", message);
}

#[callback]
fn log_info(
    ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
    arguments: &[JSValue],
) -> JSResult<JSValue> {
    let message = arguments.get(0).unwrap().as_string().unwrap();
    println!("INFO: {}", message);

    Ok(JSValue::undefined(&ctx))
}

/// Single unified callback invoked for debugger pause-loop events (Paused/Resumed/Tick).
///
/// This uses the new C API and macro-generated extern "C" callback.
#[inspector_pause_event_callback]
fn on_pause_event(ctx: JSContext, event: InspectorPauseEvent) {
    // Recover our state from the context with type-safe downcasting.
    let state = unsafe { ctx.get_shared_data_mut::<DebuggerState>() };
    if state.is_none() {
        return;
    }
    let state = state.unwrap();

    match event {
        InspectorPauseEvent::Paused => {
            println!("[Host Callback] paused");
            state.paused = true;

            // Notify main thread that we paused
            let (lock, cvar) = &*state.sync;
            let mut started = lock.lock().unwrap();
            *started = true;
            cvar.notify_one();

            // Immediately resume for a deterministic handshake demo.
            println!("[Host Callback] paused: sending Debugger.resume...");
            state
                .ctx
                .inspector_send_message(r#"{"id": 1000, "method": "Debugger.resume"}"#);
        }
        InspectorPauseEvent::Tick => {
            if state.paused {
                // Optional visibility to prove ticks are happening.
                // Comment out if it's too chatty.
                println!("[Host Callback] tick: still paused...");
            }
        }
        InspectorPauseEvent::Resumed => {
            println!("[Host Callback] resumed");
            state.paused = false;
        }
    }
}

fn main() {
    println!("=== Simple Debugger Example ===");

    let module_path: PathBuf = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from("./examples/debugger/scripts/simple_debugger.js")
        });

    println!(
        "-> [Main Thread] Using module path: {}",
        module_path.display()
    );

    // Synchronization primitive to wait for the pause event
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair_clone = pair.clone();

    // Spawn a dedicated thread for JS execution.
    // JSContext is not Send/Sync, so it must live and die on the same thread.
    let handle = thread::spawn(move || {
        println!("-> [JS Thread] Initializing Context...");
        let ctx = Arc::new(JSContext::new());
        ctx.set_inspectable(true);
        ctx.set_inspector_callback(Some(on_inspector_message));

        let _global_object = ctx.global_object();

        let object = JSObject::new(&ctx);
        let attributes = PropertyDescriptorBuilder::new()
            .writable(true)
            .configurable(true)
            .enumerable(true)
            .build();
        let function = JSFunction::callback(&ctx, Some("log"), Some(log_info));
        object
            .set_property("log", &function.into(), attributes)
            .unwrap();

        // Setup state
        let state = DebuggerState {
            ctx: ctx.clone(), // JSContext is a wrapper around a pointer, clone is cheap (ref count)
            paused: false,
            sync: pair_clone,
        };

        // Store the state on the JSContext so the unified callback can access it.
        // The data is type-safe: retrieving with the wrong type returns None.
        ctx.set_shared_data(state);

        // Register unified pause-loop callback (Paused/Resumed/Tick)
        ctx.set_inspector_pause_event_callback(Some(on_pause_event));

        // Enable Debugger
        println!("-> [JS Thread] Enabling Debugger...");
        ctx.inspector_send_message(r#"{"id": 1, "method": "Debugger.enable"}"#);
        ctx.inspector_send_message(r#"{"id": 2, "method": "Runtime.enable"}"#);

        // IMPORTANT: allow debugger statements to pause.
        ctx.inspector_send_message(
            r#"{"id": 3, "method": "Debugger.setPauseOnDebuggerStatements", "params": {"enabled": true}}"#,
        );
        ctx.inspector_send_message(
            r#"{"id": 3, "method": "Debugger.setBreakpointsActive", "params": {"active": true}}"#,
        );

        // First: request a pause at the next opportunity.
        // If pause-at-next-opportunity is correctly wired, this should emit `Debugger.paused`
        // and trigger `on_pause`.
        println!("-> [JS Thread] Requesting pause via Debugger.pause (pause-at-next-opportunity)...");
        // ctx.inspector_send_message(r#"{"id": 4, "method": "Debugger.pause"}"#);

        // Evaluate using file-based module evaluation (evaluate_module).
        println!("-> [JS Thread] Evaluating module from file...");
        println!("-> [JS Thread] Module path: {}", module_path.display());

        // NOTE: `evaluate_module` expects a filesystem path.
        match ctx.evaluate_module(module_path.to_string_lossy().as_ref()) {
            Ok(val) => {
                println!("-> [JS Thread] Module Result: {:?}", val)
            }
            Err(e) => {
                println!("-> [JS Thread] Module Error:");
                println!("   name:   {:?}", e.name());
                println!("   message:{:?}", e.message());
                println!("   stack:  {:?}", e.stack());
            }
        }

        println!("-> [JS Thread] Finished.");
    });

    // Wait for the pause to happen (signaled by on_pause)
    println!("-> [Main Thread] Waiting for debugger to pause...");
    let (lock, cvar) = &*pair;
    let mut started = lock.lock().unwrap();

    // Wait with a timeout
    let result = cvar.wait_timeout(started, Duration::from_secs(5)).unwrap();
    started = result.0;

    if *started {
        println!("✓ [Main Thread] Confirmed: Debugger paused successfully!");
    } else {
        println!("✗ [Main Thread] Timeout: Debugger did not pause.");
    }

    // Wait for thread to finish
    handle.join().unwrap();
    println!("=== Example Finished ===");
}
