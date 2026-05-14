use criterion::{black_box, criterion_group, Criterion};
use rust_jsc::{inspector_callback, InspectorOutboundMessage, JSContext};

#[inspector_callback]
fn on_message(_message: &str) {}

fn bench_inspector_connect_disconnect(c: &mut Criterion) {
    c.bench_function("inspector_session_connect_disconnect", |b| {
        b.iter(|| {
            let ctx = JSContext::new();
            let session = ctx
                .inspector_session()
                .on_message(on_message)
                .connect()
                .unwrap();
            let _ = black_box(session);
        });
    });
}

fn bench_inspector_send_protocol_message(c: &mut Criterion) {
    let ctx = JSContext::new();
    let session = ctx
        .inspector_session()
        .on_message(on_message)
        .connect()
        .unwrap();
    let message =
        InspectorOutboundMessage::borrowed(r#"{"id":1,"method":"Runtime.enable"}"#)
            .unwrap();

    c.bench_function("inspector_send_protocol_message", |b| {
        b.iter(|| {
            session.send_protocol_message(message).unwrap();
        });
    });
}

fn bench_inspector_message_validation(c: &mut Criterion) {
    c.bench_function("inspector_message_validation", |b| {
        b.iter(|| {
            let message = InspectorOutboundMessage::borrowed(
                r#"{"id":2,"method":"Debugger.enable"}"#,
            )
            .unwrap();
            black_box(message);
        });
    });
}

criterion_group!(
    benches,
    bench_inspector_connect_disconnect,
    bench_inspector_send_protocol_message,
    bench_inspector_message_validation,
);
