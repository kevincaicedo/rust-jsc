# API Freeze Audit

This audit reviews the `rust-jsc` 1.0 safe API from two perspectives:
external Rust embedders and KedoJS. The goal is to keep the stable surface
small, safe, and reusable while avoiding KedoJS-only shortcuts in the binding
crate.

## Freeze Rules

- KedoJS must not depend on `rust_jsc_sys` or raw JavaScriptCore references for
  ordinary runtime behavior.
- JavaScriptCore exceptions must surface as `JSResult` or typed errors.
- Rust panics must not cross callback, module-loader, inspector, timer, or
  async-completion boundaries.
- Ownership must be named in the type or method: owned, borrowed, protected,
  guard, resolver, builder, session, or handle.
- Safe-path ergonomics must not add unmeasured hot-path overhead. Fast paths
  must be explicit about unchecked invariants.

## Public Primitives Now Covering KedoJS

| KedoJS need | Public `rust-jsc` primitive | Status |
| --- | --- | --- |
| Context ownership | `JSGlobalContext`/`OwnedJSContext`, borrowed `JSContext`, owned/borrowed context groups | Covered for current runtime state shape. KedoJS still needs to remove misleading `Arc` from JS-thread-only state. |
| Shared runtime state | `set_shared_data`, `get_shared_data`, `get_shared_data_mut`, `drop_shared_data`, `replace_shared_data` with guard-aware status values | Covered. KedoJS should continue moving runtime failure paths from infallible `downcast_state()` to fallible state extraction. |
| JS resources/classes | `JSClassBuilder`, `JSClass::try_builder`, typed `build::<T>`, `JSClass::object::<T>`, `JSClass::object_with_prototype`, `JSClass::set_object_private_data`, `JSClassBuilder::constructor_method`, `JSClassBuilder::typed_accessor`, `JSClass::constructor_object` | Covered for class creation, explicit prototypes, private-data attachment, constructor static methods, and typed static-value accessors. Ergonomic class macros remain API polish, not a raw-binding dependency. |
| Private data reads | `PrivateDataRef<T>` and `PrivateDataMut<T>` guards | Covered for synchronous access. Async resource reads still need KedoJS-level in-flight state instead of ad hoc take/restore policy. |
| Protected JS handles | `ProtectedValue` and `ProtectedObject` | Covered. KedoJS no longer needs its own object-protection wrapper for async callbacks/resources. |
| Deferred promises | `Promise`/`JSPromise`, `PromiseResolver`, and `UnhandledRejectionHandler` | Covered for direct deferred promises, RAII-protected resolve/reject functions, and same-context callable unhandled-rejection handler registration. A richer Rust future bridge remains runtime-level work. |
| Modules | `ModuleLoader`, `ModuleLoaderBuilder`, typed module-loader macros, `ModuleSource`, `JSModuleSource`, `ModuleLoadError`, explicit synthetic modules | Covered for current file/custom/synthetic loader paths and module-id/referrer diagnostics. Explicit async completion policy remains a runtime/post-1.0 design item, not a hidden rust-jsc 1.0 blocker. |
| Typed arrays and buffers | Copied reads, owned `Vec` transfer, ArrayBuffer construction, typed buffer views, unsafe borrowed views | Covered for current KedoJS byte movement. Remaining zero-copy paths must stay benchmarked and explicit. |
| Inspector | `InspectorSessionBuilder`, `InspectorSession`, `InspectorInboundMessage`, `InspectorOutboundMessage`, and `OwnedInspectorMessage` | Covered for direct JavaScriptCore inspector channels, borrowed/copyable protocol-message ownership, and validated outbound sends. CDP translation, typed protocol events, and debugger-state modeling belong above `rust-jsc`. |

## Raw Shortcut Audit

The current KedoJS integration no longer needs the old `kedo_utils` raw
private-data helpers:

- `downcast_ptr`
- `upcast`
- `drop_ptr`
- `ManuallyDropArc`
- `ManuallyDropClone`

Those helpers reconstructed or erased Rust ownership outside the
`rust-jsc` private-data guard model. New code should use:

- `JSClass::object::<T>(ctx, Some(value))` for class-owned resource storage.
- `JSClass::set_object_private_data` for class-checked late attachment.
- `get_private_data` / `get_private_data_mut` for guarded synchronous access.
- typed state wrappers such as KedoJS `InFlightResource<T>` or
  `OneShotResource<T>` when an async operation must mark a resource as busy or
  consumed across an `await`, paired with `ProtectedObject`/`ProtectedValue`
  guards for JS handles that must survive until terminal completion.

Do not use raw private-data take/replacement as the normal async-resource
pattern. Leaving a typed state wrapper in private data keeps aliasing visible to
later calls and lets concurrent operations fail or queue predictably.

## Post-Freeze Runtime Gaps

These gaps remain real, but they are either covered for the reusable `rust-jsc`
1.0 substrate or belong above the binding layer. They do not justify private
KedoJS binding shortcuts:

| Gap | Owner | Current decision |
| --- | --- | --- |
| Async resource in-flight state | KedoJS, with `rust-jsc` guard primitives | Current KedoJS async resource paths use typed `InFlightResource<T>` or `OneShotResource<T>` state plus protected JS handles. Broader cancellation and shutdown policy remains KedoJS runtime work. |
| Panic-free KedoJS module callbacks | KedoJS | Runtime loaders should continue converting module id, referrer, and loader failures into JavaScript module errors or promise rejections, not `unwrap`/`unreachable!`. |
| Promise future bridge | `rust-jsc` API plus KedoJS runtime policy | Direct deferred promises, resolver RAII, microtask/deferred-work checkpoints, and unhandled-rejection handlers are covered. A Rust `Future` bridge remains runtime policy because JSC handles stay JS-thread-affine. |
| Inspector typed messages | `InspectorInboundMessage`, `InspectorOutboundMessage`, `OwnedInspectorMessage`, KedoJS CDP bridge | `rust-jsc` covers message ownership/lifetime and outbound C-boundary validation without hard-coding CDP. KedoJS still owns protocol parsing, typed CDP events, and debugger state. |
| Class method/property adapters | `rust-jsc` | Optional ergonomic adapters are post-builder polish, outside the 1.0 low-level macro contract. They must generate builder calls and preserve the static-table, no per-instance allocation path. |
| Performance gates | `rust-jsc` CI | Benchmark and profiler artifact paths are covered by `docs/performance-validation.md` and `scripts/performance_snapshot.sh`. Numeric allocation/instruction budgets remain deferred until trend variance is known. |

## Freeze Decision

The binding crate has enough public primitives for KedoJS to avoid raw private
data and manual protection in the current integration paths. `rust-jsc` should
not add KedoJS-specific APIs to close runtime-policy gaps. Future work should
either harden reusable primitives already listed here or live in KedoJS as
runtime policy.
