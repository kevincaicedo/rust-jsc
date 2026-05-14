# rust-jsc Modules

`rust-jsc` exposes JavaScriptCore module primitives without putting runtime
policy in WebKit. WebKit owns parsing, linking, evaluation, promise creation,
and the module registry. Rust owns resolver, fetcher, import-meta, file-system,
JSON, synthetic-module, and runtime-specific policy.

## Default File Loader

Use the default file loader when an embedding wants simple ESM files from disk:

```rust
use rust_jsc::{JSContext, ModuleLoader};

let ctx = JSContext::new();
ctx.set_module_loader(ModuleLoader::file_system());

let promise = ctx.evaluate_module("/absolute/path/main.js")?;
assert!(promise.is_object());
ctx.run_microtasks();
# Ok::<(), rust_jsc::JSError>(())
```

The loader accepts absolute paths, `file://` URLs, and `./`/`../` specifiers
relative to the importing file. Bare specifiers such as `@runtime/std` are not
file-loader policy; runtimes such as KedoJS should resolve those in their own
loader.

`import.meta.url` is set by the Rust file loader. WebKit does not add default
file metadata.

## JSON Modules

Bare `.json` imports remain supported as a rust-jsc compatibility policy:

```js
import data from "./config.json";
```

The file loader wraps that file as a JavaScript module with a default export.

When JavaScript uses import attributes, JavaScriptCore requests a JSON module:

```js
import data from "./config.json" with { type: "json" };

const module = await import("./config.json", { with: { type: "json" } });
```

In that path the Rust loader returns raw JSON and JavaScriptCore parses it as a
JSON module. Dynamic imports receive a namespace object, so the JSON value is on
`module.default`.

## WebAssembly Modules

The default file loader supports `.wasm` files and files whose first bytes match
the WebAssembly magic header. It reads those modules as bytes and returns a typed
WebAssembly module source to JavaScriptCore:

```js
import { answer } from "./answer.wasm";

const module = await import("./answer.wasm");
```

WebAssembly module compilation uses JavaScriptCore deferred work in addition to
promise microtasks. Hosts that load Wasm modules must run deferred work from an
event-loop checkpoint before expecting the module promise to settle:

```rust
let promise = ctx.evaluate_module("/absolute/path/main.js")?;
assert!(promise.is_object());
ctx.run_deferred_work();
ctx.run_microtasks();
# Ok::<(), rust_jsc::JSError>(())
```

Do not call `run_deferred_work()` from resolver, fetcher, import-meta,
inspector, class, or native function callbacks.

## Custom Loaders

Use `ModuleLoader::builder()` to install a custom resolver, fetcher, and
`import.meta` provider while keeping optional slots explicit:

```rust
use rust_jsc::{
    module_fetcher, module_import_meta_provider, module_resolver, JSContext,
    JSObject, JSResult, JSValue, ModuleImportType, ModuleLoader, ModuleSource,
};

#[module_resolver]
fn resolve(
    _ctx: JSContext,
    specifier: String,
    referrer: Option<String>,
) -> JSResult<Option<String>> {
    Ok(Some(referrer.unwrap_or(specifier)))
}

#[module_fetcher]
fn fetch(_ctx: JSContext, key: String, import_type: ModuleImportType) -> JSResult<Option<ModuleSource>> {
    let source = match import_type {
        ModuleImportType::Json => ModuleSource::Json(format!("{{\"key\":{key:?}}}")),
        ModuleImportType::WebAssembly => return Ok(None),
        ModuleImportType::Unknown | ModuleImportType::JavaScript => {
            ModuleSource::JavaScript(format!("export const key = {key:?};"))
        }
    };
    Ok(Some(source))
}

#[module_import_meta_provider]
fn import_meta(ctx: JSContext, key: String) -> JSResult<Option<JSObject>> {
    let object = JSObject::new(&ctx);
    object.set_property("url", &JSValue::string(&ctx, key), Default::default())?;
    Ok(Some(object))
}

let ctx = JSContext::new();
ctx.set_module_loader(
    ModuleLoader::builder()
        .resolve(Some(resolve))
        .fetch_source(Some(fetch))
        .import_meta(Some(import_meta))
        .build(),
);
```

The `module_resolver`, `module_fetcher`, and `module_import_meta_provider`
macros are typed adapters. They guard null callback inputs, catch Rust panics
before they cross JavaScriptCore, validate signatures at compile time, and use
return conversion traits so custom result wrappers can opt into the public API.
The older `module_resolve`, `module_fetch`, and `module_import_meta` macros
remain as low-level FFI adapters for code that needs raw `JSValue` arguments.

Custom loader callbacks can return `Result<T, ModuleLoadError>` when failures
need module id and referrer context:

```rust
use rust_jsc::{ModuleLoadError, ModuleSource};

fn disabled_std_module(
    key: String,
    referrer: Option<String>,
) -> Result<ModuleSource, ModuleLoadError> {
    Err(match referrer {
        Some(referrer) => ModuleLoadError::with_referrer(
            key,
            referrer,
            "runtime std module is disabled",
        ),
        None => ModuleLoadError::new(key, "runtime std module is disabled"),
    })
}
```

The typed loader conversion traits map `ModuleLoadError` into a JavaScript
`TypeError` message that includes the module id and optional referrer.
`rust-jsc` still does not decide KedoJS virtual-module policy; it only provides
the diagnostic shape.

For binary or typed custom loaders, prefer the typed fetch-source callback and
return `ModuleSource` or `JSModuleSource`. Only low-level raw fetch-source
callbacks should call `JSModuleSource::into_raw()` themselves. The legacy
`module_fetch` macro returns text only and cannot supply WebAssembly bytes.

### Module Lifecycle

JavaScriptCore drives module loading in stages. The Rust loader callbacks are
policy hooks for those stages:

1. `resolve`: turn an import specifier into the canonical module key.
2. `fetch_source` or `fetch`: provide source for the resolved key.
3. JavaScriptCore parses the source and recursively resolves dependencies.
4. `link_and_evaluate`: instantiate bindings and start async evaluation.
5. Promise jobs and dynamic imports settle when the host pumps microtasks.

`evaluate` is a legacy escape hatch for virtual modules. Prefer
`create_synthetic_module` for synthetic modules and `fetch_source` for text,
JSON, and WebAssembly sources.

### Loader Callback Fields

`ModuleLoaderBuilder` fills the raw `JSAPIModuleLoader` slots without forcing
callers to remember struct field names:

| Builder method | Raw field | When JavaScriptCore calls it | Return meaning |
| --- | --- | --- | --- |
| `resolve(...)` | `moduleLoaderResolve` | For every static import, dynamic import, and entry module key that needs canonicalization. | A canonical key. The same specifier/referrer pair should resolve consistently. |
| `fetch_source(...)` | `moduleLoaderFetchSource` | After resolution, when JSC wants typed source. | A transferred `JSModuleSourceRef` for JavaScript, JSON, or WebAssembly bytes. Preferred for new loaders. |
| `fetch(...)` | `moduleLoaderFetch` | Compatibility text-source path when `fetch_source` is not provided or does not handle the key. | A JavaScript or JSON text string. It cannot return WebAssembly bytes. |
| `import_meta(...)` | `moduleLoaderCreateImportMetaProperties` | When a module first evaluates `import.meta`. | A plain object with host-defined properties such as `url`. |
| `evaluate(...)` | `moduleLoaderEvaluate` | Legacy virtual-module path. | A JavaScript value for that key. New code should usually use synthetic modules instead. |

Callback arguments are low-level because they mirror the C ABI:

| Callback argument | Meaning |
| --- | --- |
| `ctx` | Borrowed JavaScriptCore context. Do not store it unless you retain the global context through the safe API. |
| `key` | Requested specifier for `resolve`; resolved key for fetch/evaluate/import-meta. |
| `referrer` | Importing module key, or `undefined` for entries when JSC has no referrer. |
| `attributes` | Import type requested by JSC: `"javascript"`, `"json"`, `"webassembly"`, or `undefined`. |
| `script_fetcher` | JavaScriptCore internal fetcher value. Treat it as an opaque value; most Rust loaders ignore it. |

The default file loader resolves:

- absolute paths and `file://` URLs to file URLs
- `./` and `../` specifiers relative to the referrer module
- `.json` files as JavaScript default-export wrappers unless import attributes
  requested a real JSON module
- `.wasm` files as typed WebAssembly sources

Bare specifiers are not file policy. A runtime loader should map them to a
synthetic module key, a package graph key, or an application-specific source.

### Entry Functions

The context module APIs differ by how much work they start:

| Rust API | C API | Use when |
| --- | --- | --- |
| `evaluate_module(path_or_key)` | `JSModuleLoadAndEvaluate` | You have an entry key/path and want loading, linking, and evaluation started in one call. |
| `load_module(key)` | `JSModuleLoad` | You want to pre-load and parse a module graph before evaluation. |
| `link_and_evaluate_module(key)` | `JSModuleLinkAndEvaluate` | You previously loaded a module and now want to instantiate and evaluate it. |
| `load_module_from_source(source, source_url, line)` | `JSModuleLoadFromSource` | You have source text and want to load/parse without evaluating yet. `source_url` is the referrer key used to resolve relative imports. |
| `evaluate_module_from_source(source, source_url, line)` | `JSModuleLoadAndEvaluateFromSource` | You have source text and want to start loading, linking, and evaluation immediately. |

All of these return a JavaScript value that is a promise on the async module
path. A returned promise means work has started; it does not mean evaluation has
finished. Run deferred work when Wasm may be pending, then run microtasks from a
host checkpoint.

## Synthetic Modules

Create synthetic modules explicitly before they are imported:

```rust
use rust_jsc::{JSContext, JSValue};

let ctx = JSContext::new();
let name = JSValue::string(&ctx, "rust-jsc");
ctx.create_synthetic_module("@runtime/config", &[("name", &name)])?;
# Ok::<(), rust_jsc::JSError>(())
```

Rules:

- Export names are copied by JavaScriptCore.
- Export values must belong to the same context.
- Duplicate export names are rejected.
- A `"default"` export creates the default export.
- Re-registering the same module key is rejected by JavaScriptCore.

The resolver must return the same resolved key used when creating the synthetic
module.

Use synthetic modules for host-owned values that do not come from source text:
configuration, native bindings, runtime capabilities, test fixtures, and
already-created JS objects. Do not use synthetic modules as a hidden filesystem
cache; a file/module loader should return source for files and reserve synthetic
modules for explicit host objects.

Static and dynamic imports both work after the resolver returns the registered
key:

```js
import config, { answer } from "@runtime/config";

const module = await import("@runtime/config");
console.log(config, answer, module.default);
```

If dynamic imports are involved, remember that the import continuation is a
promise job. Pump microtasks at a host checkpoint before expecting side effects
from `.then(...)`.

## Microtask Pumping

Module APIs return JavaScript promises. They start loading, linking, and
evaluation, but they do not run promise jobs or dynamic-import continuations by
themselves.

Call `JSContext::run_microtasks()` only at a host event-loop checkpoint:

```rust
let promise = ctx.evaluate_module("/absolute/path/main.js")?;
assert!(promise.is_object());
ctx.run_microtasks();
```

For tests that use dynamic import, running the pump more than once is acceptable
because dynamic import schedules additional promise jobs. Runtime event loops
should instead pump after each host turn until their own job queue and JSC's
observable work are quiescent.

WebAssembly module compilation also uses JSC deferred work. Run
`JSContext::run_deferred_work()` at a host event-loop checkpoint before the
microtask drain when Wasm module compilation may be pending.

Avoid calling `run_microtasks()` from resolver, fetcher, import-meta, inspector,
class, or native function callbacks. Those callbacks already run inside
JavaScriptCore execution; recursively draining microtasks there can introduce
reentrancy bugs and surprising VM-lock behavior.

## Validation

Focused module validation:

```bash
RUST_JSC_BUILD_MODE=download \
RUST_JSC_LIB_DIR=/path/to/WebKit/WebKitBuild/RustJSC-Phase3Review/JSCOnly/Release-Static/lib \
cargo test -p rust_jsc --test module_loader -- --test-threads=1
```

The integration test covers file modules, `import.meta.url`, bare JSON imports,
JSON import attributes, dynamic JavaScript imports, dynamic JSON imports, and
static/dynamic WebAssembly imports, and static/dynamic synthetic-module imports.
The `api_regression` integration test additionally uses checked-in fixture
files to cover source modules resolving real file dependencies, explicit
load-then-link evaluation, synthetic duplicate rejection, exact array helpers,
method calls, and typed-array error paths.
