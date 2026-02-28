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
    type_id: TypeId,
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

- **`get_shared_data<T>` / `get_private_data<T>`**: Returns an immutable reference `&T`. This is safe because the data is owned by the JSC object/context and will remain alive as long as the object/context is alive.
- **`get_shared_data_mut<T>` / `get_private_data_mut<T>`**: Returns a mutable reference `&mut T`. **This is `unsafe`**. You must guarantee that no other references to this data exist at the same time, otherwise you violate Rust's aliasing rules.
- **`take_shared_data<T>` / `take_private_data<T>`**: Takes ownership of the data, removing it from the JSC object/context and returning it to you. **This is `unsafe`** because if you have previously obtained references to this data, taking ownership will free the backing memory, turning those references into dangling pointers.

### Recommended Pattern: Interior Mutability

Because `get_private_data_mut` is `unsafe` and requires strict aliasing guarantees, the recommended way to mutate attached Rust data is to use **interior mutability** with types like `RefCell` or `Mutex`.

```rust
use std::cell::RefCell;
use rust_jsc::{JSContext, JSClass};

let ctx = JSContext::new();
let class = JSClass::builder("MyClass").build::<RefCell<i32>>().unwrap();
let obj = class.object(&ctx, Some(RefCell::new(0)));

// Safe mutation without `unsafe` blocks!
let cell = obj.get_private_data::<RefCell<i32>>().unwrap();
*cell.borrow_mut() += 1;

assert_eq!(*cell.borrow(), 1);
```

By using `RefCell`, you rely on safe runtime borrow checking instead of `unsafe` blocks, completely avoiding the risk of UB.

## JSContext Shared Data

A `JSContext` can hold a single piece of shared data. This is useful for storing application state that needs to be accessible from anywhere within the JavaScript execution context.

```rust
struct AppState {
    counter: u32,
}

let ctx = JSContext::new();
ctx.set_shared_data(AppState { counter: 0 });

let state = ctx.get_shared_data::<AppState>().unwrap();
assert_eq!(state.counter, 0);
```

**Important Note on Memory Leaks**: Unlike `JSObject`, a `JSContext` does not have a finalizer callback when it is destroyed. This means that any shared data you attach to a `JSContext` will **leak memory** unless you manually free it before the context is destroyed.

To prevent memory leaks, you must call `take_shared_data<T>()` or `drop_shared_data<T>()` when you are done with the context:

```rust
// Clean up the shared data before dropping the context
unsafe { ctx.drop_shared_data::<AppState>() };
```

## JSObject Private Data

Objects created from a custom `JSClass` can hold private data. The type of this data is strictly bound to the `JSClass` definition. See the [JSClass Documentation](jsclass.md) for more details.
