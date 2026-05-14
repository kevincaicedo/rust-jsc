# JSClass and Custom Objects

In JavaScriptCore, a `JSClass` acts as a blueprint for creating custom JavaScript objects that are backed by native code. Rust-JSC provides a safe and ergonomic `JSClassBuilder` to define these classes and attach Rust data to the resulting objects.

## Defining a JSClass

To create a custom class with a static Rust string name, use
`JSClass::builder()`. If the class name comes from outside Rust source code,
prefer `JSClass::try_builder()` so interior NUL bytes are reported as
`ClassError` instead of panicking. You must specify the type of the Rust data
that will be attached to instances of this class using the `.build::<T>()`
method.

```rust
use rust_jsc::JSClass;

// Define a class that will hold a `String` as its private data
fn main() -> Result<(), rust_jsc::ClassError> {
    let _class = JSClass::builder("Greeter").build::<String>()?;
    Ok(())
}
```

```rust
use rust_jsc::JSClass;

fn main() -> Result<(), rust_jsc::ClassError> {
    let _class = JSClass::try_builder("Greeter")?.build::<String>()?;
    Ok(())
}
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
use rust_jsc::JSObject;

fn read_private_data(obj: &JSObject) -> Result<(), Box<dyn std::error::Error>> {
    // Safe immutable guarded access
    let Some(greeting) = obj.get_private_data::<String>() else {
        return Err("missing Greeter private data".into());
    };
    assert_eq!(greeting.as_str(), "Hello, World!");

    // Wrong type returns None instead of causing UB
    assert!(obj.get_private_data::<i32>().is_none());
    Ok(())
}
```

### Mutating Private Data

To mutate the data, you have two options:

1. **Exclusive Guard**: Use `get_private_data_mut::<T>()`. It returns `None` while any shared or mutable private-data guard is active.
2. **Interior Mutability**: Wrap your data in a `RefCell` or `Mutex` when the data may be mutated reentrantly from nested callbacks.

```rust
use std::cell::RefCell;
use rust_jsc::{JSClass, JSContext};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = JSContext::new();
    let class = JSClass::builder("Counter").build::<RefCell<i32>>()?;

    let obj = class.object(&ctx, Some(RefCell::new(0)));

    // Safe mutation!
    let Some(counter) = obj.get_private_data::<RefCell<i32>>() else {
        return Err("missing Counter private data".into());
    };
    *counter.borrow_mut() += 1;
    Ok(())
}
```

## Callbacks and Methods

You can attach native Rust functions to your class to act as constructors,
prototype methods, constructor static methods, and static value accessors.
Prototype methods use
JavaScriptCore's static function table, so they are installed on the shared
prototype unless `JSClassAttribute::NoAutomaticPrototype` is set. Static value
accessors use JavaScriptCore's static value table and are the preferred path for
known properties. Constructor static methods are installed as own properties on
the object published by `JSClass::register`.

```rust
use rust_jsc::{callback, constructor, JSClass, JSContext, JSObject, JSResult, JSValue};

#[constructor]
fn my_constructor(
    ctx: JSContext,
    this: JSObject,
    arguments: &[JSValue],
) -> JSResult<JSValue> {
    // Initialize the object
    Ok(this.into())
}

#[callback]
fn greet(
    ctx: JSContext,
    _function: JSObject,
    this: JSObject,
    _arguments: &[JSValue],
) -> JSResult<JSValue> {
    let name = this.get_property("name")?.as_string()?.to_string();
    Ok(JSValue::string(&ctx, format!("hello {name}")))
}

#[callback]
fn version(
    ctx: JSContext,
    _function: JSObject,
    _this: JSObject,
    _arguments: &[JSValue],
) -> JSResult<JSValue> {
    Ok(JSValue::string(&ctx, "1.0"))
}

fn main() -> Result<(), rust_jsc::ClassError> {
    let _class = JSClass::try_builder("MyClass")?
        .method("greet", Some(greet))?
        .constructor_method("version", Some(version))?
        .call_as_constructor(Some(my_constructor))
        .build::<()>()?;
    Ok(())
}
```

When these callbacks are invoked by JavaScript, you can safely extract the `this` object and access its private Rust data to perform native operations.

Static value accessors currently accept JavaScriptCore property callbacks. They
are useful for embedders that already work at the raw callback boundary.
`JSClassBuilder::typed_accessor()` is the typed path for known Rust property
values. It uses a zero-sized marker type that implements `JSClassAccessor`, so
the generated JavaScriptCore callback is monomorphized and does not require a
runtime registry.

```rust
use rust_jsc::{JSError, JSClass, JSClassAccessor, JSContext, JSObject, JSResult};

struct Count;

impl JSClassAccessor for Count {
    type Value = i32;

    fn get(ctx: JSContext, object: JSObject) -> JSResult<Self::Value> {
        let Some(counter) = object.get_private_data::<i32>() else {
            return Err(JSError::new_typ(&ctx, "Counter private data missing")?);
        };
        Ok(*counter)
    }

    fn set(ctx: JSContext, object: JSObject, value: Self::Value) -> JSResult<()> {
        let Some(mut counter) = object.get_private_data_mut::<i32>() else {
            return Err(JSError::new_typ(&ctx, "Counter private data missing")?);
        };
        *counter = value;
        Ok(())
    }
}

fn main() -> Result<(), rust_jsc::ClassError> {
    let _class = JSClass::try_builder("Counter")?
        .typed_accessor::<Count>("count")?
        .build::<i32>()?;
    Ok(())
}
```

If you need to publish a class object somewhere other than the global object,
use `JSClass::constructor_object()` and attach the returned object to your
module or namespace. It installs the same constructor static methods that
`JSClass::register()` installs.

## Explicit Prototypes

Classes that use `JSClassAttribute::NoAutomaticPrototype` can still create
objects with an embedder-owned prototype through
`JSClass::object_with_prototype()`. The method validates that the prototype
belongs to the same JavaScript context before assigning it.

```rust
use rust_jsc::{JSClass, JSClassAttribute, JSContext, JSObject, JSValue};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = JSContext::new();
    let class = JSClass::builder("ManualPrototype")
        .set_attributes(JSClassAttribute::NoAutomaticPrototype.into())
        .build::<()>()?;

    let prototype = JSObject::new(&ctx);
    prototype.set_property(
        "kind",
        &JSValue::string(&ctx, "manual"),
        Default::default(),
    )?;

    let _object = class.object_with_prototype::<()>(&ctx, None, &prototype)?;
    Ok(())
}
```

## Garbage Collection and Finalization

When the JavaScript garbage collector destroys an object, Rust-JSC automatically intercepts the finalization event. It safely drops the attached `TypedData<T>`, ensuring that your Rust destructors are run and no memory is leaked.

You do not need to manually implement a `finalize` callback just to free your Rust data; Rust-JSC handles this automatically based on the `T` you provided to `.build::<T>()`.

`JSClassBuilder::build` owns exactly the reference returned by
`JSClassCreate`. `JSClass` releases that reference in `Drop`; it does not add an
extra retain count during construction.
