#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tag=""
run_cargo_fmt=true
run_cargo_check=true
run_cargo_test=true
run_cargo_clippy=true
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/rust-jsc-release-candidate.XXXXXX")"

cleanup() {
  rm -rf "$work_dir"
}
trap cleanup EXIT

usage() {
  cat <<'USAGE'
Usage: scripts/release_candidate_check.sh [OPTIONS]

Options:
  --tag TAG             Release tag to validate, for example v1.0.0.
                        Defaults to v<root Cargo.toml version>.
  --skip-cargo-fmt      Skip cargo fmt -- --check.
  --skip-cargo-check    Skip cargo check --workspace --all-targets.
  --skip-cargo-test     Skip cargo test --workspace -- --test-threads=1.
  --skip-cargo-clippy   Skip cargo clippy --workspace --all-targets.
  --package-only        Skip fmt/check/test/clippy and run package/doc checks only.
  -h, --help            Show this help.

Set RUST_JSC_BUILD_MODE/RUST_JSC_LIB_DIR/RUST_JSC_ARCHIVE as needed before
running this script. A local static JSC build usually uses:

  RUST_JSC_BUILD_MODE=download
  RUST_JSC_LIB_DIR=/path/to/WebKitBuild/.../Release-Static/lib
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --tag)
      if [ "$#" -lt 2 ]; then
        echo "missing value for --tag" >&2
        exit 2
      fi
      tag="$2"
      shift 2
      ;;
    --skip-cargo-fmt)
      run_cargo_fmt=false
      shift
      ;;
    --skip-cargo-check)
      run_cargo_check=false
      shift
      ;;
    --skip-cargo-test)
      run_cargo_test=false
      shift
      ;;
    --skip-cargo-clippy)
      run_cargo_clippy=false
      shift
      ;;
    --package-only)
      run_cargo_fmt=false
      run_cargo_check=false
      run_cargo_test=false
      run_cargo_clippy=false
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

cd "$repo_root"

manifest_version() {
  sed -n 's/^version = "\([^"]*\)".*/\1/p' "$1" | head -1
}

dependency_version() {
  local name="$1"
  sed -n "s/^${name}.*version = \"\\([^\"]*\\)\".*/\\1/p" Cargo.toml | head -1
}

root_version="$(manifest_version Cargo.toml)"
sys_version="$(manifest_version sys/Cargo.toml)"
macros_version="$(manifest_version macros/Cargo.toml)"
sys_dependency_version="$(dependency_version rust_jsc_sys)"
macros_dependency_version="$(dependency_version rust_jsc_macros)"

if [ -z "$tag" ]; then
  tag="v$root_version"
fi

if [[ "$tag" != v* ]]; then
  echo "release tag must use v<version>, got $tag" >&2
  exit 1
fi

tag_version="${tag#v}"
if [ "$tag_version" != "$root_version" ]; then
  echo "tag version $tag_version does not match root version $root_version" >&2
  exit 1
fi

if [ "$sys_version" != "$root_version" ] || [ "$sys_dependency_version" != "=$root_version" ]; then
  echo "rust_jsc_sys version/dependency ($sys_version/$sys_dependency_version) must match root version $root_version exactly" >&2
  exit 1
fi

if [ "$macros_version" != "$root_version" ] || [ "$macros_dependency_version" != "=$root_version" ]; then
  echo "rust_jsc_macros version/dependency ($macros_version/$macros_dependency_version) must match root version $root_version exactly" >&2
  exit 1
fi

echo "versions aligned exactly for $tag"

if compgen -G ".github/workflows/*.yml" >/dev/null; then
  ruby -e 'ARGV.each { |file| require "yaml"; YAML.load_file(file); puts "workflow yaml ok: #{file}" }' .github/workflows/*.yml
fi

check_release_workflow_semantics() {
  ruby <<'RUBY'
checks = [
  [
    ".github/workflows/cargo-publish.yml",
    "DRY_RUN=\"true\"",
    "cargo-publish.yml must default tag-triggered runs to dry-run mode"
  ],
  [
    ".github/workflows/cargo-publish.yml",
    "github.event_name",
    "cargo-publish.yml must distinguish manual dispatch from tag-triggered runs"
  ],
  [
    ".github/workflows/cargo-publish.yml",
    "dry_run=false",
    "cargo-publish.yml must document explicit manual publish mode"
  ],
  [
    ".github/workflows/build-release.yml",
    "contents: write",
    "build-release.yml release job must be able to create the sys-v release"
  ],
  [
    ".github/workflows/release-crates.yml",
    "contents: write",
    "release-crates.yml release job must be able to create the v release"
  ],
]

ok = true
checks.each do |file, needle, message|
  unless File.read(file).include?(needle)
    warn "#{message}: missing #{needle.inspect} in #{file}"
    ok = false
  end
end

exit(ok ? 0 : 1)
RUBY
}

check_release_workflow_semantics
echo "release workflow semantics ok"

check_release_documentation_semantics() {
  ruby <<'RUBY'
def manifest_name(path)
  File.read(path)[/^name\s*=\s*"([^"]+)"/, 1] or raise "missing package name in #{path}"
end

def manifest_readme(path)
  File.read(path)[/^readme\s*=\s*"([^"]+)"/, 1] or raise "missing readme field in #{path}"
end

root_name = manifest_name("Cargo.toml")
sys_name = manifest_name("sys/Cargo.toml")
macros_name = manifest_name("macros/Cargo.toml")
readme = File.read("README.md")
release_process = File.read("docs/release-process.md")
script = File.read("scripts/verify_release_artifacts.sh")

checks = [
  [
    File.file?(File.join(File.dirname("Cargo.toml"), manifest_readme("Cargo.toml"))),
    "#{root_name} manifest readme must point at an existing file"
  ],
  [
    File.file?(File.join(File.dirname("sys/Cargo.toml"), manifest_readme("sys/Cargo.toml"))),
    "#{sys_name} manifest readme must point at an existing file"
  ],
  [
    File.file?(File.join(File.dirname("macros/Cargo.toml"), manifest_readme("macros/Cargo.toml"))),
    "#{macros_name} manifest readme must point at an existing file"
  ],
  [
    readme.include?("https://img.shields.io/crates/v/#{root_name}.svg"),
    "README crates.io badge must use the root package name #{root_name}"
  ],
  [
    readme.include?("https://crates.io/crates/#{root_name}"),
    "README crates.io link must use the root package name #{root_name}"
  ],
  [
    readme.include?("#{root_name} = { version = \""),
    "README install snippet must use the root package name #{root_name}"
  ],
  [
    release_process.include?("`#{sys_name}`, `#{macros_name}`, then `#{root_name}`"),
    "release process publish order must match package names"
  ],
  [
    script.include?(sys_name) && script.include?(macros_name) && script.include?(root_name),
    "release artifact verifier must check all package names"
  ],
]

failed = checks.reject(&:first).map(&:last)
unless failed.empty?
  failed.each { |message| warn message }
  exit 1
end
RUBY
}

check_release_documentation_semantics
echo "release documentation semantics ok"

bash -n scripts/verify_release_artifacts.sh
scripts/verify_release_artifacts.sh --self-test
echo "release artifact verifier ok"

check_repo_markdown_links() {
  ruby -e '
    ok = true
    ARGV.each do |file|
      base = File.dirname(file)
      File.read(file).scan(/\[[^\]]*\]\(([^)]+)\)/).flatten.each do |href|
        next if href =~ /\A[a-z][a-z0-9+.-]*:/ || href.start_with?("#")
        path = href.split("#", 2).first
        next if path.empty?
        full = File.expand_path(path, base)
        unless File.exist?(full)
          warn "broken repo link: #{file} -> #{href}"
          ok = false
        end
      end
    end
    exit(ok ? 0 : 1)
  ' README.md docs/*.md sys/README.md examples/profiling/README.md
}

check_package_markdown_links() {
  local label="$1"
  local prefix="$2"
  local package_list="$3"

  ruby -rset -rpathname - "$label" "$prefix" "$package_list" <<'RUBY'
label, prefix, package_list = ARGV
package_files = File.readlines(package_list, chomp: true).to_set
ok = true

package_files.grep(/\.md\z/).each do |file|
  source = prefix == "." ? file : File.join(prefix, file)
  base = Pathname.new(file).dirname

  File.read(source).scan(/\[[^\]]*\]\(([^)]+)\)/).flatten.each do |href|
    next if href =~ /\A[a-z][a-z0-9+.-]*:/ || href.start_with?("#")

    path = href.split("#", 2).first
    next if path.empty?

    target = (base + path).cleanpath.to_s
    unless package_files.include?(target)
      warn "missing #{label} packaged link: #{file} -> #{href} resolves #{target}"
      ok = false
    end
  end
end

exit(ok ? 0 : 1)
RUBY
}

check_repo_markdown_links
echo "repository markdown links ok"

cargo package -p rust_jsc_sys --allow-dirty --no-verify --locked
cargo package -p rust_jsc_sys --allow-dirty --no-verify --locked --list > "$work_dir/rust_jsc_sys-package-files.txt"
check_package_markdown_links rust_jsc_sys sys "$work_dir/rust_jsc_sys-package-files.txt"
echo "rust_jsc_sys package links ok"

cargo package -p rust_jsc_macros --allow-dirty --no-verify --locked
cargo package -p rust_jsc_macros --allow-dirty --no-verify --locked --list > "$work_dir/rust_jsc_macros-package-files.txt"
check_package_markdown_links rust_jsc_macros macros "$work_dir/rust_jsc_macros-package-files.txt"
echo "rust_jsc_macros package links ok"

cargo package -p rust_jsc --allow-dirty --no-verify --locked --list > "$work_dir/rust_jsc-package-files.txt"
check_package_markdown_links rust_jsc . "$work_dir/rust_jsc-package-files.txt"
echo "rust_jsc package file list and links ok"

if $run_cargo_fmt; then
  cargo fmt -- --check
fi

if $run_cargo_check; then
  cargo check --workspace --all-targets
fi

if $run_cargo_test; then
  cargo test --workspace -- --test-threads=1
fi

if $run_cargo_clippy; then
  cargo clippy --workspace --all-targets -- -A clippy::approx_constant
fi

git diff --check

echo "release candidate preflight passed for $tag"
