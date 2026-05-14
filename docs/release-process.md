# rust-jsc Release Process

The release process has two related tracks:

- `rust_jsc_sys` static JavaScriptCore archive releases tagged as
  `sys-v<rust_jsc_sys version>`
- Rust crate releases tagged as `v<rust_jsc version>` and published in the
  order `rust_jsc_sys`, `rust_jsc_macros`, then `rust_jsc`

Do not publish crates against archive assets that have not passed checksum,
metadata, and target test validation.

## Release Inputs

Before cutting a release, record:

- the `rust_jsc_sys` version in `sys/Cargo.toml`
- the `rust_jsc_macros` version in `macros/Cargo.toml`
- the root `rust_jsc` version in `Cargo.toml`
- the WebKit submodule commit
- the branch or tag being released
- any known unsupported target or deferred feature

The version in the tag must match the corresponding Cargo manifest. The archive
workflow checks `sys-v<version>` against `sys/Cargo.toml`; the crate workflows
check `v<version>` against the root `Cargo.toml`. For a stable release,
`rust_jsc`, `rust_jsc_sys`, and `rust_jsc_macros` are versioned together, and
the root crate's path dependency constraints must pin that exact same version.
This keeps a `v1.0.0` publish from depending on older dependency crate versions
or resolving against a future sibling crate with a different generated
binding/archive contract.

## Static Archive Matrix

The archive workflow builds these targets:

| Target | Artifact |
| --- | --- |
| `x86_64-apple-darwin` | `libjsc-x86_64-apple-darwin.a.gz` |
| `aarch64-apple-darwin` | `libjsc-aarch64-apple-darwin.a.gz` |
| `x86_64-unknown-linux-gnu` | `libjsc-x86_64-unknown-linux-gnu.a.gz` |
| `aarch64-unknown-linux-gnu` | `libjsc-aarch64-unknown-linux-gnu.a.gz` |
| `x86_64-unknown-linux-musl` | `libjsc-x86_64-unknown-linux-musl.a.gz` |
| `aarch64-unknown-linux-musl` | `libjsc-aarch64-unknown-linux-musl.a.gz` |

macOS archive builds run on native hosted runners: `macos-15-intel` for
`x86_64-apple-darwin` and `macos-15` for `aarch64-apple-darwin`. Linux builds
use Docker Buildx and native ARM runners for aarch64.

Each archive must include:

```text
libJavaScriptCore.a
libWTF.a
libbmalloc.a
```

When a JSCOnly build emits `libJavaScriptCoreJIT.a`, it is packaged and linked
after `libJavaScriptCore.a`. Linux release archives also bundle:

```text
libstdc++.a
libicui18n.a
libicuuc.a
libicudata.a
libatomic.a
```

## Archive Release

The archive workflow is `.github/workflows/build-release.yml`.

It runs when `sys/**`, `WebKit/**`, the Dockerfiles, `Makefile`, `.gitmodules`,
or the workflow change on `main`, on `sys-v*` tags, and when manually
triggered. The manual trigger has `skip_cache` for Docker layer cache
investigations and `publish_release` for creating a `sys-v<version>` release
from a manually reviewed run. Pushes to `main` build and upload short-retention
artifacts only; they do not publish releases.

The workflow:

1. Reads the `rust_jsc_sys` version from `sys/Cargo.toml`.
2. Builds JSC static libraries for every supported target.
3. Packages deterministic `libjsc-<target>.a.gz` archives.
4. Uploads each archive with its `.sha256` sidecar and metadata JSON, failing
   if any expected file is absent.
5. Runs a macOS aarch64 library test against the freshly built archive.
6. Collects all release files, verifies that every supported target has its
   archive, `.sha256` sidecar, and metadata JSON, creates `SHA256SUMS`, and
   verifies the manifest.
7. Publishes a GitHub release tagged `sys-v<version>` only for a `sys-v*` tag or
   a manual run with `publish_release=true`.

If any target archive is missing a required library or checksum verification
fails, stop the release. Do not publish a partial `sys-v<version>` release
without documenting the unsupported target in the master plan.

## Crate Release

Crate release creation is split from crate publishing.

`.github/workflows/release-crates.yml` creates the GitHub release for
`v<rust_jsc version>`. It runs on `v*` tags and manual dispatch, validates the
tag against the root `Cargo.toml`, verifies the member crate versions and root
dependency constraints are aligned, verifies that the matching
`sys-v<version>` release exists with every supported archive, `.sha256`
sidecar, metadata JSON file, and exact `SHA256SUMS` archive row, and fails when
the release already exists.

`.github/workflows/cargo-publish.yml` runs on `v*` tags and manual dispatch. It
checks out the tag, verifies that the tag version matches the root
`Cargo.toml`, verifies that `rust_jsc_sys` and `rust_jsc_macros` are the same
version as the root crate and are pinned by exact root dependency constraints,
waits briefly for the matching `v<version>` GitHub release to exist, verifies
that the matching `sys-v<version>` release exposes `SHA256SUMS`, every
supported target archive, every per-archive `.sha256` sidecar, and every
per-target metadata JSON file, and that `SHA256SUMS` contains an exact
filename-field match for every archive. It tests the supported target matrix
with `cargo test --workspace --target <target> -- --test-threads=1` and
`RUST_JSC_BUILD_MODE=download`, then either dry-runs or publishes:

```text
rust_jsc_sys
rust_jsc_macros
rust_jsc
```

Tag-triggered `cargo-publish.yml` runs intentionally default to dry-run mode.
Manual `workflow_dispatch` also defaults to dry-run mode. Dry-run mode runs
`cargo publish --dry-run` for `rust_jsc_sys` and `rust_jsc_macros`, then
performs locked root package inspection for `rust_jsc`. It still verifies the
GitHub release and archive-release prerequisites, so the tag-triggered dry-run
exercises the same release handoff used by publishing. The root crate cannot
complete a real upload-preparation dry-run until the exact dependency crate
versions are visible in crates.io, so the full root upload-preparation check
remains part of the actual ordered publish after dependency crates are
published.

The actual crates.io publish requires a manual `workflow_dispatch` run with
`dry_run=false`. The root crate publish waits briefly after publishing the
dependency crates so the crates.io index can observe them. A publish rerun must
fail unless the operator intentionally creates a new patch version; the workflow
no longer hides publish failures behind an unconditional "already published"
fallback. Set `dry_run=false` only when the local release-candidate preflight,
archive release, native integration run, crate-release creation, and
tag-triggered cargo-publish dry-run have all passed on the exact candidate.

Local preflight can package `rust_jsc_sys` and `rust_jsc_macros` directly. The
root `rust_jsc` crate can be inspected with `cargo package --list`, but a full
root package upload-preparation run requires the matching `rust_jsc_sys` and
`rust_jsc_macros` versions to be visible in the registry. That dependency-order
constraint is why `cargo-publish.yml` publishes the dependency crates first and
only then publishes the root crate.

## v1.0.0 Release Execution Checklist

This checklist is the remaining external M8 gate for the stable release. Local
preflight proves the candidate is internally consistent; it does not prove that
the released archives, GitHub releases, target-matrix tests, or crates.io
publishes exist. Do not call `v1.0.0` shipped until every row below has a run
URL, artifact URL, release URL, registry URL, or review disposition recorded in
the release issue or release notes. Running GitHub workflows, creating release
tags/releases, and publishing crates are human/operator release steps.

| Gate | Required evidence |
| --- | --- |
| Candidate hygiene | Reviewable release branch or tag commit set, no unrelated dirty state, WebKit fork patch areas isolated by logical topic, recorded WebKit submodule commit, exact `1.0.0` versions in all three manifests, and root dependency constraints pinned to `=1.0.0`. |
| WebKit fork expert review | `runtime-webkit-jsc-cpp-expert` review of `git -C rust-jsc/WebKit diff --name-status main...kedo-webkit/rebase-2026-05`, `git -C rust-jsc/WebKit diff --check main...kedo-webkit/rebase-2026-05` result, and fixed or explicitly deferred disposition for every finding. No critical/high finding may remain open before the release handoff. |
| Local preflight | Captured log from `RUST_JSC_BUILD_MODE=download RUST_JSC_LIB_DIR=/home/kcaicedo/Documents/Projects/runtime/rust-jsc/WebKit/WebKitBuild/RustJSC-Phase3Final/JSCOnly/Release-Static/lib scripts/release_candidate_check.sh --tag v1.0.0` ending with `release candidate preflight passed for v1.0.0`. |
| Pull-request CI | Successful `.github/workflows/ci.yml` run for the exact branch or commit that will be tagged. |
| WebKit fork CI | Successful `.github/workflows/webkit-fork.yml` run with Linux `jsc`, `testapi`, `TestJavaScriptCore`, whitespace check, and macOS generated-binding drift check. |
| Security CI | Successful `.github/workflows/security.yml` run with `cargo audit --deny warnings` and archive checksum/path security checks. |
| Benchmark CI | Successful `.github/workflows/benchmarks.yml` smoke run for the exact candidate; record full Criterion, flamegraph, or DHAT artifacts only when release notes make performance claims. |
| Archive release | Successful `.github/workflows/build-release.yml` run for `sys-v1.0.0`, the `sys-v1.0.0` GitHub release URL, all six supported archives, all six `.sha256` sidecars, all six metadata JSON files, and a complete `SHA256SUMS` containing an exact filename-field row for every archive. Run `scripts/verify_release_artifacts.sh --tag v1.0.0 --archive-only` after this step. |
| Native integration | Successful `.github/workflows/native-integration.yml` run after `sys-v1.0.0` exists, proving the released archives work on Linux x86_64/aarch64 and macOS arm64/x86_64. |
| Crate release | Successful `.github/workflows/release-crates.yml` run and `v1.0.0` GitHub release URL after the archive release prerequisites pass. |
| Cargo publish dry-run | Successful tag-triggered or manual `.github/workflows/cargo-publish.yml` run with `dry_run=true` for `v1.0.0`, including target-matrix tests against the released archives. |
| Cargo publish | Successful manual `.github/workflows/cargo-publish.yml` run with `dry_run=false`, plus crates.io URLs for `rust_jsc_sys`, `rust_jsc_macros`, and `rust_jsc` version `1.0.0`. Run `scripts/verify_release_artifacts.sh --tag v1.0.0` after crates.io indexing catches up. |

Open the `rust-jsc v1.0 release` issue template in GitHub, or use this
inline template, to record the evidence. Keep
`docs/release-notes-v1.0.0.md` in draft state until the evidence rows are
filled and the post-publish verifier has passed:

```markdown
## rust-jsc v1.0.0 Release Evidence

- Release branch or PR:
- Root commit SHA:
- WebKit submodule SHA:
- WebKit fork expert review:
- WebKit fork diff-check result:
- Local preflight log:
- Pull-request CI run:
- WebKit fork CI run:
- Security CI run:
- Benchmark CI run:
- sys-v1.0.0 archive workflow run:
- sys-v1.0.0 GitHub release:
- Archive verifier log:
- Native integration workflow run:
- v1.0.0 crate-release workflow run:
- v1.0.0 GitHub release:
- Cargo publish dry-run workflow run:
- Cargo publish workflow run:
- rust_jsc_sys crates.io v1.0.0:
- rust_jsc_macros crates.io v1.0.0:
- rust_jsc crates.io v1.0.0:
- Post-publish verifier log:
- Fresh-consumer default-download smoke command/result:
- Performance artifact URLs, if release notes make a performance claim:
- Bytecode/precompiled-app note: deferred to a separate startup-optimization plan.
```

Use the tag-driven archive path for the static libraries:

```bash
git tag sys-v1.0.0
git push origin sys-v1.0.0
```

The manual archive path is allowed only for a reviewed release branch and must
set `publish_release=true`:

```bash
gh workflow run build-release.yml --ref <release-ref> \
  -f skip_cache=false \
  -f publish_release=true
```

After the archive release, run the validation workflows on the exact candidate
and verify `sys-v1.0.0` exposes exactly the complete release set:

```text
libjsc-x86_64-apple-darwin.a.gz
libjsc-x86_64-apple-darwin.a.gz.sha256
libjsc-x86_64-apple-darwin.metadata.json
libjsc-aarch64-apple-darwin.a.gz
libjsc-aarch64-apple-darwin.a.gz.sha256
libjsc-aarch64-apple-darwin.metadata.json
libjsc-x86_64-unknown-linux-gnu.a.gz
libjsc-x86_64-unknown-linux-gnu.a.gz.sha256
libjsc-x86_64-unknown-linux-gnu.metadata.json
libjsc-aarch64-unknown-linux-gnu.a.gz
libjsc-aarch64-unknown-linux-gnu.a.gz.sha256
libjsc-aarch64-unknown-linux-gnu.metadata.json
libjsc-x86_64-unknown-linux-musl.a.gz
libjsc-x86_64-unknown-linux-musl.a.gz.sha256
libjsc-x86_64-unknown-linux-musl.metadata.json
libjsc-aarch64-unknown-linux-musl.a.gz
libjsc-aarch64-unknown-linux-musl.a.gz.sha256
libjsc-aarch64-unknown-linux-musl.metadata.json
SHA256SUMS
```

Only after those rows are recorded, create the Rust crate release tag:

```bash
git tag v1.0.0
git push origin v1.0.0
```

The `v1.0.0` tag creates the GitHub release and runs cargo-publish in dry-run
mode. After both pass, publish crates manually:

```bash
gh workflow run cargo-publish.yml --ref v1.0.0 \
  -f tag=v1.0.0 \
  -f dry_run=false
```

As a post-publish smoke test, build a fresh consumer checkout without
`RUST_JSC_LIB_DIR` or `RUST_JSC_ARCHIVE` overrides so the default download path
fetches the released `sys-v1.0.0` archive and verifies its exact `SHA256SUMS`
row before extraction.

The release artifact verifier is safe to run from any checkout after the
GitHub releases exist:

```bash
scripts/verify_release_artifacts.sh --tag v1.0.0
```

Use `--archive-only` immediately after the `sys-v1.0.0` archive release, before
the `v1.0.0` crate release exists. Use `--download-archives` when the release
issue needs local checksum evidence for the archive bytes, not only presence of
the archive, sidecar, metadata, and exact `SHA256SUMS` rows. The verifier also
checks sidecar rows against `SHA256SUMS` and validates metadata fields for the
archive name, archive hash, target triple, sys crate version, format version,
repository/WebKit commits, and required library list for the target. Its
offline self-test covers both Linux and macOS required-library policies.

## Validation Gates

Before release, run the local release-candidate preflight with the same
JavaScriptCore archive inputs the release will use. For a local static archive:

```bash
RUST_JSC_BUILD_MODE=download \
RUST_JSC_LIB_DIR=/path/to/WebKitBuild/.../Release-Static/lib \
scripts/release_candidate_check.sh --tag v1.0.0
```

This verifies exact crate-version alignment, parses workflow YAML, checks
release workflow semantics for staged dry-run/manual publishing and release
creation permissions, validates README/release-script package-name semantics,
validates manifest README metadata, validates repository and package-local
Markdown links, syntax-checks and self-tests the release artifact verifier,
packages `rust_jsc_sys` and
`rust_jsc_macros` with locked dependency
resolution, inspects the root `rust_jsc` package file list with locked
dependency resolution, runs `cargo fmt -- --check`, `cargo check --workspace
--all-targets`, `cargo test --workspace -- --test-threads=1`, `cargo clippy
--workspace --all-targets -- -A clippy::approx_constant`, and diff whitespace.
The root crate's full
upload-preparation step still runs in `cargo-publish.yml` after the dependency
crates are published or visible in the registry. Use
`--package-only` or the individual `--skip-cargo-*` flags only for targeted
debugging after a full preflight has already established the release-candidate
baseline.

The underlying checks are:

```bash
cargo fmt -- --check
cargo test --workspace -- --test-threads=1
cargo clippy --workspace --all-targets -- -A clippy::approx_constant
git diff --check
```

Pull-request CI enforces the fast release gate against Linux x86_64. It uses a
released archive only when the archive URL exists and `SHA256SUMS` contains an
exact row for that archive; otherwise it falls back to a local Docker build for
PR validation. It checks formatting, PR diff whitespace, clippy across the
workspace, workspace tests including integration tests, doctests, and macro
`trybuild`, plus example compilation. It also runs deterministic example smoke
tests for the module example and the `api_showcase` bins. Debugger, profiling,
and stress examples compile in PR CI; their full runtime behavior remains
covered by targeted manual, benchmark, or sanitizer workflows because they are
slower or require debugger/profiler-specific interpretation.

The benchmark workflow is a separate performance guardrail. It uses
`scripts/performance_snapshot.sh` so local and CI runs produce the same
`environment.txt` and benchmark/profiler logs. Like PR CI, it only uses a
released archive when the archive and exact `SHA256SUMS` row are available;
otherwise it falls back to a local Docker build for the benchmark run. Pull
requests that touch benchmark or Rust binding paths compile the full Criterion
benchmark suite, run bounded smoke samples for `context_create` and
`macro_callback_call_with_args`, and upload the smoke directory as a
short-retention artifact. Weekly scheduled runs and manual benchmark dispatches
run the full Criterion suite, upload `all_benchmarks.txt`, `performance-full/`,
and `target/criterion/`, and store main-branch trend data through
`benchmark-action/github-action-benchmark`. Comment-triggered `/run-benchmarks`
runs on pull requests still execute the full suite and report the artifact link
back to the PR. Benchmark CI catches broken or extreme regressions; release
notes still need same-workload benchmark evidence before claiming a performance
improvement.

Native integration CI lives in `.github/workflows/native-integration.yml`. It
downloads the released `sys-v<rust_jsc_sys version>` archives for Linux
x86_64/aarch64 and macOS arm64/x86_64, verifies the archive against the exact
matching `SHA256SUMS` row, extracts the archive, and runs workspace tests for
the target with `--test-threads=1` against the prebuilt static libraries.
macOS x86_64 tests run on
`macos-15-intel`; arm64 tests run on `macos-15`. This catches archive linkage
and target-specific integration regressions before a crate release.

WebKit fork CI lives in `.github/workflows/webkit-fork.yml`. The Linux job
builds the JSCOnly fork, runs `jsc`, `testapi`, `TestJavaScriptCore`, and
`git diff --check`. The macOS job rebuilds JSCOnly headers, regenerates
`rust_jsc_sys` bindings with the local fork headers, and fails if
`sys/src/lib.rs` changes without being committed.

For local static JavaScriptCore validation:

```bash
RUST_JSC_BUILD_MODE=download \
RUST_JSC_LIB_DIR=/home/kcaicedo/Documents/Projects/runtime/rust-jsc/WebKit/WebKitBuild/RustJSCRuntime/JSCOnly/Release-Static/lib \
cargo test --workspace -- --test-threads=1
```

JSC tests use `--test-threads=1` because JavaScriptCore global state and
inspector teardown are not safe to stress through parallel Rust test bodies.

For memory-safety release candidates, run the sanitizer workflow or local
targets:

```bash
make test-webkit-api-asan
make test-webkit-api-ubsan
make test-rust-asan
make test-rust-ubsan
```

Sanitizer artifacts are validation evidence only. They are not published as the
default archive set.

The security workflow runs separately from PR CI. Pull requests get GitHub
dependency review, every run gets `cargo audit --deny warnings`, and the
archive-security job runs `scripts/validate_archive_security.sh`. That script
creates local tampered archives and proves `rust_jsc_sys` rejects a bad
checksum before extraction and rejects `..` archive paths before extraction.
These checks are intentionally small and do not build JavaScriptCore.

## Failure And Rollback

If an archive release fails before the GitHub release is created, fix the
branch and rerun the workflow. If a `sys-v<version>` release exists with bad
assets, do not silently replace it for downstream users. Create a new patch
version, publish a new `sys-v<version>` release, and record the reason in the
release notes and master plan.

If crate publishing fails after one dependency crate is published, preserve the
published crate and continue with a fixed patch version or a manually reviewed
rerun. The publish workflow intentionally fails on duplicate or rejected
publishes instead of hiding them behind an "already published" fallback.

If CI cannot use a released archive because `SHA256SUMS`, the target archive,
or the exact manifest row is missing, CI falls back to a local Docker build only
for pull-request validation. Treat that as a release gap, not as proof that the
archive release is complete.
