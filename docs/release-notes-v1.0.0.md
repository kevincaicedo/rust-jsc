# rust-jsc v1.0.0 Release Notes Draft

Status: draft until every release-evidence placeholder below is filled. Do not
publish these notes as final release notes until the `sys-v1.0.0` archive
release, `v1.0.0` crate release, crates.io publish, and post-publish verifier
have all passed on the exact release candidate.

## Release Evidence

Fill these rows from the `rust-jsc v1.0 release` issue before closeout:

| Evidence | URL or result |
| --- | --- |
| Release branch or PR | TODO |
| Root commit SHA | TODO |
| WebKit submodule SHA | TODO |
| WebKit fork expert review | Completed locally on May 23, 2026; see `docs/kedo-webkit-hardening-plan.md`. Re-run only if the WebKit fork diff changes before tagging. |
| WebKit fork diff-check result | Passed locally: `git -C rust-jsc/WebKit diff --check main...kedo-webkit/rebase-2026-05`. |
| Local preflight log | TODO |
| Pull-request CI run | TODO |
| WebKit fork CI run | TODO |
| Security CI run | TODO |
| Benchmark CI run | TODO |
| `sys-v1.0.0` archive workflow run | TODO |
| `sys-v1.0.0` GitHub release | TODO |
| Archive verifier log | TODO |
| Native integration workflow run | TODO |
| `v1.0.0` crate-release workflow run | TODO |
| `v1.0.0` GitHub release | TODO |
| Cargo publish dry-run workflow run | TODO |
| Cargo publish workflow run | TODO |
| `rust_jsc_sys` crates.io v1.0.0 | TODO |
| `rust_jsc_macros` crates.io v1.0.0 | TODO |
| `rust_jsc` crates.io v1.0.0 | TODO |
| Post-publish verifier log | TODO |
| Fresh-consumer default-download smoke command/result | TODO |

## Highlights

- Safe Rust wrappers for the core JavaScriptCore embedding surface: contexts,
  context groups, values, objects, arrays, strings, typed arrays, classes,
  functions, errors, promises, modules, and inspector basics.
- Ownership-explicit handles for JavaScriptCore resources, including RAII
  protection for JS values and typed private-data/shared-data guards.
- Checked property access, typed extraction/conversion traits, and fallible
  builder APIs for common embedding paths.
- Callback and module-loader macro support with documented supported
  signatures, compile-time diagnostics, unit tests, and `trybuild` fixtures.
- Deterministic static JavaScriptCore archive workflow with per-target
  archives, `.sha256` sidecars, metadata JSON, and `SHA256SUMS` verification.
- Release gates for PR CI, WebKit fork CI, security checks, benchmark smoke
  checks, native archive integration, crate release creation, cargo publish
  dry-run, manual crates.io publish, and post-publish artifact verification.

## Safety And API Contracts

- `JSContext`, `JSGlobalContext`, JavaScriptCore values, protected values, and
  private data wrappers remain thread-affine unless a type explicitly documents
  a safe cross-thread handoff.
- Public fallible APIs surface JavaScriptCore exceptions through typed results
  instead of silently ignoring exception slots.
- Callback, host-op, module-loader, promise, inspector, and finalizer paths are
  designed so Rust panics do not cross JavaScriptCore ABI boundaries.
- Unsafe escape hatches remain narrow and documented with caller obligations;
  normal embedding paths should use checked safe wrappers and builders.
- KedoJS-specific runtime policy stays outside `rust-jsc`; this crate exposes
  reusable JavaScriptCore primitives rather than filesystem, loader, bytecode,
  or runtime policy.

## Build And Release

The default release path uses `rust_jsc_sys` static archive downloads. The
archive release for `sys-v1.0.0` must include these supported targets:

| Target | Archive |
| --- | --- |
| `x86_64-apple-darwin` | `libjsc-x86_64-apple-darwin.a.gz` |
| `aarch64-apple-darwin` | `libjsc-aarch64-apple-darwin.a.gz` |
| `x86_64-unknown-linux-gnu` | `libjsc-x86_64-unknown-linux-gnu.a.gz` |
| `aarch64-unknown-linux-gnu` | `libjsc-aarch64-unknown-linux-gnu.a.gz` |
| `x86_64-unknown-linux-musl` | `libjsc-x86_64-unknown-linux-musl.a.gz` |
| `aarch64-unknown-linux-musl` | `libjsc-aarch64-unknown-linux-musl.a.gz` |

Every archive must have a `.sha256` sidecar, a metadata JSON file, and an exact
filename row in `SHA256SUMS`. Run the verifier after the archive release and
again after crates.io publish:

```bash
scripts/verify_release_artifacts.sh --tag v1.0.0 --archive-only
scripts/verify_release_artifacts.sh --tag v1.0.0
```

## Migration Notes

- Package names are `rust_jsc`, `rust_jsc_sys`, and `rust_jsc_macros`.
- The `v1.0.0` release must publish the three crates at the same version, with
  root dependency constraints pinned to exact sibling versions.
- Publish order is `rust_jsc_sys`, then `rust_jsc_macros`, then `rust_jsc`.
- Deprecated misspelled or legacy names remain migration-window aliases where
  retained; new code should use the corrected public API names.
- Bytecode-cache and precompiled-app support are intentionally out of scope for
  `rust-jsc` 1.0. They are deferred to a separate startup-optimization plan.

## Performance Notes

This draft makes no broad throughput, latency, startup, or memory-improvement
claim. Add a performance claim only when the release issue records matching
same-workload benchmark/profiler artifact URLs, environment notes, and the
metric being compared.

The benchmark workflow and `scripts/performance_snapshot.sh` provide the
release substrate for performance evidence, but workflow existence alone is
not proof of a user-visible performance improvement.

## Known Follow-Ups

- KedoJS CDP translation, source-map handling, runtime policy, and debugger
  queue behavior remain KedoJS-owned follow-up work.
- KedoJS still has runtime-owned warning debt and a legacy raw core
  module-loader path to migrate to the public `rust-jsc` API path.
- Rust `Future` bridges, cancellation semantics, and startup/HTTP/stdlib
  performance budgets require separate runtime evidence before they are
  release claims.
- Bytecode-cache and precompiled-app support remain deferred outside this
  `rust-jsc` 1.0 plan.

## Final Verification Checklist

- [ ] Release branch or tag points at the reviewed candidate.
- [x] WebKit fork expert review has no open critical/high findings for the
      current `main...kedo-webkit/rebase-2026-05` diff.
- [ ] Local preflight log ends with
      `release candidate preflight passed for v1.0.0`.
- [ ] Required GitHub validation workflows passed on the exact candidate.
- [ ] `sys-v1.0.0` release contains all supported archives, sidecars,
      metadata files, and `SHA256SUMS`.
- [ ] Native integration passed against released archives.
- [ ] `v1.0.0` crate release exists after archive release validation.
- [ ] Cargo publish dry-run passed.
- [ ] Crates were published manually in dependency order.
- [ ] Post-publish verifier passed after crates.io indexing caught up.
- [ ] Fresh-consumer default-download smoke test passed without
      `RUST_JSC_LIB_DIR` or `RUST_JSC_ARCHIVE`.
- [ ] Release notes contain no performance claim without matching artifact
      evidence.
