#!/usr/bin/env bash
set -euo pipefail

repo="kevincaicedo/rust-jsc"
version=""
download_archives=false
archive_only=false
skip_crates=false
self_test=false
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/rust-jsc-release-artifacts.XXXXXX")"

cleanup() {
  rm -rf "$work_dir"
}
trap cleanup EXIT

usage() {
  cat <<'USAGE'
Usage: scripts/verify_release_artifacts.sh --version VERSION [OPTIONS]

Options:
  --version VERSION       Release version to verify, for example 1.0.0.
  --tag TAG               Crate release tag to verify, for example v1.0.0.
                          Equivalent to --version 1.0.0.
  --repo OWNER/REPO       GitHub repository. Defaults to kevincaicedo/rust-jsc.
  --download-archives     Download archives and verify checksums locally.
                          Without this, the script verifies release presence,
                          asset presence, sidecars, metadata, and exact
                          SHA256SUMS filename rows.
  --archive-only          Verify only sys-v<version> release assets. Use this
                          before the v<version> crate release exists.
  --skip-crates           Do not verify crates.io entries.
  --self-test             Run local verifier checks without network access.
  -h, --help              Show this help.
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      if [ "$#" -lt 2 ]; then
        echo "missing value for --version" >&2
        exit 2
      fi
      version="$2"
      shift 2
      ;;
    --tag)
      if [ "$#" -lt 2 ]; then
        echo "missing value for --tag" >&2
        exit 2
      fi
      tag="$2"
      if [[ "$tag" != v* ]]; then
        echo "--tag must use v<version>, got $tag" >&2
        exit 2
      fi
      version="${tag#v}"
      shift 2
      ;;
    --repo)
      if [ "$#" -lt 2 ]; then
        echo "missing value for --repo" >&2
        exit 2
      fi
      repo="$2"
      shift 2
      ;;
    --download-archives)
      download_archives=true
      shift
      ;;
    --archive-only)
      archive_only=true
      skip_crates=true
      shift
      ;;
    --skip-crates)
      skip_crates=true
      shift
      ;;
    --self-test)
      self_test=true
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

if $self_test && [ -z "$version" ]; then
  version="1.0.0"
fi

if [ -z "$version" ]; then
  echo "--version or --tag is required" >&2
  usage >&2
  exit 2
fi

if [[ "$version" == v* ]]; then
  echo "--version must not include the v prefix; use --tag for tags" >&2
  exit 2
fi

if [[ "$repo" != */* ]]; then
  echo "--repo must use OWNER/REPO, got $repo" >&2
  exit 2
fi

sys_tag="sys-v${version}"
crate_tag="v${version}"
api_base="https://api.github.com/repos/${repo}"
download_base="https://github.com/${repo}/releases/download/${sys_tag}"
user_agent="rust-jsc-release-verifier"

archives=(
  libjsc-x86_64-apple-darwin.a.gz
  libjsc-aarch64-apple-darwin.a.gz
  libjsc-x86_64-unknown-linux-gnu.a.gz
  libjsc-aarch64-unknown-linux-gnu.a.gz
  libjsc-x86_64-unknown-linux-musl.a.gz
  libjsc-aarch64-unknown-linux-musl.a.gz
)

crates=(
  rust_jsc_sys
  rust_jsc_macros
  rust_jsc
)

curl_get() {
  curl --fail --silent --show-error --location -A "$user_agent" "$@"
}

curl_head() {
  curl --fail --silent --show-error --head --location -A "$user_agent" "$@" >/dev/null
}

verify_release_exists() {
  local tag="$1"
  curl_get "${api_base}/releases/tags/${tag}" >/dev/null
  echo "release exists: ${tag}"
}

verify_sha_row() {
  local archive="$1"
  if ! awk -v archive="$archive" '$2 == archive { found=1 } END { exit found ? 0 : 1 }' "$work_dir/SHA256SUMS"; then
    echo "SHA256SUMS is missing an exact filename row for ${archive}" >&2
    exit 1
  fi
}

expected_sha_for_archive() {
  local archive="$1"
  awk -v archive="$archive" '$2 == archive { print $1; found=1 } END { exit found ? 0 : 1 }' "$work_dir/SHA256SUMS"
}

verify_sidecar() {
  local archive="$1"
  local sidecar_path="$2"
  local expected_sha="$3"

  awk -v archive="$archive" -v expected_sha="$expected_sha" '
    $1 == expected_sha && $2 == archive { found=1 }
    END { exit found ? 0 : 1 }
  ' "$sidecar_path" || {
    echo "${sidecar_path} does not contain the expected checksum row for ${archive}" >&2
    exit 1
  }
}

verify_metadata() {
  local metadata_path="$1"
  local archive="$2"
  local expected_sha="$3"
  local target="$4"

  ruby -rjson - "$metadata_path" "$archive" "$expected_sha" "$version" "$target" <<'RUBY'
metadata_path, archive, expected_sha, version, target = ARGV
data = JSON.parse(File.read(metadata_path))
errors = []

errors << "archive must be #{archive}" unless data["archive"] == archive
errors << "archive_sha256 must match SHA256SUMS" unless data["archive_sha256"] == expected_sha
errors << "format_version must be 1" unless data["format_version"] == 1
errors << "rust_jsc_sys_version must be #{version}" unless data["rust_jsc_sys_version"] == version
errors << "target_triple must be #{target}" unless data["target_triple"] == target

required_keys = [
  "archive",
  "archive_sha256",
  "compiler_versions",
  "format_version",
  "included_libraries",
  "repository_commit",
  "rust_jsc_sys_version",
  "target_triple",
  "webkit_commit",
]
missing_keys = required_keys.reject { |key| data.key?(key) }
errors << "missing metadata keys: #{missing_keys.join(", ")}" unless missing_keys.empty?

unless data["archive_sha256"].is_a?(String) && data["archive_sha256"].match?(/\A[0-9a-f]{64}\z/)
  errors << "archive_sha256 must be a lowercase SHA-256 hex string"
end

unless data["repository_commit"].is_a?(String) && data["repository_commit"].match?(/\A[0-9a-f]{40}\z/)
  errors << "repository_commit must be a 40-character git SHA"
end

unless data["webkit_commit"].is_a?(String) && data["webkit_commit"].match?(/\A[0-9a-f]{40}\z/)
  errors << "webkit_commit must be a 40-character git SHA"
end

unless data["compiler_versions"].is_a?(Hash)
  errors << "compiler_versions must be an object"
end

unless data["included_libraries"].is_a?(Array) && !data["included_libraries"].empty?
  errors << "included_libraries must be a non-empty array"
end

libraries = Array(data["included_libraries"])
library_names = libraries.map { |entry| entry["name"] }
required_libraries = ["libJavaScriptCore.a", "libWTF.a", "libbmalloc.a"]

if target.end_with?("-apple-darwin")
  required_libraries << "libJavaScriptCoreJIT.a"
end

if target.include?("-unknown-linux-")
  required_libraries.concat([
    "libstdc++.a",
    "libicui18n.a",
    "libicuuc.a",
    "libicudata.a",
    "libatomic.a",
  ])
end

missing_libraries = required_libraries - library_names
errors << "missing included libraries: #{missing_libraries.join(", ")}" unless missing_libraries.empty?

libraries.each do |entry|
  unless entry["name"].is_a?(String) && !entry["name"].empty?
    errors << "included library has invalid name"
  end
  unless entry["sha256"].is_a?(String) && entry["sha256"].match?(/\A[0-9a-f]{64}\z/)
    errors << "included library #{entry["name"] || "<unknown>"} has invalid sha256"
  end
  unless entry["size"].is_a?(Integer) && entry["size"].positive?
    errors << "included library #{entry["name"] || "<unknown>"} has invalid size"
  end
end

unless errors.empty?
  warn "#{metadata_path} failed validation:"
  errors.each { |error| warn "  - #{error}" }
  exit 1
end
RUBY
}

verify_archive_assets() {
  mkdir -p "$work_dir/assets"
  curl_get "${download_base}/SHA256SUMS" -o "$work_dir/SHA256SUMS"

  for archive in "${archives[@]}"; do
    sidecar="${archive}.sha256"
    metadata="${archive%.a.gz}.metadata.json"
    target="${archive#libjsc-}"
    target="${target%.a.gz}"
    expected_sha="$(expected_sha_for_archive "$archive")"
    if [[ ! "$expected_sha" =~ ^[0-9a-f]{64}$ ]]; then
      echo "SHA256SUMS has an invalid checksum for ${archive}: ${expected_sha}" >&2
      exit 1
    fi

    verify_sha_row "$archive"
    curl_head "${download_base}/${archive}"
    curl_get "${download_base}/${sidecar}" -o "$work_dir/assets/${sidecar}"
    curl_get "${download_base}/${metadata}" -o "$work_dir/assets/${metadata}"
    verify_sidecar "$archive" "$work_dir/assets/${sidecar}" "$expected_sha"
    verify_metadata "$work_dir/assets/${metadata}" "$archive" "$expected_sha" "$target"

    if $download_archives; then
      curl_get "${download_base}/${archive}" -o "$work_dir/assets/${archive}"
      (
        cd "$work_dir/assets"
        if command -v sha256sum >/dev/null 2>&1; then
          sha256sum -c "$sidecar"
        else
          shasum -a 256 -c "$sidecar"
        fi
      )
    fi

    echo "archive assets ok: ${archive}"
  done
}

verify_crates() {
  for crate_name in "${crates[@]}"; do
    curl_get "https://crates.io/api/v1/crates/${crate_name}/${version}" >/dev/null
    echo "crate exists: ${crate_name} ${version}"
  done
}

write_self_test_metadata() {
  local archive="$1"
  local target="$2"
  shift 2
  local libraries=("$@")
  local sha="0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
  local git_sha="0123456789abcdef0123456789abcdef01234567"
  local sidecar="$work_dir/${archive}.sha256"
  local metadata="$work_dir/${archive%.a.gz}.metadata.json"

  printf '%s  %s\n' "$sha" "$archive" >> "$work_dir/SHA256SUMS"
  printf '%s  %s\n' "$sha" "$archive" > "$sidecar"

  ruby -rjson - "$metadata" "$archive" "$target" "$sha" "$git_sha" "$version" "${libraries[@]}" <<'RUBY'
metadata_path, archive, target, sha, git_sha, version, *libraries = ARGV
metadata = {
  "archive" => archive,
  "archive_sha256" => sha,
  "compiler_versions" => {},
  "format_version" => 1,
  "included_libraries" => libraries.map do |name|
    { "name" => name, "sha256" => sha, "size" => 1 }
  end,
  "repository_commit" => git_sha,
  "rust_jsc_sys_version" => version,
  "target_triple" => target,
  "webkit_commit" => git_sha,
}
File.write(metadata_path, JSON.pretty_generate(metadata) + "\n")
RUBY

  verify_sha_row "$archive"
  expected_sha_for_archive "$archive" >/dev/null
  verify_sidecar "$archive" "$sidecar" "$sha"
  verify_metadata "$metadata" "$archive" "$sha" "$target"
}

run_self_test() {
  : > "$work_dir/SHA256SUMS"

  write_self_test_metadata \
    "libjsc-x86_64-unknown-linux-gnu.a.gz" \
    "x86_64-unknown-linux-gnu" \
    libJavaScriptCore.a \
    libWTF.a \
    libbmalloc.a \
    libstdc++.a \
    libicui18n.a \
    libicuuc.a \
    libicudata.a \
    libatomic.a

  write_self_test_metadata \
    "libjsc-aarch64-apple-darwin.a.gz" \
    "aarch64-apple-darwin" \
    libJavaScriptCore.a \
    libJavaScriptCoreJIT.a \
    libWTF.a \
    libbmalloc.a

  echo "release artifact verifier self-test passed"
}

if $self_test; then
  run_self_test
  exit 0
fi

verify_release_exists "$sys_tag"
verify_archive_assets

if ! $archive_only; then
  verify_release_exists "$crate_tag"

  if ! $skip_crates; then
    verify_crates
  fi

  echo "release artifacts verified for ${crate_tag}"
else
  echo "archive release artifacts verified for ${sys_tag}"
fi
