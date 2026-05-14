# Performance Validation

`rust-jsc` performance work is evidence-driven. Do not claim a speedup,
allocation reduction, or build-time improvement unless the workload, environment,
before/after numbers, and artifact paths are recorded.

This document defines the 1.0 performance guardrail. It is not a list of
accepted performance wins.

## Workloads

The Criterion suite lives under `benches/benchmarks/` and is wired through
`benches/rust_jsc_bench.rs`. It covers:

| Area | Workloads |
| --- | --- |
| Context and eval | context creation, simple and complex script evaluation |
| Modules | source modules, synthetic modules, custom loader resolution/fetch/import-meta |
| Callbacks and functions | ordinary calls, typed callback macros, legacy raw-slice callbacks, manual raw callbacks |
| Objects and classes | property access, class construction, prototype/static methods |
| Arrays and typed arrays | array construction/push/read, typed-array creation and conversion |
| Strings and values | string conversion and primitive value conversion paths |
| Promises and jobs | deferred promise creation/resolution, unhandled-rejection handler setup |
| Inspector | direct session connect/send/message validation |
| Embedding | minimal host callback and promise checkpoint loop |
| GC | forced collection and object churn |

## Snapshot Script

Use `scripts/performance_snapshot.sh` from the repository root. It writes
`environment.txt` with the Rust toolchain, target, JSC build inputs, OS, and CPU
information before running the selected workload.

Smoke run:

```bash
RUST_JSC_BUILD_MODE=download \
RUST_JSC_LIB_DIR=/path/to/JSCOnly/Release-Static/lib \
bash scripts/performance_snapshot.sh --output-dir .artifacts/performance/smoke --smoke
```

Full Criterion run:

```bash
RUST_JSC_BUILD_MODE=download \
RUST_JSC_LIB_DIR=/path/to/JSCOnly/Release-Static/lib \
bash scripts/performance_snapshot.sh --output-dir .artifacts/performance/full --full
```

Heap profile:

```bash
RUST_JSC_BUILD_MODE=download \
RUST_JSC_LIB_DIR=/path/to/JSCOnly/Release-Static/lib \
bash scripts/performance_snapshot.sh --output-dir .artifacts/performance/dhat --dhat
```

CPU flamegraph:

```bash
cargo install flamegraph
RUST_JSC_BUILD_MODE=download \
RUST_JSC_LIB_DIR=/path/to/JSCOnly/Release-Static/lib \
bash scripts/performance_snapshot.sh --output-dir .artifacts/performance/flamegraph --flamegraph
```

## CI Guardrail

`.github/workflows/benchmarks.yml` uses the same snapshot script:

- Pull requests that touch Rust binding or benchmark paths run `--smoke`.
- Scheduled, manual, and `/run-benchmarks` runs execute `--full`.
- Manual DHAT and flamegraph jobs use `--dhat` and `--flamegraph`.
- Main-branch full runs store trend data, but alerts comment instead of failing
  until normal variance is understood.

The CI benchmark gate proves buildability and catches extreme regressions. It is
not a release-note evidence source by itself.

## Evidence Rules

For a performance change, record:

- the measurable question;
- the exact command and selected benchmark name;
- the JSC build mode, archive or local library path, target triple, OS, CPU, and
  Rust toolchain;
- same-workload baseline and after-change numbers;
- Criterion, DHAT, flamegraph, or perf artifact paths;
- whether the result is accepted, rejected, revised, or evidence-pending.

Single local runs are exploratory. Use them to decide whether to keep
investigating, not to claim broad speedups.

## Deferred Budgets

These are intentionally not accepted as 1.0 blockers:

| Topic | 1.0 decision |
| --- | --- |
| Build-mode timing | Direct CMake/Ninja is the supported source path. ccache/sccache and incremental build-time claims require clean and incremental measurements before adoption. |
| Allocation and instruction budgets | The benchmark suite now provides stable workloads, and DHAT/flamegraph hooks exist. Numeric allocation or instruction budgets need variance data from repeated runs before they can block PRs. |
| Additional macro wrapper rewrites | Do not replace generated wrapper allocations with borrowed/raw views unless profiler evidence identifies a specific hot wrapper and tests prove the lifetime contract. |
| Source-by-source memory accounting | DHAT captures Rust allocator traffic; JavaScriptCore heap, Rust heap, protected values, private data, modules, inspector state, and external buffers still need a separate report before memory claims. |

When a future task needs one of these budgets, add the budget and its measured
baseline in this document before making the optimization.
