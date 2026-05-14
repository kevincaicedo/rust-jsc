#!/usr/bin/env python3
import argparse
import gzip
import hashlib
import json
import os
import re
import subprocess
import tarfile
from pathlib import Path


def sha256_file(path):
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def command_output(args, cwd=None):
    try:
        output = subprocess.check_output(
            args,
            cwd=cwd,
            stderr=subprocess.STDOUT,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return output.strip().splitlines()[0] if output.strip() else None


def git_commit(path):
    if path is None:
        return None
    return command_output(["git", "rev-parse", "HEAD"], cwd=path)


def read_sys_version(repo_root):
    cargo_toml = repo_root / "sys" / "Cargo.toml"
    match = re.search(r'^version\s*=\s*"([^"]+)"', cargo_toml.read_text(), re.M)
    if not match:
        raise RuntimeError("could not read sys package version from {}".format(cargo_toml))
    return match.group(1)


def metadata_path_for_archive(archive_path, target_triple):
    expected_name = "libjsc-{}.a.gz".format(target_triple)
    if archive_path.name == expected_name:
        return archive_path.with_name("libjsc-{}.metadata.json".format(target_triple))
    return Path(str(archive_path) + ".metadata.json")


def sidecar_path_for_archive(archive_path):
    return Path(str(archive_path) + ".sha256")


def write_deterministic_tar_gz(archive_path, libs):
    tmp_path = Path(str(archive_path) + ".part")
    if tmp_path.exists():
        tmp_path.unlink()

    with tmp_path.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", compresslevel=9, fileobj=raw, mtime=0) as gz:
            with tarfile.open(fileobj=gz, mode="w") as tar:
                for lib in libs:
                    info = tar.gettarinfo(str(lib), arcname=lib.name)
                    info.uid = 0
                    info.gid = 0
                    info.uname = ""
                    info.gname = ""
                    info.mode = 0o644
                    info.mtime = 0
                    with lib.open("rb") as handle:
                        tar.addfile(info, handle)

    os.replace(tmp_path, archive_path)


def compiler_versions():
    cc = os.environ.get("CC", "cc")
    cxx = os.environ.get("CXX", "c++")
    return {
        "rustc": command_output(["rustc", "--version"]),
        "cmake": command_output(["cmake", "--version"]),
        "ninja": command_output(["ninja", "--version"])
        or command_output(["ninja-build", "--version"]),
        "cc": command_output([cc, "--version"]),
        "cxx": command_output([cxx, "--version"]),
    }


def main():
    parser = argparse.ArgumentParser(description="Package deterministic rust-jsc static archives")
    parser.add_argument("--lib-dir", required=True, type=Path)
    parser.add_argument("--target-triple", required=True)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--archive-out", type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--webkit-dir", type=Path)
    parser.add_argument("--build-dir", type=Path)
    args = parser.parse_args()

    lib_dir = args.lib_dir.resolve()
    if not lib_dir.is_dir():
        raise RuntimeError("static library directory not found: {}".format(lib_dir))

    libs = sorted(lib_dir.glob("*.a"), key=lambda path: path.name)
    if not libs:
        raise RuntimeError("no .a files found in {}".format(lib_dir))

    if args.archive_out:
        archive_path = args.archive_out.resolve()
    else:
        output_dir = (args.output_dir or Path.cwd()).resolve()
        archive_path = output_dir / "libjsc-{}.a.gz".format(args.target_triple)

    archive_path.parent.mkdir(parents=True, exist_ok=True)
    write_deterministic_tar_gz(archive_path, libs)
    archive_sha256 = sha256_file(archive_path)

    sidecar_path = sidecar_path_for_archive(archive_path)
    sidecar_path.write_text("{}  {}\n".format(archive_sha256, archive_path.name))

    repo_root = args.repo_root.resolve()
    webkit_dir = args.webkit_dir.resolve() if args.webkit_dir else repo_root / "WebKit"
    metadata = {
        "archive": archive_path.name,
        "archive_sha256": archive_sha256,
        "build_dir": str(args.build_dir.resolve()) if args.build_dir else None,
        "compiler_versions": compiler_versions(),
        "format_version": 1,
        "included_libraries": [
            {
                "name": lib.name,
                "sha256": sha256_file(lib),
                "size": lib.stat().st_size,
            }
            for lib in libs
        ],
        "repository_commit": git_commit(repo_root),
        "rust_jsc_sys_version": read_sys_version(repo_root),
        "target_triple": args.target_triple,
        "webkit_commit": git_commit(webkit_dir),
    }

    metadata_path = metadata_path_for_archive(archive_path, args.target_triple)
    tmp_metadata_path = Path(str(metadata_path) + ".part")
    tmp_metadata_path.write_text(json.dumps(metadata, indent=2, sort_keys=True) + "\n")
    os.replace(tmp_metadata_path, metadata_path)

    print("archive={}".format(archive_path))
    print("sha256={}".format(sidecar_path))
    print("metadata={}".format(metadata_path))


if __name__ == "__main__":
    main()
