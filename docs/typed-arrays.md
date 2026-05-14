# Typed Arrays and ArrayBuffers

This guide covers the safe typed-array and ArrayBuffer path for embedding
users. The default API copies or transfers Rust-owned storage into
JavaScriptCore-managed backing stores. Borrowed no-copy views exist, but they
are explicit `unsafe` escape hatches because JavaScriptCore byte pointers are
temporary and external buffers can outlive Rust slices.

## Reading JavaScript Typed Arrays

Use `JSTypedArray::from_value` to check that a JavaScript value is a typed
array, then copy the contents into a Rust `Vec`.

```rust
use rust_jsc::{JSContext, JSResult, JSTypedArray, JSTypedArrayType};

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let value = ctx.evaluate_script("new Uint8Array([5, 4, 4, 5])", None)?;
    let array = JSTypedArray::from_value(&value)?;

    assert_eq!(array.array_type()?, JSTypedArrayType::Uint8Array);
    assert_eq!(array.len()?, 4);
    assert_eq!(array.byte_len()?, 4);
    assert_eq!(array.as_vec::<u8>()?, vec![5, 4, 4, 5]);
    Ok(())
}
```

`as_vec<T>` is the safe read path. `T` must be one of rust-jsc's sealed
`JSTypedArrayElement` primitives and must match the JavaScript typed-array
kind. For example, `Uint16Array` reads as `u16`, while `Uint8Array` and
`Uint8ClampedArray` read as `u8`.

## Creating Typed Arrays From Rust

Use `JSTypedArray::with_bytes` when the Rust caller should keep its source
buffer. This copies the slice into JavaScriptCore-owned storage, so later Rust
mutations to the original slice do not affect the JavaScript array.

```rust
use rust_jsc::{JSContext, JSResult, JSTypedArray, JSTypedArrayType};

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let mut source = vec![1u16, 2, 3, 4];
    let array = JSTypedArray::with_bytes(
        &ctx,
        source.as_slice(),
        JSTypedArrayType::Uint16Array,
    )?;

    source[0] = 99;
    assert_eq!(array.as_vec::<u16>()?, vec![1, 2, 3, 4]);
    Ok(())
}
```

Use `JSTypedArray::with_owned_bytes` when Rust can transfer ownership of a
`Vec<T>` to JavaScriptCore without copying the vector buffer.

```rust
use rust_jsc::{JSContext, JSResult, JSTypedArray, JSTypedArrayType};

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let array = JSTypedArray::with_owned_bytes(
        &ctx,
        vec![10u32, 20, 30],
        JSTypedArrayType::Uint32Array,
    )?;

    assert_eq!(array.len()?, 3);
    assert_eq!(array.as_vec::<u32>()?, vec![10, 20, 30]);
    Ok(())
}
```

rust-jsc stores the vector in a small Rust-owned external-buffer record and
passes a deallocator to JavaScriptCore. JavaScriptCore calls that deallocator
when the backing store is destroyed.

## ArrayBuffers

Use `JSArrayBuffer::from_bytes` for copied byte buffers and
`JSArrayBuffer::from_vec` for owned no-copy transfer.

```rust
use rust_jsc::{JSArrayBuffer, JSContext, JSResult};

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let copied = JSArrayBuffer::from_bytes(&ctx, &[1, 2, 3, 4])?;
    assert_eq!(copied.as_vec()?, vec![1, 2, 3, 4]);

    let transferred = JSArrayBuffer::from_vec(&ctx, vec![9, 8, 7, 6])?;
    assert_eq!(transferred.len()?, 4);
    Ok(())
}
```

`JSArrayBuffer::new` remains available as the historical copied constructor.
New code should prefer `from_bytes` or `from_vec` because the ownership choice
is visible in the method name.

## Typed Views Over ArrayBuffers

Create typed views over existing JavaScript ArrayBuffers with
`JSTypedArray::with_buffer` or `JSTypedArray::with_buffer_and_offset`.

```rust
use rust_jsc::{JSArrayBuffer, JSContext, JSResult, JSTypedArray, JSTypedArrayType};

fn main() -> JSResult<()> {
    let ctx = JSContext::new();
    let buffer = JSArrayBuffer::from_vec(&ctx, vec![1, 2, 3, 4])?;
    let array = JSTypedArray::with_buffer(&ctx, buffer, JSTypedArrayType::Uint8Array)?;

    assert_eq!(array.as_vec::<u8>()?, vec![1, 2, 3, 4]);
    Ok(())
}
```

The offset constructor validates that the byte offset is inside the buffer and
that the remaining byte length is aligned for the requested JavaScript element
type.

## Unsafe Borrowed Views

`JSTypedArray::bytes`, `JSTypedArray::bytes_from_value`,
`JSTypedArray::with_bytes_no_copy`, `JSArrayBuffer::bytes`, and
`JSArrayBuffer::with_bytes_no_copy` are unsafe by design.

Use them only when the caller can prove the JavaScriptCore pointer and backing
store contracts:

- no JavaScriptCore API call runs while a returned slice is live
- no other Rust alias reaches the same backing store while the slice is live
- caller-owned no-copy buffers stay allocated and pinned until every reachable
  JavaScript view is gone
- Rust does not free or mutate caller-owned storage while JavaScriptCore can
  access it

The safe copied and owned-transfer APIs should be the default for runtime code.
