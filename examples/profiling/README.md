# Profiling rust-jsc

This directory contains tools for examples/profiling the rust-jsc library's memory
allocation patterns and CPU hot paths.

## DHAT Heap Profiling

DHAT tracks every heap allocation (via Rust's global allocator) and produces a
JSON report you can explore in an interactive viewer.

### Run

```bash
cargo run --manifest-path examples/profiling/Cargo.toml --release
```

This produces `dhat-heap.json` in the current directory.

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

### Generate for Benchmarks

```bash
# Profile a specific benchmark
cargo flamegraph --bench context_bench -- --bench "evaluate_complex_script"

# Profile all context benchmarks
cargo flamegraph --bench context_bench -- --bench
```

### Generate for Functional Tests

```bash
cargo flamegraph --manifest-path functional_tests/Cargo.toml --bin functional_tests
```

### Generate for Profiling Harness

```bash
cargo flamegraph --manifest-path examples/profiling/Cargo.toml --bin profile_heap
```

The output is `flamegraph.svg` in the current directory. Open it in a browser
for an interactive zoomable view.

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

The functional test suite (`functional_tests/`) uses this to report memory
snapshots before and after stress tests.

## Interpreting Results

| Metric | Healthy | Concern |
|--------|---------|---------|
| Peak heap (DHAT) | Stable across versions | >20% increase = regression |
| Total allocations | Decreasing or stable | Sudden spike = new alloc pattern |
| JSC heapSize after GC | Returns near baseline | Growing = potential leak in native code |
| Flamegraph hotspots | FFI boundary + JSC internals | Rust-side hotspots in property marshaling |
