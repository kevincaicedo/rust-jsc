---
name: rust-jsc v1.0 release
about: Track the remaining v1.0 GitHub release, archive, CI, and crates.io evidence.
title: "rust-jsc v1.0.0 release evidence"
labels: release
assignees: ""
---

## Release Scope

- [ ] This issue tracks only the `rust-jsc` v1.0.0 release execution.
- [ ] Bytecode-cache and precompiled-app work is deferred to a separate startup-optimization plan.
- [ ] No performance claim is included unless the matching benchmark/profiler artifact URLs are recorded below.

## Candidate

- Release branch or PR:
- Root commit SHA:
- WebKit submodule SHA:
- Confirmed exact `1.0.0` versions:
  - [ ] `Cargo.toml`
  - [ ] `sys/Cargo.toml`
  - [ ] `macros/Cargo.toml`
- Confirmed root dependencies pinned to exact sibling crate versions:
  - [ ] `rust_jsc_sys = "=1.0.0"`
  - [ ] `rust_jsc_macros = "=1.0.0"`

## WebKit Fork Expert Review

- Review command:

```bash
git -C rust-jsc/WebKit diff --name-status main...kedo-webkit/rebase-2026-05
```

- Diff check command:

```bash
git -C rust-jsc/WebKit diff --check main...kedo-webkit/rebase-2026-05
```

- Review notes or attachment:
- Diff check result:
- [ ] No critical/high finding remains open.
- [ ] Every medium/low finding is fixed or explicitly deferred with owner,
      rationale, and validation evidence.

## Local Preflight

- Command:

```bash
RUST_JSC_BUILD_MODE=download \
RUST_JSC_LIB_DIR=/home/kcaicedo/Documents/Projects/runtime/rust-jsc/WebKit/WebKitBuild/RustJSC-Phase3Final/JSCOnly/Release-Static/lib \
scripts/release_candidate_check.sh --tag v1.0.0
```

- Log URL or attachment:
- [ ] Log ends with `release candidate preflight passed for v1.0.0`.

## GitHub Validation

- Pull-request CI run:
- WebKit fork CI run:
- Security CI run:
- Benchmark CI run:
- Performance artifact URLs, if release notes make a performance claim:

## Archive Release

- `sys-v1.0.0` archive workflow run:
- `sys-v1.0.0` GitHub release:
- Archive verifier command:

```bash
scripts/verify_release_artifacts.sh --tag v1.0.0 --archive-only
```

- Archive verifier log:
- [ ] `SHA256SUMS` is present.
- [ ] Every supported archive has an exact `SHA256SUMS` row.
- [ ] Every supported archive has a `.sha256` sidecar.
- [ ] Every supported archive has metadata JSON.

## Native Integration

- Native integration workflow run:
- [ ] Released archives work on Linux x86_64.
- [ ] Released archives work on Linux aarch64.
- [ ] Released archives work on macOS x86_64.
- [ ] Released archives work on macOS arm64.

## Crate Release And Publish

- `v1.0.0` crate-release workflow run:
- `v1.0.0` GitHub release:
- Cargo publish dry-run workflow run:
- Cargo publish workflow run:
- `rust_jsc_sys` crates.io v1.0.0:
- `rust_jsc_macros` crates.io v1.0.0:
- `rust_jsc` crates.io v1.0.0:

## Post-Publish Verification

- Post-publish verifier command:

```bash
scripts/verify_release_artifacts.sh --tag v1.0.0
```

- Post-publish verifier log:
- Fresh-consumer default-download smoke command/result:

## Closeout

- [ ] `docs/release-notes-v1.0.0.md` has been filled from this issue.
- [ ] Release notes include the archive release URL.
- [ ] Release notes include the crate release URL.
- [ ] Release notes include crates.io package URLs.
- [ ] Release notes include CI workflow URLs.
- [ ] Release notes include performance artifacts for every performance claim, or make no performance claim.
