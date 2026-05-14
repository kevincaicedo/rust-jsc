#!/usr/bin/env bash
set -euo pipefail

output_dir=".artifacts/performance/$(date -u +%Y%m%dT%H%M%SZ)"
run_smoke=false
run_full=false
run_dhat=false
run_flamegraph=false

usage() {
  cat <<'USAGE'
Usage: scripts/performance_snapshot.sh [OPTIONS]

Options:
  --output-dir DIR   Directory for logs and profiler artifacts.
  --smoke            Run bounded benchmark smoke samples.
  --full             Run the full Criterion benchmark suite.
  --dhat             Run the DHAT heap profiling example.
  --flamegraph       Run a flamegraph capture for the stress example.
  -h, --help         Show this help.

If no run mode is selected, --smoke is used.
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --output-dir)
      if [ "$#" -lt 2 ]; then
        echo "missing value for --output-dir" >&2
        exit 2
      fi
      output_dir="$2"
      shift 2
      ;;
    --smoke)
      run_smoke=true
      shift
      ;;
    --full)
      run_full=true
      shift
      ;;
    --dhat)
      run_dhat=true
      shift
      ;;
    --flamegraph)
      run_flamegraph=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if ! $run_smoke && ! $run_full && ! $run_dhat && ! $run_flamegraph; then
  run_smoke=true
fi

mkdir -p "$output_dir"

{
  echo "timestamp_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "pwd=$PWD"
  echo "rustc=$(rustc --version)"
  echo "cargo=$(cargo --version)"
  echo "target=${CARGO_BUILD_TARGET:-default}"
  echo "rust_jsc_build_mode=${RUST_JSC_BUILD_MODE:-unset}"
  echo "rust_jsc_lib_dir=${RUST_JSC_LIB_DIR:-unset}"
  echo "rust_jsc_archive=${RUST_JSC_ARCHIVE:-unset}"
  uname -a
  if command -v lscpu >/dev/null 2>&1; then
    lscpu
  elif command -v sysctl >/dev/null 2>&1; then
    sysctl -n machdep.cpu.brand_string hw.ncpu hw.memsize 2>/dev/null || true
  fi
} > "$output_dir/environment.txt"

if $run_smoke || $run_full; then
  cargo bench --bench rust_jsc_bench --no-run \
    2>&1 | tee "$output_dir/bench-build.log"
fi

if $run_smoke; then
  {
    cargo bench --bench rust_jsc_bench -- context_create \
      --sample-size 10 --warm-up-time 1 --measurement-time 1
    cargo bench --bench rust_jsc_bench -- macro_callback_call_with_args \
      --sample-size 10 --warm-up-time 1 --measurement-time 1
  } 2>&1 | tee "$output_dir/benchmark_smoke.txt"
fi

if $run_full; then
  cargo bench --bench rust_jsc_bench -- --output-format bencher \
    2>&1 | tee "$output_dir/all_benchmarks.txt"
fi

if $run_dhat; then
  rm -f dhat-heap.json
  cargo run --manifest-path examples/profiling/Cargo.toml --release \
    2>&1 | tee "$output_dir/dhat_output.txt"
  if [ -f dhat-heap.json ]; then
    mv dhat-heap.json "$output_dir/dhat-heap.json"
  else
    echo "DHAT did not produce dhat-heap.json" >&2
    exit 1
  fi
fi

if $run_flamegraph; then
  if ! cargo flamegraph --version >/dev/null 2>&1; then
    echo "cargo flamegraph is not installed; run: cargo install flamegraph" >&2
    exit 1
  fi
  cargo flamegraph --manifest-path examples/stress/Cargo.toml \
    -o "$output_dir/flamegraph_stress.svg" \
    --release
fi

echo "performance artifacts: $output_dir"
