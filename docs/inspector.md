# Inspector And Debugger Guide

`rust-jsc` exposes JavaScriptCore's direct inspector channel for runtimes that
need a local debugger frontend, a CDP bridge, or a test harness around the WebKit
Inspector protocol. The safe path is [`InspectorSession`], created from a
context with `JSContext::inspector_session()`.

The session borrows the context, installs protocol callbacks, sends protocol
messages through a fallible API, and disconnects on drop. Context and inspector
handles remain JavaScriptCore-thread-affine.

## Direct Session

```rust
use rust_jsc::{
    inspector_callback, InspectorOutboundMessage, InspectorSession, JSContext,
    JSResult,
};

#[inspector_callback]
fn on_message(message: &str) {
    println!("{message}");
}

fn attach(ctx: &JSContext) -> JSResult<InspectorSession<'_>> {
    ctx.inspector_session()
        .on_message(on_message)
        .connect()
}

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let session = attach(&ctx)?;
    let message =
        InspectorOutboundMessage::borrowed(r#"{"id":1,"method":"Runtime.enable"}"#)?;
    session.send_protocol_message(message)?;
    Ok(())
}
```

`InspectorSession::send_message` rejects protocol strings with interior NUL
bytes before crossing the C API. The low-level `JSContext::inspector_send_message`
method has the same fallible contract for embedders that still manage callback
registration manually.

Use `InspectorOutboundMessage` when a debugger bridge has already built a
protocol string and wants to validate the C API boundary before sending. Use
`InspectorInboundMessage` inside callbacks to name the borrowed lifetime, and
copy it into `OwnedInspectorMessage` before queueing or crossing async/thread
boundaries.

## Pause Pump

Debugger pauses enter JavaScriptCore's nested pause loop. Register a pause
callback only when the host has a pump contract:

```rust
use rust_jsc::{
    inspector_callback, inspector_pause_event_callback, InspectorOutboundMessage,
    InspectorPauseEvent, JSContext, JSResult,
};

#[inspector_callback]
fn on_message(_message: &str) {}

#[inspector_pause_event_callback]
fn on_pause_event(ctx: JSContext, event: InspectorPauseEvent) {
    if event == InspectorPauseEvent::Paused {
        if let Ok(message) =
            InspectorOutboundMessage::borrowed(r#"{"id":2,"method":"Debugger.resume"}"#)
        {
            let _ = ctx.inspector_send_message(message.as_str());
        }
    }
}

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let _session = ctx
        .inspector_session()
        .on_message(on_message)
        .on_pause_event(on_pause_event)
        .connect()?;
    Ok(())
}
```

Pause callbacks run on the JavaScriptCore thread while execution is paused. They
should queue host work or send small protocol messages. They must not release the
context, destroy the VM, block indefinitely, or run unrelated JavaScript.

## Message Ownership

`#[inspector_callback]` receives a borrowed UTF-8 message slice. Wrap it in
`InspectorInboundMessage::borrowed(message)` when the callback wants to make
the lifetime explicit. The pointer is valid only for the callback duration.
Copy the string with `to_owned_message()` before returning when the message
must be stored, forwarded to another thread, or decoded later.

The inspector session does not store Rust callback state. Store debugger state
in a runtime-owned structure, a channel, or typed context shared data, then keep
all JavaScriptCore handle access on the owning JavaScriptCore thread.

## KedoJS And CDP Bridge Expectations

The rust-jsc layer owns the direct JavaScriptCore channel and the local safety
rules. A KedoJS CDP bridge should build on top of it by:

- translating CDP frontend messages into WebKit Inspector protocol messages
- copying inbound protocol messages before crossing thread or async boundaries
- using pause events as wakeups for a host pump, not as arbitrary execution hooks
- keeping breakpoint, step, resume, and evaluation state in KedoJS-owned types
- dropping the `InspectorSession` during runtime shutdown before releasing the
  JavaScriptCore context

Typed debugger state, richer inbound/outbound message wrappers, and the full CDP
translation layer remain runtime-level work. The 1.0 rust-jsc gate is the safe
session primitive, documented callback lifetime contract, and drop-time
disconnect behavior.
