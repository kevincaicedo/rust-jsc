#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/rust-jsc-archive-security.XXXXXX")"

cleanup() {
  rm -rf "$work_dir"
}
trap cleanup EXIT

make_archive() {
  local archive_path="$1"
  local entry_name="$2"
  python3 - "$archive_path" "$entry_name" <<'PY'
import gzip
import sys
import tarfile
from pathlib import Path

archive_path = Path(sys.argv[1])
entry_name = sys.argv[2]
payload = b"not a real static library"

with archive_path.open("wb") as raw:
    with gzip.GzipFile(filename="", mode="wb", compresslevel=9, fileobj=raw, mtime=0) as gz:
        with tarfile.open(fileobj=gz, mode="w") as tar:
            info = tarfile.TarInfo(entry_name)
            info.size = len(payload)
            info.mode = 0o644
            info.uid = 0
            info.gid = 0
            info.uname = ""
            info.gname = ""
            info.mtime = 0
            tar.addfile(info, fileobj=__import__("io").BytesIO(payload))
PY
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    openssl dgst -sha256 -r "$1" | awk '{print $1}'
  fi
}

expect_failure() {
  local label="$1"
  local archive_path="$2"
  local expected_message="$3"
  local target_dir="$work_dir/target-$label"
  local log_path="$work_dir/$label.log"

  if (
    cd "$repo_root"
    CARGO_TARGET_DIR="$target_dir" \
      RUST_JSC_BUILD_MODE=download \
      RUST_JSC_ARCHIVE="$archive_path" \
      cargo check -p rust_jsc_sys
  ) >"$log_path" 2>&1; then
    echo "expected $label archive validation to fail" >&2
    cat "$log_path" >&2
    return 1
  fi

  if ! grep -F "$expected_message" "$log_path" >/dev/null; then
    echo "$label archive validation failed, but not with the expected message" >&2
    echo "expected to find: $expected_message" >&2
    cat "$log_path" >&2
    return 1
  fi

  echo "$label archive validation rejected as expected"
}

bad_checksum_archive="$work_dir/bad-checksum.a.gz"
make_archive "$bad_checksum_archive" "libJavaScriptCore.a"
printf '%064d  %s\n' 0 "$(basename "$bad_checksum_archive")" > "${bad_checksum_archive}.sha256"
expect_failure "bad-checksum" "$bad_checksum_archive" "JavaScriptCore archive checksum mismatch"

unsafe_path_archive="$work_dir/unsafe-path.a.gz"
make_archive "$unsafe_path_archive" "../evil"
unsafe_hash="$(sha256_of "$unsafe_path_archive")"
printf '%s  %s\n' "$unsafe_hash" "$(basename "$unsafe_path_archive")" > "${unsafe_path_archive}.sha256"
expect_failure "unsafe-path" "$unsafe_path_archive" "contains unsafe path component"

echo "archive security checks passed"
