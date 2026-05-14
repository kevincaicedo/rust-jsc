# rust-jsc API Design Guide

This guide defines the public API shape expected for the 1.0 stabilization
work. It is written for `rust-jsc` embedding users, KedoJS runtime code, macro
authors, and maintainers of the JavaScriptCore fork.

## Design Goals

The default path should be safe, short, and clear:

```rust
use rust_jsc::callback;

#[callback]
fn add(left: f64, right: f64) -> f64 {
    left + right
}
```

The API should make JavaScriptCore semantics visible without making normal Rust
code look like raw C API glue:

- fallible JavaScriptCore calls return `JSResult<T>`
- borrowed handles do not release JavaScriptCore references
- owned handles release exactly once in `Drop`
- callbacks and module-loader hooks never panic across the C ABI
- async JavaScript completion is represented by promises and explicit host
  microtask or deferred-work pumps

## Ownership Vocabulary

Use names that state ownership and lifetime directly.

| Name | Meaning |
| --- | --- |
| `JSContext` | Borrowed JavaScriptCore context view. It does not retain or release. |
| `JSGlobalContext` / `OwnedJSContext` | Owned global context. Releases in `Drop`. |
| `JSContextGroup` | Borrowed context-group view. It does not release. |
| `OwnedJSContextGroup` | Retained context group. Releases in `Drop`. |
| `JSString` | Owned JavaScriptCore string. Releases in `Drop`. |
| `JSStringProtected` | RAII retained JavaScriptCore string; use `into_raw` only for explicit ownership transfer. |
| `ProtectedValue` | RAII protected JS handle. Protects on creation, unprotects in `Drop`. |
| `ProtectedObject` | Object-typed RAII protected JS handle for callable/resource objects. |
| `Promise` / `JSPromise` | JavaScript promise handle. Context-affine and not `Send` or `Sync`. |
| `PromiseResolver` | RAII-protected resolve/reject functions for a deferred promise. |
| `Builder` | Owns all temporary strings, callbacks, and configuration until build time. |
| `Session` | Owns an attached protocol, inspector, debugger, or runtime interaction until dropped. |

Misspelled legacy names stay as deprecated aliases during the 1.0 migration
window. New code must use `JSStringProtected`, `is_constructor`, and
`JSFunction::constructor`.

## Context Affinity

JavaScriptCore values are context and group affine. A `JSValue`, `JSObject`,
class instance, protected value, inspector session, promise resolver, or typed
array view must be used on the JavaScriptCore thread that owns its context
unless a higher-level API explicitly documents a safe handoff protocol.

`rust-jsc` types intentionally avoid `Send` and `Sync` for borrowed or owned
context handles. KedoJS should move work across threads through Rust-owned
messages, channels, buffers, or promises, then touch JavaScriptCore again only
from the JS thread.

## Conversion Traits

The conversion traits define the safe path for callbacks, builders, and typed
module-loader APIs.

| Trait | Use |
| --- | --- |
| `TryFromJSValue` | Fallible JS-to-Rust conversion. Used by typed callback macro arguments. |
| `FromJSValue` | Narrow infallible JS-to-Rust conversion. Use sparingly. |
| `IntoJSValue` | Fallible Rust-to-JS conversion with an explicit context. |
| `IntoJSResult` | Callback return conversion extension point. |
| `IntoModuleResolveResult` | Typed module resolver return conversion. |
| `IntoModuleSourceResult` | Typed module fetch-source return conversion. |
| `IntoImportMetaResult` | Typed `import.meta` object return conversion. |

`TryFromJSValue` is the default for user input. Numeric conversions that narrow
the value must reject `NaN`, infinities, fractional values, and out-of-range
values instead of silently truncating.

`Option<T>` treats JavaScript `undefined` and `null` as `None`; otherwise it
uses `T: TryFromJSValue`. Missing optional callback or constructor arguments
are also `None`.

`Rest<T>` is the typed rest-argument wrapper for callback and constructor
macros. It must be the final user argument in the signature and converts every
remaining JavaScript argument with `T: TryFromJSValue`.

Callback and constructor return values go through `IntoJSResult`. This keeps
the raw `JSResult<JSValue>` path valid while allowing typed return values such
as `String`, `bool`, numbers, `JSObject`, `Option<T>`, `()`, and `JSResult<T>`
where `T: IntoJSValue`. Unsupported return types should fail at compile time
through the generated trait bound.

Custom callback extractors are registered by implementing `TryFromJSValue` for
the target type. Custom callback returns implement `IntoJSValue`; custom module
loader return wrappers implement the relevant `IntoModule*` trait. This keeps
extension points static and avoids a runtime registry or dynamic dispatch in
the callback path.

## Callback Role Injection

New `#[callback]` and `#[constructor]` signatures should request only the roles
they need. Plain typed parameters are JavaScript arguments converted through
`TryFromJSValue`, so a short callback can be written as:

```rust
#[callback]
fn add(left: f64, right: f64) -> f64 {
    left + right
}
```

When a wrapper needs JavaScriptCore ABI roles, use the explicit zero-cost role
markers before JavaScript arguments:

- `CallbackContext`: borrowed JavaScriptCore context view.
- `CallbackFunction`: function object being called.
- `ThisObject`: JavaScript `this` receiver for `#[callback]`.
- `ConstructorObject`: constructor object for `#[constructor]`.

The legacy positional callback prefix `(JSContext, JSObject, JSObject, ...)`
and constructor prefix `(JSContext, JSObject, ...)` remain supported during the
1.0 migration window. New ergonomic signatures use role markers so an ordinary
`JSObject` parameter can unambiguously mean a converted JavaScript argument.

## Typed Calls

When the expected return type is known, use the typed call helpers instead of
calling first and converting later:

- `JSFunction::call_typed`
- `JSFunction::call_constructor_typed`
- `JSObject::call_typed`
- `JSObject::call_as_constructor_typed`
- `JSObject::call_method_typed`

These helpers preserve the same JavaScriptCore exception behavior as the raw
safe calls, then convert the result through `TryFromJSValue`. Conversion failure
is reported as `JSError`; it does not panic and does not cross FFI.

`JSFunction` also exposes small metadata helpers:

- `JSFunction::name`
- `JSFunction::display_name`
- `JSFunction::source`

These helpers deliberately use observable JavaScript property access and
`toString` semantics, so accessor errors, overridden `toString` errors, and
string-conversion errors are returned as `JSError`.

## Builders

Builders are the preferred shape for multi-step JavaScriptCore resources:

- classes and prototypes
- module loaders and synthetic modules
- inspector sessions
- promise resolver hooks
- embedding/runtime setup

A builder should own temporary strings and callback tables until the C API has
copied or retained what it needs. It should validate duplicate names, null
callbacks, unsupported combinations, and context mismatches before crossing FFI.

`InspectorSessionBuilder` is the direct inspector safe path. It requires a
message callback, makes the context inspectable by default, optionally registers
a pause-loop callback, and returns an `InspectorSession` that disconnects on
drop. Protocol messages are sent through `InspectorSession::send_message`, which
is fallible for strings that cannot cross the C API.

## Object Properties

Use `PropertyKey` when code needs one API for string names, JavaScript value
keys, and array-index keys. `JSObject::set_property_by_key`,
`get_property_by_key`, `has_property_by_key`, and `delete_property_by_key`
preserve JavaScript property-key semantics while checking value keys and values
against the object's context before crossing FFI.

The older string-specific and value-specific methods remain available for
low-level compatibility, but new ergonomic APIs should prefer typed keys over
ambiguous raw strings or ad hoc `JSValue` arguments.

Use `PropertyKey::symbol(&ctx, description)` for symbol-keyed properties. Symbol
keys are JavaScript values, so the object API also propagates `ToPropertyKey`
exceptions when a value key has a throwing conversion hook.

`PropertyDescriptor::builder` is the safe path for JavaScriptCore property
attributes. Prefer named builder steps such as `read_only`,
`non_enumerable`, and `non_configurable` when the desired descriptor is static;
boolean setters remain available when a caller is translating runtime policy.

`JSObject::get_property_names` returns an RAII iterator that releases the
JavaScriptCore property-name array on drop. The iterator is exact-size and fused
so callers can pre-size collections and safely continue polling after
exhaustion.

## Inspector Sessions

Inspector callbacks are direct JavaScriptCore protocol hooks. The message
callback receives a borrowed UTF-8 slice for the callback duration only. The
pause callback is a debugger-pump notification that runs on the JavaScriptCore
thread while execution is paused.

The safe wrapper is `InspectorSession`, not a long-lived raw callback slot:

- one session owns the direct frontend connection for a context
- the session disconnects and clears callbacks on drop
- `send_message` returns `JSResult<()>` instead of panicking on invalid C strings
- pause callbacks should queue host work or send simple protocol messages only
- CDP translation and richer debugger state stay above rust-jsc, in KedoJS or
  another embedding runtime

## Macros

Macros are adapters, not a hidden second API. They should generate calls into
safe builders, conversion traits, and typed wrappers.

Macro requirements:

- reject unsupported signatures at compile time
- catch Rust panics before returning across JavaScriptCore
- guard null raw inputs from C callbacks
- avoid allocating argument vectors for typed callback arguments
- convert JavaScript exceptions through `JSResult`
- keep generated local names private and collision resistant

The current fixed-arity typed callback path is the recommended macro path when
the handler can express its arguments through `TryFromJSValue`. Its Phase 4
guardrail is no more than 5% mean overhead versus a manual raw callback on the
same primitive two-argument workload. If a future macro shape exceeds that
budget, keep it documented as ergonomic-only until profiling explains the cost
or the generated code is tightened.

Low-level legacy signatures remain available where needed, for example
`&[JSValue]` callback or constructor arguments. New typed signatures should use
owned conversion targets such as `String`, `JSString`, `f64`, `bool`,
`JSObject`, `JSArray`, `JSTypedArray`, `Option<T>`, or final-position
`Rest<T>`.

Class lifecycle macros such as `#[initialize]` and `#[finalize]` are low-level
adapters. They should validate exact JavaScriptCore callback shapes, guard null
raw inputs, and catch panics before returning across FFI.

## Unsafe Escape Hatches

Unchecked or raw APIs are allowed only when the safety invariant is local,
documented, and testable. Public unsafe APIs must include a `# Safety` section.

Good escape hatches:

- `unsafe borrowed(raw)` constructors for callback adapters
- `unsafe JSValue::from_raw_unchecked` and
  `unsafe JSObject::from_raw_unchecked` when the caller can prove the raw
  handle belongs to the live context and remains valid for the wrapper use
- `unsafe JSString::from_owned_ref` for owned JavaScriptCore strings returned
  by copy/create APIs, and `unsafe JSString::retain_from_ref` for borrowed
  strings that must become Rust-owned wrappers
- unchecked conversions that state exact context, type, length, and lifetime
  requirements
- raw sys APIs in `rust_jsc_sys` for C ABI validation and WebKit work

Bad escape hatches:

- storing raw `JSValueRef` without context ownership
- releasing caller-owned JavaScriptCore references
- clearing private data when the Rust type does not match
- using `Arc<JSContext>` to imply cross-thread JavaScriptCore safety

## Typed Arrays and ArrayBuffers

Safe typed-array APIs should not expose borrowed JavaScriptCore byte pointers by
default. JavaScriptCore documents those pointers as temporary, so the safe path
copies bytes into Rust-owned output or transfers Rust-owned input into a
JavaScriptCore backing store with an explicit deallocator.

Use these APIs for normal runtime code:

- `JSTypedArray::as_vec<T>` copies a typed array into a Rust `Vec<T>`.
- `JSTypedArray::with_bytes<T>` copies a Rust slice into a new typed array.
- `JSTypedArray::with_owned_bytes<T>` transfers a `Vec<T>` to JavaScriptCore
  without copying the vector buffer.
- `JSArrayBuffer::from_bytes` copies a Rust byte slice.
- `JSArrayBuffer::from_vec` transfers a Rust byte vector.

`JSTypedArrayElement` is sealed to primitive numeric element types so the safe
API cannot reinterpret JavaScript bytes as Rust types with destructors or
arbitrary invalid states. Safe typed-array reads also verify that the Rust
element type matches the JavaScript typed-array kind.

Borrowed no-copy APIs such as `JSTypedArray::bytes`,
`JSTypedArray::bytes_from_value`, `JSTypedArray::with_bytes_no_copy`,
`JSArrayBuffer::bytes`, and `JSArrayBuffer::with_bytes_no_copy` are unsafe
escape hatches. Their callers own the proof that no JavaScriptCore API runs
while a returned slice is live and that caller-owned external storage remains
allocated and pinned until all JavaScript views are gone.

## Private Data

The safe private-data path should go through the owner that knows the type
contract. `JSClass::set_object_private_data` attaches data only when `T`
matches the class type, the target object is an instance of that class, and the
private-data slot is empty.

Erased object and context private-data APIs are safe only for rust-jsc-owned
private/shared-data pointers. The implementation records pointers created by
`PrivateDataWrapper::into_raw` in a thread-local provenance registry before any
safe erased accessor reads `PrivateDataHeader`. Foreign private pointers are
reported as absence or type mismatch; callers that install arbitrary private
pointers through `rust_jsc_sys` stay responsible for their own unsafe access
and cleanup path.

`get_private_data` and `get_shared_data` return `PrivateDataRef<T>` guards.
`get_private_data_mut` and `get_shared_data_mut` return `PrivateDataMut<T>`
guards only when no other guard is active. Safe take, drop, and shared-data
replacement paths return `Borrowed` status variants instead of invalidating an
active guard. Raw object replacement remains unsafe because the caller must also
prove the target object's class finalizer agrees with the replacement type.

Use `ProtectedObject` when Rust needs to store an object-typed JavaScript handle
outside JavaScriptCore's visible object graph, such as a callback function,
stream resource, or host completion object. It protects on creation, unprotects
on drop, and dereferences to `JSObject` for normal calls.

## Class Builders

`JSClass::try_builder` is the safe-path constructor when class names come from
runtime input. It rejects interior NUL bytes as `ClassError` instead of
panicking while `JSClass::builder` remains the concise path for static Rust
names.

`JSClassBuilder::method` installs prototype methods through JavaScriptCore's
static function table. This keeps method entries shared through the generated
prototype and avoids per-instance method allocation. The builder rejects missing
method callbacks and invalid method names before calling JavaScriptCore.

`JSClassBuilder::constructor_method` installs static methods on the class object
created by `JSClass::constructor_object` and `JSClass::register`. These methods
are own properties of the registered constructor object rather than prototype
members, so `ClassName.method()` does not add methods to every instance.

`JSClassBuilder::accessor` installs known properties through JavaScriptCore's
static value table. It rejects empty accessor definitions and invalid accessor
names before calling JavaScriptCore. `JSClassBuilder::typed_accessor` is the
safe typed path for known Rust property values: users implement
`JSClassAccessor` on a marker type, and the builder installs monomorphized
JavaScriptCore getter/setter adapters that convert through `TryFromJSValue` and
`IntoJSValue` and catch panics before the C ABI boundary.

`JSClass::object_with_prototype` is the explicit prototype setup path for
classes that opt out of JavaScriptCore's automatic prototype generation or need
embedder-owned prototypes. It validates context affinity before assigning the
prototype. `JSObject::set_prototype_checked` provides the same fallible check
for objects assembled outside a class builder.

`JSClassBuilder::build` follows JavaScriptCore's create rule: `JSClassCreate`
returns an owned class reference, and `JSClass` releases that single reference
in `Drop`. The builder must not call `JSClassRetain` after creation unless a
separate owner is intentionally introduced.

## Performance Contract

Ergonomic APIs should not silently become much slower than raw bindings.

Before making performance claims, record the workload, environment, JSC build
mode, target triple, command, and artifact path. Benchmarks should cover:

- context creation
- script and module evaluation
- property get/set
- function and callback calls
- string conversion
- typed arrays and ArrayBuffers
- promises and microtask drains
- inspector message paths

The typed callback and constructor macros parse arguments directly from
JavaScriptCore's raw argument slice. They only build a `Vec<JSValue>` for the
explicit legacy `&[JSValue]` form.

The current 1.0 review keeps context/value/object lifetime as an open release
gate. Safe `JSContext`, `JSValue`, and `JSObject` wrappers are context-affine,
but they do not yet encode the owning global-context lifetime in Rust types or
automatically retain/protect every returned handle. A v1 release must either
land the lifetime/ownership redesign or explicitly document the accepted
short-lived-handle compromise before publishing.

Object calls, constructor calls, method calls, array/date/regexp/error
construction stage raw `JSValueRef` argument arrays on the stack for up to eight
arguments and fall back to a heap allocation for larger argument lists. This
keeps the common small-argument path allocation-free without changing the public
API or hiding JavaScriptCore exception behavior.
