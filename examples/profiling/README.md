# Profiling rust-jsc

This directory contains the DHAT heap-profiling harness used by
`scripts/performance_snapshot.sh`.

For the full performance workflow, benchmark commands, and CI artifact contract,
see [../../docs/performance-validation.md](../../docs/performance-validation.md).

## DHAT Heap Profiling

DHAT tracks every heap allocation (via Rust's global allocator) and produces a
JSON report you can explore in an interactive viewer.

### Run

```bash
RUST_JSC_BUILD_MODE=download \
RUST_JSC_LIB_DIR=/path/to/JSCOnly/Release-Static/lib \
bash scripts/performance_snapshot.sh --output-dir .artifacts/performance/dhat --dhat
```

This produces `dhat-heap.json`, `dhat_output.txt`, and `environment.txt` in the
chosen output directory.

### View

Open [dh_view](https://nnethercote.github.io/dh_view/dh_view.html) in your
browser and load `dhat-heap.json`.

Key metrics to look for:
- **Total bytes allocated** — How much memory the workload allocated in total.
- **Peak heap usage** — Maximum bytes alive at any point.
- **Allocation hotspots** — Which functions/call sites allocate the most.
- **Short-lived allocations** — Objects allocated and freed quickly (GC pressure).

## Flamegraph Generation

Flamegraphs show CPU time spent in each function, making hot paths visually
obvious.

### Install

```bash
cargo install flamegraph
```

### Generate For The Stress Example

```bash
RUST_JSC_BUILD_MODE=download \
RUST_JSC_LIB_DIR=/path/to/JSCOnly/Release-Static/lib \
bash scripts/performance_snapshot.sh --output-dir .artifacts/performance/flamegraph --flamegraph
```

The output is `flamegraph_stress.svg` in the chosen output directory. Open it in
a browser for an interactive zoomable view.

> **macOS note:** You may need to run with `sudo` or use `dtrace` permissions.
> See `cargo flamegraph --help` for platform-specific instructions.

## Memory Leak Detection

### macOS (leaks)

```bash
cargo build --lib
leaks --atExit -- cargo test --lib -- --test-threads=1
```

### Linux (valgrind)

```bash
cargo build --lib
valgrind --tool=memcheck --leak-check=full \
    --suppressions=valgrind.supp \
    cargo test --lib -- --test-threads=1
```

## JSC-Level Memory Tracking

Use `JSContext::get_memory_usage()` at strategic points in your code/tests to
track JSC-internal memory:

```rust
let ctx = JSContext::new();
let usage = ctx.get_memory_usage();
// Returns an object with: heapSize, heapCapacity, extraMemorySize, objectCount, etc.
```

Use the profiling harness or targeted tests to capture snapshots before and
after stress scenarios.

## Interpreting Results

| Metric | Healthy | Concern |
|--------|---------|---------|
| Peak heap (DHAT) | Stable across versions | >20% increase = regression |
| Total allocations | Decreasing or stable | Sudden spike = new alloc pattern |
| JSC heapSize after GC | Returns near baseline | Growing = potential leak in native code |
| Flamegraph hotspots | FFI boundary + JSC internals | Rust-side hotspots in property marshaling |
