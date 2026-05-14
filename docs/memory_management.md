# Memory Management and API Safety

Rust-JSC provides a safe and ergonomic wrapper around the JavaScriptCore (JSC) C API. One of the biggest challenges when bridging Rust and C is managing memory and ensuring type safety, especially when attaching Rust data to JavaScript objects.

This document explains the safety guarantees provided by Rust-JSC, how memory is managed, and how to safely attach Rust data to JavaScript objects and contexts.

## The Problem: Type Confusion and Use-After-Free

In the JSC C API, you can attach arbitrary data to a `JSObject` or a `JSContext` using a `void*` pointer (often called "private data" or "shared data"). 

```c
// C API example
JSObjectSetPrivate(object, my_rust_pointer);
void* ptr = JSObjectGetPrivate(object);
```

In Rust, this is highly unsafe. If you store a `Box<String>` but later try to read it as a `Box<i32>`, you will cause Undefined Behavior (UB) due to type confusion. Furthermore, if the JavaScript garbage collector frees the object, or if you manually take ownership of the data and free it, any remaining Rust references to that data will become dangling pointers, leading to Use-After-Free (UAF) vulnerabilities.

## The Solution: `PrivateDataWrapper` and `TypedData<T>`

To solve these issues, Rust-JSC introduces a type-safe wrapper for all private and shared data: `PrivateDataWrapper`.

Under the hood, when you attach Rust data to a JSC object or context, Rust-JSC does not just store the raw pointer. Instead, it wraps your data in a `TypedData<T>` struct:

```rust
#[repr(C)]
struct TypedData<T> {
    header: PrivateDataHeader,
    data: T,
}
```

This struct is allocated on the heap, and the raw pointer to this allocation is given to JSC. 

### 1. Type Safety via `TypeId`

Because `TypedData<T>` is `#[repr(C)]`, the `type_id` is always at the very beginning of the allocation. When you attempt to retrieve the data, Rust-JSC first reads the `TypeId` and compares it to the type you are requesting.

- If the types match, you get a safe reference to your data.
- If the types do not match, Rust-JSC returns `None`.

This completely eliminates type confusion. You can safely attempt to downcast the private data without risking UB.

### 2. Memory Safety and Ownership

Rust-JSC provides several methods to interact with attached data:

- **`get_shared_data<T>` / `get_private_data<T>`**: Returns a `PrivateDataRef<T>` immutable guard. Shared guards can coexist with other shared guards.
- **`get_shared_data_mut<T>` / `get_private_data_mut<T>`**: Returns a `PrivateDataMut<T>` exclusive guard. This returns `None` while any shared or mutable guard is active.
- **`take_shared_data<T>` / `take_private_data<T>`**: Takes ownership of the data, removing it from the JSC object/context and returning it. If a guard is active, the operation returns `PrivateDataTakeResult::Borrowed` and leaves the data in place.
- **`drop_shared_data<T>` / `drop_private_data<T>`**: Drops the data in place when the type matches and no guard is active. Active guards return `PrivateDataDropStatus::Borrowed`.

### Recommended Pattern: Interior Mutability

For simple local mutation, `get_private_data_mut` and `get_shared_data_mut`
provide safe exclusive guards. For nested callback patterns, the recommended
shape is still **interior mutability** with types like `RefCell`, because the
runtime borrow may otherwise intentionally block reentrant access.

```rust
use rust_jsc::{JSContext, JSClass};
use std::cell::RefCell;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = JSContext::new();
    let class = JSClass::builder("MyClass").build::<RefCell<i32>>()?;
    let obj = class.object(&ctx, Some(RefCell::new(0)));

    // Safe mutation without `unsafe` blocks!
    let Some(cell) = obj.get_private_data::<RefCell<i32>>() else {
        return Err("missing MyClass private data".into());
    };
    *cell.borrow_mut() += 1;

    assert_eq!(*cell.borrow(), 1);
    Ok(())
}
```

By using `RefCell`, you rely on safe runtime borrow checking instead of `unsafe` blocks, completely avoiding the risk of UB.

## JSContext Shared Data

A `JSContext` can hold a single piece of shared data. This is useful for storing application state that needs to be accessible from anywhere within the JavaScript execution context.

```rust
use rust_jsc::JSContext;

struct AppState {
    counter: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = JSContext::new();
    ctx.set_shared_data(AppState { counter: 0 });

    let Some(state) = ctx.get_shared_data::<AppState>() else {
        return Err("missing AppState shared data".into());
    };
    assert_eq!(state.counter, 0);
    Ok(())
}
```

Owned global contexts drop Rust-owned shared data when the context is released.
You can still take or drop shared data earlier when shutdown order matters:

```rust
// Clean up the shared data before dropping the context.
assert!(ctx.drop_shared_data::<AppState>().is_dropped());
```

`set_shared_data<T>()` only stores into an empty slot. Use the explicit
replacement APIs when you intend to drop or replace existing Rust-owned data.
Wrong-type and actively borrowed take/drop attempts leave the original pointer
in place.

## Protected JavaScript Values

Use `JSValue::protected()` when Rust needs to store a JavaScript value somewhere
the JavaScriptCore garbage collector cannot discover, such as a Rust struct,
future completion, or host callback table. Use `ProtectedObject::new(object)` or
`JSObject::into_protected()` for object-typed handles that should stay callable
or otherwise usable while stored by Rust.

```rust
use rust_jsc::{JSContext, JSResult, JSValue};

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let value = JSValue::string(&ctx, "keep me");
    let protected = value.protected();

    assert_eq!(protected.value().as_string()?.to_string(), "keep me");

    let callback = ctx
        .evaluate_script("(function(value) { return value + 1; })", None)?
        .as_object()?
        .into_protected();
    let result = callback.call(None, &[JSValue::number(&ctx, 41.0)])?;
    assert_eq!(result.as_number()?, 42.0);
    Ok(())
}
```

`ProtectedValue` is an RAII guard. It calls `JSValueProtect` when created and
`JSValueUnprotect` exactly once when dropped. Cloning a `ProtectedValue` creates
another protection count, and each clone releases its own count on drop.
`ProtectedObject` follows the same protection-count rules while keeping the
object type available for calls and resource access.
`PromiseResolver` uses `ProtectedValue` internally for its resolve and reject
functions, so resolver clones and drops follow the same protection-count rules.
Manual `JSValue::protect()` and `JSValue::unprotect()` remain available for
low-level compatibility, but the guard is the preferred safe path.

## Typed Array and ArrayBuffer Storage

Safe typed-array and ArrayBuffer constructors either copy Rust slices or move an
owned `Vec` into JavaScriptCore with a Rust deallocator. This avoids returning a
JavaScript object backed by a Rust stack frame or caller-owned slice that can be
freed too early.

Use `JSTypedArray::with_bytes` and `JSArrayBuffer::from_bytes` when Rust keeps
the source buffer. Use `JSTypedArray::with_owned_bytes` and
`JSArrayBuffer::from_vec` when ownership can move to JavaScriptCore without
copying the vector buffer.

Borrowed no-copy constructors and borrowed byte-slice accessors are unsafe.
JavaScriptCore byte pointers are temporary and caller-owned no-copy buffers must
remain allocated, pinned, and exclusively available to JavaScriptCore until all
reachable JavaScript views are gone.

## JSObject Private Data

Objects created from a custom `JSClass` can hold private data. The type of this
data is strictly bound to the `JSClass` definition. See the
[JSClass Documentation](jsclass.md) for more details.

The safe path for attaching private data after object creation is
`JSClass::set_object_private_data`:

```rust
use rust_jsc::{JSClass, JSContext, PrivateDataSetStatus};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = JSContext::new();
    let class = JSClass::builder("State").build::<i32>()?;
    let object = class.object::<i32>(&ctx, None);

    assert_eq!(
        class.set_object_private_data(&object, 42)?,
        PrivateDataSetStatus::Set
    );
    let Some(data) = object.get_private_data::<i32>() else {
        return Err("missing State private data".into());
    };
    assert_eq!(*data, 42);
    Ok(())
}
```

This method checks that the Rust type matches the class type, verifies that the
object is an instance of that class, and refuses to replace an occupied slot.
Use the lower-level unsafe object replacement API only when you also own the
class/finalizer contract. It still refuses replacement while a safe
private-data guard is active.
