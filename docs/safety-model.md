# rust-jsc Safety Model

rust-jsc's safe API is responsible for making JavaScriptCore ownership,
context affinity, exception behavior, and callback boundaries explicit. Raw C
APIs remain available through `rust_jsc_sys`, but application and runtime code
should use the safe wrappers unless it is implementing a new wrapper with a
documented invariant.

## Context And Group Ownership

`JSContext` is a borrowed JavaScriptCore context view. It does not retain or
release the underlying `JSContextRef`.

`JSGlobalContext` is the owned global-context handle. `OwnedJSContext` is an
alias for that owned handle. It releases its `JSGlobalContextRef` exactly once
in `Drop`.

`JSContextGroup` is a borrowed group view. `OwnedJSContextGroup` retains and
releases the group through RAII.

Contexts and groups are not `Send` or `Sync`. A context, its values, objects,
protected handles, promises, typed arrays, and inspector sessions must be used
on the JavaScriptCore thread that owns the VM/group unless a higher-level API
documents a safe handoff protocol.

## Values, Objects, And Context Matching

`JSValue` and `JSObject` handles are context-affine. APIs that combine values
or objects from different contexts must check context identity before crossing
the JavaScriptCore C API.

Open 1.0 gate: these wrappers currently do not carry a Rust lifetime tied to
their owning `JSGlobalContext`, and ordinary `JSValue`/`JSObject` handles are
not automatically protected roots. Code that stores JavaScript handles outside
the immediate call path should use `ProtectedValue` or `ProtectedObject`, and
the release must choose between lifetime-parameterized borrowed wrappers,
retained/protected owned wrappers, or an explicitly documented short-lived
handle compromise before publishing.

Fallible JavaScriptCore operations return `JSResult<T>`. They must propagate
exception slots as `JSError` instead of constructing null handles, returning
success after an exception, or panicking. Public helpers that perform observable
JavaScript operations, such as property access or function calls, keep those
operations fallible.

Protected handles keep JavaScript values alive while Rust stores them outside
JavaScriptCore's object graph. `ProtectedValue` and `ProtectedObject` call
`JSValueProtect` on creation and `JSValueUnprotect` exactly once in `Drop`.
Clone creates a separate protection count.

## Private And Shared Rust Data

Rust data attached to JavaScriptCore private-data or shared-data slots is stored
through a typed wrapper, not as an untracked raw pointer. Reads verify `TypeId`
before returning a typed guard.

Safe erased access also verifies rust-jsc provenance before reading the typed
wrapper header. Pointers produced by `PrivateDataWrapper::into_raw` are
registered in a thread-local registry and unregistered on successful take/drop
or finalization. If a private-data slot contains a foreign pointer installed by
raw JavaScriptCore APIs, safe rust-jsc accessors treat it as absent or type
mismatched instead of reading arbitrary memory as `PrivateDataHeader`. This
registry is thread-local because JavaScriptCore handles and private data are
context/thread affine in the safe API.

Safe private/shared data APIs follow these rules:

- immutable guards may coexist with other immutable guards
- mutable guards require exclusive access
- take, drop, and replacement operations refuse to invalidate active guards
- wrong-type take/drop attempts leave the original pointer in place
- ordinary shared-data set only writes an empty slot
- explicit replacement APIs name the ownership transfer and drop old Rust data

Objects created from a `JSClass` bind their private-data type to the class
contract. Use `JSClass::set_object_private_data` for safe post-construction
attachment; it verifies the object/class/type relationship and refuses to
replace an occupied slot.

## Classes And Finalization

Class builders own temporary strings and callback tables until
`JSClassCreate`. The class-name pointer passed to JavaScriptCore must never
outlive its backing `CString`.

Class finalizers run from JavaScriptCore. They must not panic across the C ABI,
must drop only Rust-owned private data for the matching class/type contract,
and must leave foreign or wrong-type pointers untouched.

Lifecycle macros validate supported signatures at compile time and catch Rust
panics before returning to JavaScriptCore. Unsupported signatures should fail
the macro expansion rather than becoming runtime panics inside an FFI wrapper.

## Callbacks And Exceptions

Rust callbacks, constructors, module-loader hooks, inspector callbacks, timer
callbacks, and async completion paths must not unwind across JavaScriptCore's C
ABI. Generated wrappers catch panics, convert typed argument failures into
JavaScript errors where appropriate, and return through the C API's exception
path.

Callbacks receive borrowed handles. They must not release the callback context,
store borrowed message slices, or keep borrowed JavaScriptCore handles beyond
the owner without retaining or protecting through a documented safe wrapper.

Use `TryFromJSValue` for JavaScript input and `IntoJSResult` or
`IntoJSValue` for return conversion. Conversion failure is a normal fallible
path, not a panic.

## Modules, Promises, And Host Pumps

Module evaluation returns a JavaScript promise. rust-jsc does not hide
microtask or deferred-work policy inside module evaluation. Embedders choose
when to call `run_deferred_work()` and `run_microtasks()` at host event-loop
checkpoints.

Module-loader callbacks should return typed results or `ModuleLoadError` so
diagnostics preserve the module id, optional referrer, and error message.
Runtime-specific resolution policy belongs in the embedding runtime, not in
raw JavaScriptCore shortcuts.

Deferred promises use RAII-protected resolver functions. A
`PromiseResolver` is context-affine and should live in JS-thread-owned runtime
state unless the embedding runtime owns a safe cross-thread completion queue
that moves only Rust-owned data between threads.

Unhandled-rejection handlers must be callable and from the same context.
`UnhandledRejectionHandler` protects the callback while host state stores the
guard.

## Typed Arrays And ArrayBuffers

Safe constructors either copy Rust slices into JavaScriptCore-owned storage or
move an owned `Vec` into JavaScriptCore with a Rust deallocator.

Borrowed no-copy constructors and borrowed byte-slice accessors are `unsafe`.
The caller must prove that:

- no JavaScriptCore API call runs while a returned Rust slice is live
- caller-owned no-copy buffers stay allocated and pinned while JavaScript can
  reach them
- Rust does not free or mutate storage that JavaScriptCore can still access
- typed views are correctly aligned and sized for the requested element type

Use safe copied reads such as `JSTypedArray::as_vec<T>()` for runtime code
unless profiling proves that a borrowed escape hatch is required and the owner
can state the lifetime contract.

## Inspector Sessions

`InspectorSession` owns a direct frontend connection for one context and
disconnects on drop. Protocol sends are fallible because strings with interior
NUL bytes cannot cross the C API as C strings.

Inspector message callbacks receive borrowed UTF-8 bytes valid only for the
callback duration. Wrap borrowed input in `InspectorInboundMessage` to name the
lifetime, and copy it into `OwnedInspectorMessage` before queueing it or
crossing an async/thread boundary.

Pause-loop callbacks are pump notifications on the JavaScriptCore thread. They
may queue host work or send simple protocol messages, but they must not destroy
the VM, release the context, block indefinitely, or run arbitrary JavaScript.

## Unsafe Escape Hatches

Unsafe APIs are acceptable only when the safe API cannot represent the required
contract without excessive overhead. Each unsafe block needs a preceding
`// SAFETY:` comment naming the invariant. Each unsafe public function needs a
`# Safety` rustdoc section.

New unsafe wrappers should keep raw pointers at the boundary, convert them
immediately into typed owned or borrowed handles, and document ownership,
thread-affinity, exception, and lifetime rules. If an unchecked fast path exists
for performance, the method name or rustdoc must state the preconditions.
