//! Minimal Breakpoint Debugger Example (module-based, single-process)
//!
//! Goal: demonstrate a complete breakpoint workflow:
//! - Enable Debugger + Runtime
//! - Activate breakpoints (Debugger.setBreakpointsActive)
//! - Load a JS module from a file (evaluate_module)
//! - Set breakpoint by URL
//! - Run code to hit breakpoint
//! - On pause: evaluate expressions (Debugger.evaluateOnCallFrame)
//! - Step (Debugger.stepNext) a few times
//! - Resume
//!
//! Usage:
//!   cargo run --manifest-path examples/debugger/Cargo.toml --bin breakpoint_debugger -- <path/to/module.js> [break_line_0_based]
//!
//! Defaults:
//!   module: ./examples/debugger/scripts/breakpoint_debugger.js
//!   break_line_0_based: 28 (line where `const result = {` is assigned)
//!
//! Notes:
//! - WebKit Inspector lineNumber is 0-based.
//! - This example is intentionally minimal and uses the inspector pause callbacks
//!   to reliably send step/resume commands while the VM is paused.

use rust_jsc::context::InspectorPauseEvent;
use rust_jsc::JSContext;
use rust_jsc_macros::{inspector_callback, inspector_pause_event_callback};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Condvar, Mutex, OnceLock};

#[derive(Debug)]
struct Shared {
    // Protocol messages received from JSC (raw JSON strings).
    messages: Vec<String>,

    // Whether we observed a Debugger.paused event.
    paused: bool,

    // How many stepNext operations we have performed in the current pause session.
    steps_done: u32,

    // How many steps we want before resuming.
    steps_target: u32,

    // If set, we will set breakpoint by this URL (usually the absolute path from scriptParsed).
    // We'll fill this from Debugger.scriptParsed (module: true) if the URL matches the module filename.
    module_url: Option<String>,

    // We sent resume and saw Debugger.resumed (or at least sent resume).
    resumed: bool,
}

#[derive(Clone)]
struct Sync {
    inner: Arc<(Mutex<Shared>, Condvar)>,
}

impl Sync {
    fn new() -> Self {
        let shared = Shared {
            messages: Vec::new(),
            paused: false,
            steps_done: 0,
            steps_target: 3,
            module_url: None,
            resumed: false,
        };

        Self {
            inner: Arc::new((Mutex::new(shared), Condvar::new())),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Shared> {
        (self.inner.0).lock().unwrap()
    }

    fn notify(&self) {
        (self.inner.1).notify_all();
    }

    fn wait_for<F: Fn(&Shared) -> bool>(
        &self,
        timeout: std::time::Duration,
        pred: F,
    ) -> bool {
        let (lock, cvar) = &*self.inner;
        let mut guard = lock.lock().unwrap();

        let start = std::time::Instant::now();
        while !pred(&guard) {
            let remaining = match timeout.checked_sub(start.elapsed()) {
                Some(r) => r,
                None => return false,
            };
            let (g, waitres) = cvar.wait_timeout(guard, remaining).unwrap();
            guard = g;
            if waitres.timed_out() && !pred(&guard) {
                return false;
            }
        }
        true
    }

    fn push_message(&self, msg: &str) {
        let mut s = self.lock();
        s.messages.push(msg.to_string());

        // Capture module URL from scriptParsed so we can set breakpoint by URL reliably.
        // We prefer the absolute URL emitted by Inspector.
        if let Ok(json) = serde_json::from_str::<Value>(msg) {
            if json.get("method").and_then(|m| m.as_str())
                == Some("Debugger.scriptParsed")
            {
                if let Some(params) = json.get("params") {
                    let is_module = params
                        .get("module")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if !is_module {
                        // This example is module-based; ignore non-module scripts.
                    } else if let Some(url) = params.get("url").and_then(|u| u.as_str()) {
                        // Store the most recent module URL. This is simplest and works for the default script.
                        // If you load multiple modules, you may want to match by filename.
                        s.module_url = Some(url.to_string());
                    }
                }
            } else if json.get("method").and_then(|m| m.as_str())
                == Some("Debugger.paused")
            {
                s.paused = true;
            } else if json.get("method").and_then(|m| m.as_str())
                == Some("Debugger.resumed")
            {
                s.resumed = true;
            }
        }

        drop(s);
        self.notify();
    }

    fn take_latest_paused_callframe_id(&self) -> Option<String> {
        let s = self.lock();
        // Walk messages backwards to find the last Debugger.paused, then grab first callFrameId.
        for msg in s.messages.iter().rev() {
            let Ok(json) = serde_json::from_str::<Value>(msg) else {
                continue;
            };
            if json.get("method").and_then(|m| m.as_str()) != Some("Debugger.paused") {
                continue;
            }
            let params = json.get("params")?;
            let frames = params.get("callFrames")?.as_array()?;
            let first = frames.first()?;
            let call_frame_id = first.get("callFrameId")?.as_str()?;
            return Some(call_frame_id.to_string());
        }
        None
    }

    fn module_url(&self) -> Option<String> {
        self.lock().module_url.clone()
    }
}

static INSPECTOR_TX: OnceLock<mpsc::Sender<String>> = OnceLock::new();

#[inspector_callback]
fn on_inspector_message(message: &str) {
    // NOTE: This prints all protocol traffic. It is noisy but valuable when iterating.
    println!("[Inspector Protocol] {}", message);

    // Forward to the JS-thread-owned receiver so it can update `Sync` without unsafe globals.
    if let Some(tx) = INSPECTOR_TX.get() {
        let _ = tx.send(message.to_string());
    }
}

struct HostState {
    ctx: Arc<JSContext>,
    sync: Sync,
    inspector_rx: Arc<Mutex<mpsc::Receiver<String>>>,
}

fn on_pause(state: &mut HostState) {
    println!("[Host Callback] on_pause: paused!");

    // Drain any pending inspector messages first (the Debugger.paused event may not have been processed yet)
    {
        let rx = state.inspector_rx.lock().unwrap();
        while let Ok(msg) = rx.try_recv() {
            state.sync.push_message(&msg);
        }
    }

    // Evaluate some expressions on the top frame.
    if let Some(call_frame_id) = state.sync.take_latest_paused_callframe_id() {
        // IMPORTANT: `callFrameId` is a JSON string; build payload via serde_json to avoid invalid JSON.
        let call_frame_id_json = Value::String(call_frame_id);

        // These expressions match `examples/debugger/scripts/breakpoint_debugger.js`
        // inside `compute()` at the breakpoint site.
        let evals: [(&str, i64, Value); 6] = [
            ("x", 2001, Value::String("x".to_string())),
            ("y", 2002, Value::String("y".to_string())),
            ("sum", 2003, Value::String("sum".to_string())),
            ("obj", 2004, Value::String("obj".to_string())),
            ("obj.nested", 2005, Value::String("obj.nested".to_string())),
            ("result", 2006, Value::String("result".to_string())),
        ];

        for (label, id, expr) in evals {
            println!("[Host Callback] evaluateOnCallFrame({label})...");

            let msg = serde_json::json!({
                "id": id,
                "method": "Debugger.evaluateOnCallFrame",
                "params": {
                    "callFrameId": call_frame_id_json,
                    "expression": expr,
                    "returnByValue": true,
                    "generatePreview": true
                }
            });

            state.ctx.inspector_send_message(&msg.to_string());
        }
    } else {
        println!("[Host Callback] on_pause: could not find callFrameId from Debugger.paused event");
    }

    // Stepping: driven from on_tick while paused.
}

fn on_tick(state: &mut HostState) {
    // While paused, we can decide what to do next.
    // We'll perform N stepNext operations and then resume.

    let mut shared = state.sync.lock();
    if !shared.paused {
        return;
    }

    if shared.steps_done < shared.steps_target {
        shared.steps_done += 1;
        let step_id = 3000 + shared.steps_done as i64;
        drop(shared);

        println!("[Host Callback] on_tick: stepNext (#{})", step_id);
        state.ctx.inspector_send_message(&format!(
            r#"{{"id": {step_id}, "method": "Debugger.stepNext"}}"#
        ));
        return;
    }

    if !shared.resumed {
        drop(shared);
        println!("[Host Callback] on_tick: resume");
        state
            .ctx
            .inspector_send_message(r#"{"id": 4000, "method": "Debugger.resume"}"#);
        return;
    }
}

fn on_resume(state: &mut HostState) {
    println!("[Host Callback] on_resume: resumed");
    // Nothing else to do; main thread will exit when JS thread finishes.
    let mut shared = state.sync.lock();
    shared.paused = false;
    drop(shared);
    state.sync.notify();
}

#[inspector_pause_event_callback]
fn on_pause_event(ctx: JSContext, event: InspectorPauseEvent) {
    let mut state_box = ctx.get_shared_data::<HostState>().unwrap();
    let state = &mut *state_box;

    match event {
        InspectorPauseEvent::Paused => on_pause(state),
        InspectorPauseEvent::Tick => on_tick(state),
        InspectorPauseEvent::Resumed => on_resume(state),
    }

    let _ = Box::into_raw(state_box);
}

fn main() {
    println!("=== Breakpoint Debugger Example (module) ===");

    let module_path: PathBuf = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from("./examples/debugger/scripts/breakpoint_debugger.js")
        });

    let break_line_0: u32 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(28);

    println!("-> Using module path: {}", module_path.display());
    println!("-> Breakpoint line (0-based): {}", break_line_0);

    let sync = Sync::new();

    // Channel for inspector protocol messages (collected by JS thread, applied to `Sync` there).
    let (inspector_tx, inspector_rx) = mpsc::channel::<String>();
    let _ = INSPECTOR_TX.set(inspector_tx);

    // Spawn JS thread (JSContext must live on that thread).
    let sync_for_js = sync.clone();
    let module_path_for_js = module_path.clone();

    let handle = std::thread::spawn(move || {
        println!("-> [JS Thread] Initializing Context...");
        let ctx = Arc::new(JSContext::new());
        ctx.set_inspectable(true);
        ctx.set_inspector_callback(Some(on_inspector_message));

        // Shared inspector_rx for HostState and drain helper
        let inspector_rx_shared = Arc::new(Mutex::new(inspector_rx));
        let inspector_rx_for_drain = inspector_rx_shared.clone();

        // Helper: drain pending inspector messages and apply them to `Sync`.
        let drain_inspector_rx = || {
            let rx = inspector_rx_for_drain.lock().unwrap();
            while let Ok(msg) = rx.try_recv() {
                sync_for_js.push_message(&msg);
            }
        };

        // Register pause handlers (host callbacks while paused).
        let host_state = Box::new(HostState {
            ctx: ctx.clone(),
            sync: sync_for_js.clone(),
            inspector_rx: inspector_rx_shared.clone(),
        });

        ctx.set_shared_data::<HostState>(host_state);
        ctx.set_inspector_pause_event_callback(Some(on_pause_event));

        // Enable domains.
        println!("-> [JS Thread] Enabling Debugger/Runtime...");
        ctx.inspector_send_message(r#"{"id": 1, "method": "Debugger.enable"}"#);
        ctx.inspector_send_message(r#"{"id": 2, "method": "Runtime.enable"}"#);

        // Ensure debugger statements can pause.
        ctx.inspector_send_message(
            r#"{"id": 3, "method": "Debugger.setPauseOnDebuggerStatements", "params": {"enabled": true}}"#,
        );

        // IMPORTANT for breakpoint workflows.
        ctx.inspector_send_message(
            r#"{"id": 4, "method": "Debugger.setBreakpointsActive", "params": {"active": true}}"#,
        );

        // Load module (defines exports, but does not hit any breakpoint until we call into it).
        println!("-> [JS Thread] Evaluating module...");
        let module_spec = module_path_for_js.to_string_lossy().to_string();
        match ctx.evaluate_module(&module_spec) {
            Ok(v) => println!("-> [JS Thread] evaluate_module result: {:?}", v),
            Err(e) => {
                println!("-> [JS Thread] evaluate_module error:");
                println!("   name:   {:?}", e.name());
                println!("   message:{:?}", e.message());
                println!("   stack:  {:?}", e.stack());
                return;
            }
        }

        // Process any pending scriptParsed / etc.
        drain_inspector_rx();

        // Wait until we have a module URL from Debugger.scriptParsed (used by setBreakpointByUrl).
        if !sync_for_js.wait_for(std::time::Duration::from_secs(2), |s| {
            s.module_url.is_some()
        }) {
            println!("-> [JS Thread] ERROR: did not observe Debugger.scriptParsed (module_url missing)");
            return;
        }
        let url = sync_for_js.module_url().unwrap();
        println!("-> [JS Thread] Observed module url: {url}");

        // Set breakpoint by URL.
        println!("-> [JS Thread] Setting breakpoint...");
        let msg = format!(
            r#"{{"id": 10, "method": "Debugger.setBreakpointByUrl", "params": {{"url": "{}", "lineNumber": {}}}}}"#,
            url, break_line_0
        );
        ctx.inspector_send_message(&msg);

        // Drain responses (so we don't miss state updates).
        std::thread::sleep(std::time::Duration::from_millis(50));
        drain_inspector_rx();

        // IMPORTANT: actually execute code that hits the breakpoint inside `compute()`.
        // We use a runner module that statically imports and calls compute() at top-level.
        // This ensures the breakpoint is hit in the correct call frame with x/obj/... in scope.
        println!("-> [JS Thread] Loading runner module to hit the breakpoint...");
        let runner_path = "./examples/debugger/scripts/breakpoint_runner.js";
        match ctx.evaluate_module(runner_path) {
            Ok(v) => println!("-> [JS Thread] Runner module result: {:?}", v),
            Err(e) => {
                println!("-> [JS Thread] Runner module error:");
                println!("   name:   {:?}", e.name());
                println!("   message:{:?}", e.message());
                println!("   stack:  {:?}", e.stack());
            }
        }

        // Allow time for pause/step/resume to happen while the nested pause-loop runs.
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Final drain for any trailing events.
        drain_inspector_rx();

        println!("-> [JS Thread] Done.");
    });

    // Main thread: wait to see the pause at least once (demo success condition).
    println!("-> [Main Thread] Waiting for Debugger.paused...");
    let ok = sync.wait_for(std::time::Duration::from_secs(10), |s| s.paused);
    if ok {
        println!("✓ [Main Thread] Observed Debugger.paused");
    } else {
        println!("✗ [Main Thread] Timeout waiting for Debugger.paused");
    }

    let _ = handle.join();
    println!("=== Example Finished ===");
}
