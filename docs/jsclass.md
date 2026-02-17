# JSClass and Custom Objects

In JavaScriptCore, a `JSClass` acts as a blueprint for creating custom JavaScript objects that are backed by native code. Rust-JSC provides a safe and ergonomic `JSClassBuilder` to define these classes and attach Rust data to the resulting objects.

## Defining a JSClass

To create a custom class, use `JSClass::builder()`. You must specify the type of the Rust data that will be attached to instances of this class using the `.build::<T>()` method.

```rust
use rust_jsc::{JSClass, JSContext};

// Define a class that will hold a `String` as its private data
let class = JSClass::builder("Greeter")
    .build::<String>()
    .expect("Failed to build class");
```

### The `type_id` Binding

When you call `.build::<T>()`, Rust-JSC internally records the `TypeId` of `T` inside the `JSClass`. This ensures that any object created from this class is strictly bound to the type `T`. 

This is a crucial safety feature: it prevents you from accidentally attaching an `i32` to an object that was created from a class expecting a `String`.

## Creating Objects

Once you have a `JSClass`, you can instantiate objects from it using the `.object()` method. You can optionally provide the initial Rust data to attach to the object.

```rust
let ctx = JSContext::new();

// Create an object and attach a String to it
let obj = class.object::<String>(&ctx, Some(String::from("Hello, World!")));
```

If you try to pass data of the wrong type, the compiler will reject it.

## Accessing Private Data

You can retrieve the attached Rust data using the `get_private_data` methods on `JSObject`. Thanks to the `PrivateDataWrapper` architecture, this is completely type-safe.

```rust
// Safe immutable access
let greeting = obj.get_private_data::<String>().unwrap();
assert_eq!(greeting, "Hello, World!");

// Wrong type returns None instead of causing UB
assert!(obj.get_private_data::<i32>().is_none());
```

### Mutating Private Data

To mutate the data, you have two options:

1. **Unsafe Mutable Reference**: Use `get_private_data_mut::<T>()`. You must manually guarantee that no other references to this data exist.
2. **Safe Interior Mutability (Recommended)**: Wrap your data in a `RefCell` or `Mutex` when defining the class.

```rust
use std::cell::RefCell;

let class = JSClass::builder("Counter")
    .build::<RefCell<i32>>()
    .unwrap();

let obj = class.object(&ctx, Some(RefCell::new(0)));

// Safe mutation!
*obj.get_private_data::<RefCell<i32>>().unwrap().borrow_mut() += 1;
```

## Callbacks and Methods

You can attach native Rust functions to your class to act as methods, constructors, or property getters/setters.

```rust
use rust_jsc::{JSContext, JSObject, JSValue, JSResult};
use rust_jsc_macros::constructor;

#[constructor]
fn my_constructor(
    ctx: JSContext,
    this: JSObject,
    arguments: &[JSValue],
) -> JSResult<JSValue> {
    // Initialize the object
    Ok(this.into())
}

let class = JSClass::builder("MyClass")
    .call_as_constructor(Some(my_constructor))
    .build::<()>()
    .unwrap();
```

When these callbacks are invoked by JavaScript, you can safely extract the `this` object and access its private Rust data to perform native operations.

## Garbage Collection and Finalization

When the JavaScript garbage collector destroys an object, Rust-JSC automatically intercepts the finalization event. It safely drops the attached `TypedData<T>`, ensuring that your Rust destructors are run and no memory is leaked.

You do not need to manually implement a `finalize` callback just to free your Rust data; Rust-JSC handles this automatically based on the `T` you provided to `.build::<T>()`.
