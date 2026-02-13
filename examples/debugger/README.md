# Rust JSC Inspector/Debugger Examples

This directory contains two complete working examples demonstrating how to use the JavaScriptCore Inspector protocol from Rust for debugging JavaScript code.

## Examples

### 1. Simple Debugger (`simple_debugger`)

Demonstrates the minimal setup for pause/resume debugging with `debugger;` statements.

**Features:**
- Initialize JSContext with inspector support
- Set up type-safe inspector handlers (pause, tick, resume callbacks)
- Pause execution via `debugger;` statement
- Automatically resume from the host on pause

**Usage:**
```bash
cargo run --manifest-path examples/debugger/Cargo.toml --bin simple_debugger [path/to/module.js]
```

**Default script:** `./examples/debugger/scripts/simple_debugger.js`

**Example:**
```bash
# Use default script
cargo run --manifest-path examples/debugger/Cargo.toml --bin simple_debugger

# Or specify a custom module
cargo run --manifest-path examples/debugger/Cargo.toml --bin simple_debugger -- ./my_script.js
```

### 2. Breakpoint Debugger (`breakpoint_debugger`)

Demonstrates a complete breakpoint workflow with evaluation and stepping.

**Features:**
- Enable Debugger and Runtime domains
- Activate breakpoints via `Debugger.setBreakpointsActive`
- Load JavaScript modules from files
- Set breakpoints by URL and line number
- Evaluate expressions on paused call frames (`Debugger.evaluateOnCallFrame`)
- Step through code (`Debugger.stepNext`)
- Resume execution programmatically

**Usage:**
```bash
cargo run --manifest-path examples/debugger/Cargo.toml --bin breakpoint_debugger [path/to/module.js] [line_number_0_based]
```

**Defaults:**
- Module: `./examples/debugger/scripts/breakpoint_debugger.js`
- Line: `28` (0-based, where `const result = {` is assigned in `compute()`)

**Example:**
```bash
# Use defaults (breakpoint at line 28 in breakpoint_debugger.js)
cargo run --manifest-path examples/debugger/Cargo.toml --bin breakpoint_debugger

# Custom module and line
cargo run --manifest-path examples/debugger/Cargo.toml --bin breakpoint_debugger -- ./my_module.js 15
```

**What it demonstrates:**
1. Loads `breakpoint_debugger.js` which exports a `compute()` function
2. Sets a breakpoint at the specified line (default: line 28, inside `compute()`)
3. Loads `breakpoint_runner.js` which imports and calls `compute(42)`
4. When the breakpoint is hit:
   - Evaluates variables in the paused call frame: `x`, `y`, `sum`, `obj`, `obj.nested`, `result`
   - Performs 3 `stepNext` operations to step through the code
   - Resumes execution

**Expected output highlights:**
```
✓ [Main Thread] Observed Debugger.paused
[Host Callback] on_pause: paused!
[Host Callback] evaluateOnCallFrame(x)...
{"result":{"result":{"type":"number","value":43},...}}
[Host Callback] evaluateOnCallFrame(obj)...
{"result":{"result":{"type":"object","value":{"input":42,"nested":{"a":1,"b":2},...}}}}
[Host Callback] on_tick: stepNext (#3001)
[Host Callback] on_resume: resumed
```

## JavaScript Test Scripts

### `scripts/simple_debugger.js`
A minimal module with a `debugger;` statement for testing pause/resume.

### `scripts/breakpoint_debugger.js`
A module exporting a `compute()` function with local variables, nested objects, and predictable stepping points.

### `scripts/breakpoint_runner.js`
A runner module that imports `breakpoint_debugger.js` and calls `compute()` to trigger the breakpoint in a real call frame context.

## Key Concepts

### Inspector Protocol Messages
Both examples demonstrate bidirectional communication with JSC's Inspector:
- **Host → JSC**: Send commands like `Debugger.enable`, `Debugger.setBreakpointByUrl`, `Debugger.resume`
- **JSC → Host**: Receive events like `Debugger.scriptParsed`, `Debugger.paused`

### Pause Callbacks
When the debugger pauses (via breakpoint or `debugger;` statement), JSC enters a nested event loop and invokes your Rust callbacks:
- `on_pause`: Called once when entering paused state
- `on_tick`: Called repeatedly while paused (for processing commands)
- `on_resume`: Called when execution resumes

### Thread Safety
`JSContext` is **not Send/Sync** and must live on a single thread. Both examples:
1. Spawn a dedicated JS thread
2. Create the `JSContext` on that thread
3. Use thread-safe synchronization primitives (`Arc`, `Mutex`, `Condvar`, `mpsc`) to coordinate with the main thread

### Module Loading
Both examples use `JSContext::evaluate_module()` with file paths:
- Module specifiers must be absolute or start with `./` or `../`
- Example: `"./examples/debugger/scripts/breakpoint_debugger.js"`

## Building and Running

From the repository root:

```bash
# Build both examples
cargo build --manifest-path examples/debugger/Cargo.toml

# Run simple debugger
cargo run --manifest-path examples/debugger/Cargo.toml --bin simple_debugger

# Run breakpoint debugger
cargo run --manifest-path examples/debugger/Cargo.toml --bin breakpoint_debugger
```

## Troubleshooting

### "Debugger did not pause"
- Ensure `Debugger.enable` is called before evaluation
- For `debugger;` statements: ensure `Debugger.setPauseOnDebuggerStatements` is enabled
- For breakpoints: ensure `Debugger.setBreakpointsActive` is set to `true`
- Check that the breakpoint line number is 0-based and matches actual executable code

### "Can't find variable: x" when evaluating
- Ensure you're evaluating in the correct call frame (use `callFrameId` from `Debugger.paused`)
- If using breakpoints, ensure the breakpoint is hit inside the function where the variable is in scope
- Don't set breakpoints in synthetic scripts (like inline `evaluate_script()` calls); use actual module files

### Module loading errors
- Module paths must be relative (`./...`) or absolute
- For `evaluate_module()`, the path must exist on the filesystem
- The runner module pattern (importing and calling code at top-level) ensures breakpoints are hit in real call frames

## Further Reading

- [Chrome DevTools Protocol - Debugger Domain](https://chromedevtools.github.io/devtools-protocol/tot/Debugger/)
- [WebKit Inspector Protocol](https://github.com/WebKit/WebKit/tree/main/Source/JavaScriptCore/inspector/protocol)
- [rust-jsc Documentation](../../README.md)