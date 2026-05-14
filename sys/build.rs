use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BuildMode {
    Download,
    Source,
    System,
}

fn main() {
    if env::var_os("DOCS_RS").is_some() {
        return;
    }

    emit_rerun_directives();
    check_supported_platform();

    match build_mode() {
        BuildMode::Download => {
            let lib_dir = setup_prebuilt_static_libs();
            link_static_libs(&lib_dir);
        }
        BuildMode::Source => {
            let lib_dir = build_jsc_from_source();
            link_static_libs(&lib_dir);
        }
        BuildMode::System => link_system_jsc(),
    }
}

fn build_mode() -> BuildMode {
    if env_truthy("RUST_JSC_FROM_SOURCE") {
        return BuildMode::Source;
    }

    let explicit_mode = env::var("RUST_JSC_BUILD_MODE").ok();
    if explicit_mode.is_none() && env::var_os("RUST_JSC_FRAMEWORK_PATH").is_some() {
        warn("RUST_JSC_FRAMEWORK_PATH is a legacy system-mode alias; prefer RUST_JSC_BUILD_MODE=system");
        return BuildMode::System;
    }

    match explicit_mode
        .unwrap_or_else(|| "download".into())
        .to_ascii_lowercase()
        .as_str()
    {
        "download" => BuildMode::Download,
        "auto" | "custom" | "static" | "archive" => {
            warn("RUST_JSC_BUILD_MODE=auto|custom|static|archive is deprecated; use download");
            BuildMode::Download
        }
        "framework" => {
            warn("RUST_JSC_BUILD_MODE=framework is deprecated; use system for dynamic/framework experiments");
            BuildMode::System
        }
        "source" => BuildMode::Source,
        "system" => BuildMode::System,
        other => panic!(
            "Unsupported RUST_JSC_BUILD_MODE={other}. Use download, source, or system."
        ),
    }
}

fn check_supported_platform() {
    let target_os = target_os();
    let target_arch = target_arch();

    if target_os != "linux" && target_os != "macos" {
        panic!("Unsupported target OS: {target_os}");
    }

    if target_arch != "x86_64" && target_arch != "aarch64" {
        panic!("Unsupported target architecture: {target_arch}");
    }
}

fn emit_rerun_directives() {
    for name in [
        "RUST_JSC_BUILD_MODE",
        "RUST_JSC_ARCHIVE",
        "RUST_JSC_ARCHIVE_OUT",
        "RUST_JSC_CMAKE_ARGS",
        "RUST_JSC_LIB_DIR",
        "RUST_JSC_MIRROR",
        "RUST_JSC_FROM_SOURCE",
        "RUST_JSC_WEBKIT_DIR",
        "RUST_JSC_BUILD_DIR",
        "RUST_JSC_BUILD_PROFILE",
        "RUST_JSC_JOBS",
        "RUST_JSC_SYSTEM_LIBS_PATH",
        "RUST_JSC_SYSTEM_LIB_NAME",
        "SYSTEM_LIBS_PATH",
        // Legacy aliases kept for the 1.0 migration window.
        "RUST_JSC_CUSTOM_ARCHIVE",
        "RUST_JSC_CUSTOM_BUILD_PATH",
        "RUST_JSC_FRAMEWORK_PATH",
        "RUST_JSC_CMAKE_GENERATOR",
        "RUST_JSC_STATIC",
        "RUST_JSC_FORCE_SOURCE_BUILD",
        "RUST_JSC_FORCE_CMAKE_CONFIGURE",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
}

fn warn(message: &str) {
    println!("cargo:warning={message}");
}

fn target_os() -> String {
    env::var("CARGO_CFG_TARGET_OS").unwrap()
}

fn target_arch() -> String {
    env::var("CARGO_CFG_TARGET_ARCH").unwrap()
}

fn target_env() -> String {
    env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default()
}

fn env_truthy(name: &str) -> bool {
    matches!(
        env::var(name).map(|value| value.to_ascii_lowercase()),
        Ok(value) if matches!(value.as_str(), "1" | "true" | "yes" | "on")
    )
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).map(PathBuf::from)
}

fn env_path_alias(primary: &str, legacy: &str) -> Option<PathBuf> {
    env_path(primary).or_else(|| env_path(legacy))
}

fn env_var_alias(primary: &str, legacy: &str) -> Option<String> {
    env::var(primary).ok().or_else(|| env::var(legacy).ok())
}

fn static_lib_file() -> String {
    format!("libjsc-{}.a.gz", target_triple_name())
}

fn target_triple_name() -> String {
    let target_arch = target_arch();
    let target_os = target_os();
    let target_env = target_env();
    match (
        target_os.as_ref(),
        target_arch.as_ref(),
        target_env.as_ref(),
    ) {
        ("linux", "x86_64", "musl") => "x86_64-unknown-linux-musl",
        ("linux", "aarch64", "musl") => "aarch64-unknown-linux-musl",
        ("linux", "x86_64", _) => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64", _) => "aarch64-unknown-linux-gnu",
        ("macos", "x86_64", _) => "x86_64-apple-darwin",
        ("macos", "aarch64", _) => "aarch64-apple-darwin",
        _ => panic!("Unsupported target OS or architecture: {target_os}-{target_arch}"),
    }
    .into()
}

fn static_lib_url() -> String {
    if let Some(custom_archive) = archive_override() {
        return custom_archive;
    }

    format!("{}/{}", release_base_url(), static_lib_file())
}

fn release_base_url() -> String {
    let default_base = "https://github.com/kevincaicedo/rust-jsc/releases/download";
    let base = env::var("RUST_JSC_MIRROR").unwrap_or_else(|_| default_base.into());
    let version = env::var("CARGO_PKG_VERSION").unwrap();

    format!("{}/sys-v{version}", base.trim_end_matches('/'))
}

fn archive_override() -> Option<String> {
    env_var_alias("RUST_JSC_ARCHIVE", "RUST_JSC_CUSTOM_ARCHIVE")
}

fn downloaded_lib_dir() -> PathBuf {
    PathBuf::from(env::var("OUT_DIR").unwrap())
        .join(env::var("CARGO_PKG_VERSION").unwrap())
}

fn setup_prebuilt_static_libs() -> PathBuf {
    if archive_override().is_some() {
        return setup_downloaded_static_libs();
    }

    if let Some(lib_dir) =
        env_path_alias("RUST_JSC_LIB_DIR", "RUST_JSC_CUSTOM_BUILD_PATH")
    {
        return find_static_lib_dir(&lib_dir).unwrap_or_else(|| {
            panic!(
                "Could not find static JavaScriptCore archives under {}. Expected libJavaScriptCore.a, libWTF.a, and libbmalloc.a.",
                lib_dir.display()
            )
        });
    }

    setup_downloaded_static_libs()
}

fn setup_downloaded_static_libs() -> PathBuf {
    let output_path = downloaded_lib_dir();
    let filename = static_lib_file();
    let archive_path = output_path.join(&filename);
    let archive_url = static_lib_url();
    let has_archive_override = archive_override().is_some();

    if has_archive_override || !archive_path.exists() {
        fetch_static_lib(&archive_url, &output_path, &filename);
    }

    let manifest_path = ensure_checksum_manifest(&archive_url, &output_path, &filename);
    verify_archive_checksum(&archive_path, &manifest_path, &filename, &archive_url);

    if has_archive_override || !static_libs_exist(&output_path) {
        extract_static_lib(&archive_path, &output_path);
    }

    if !static_libs_exist(&output_path) {
        panic!(
            "JavaScriptCore archive extracted but required static libs were not found in {}",
            output_path.display()
        );
    }

    output_path
}

fn fetch_static_lib(url: &str, output_path: &Path, filename: &str) {
    let archive_path = output_path.join(filename);

    if let Some(local_archive) = local_archive_path(url) {
        copy_file_atomic(&local_archive, &archive_path, "JavaScriptCore archive");
        return;
    }

    if !url.starts_with("http://") && !url.starts_with("https://") {
        panic!("RUST_JSC_ARCHIVE points to a local archive that does not exist: {url}");
    }

    download_remote_file(url, output_path, filename, "static JavaScriptCore archive");
}

fn download_remote_file(url: &str, output_path: &Path, filename: &str, label: &str) {
    let output = Command::new("python3")
        .arg("scripts/download_file.py")
        .arg(url)
        .arg(output_path)
        .arg(filename)
        .output()
        .unwrap_or_else(|error| panic!("Failed to download {label} from {url}: {error}"));

    if !output.status.success() {
        panic!(
            "Failed to download {label} from {url}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn ensure_checksum_manifest(
    archive_url: &str,
    output_path: &Path,
    filename: &str,
) -> PathBuf {
    let manifest_filename = checksum_manifest_filename(filename);
    let manifest_path = output_path.join(&manifest_filename);
    let has_archive_override = archive_override().is_some();

    if has_archive_override || !manifest_path.exists() {
        if let Some(local_archive) = local_archive_path(archive_url) {
            let local_manifest = local_checksum_manifest_path(&local_archive);
            copy_file_atomic(
                &local_manifest,
                &manifest_path,
                "JavaScriptCore checksum manifest",
            );
        } else {
            let manifest_url = checksum_manifest_url(archive_url);
            download_remote_file(
                &manifest_url,
                output_path,
                &manifest_filename,
                "JavaScriptCore checksum manifest",
            );
        }
    }

    manifest_path
}

fn checksum_manifest_filename(filename: &str) -> String {
    if archive_override().is_some() {
        format!("{filename}.sha256")
    } else {
        "SHA256SUMS".into()
    }
}

fn checksum_manifest_url(archive_url: &str) -> String {
    if archive_override().is_some() {
        format!("{archive_url}.sha256")
    } else {
        format!("{}/SHA256SUMS", release_base_url())
    }
}

fn local_checksum_manifest_path(local_archive: &Path) -> PathBuf {
    let sidecar = checksum_sidecar_path(local_archive);
    if sidecar.is_file() {
        return sidecar;
    }

    if let Some(parent) = local_archive.parent() {
        let manifest = parent.join("SHA256SUMS");
        if manifest.is_file() {
            return manifest;
        }
    }

    panic!(
        "Local JavaScriptCore archive {} requires checksum sidecar {} or SHA256SUMS in the same directory",
        local_archive.display(),
        sidecar.display()
    );
}

fn checksum_sidecar_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(".sha256");
    PathBuf::from(value)
}

fn copy_file_atomic(source: &Path, destination: &Path, label: &str) {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|error| {
            panic!(
                "Failed to create {} output directory {}: {error}",
                label,
                parent.display()
            )
        });
    }

    let part_path = {
        let mut value = destination.as_os_str().to_os_string();
        value.push(".part");
        PathBuf::from(value)
    };

    fs::copy(source, &part_path).unwrap_or_else(|error| {
        panic!(
            "Failed to copy {label} {} to {}: {error}",
            source.display(),
            destination.display()
        )
    });
    fs::rename(&part_path, destination).unwrap_or_else(|error| {
        panic!(
            "Failed to atomically install {label} {}: {error}",
            destination.display()
        )
    });
}

fn local_archive_path(value: &str) -> Option<PathBuf> {
    if value.starts_with("http://") || value.starts_with("https://") {
        return None;
    }

    let path = value
        .strip_prefix("file://")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(value));

    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

fn verify_archive_checksum(
    archive_path: &Path,
    manifest_path: &Path,
    archive_filename: &str,
    archive_url: &str,
) {
    let expected = expected_sha256(manifest_path, archive_filename, archive_url);
    let actual = sha256_file(archive_path);

    if actual != expected {
        panic!(
            "JavaScriptCore archive checksum mismatch for {}\nexpected: {}\nactual:   {}\nmanifest: {}",
            archive_path.display(),
            expected,
            actual,
            manifest_path.display()
        );
    }
}

fn expected_sha256(
    manifest_path: &Path,
    archive_filename: &str,
    archive_url: &str,
) -> String {
    let contents = fs::read_to_string(manifest_path).unwrap_or_else(|error| {
        panic!(
            "Failed to read JavaScriptCore checksum manifest {}: {error}",
            manifest_path.display()
        )
    });

    let archive_url_filename = archive_url
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(archive_filename);
    let candidates = [archive_filename, archive_url_filename];
    let mut entries = Vec::new();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if is_sha256_hex(line) {
            entries.push((line.to_ascii_lowercase(), String::new()));
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(hash) = parts.next() else {
            continue;
        };
        if !is_sha256_hex(hash) {
            continue;
        }
        let path = parts.next().unwrap_or_default().trim_start_matches('*');
        entries.push((hash.to_ascii_lowercase(), path.to_string()));
    }

    for (hash, path) in &entries {
        let file_name = Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(path);
        if candidates.iter().any(|candidate| *candidate == file_name) {
            return hash.clone();
        }
    }

    if entries.len() == 1 {
        return entries[0].0.clone();
    }

    panic!(
        "JavaScriptCore checksum manifest {} does not contain an entry for {}",
        manifest_path.display(),
        archive_filename
    );
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_file(path: &Path) -> String {
    if let Some(hash) = sha256_with_command("sha256sum", &[path]) {
        return hash;
    }

    if let Some(hash) =
        sha256_with_command("shasum", &[Path::new("-a"), Path::new("256"), path])
    {
        return hash;
    }

    if let Some(hash) = sha256_with_command(
        "openssl",
        &[
            Path::new("dgst"),
            Path::new("-sha256"),
            Path::new("-r"),
            path,
        ],
    ) {
        return hash;
    }

    panic!(
        "Could not compute SHA-256 for {}. Install sha256sum, shasum, or openssl.",
        path.display()
    );
}

fn sha256_with_command(command: &str, args: &[&Path]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .split_whitespace()
        .find(|part| is_sha256_hex(part))
        .map(|part| part.to_ascii_lowercase())
}

fn extract_static_lib(archive_path: &Path, output_path: &Path) {
    validate_archive_paths(archive_path);

    fs::create_dir_all(output_path).unwrap_or_else(|error| {
        panic!(
            "Failed to create static library output directory {}: {error}",
            output_path.display()
        )
    });

    let output = Command::new("tar")
        .arg("-xzf")
        .arg(archive_path)
        .arg("-C")
        .arg(output_path)
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "Failed to extract JavaScriptCore archive {}: {error}",
                archive_path.display()
            )
        });

    if !output.status.success() {
        panic!(
            "Failed to extract JavaScriptCore archive {}\nstdout:\n{}\nstderr:\n{}",
            archive_path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn validate_archive_paths(archive_path: &Path) {
    let output = Command::new("tar")
        .arg("-tzf")
        .arg(archive_path)
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "Failed to list JavaScriptCore archive {} before extraction: {error}",
                archive_path.display()
            )
        });

    if !output.status.success() {
        panic!(
            "Failed to list JavaScriptCore archive {}\nstdout:\n{}\nstderr:\n{}",
            archive_path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let listing = String::from_utf8_lossy(&output.stdout);
    let mut entries = 0usize;
    for entry in listing.lines() {
        entries += 1;
        validate_archive_entry_path(entry, archive_path);
    }

    if entries == 0 {
        panic!("JavaScriptCore archive {} is empty", archive_path.display());
    }
}

fn validate_archive_entry_path(entry: &str, archive_path: &Path) {
    if entry.is_empty() {
        panic!(
            "JavaScriptCore archive {} contains an empty path",
            archive_path.display()
        );
    }

    let path = Path::new(entry);
    if path.is_absolute() {
        panic!(
            "JavaScriptCore archive {} contains unsafe absolute path: {entry}",
            archive_path.display()
        );
    }

    for component in path.components() {
        use std::path::Component;
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                panic!(
                    "JavaScriptCore archive {} contains unsafe path component: {entry}",
                    archive_path.display()
                );
            }
        }
    }
}

fn build_jsc_from_source() -> PathBuf {
    let webkit_dir = env_path("RUST_JSC_WEBKIT_DIR").unwrap_or_else(default_webkit_dir);
    let profile = source_build_profile();
    let build_dir = env_path("RUST_JSC_BUILD_DIR").unwrap_or_else(|| {
        webkit_dir
            .join("WebKitBuild")
            .join("RustJSC")
            .join("JSCOnly")
            .join(format!("{profile}-Static"))
    });

    let force_configure = env_truthy("RUST_JSC_FORCE_CMAKE_CONFIGURE");
    let force_build = env_truthy("RUST_JSC_FORCE_SOURCE_BUILD");

    if force_configure || !build_dir.join("CMakeCache.txt").exists() {
        configure_jsc(&webkit_dir, &build_dir, &profile);
    }

    if force_build || find_static_lib_dir(&build_dir).is_none() {
        build_jsc_targets(&build_dir);
    }

    let lib_dir = find_static_lib_dir(&build_dir).unwrap_or_else(|| {
        panic!(
            "Source build completed but static JavaScriptCore archives were not found under {}",
            build_dir.display()
        )
    });

    ensure_jsc_jit_static_archive(&lib_dir);

    if let Some(archive_out) = env_path("RUST_JSC_ARCHIVE_OUT") {
        archive_static_libs(&lib_dir, &archive_out);
    }

    lib_dir
}

fn source_build_profile() -> String {
    let profile = env::var("RUST_JSC_BUILD_PROFILE").unwrap_or_else(|_| "Release".into());
    match profile.as_str() {
        "Release" | "Debug" => profile,
        other => {
            panic!("Unsupported RUST_JSC_BUILD_PROFILE={other}. Use Release or Debug.")
        }
    }
}

fn default_webkit_dir() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .expect("sys crate should have a parent directory")
        .join("WebKit")
}

fn configure_jsc(webkit_dir: &Path, build_dir: &Path, profile: &str) {
    if !command_exists("ninja") && !command_exists("ninja-build") {
        panic!(
            "RUST_JSC_BUILD_MODE=source requires Ninja. Install ninja or ninja-build."
        );
    }

    if let Ok(generator) = env::var("RUST_JSC_CMAKE_GENERATOR") {
        if generator != "Ninja" {
            warn("RUST_JSC_CMAKE_GENERATOR is deprecated and ignored; source builds always use Ninja");
        }
    }

    if env::var_os("RUST_JSC_STATIC").is_some() {
        warn("RUST_JSC_STATIC is deprecated and ignored; source builds are static by default");
    }

    let mut args = vec![
        "-S".into(),
        webkit_dir.display().to_string(),
        "-B".into(),
        build_dir.display().to_string(),
        "-G".into(),
        "Ninja".into(),
    ];

    args.extend([
        "-DPORT=JSCOnly".into(),
        format!("-DCMAKE_BUILD_TYPE={profile}"),
        "-DSHOW_BINDINGS_GENERATION_PROGRESS=1".into(),
        "-DDEVELOPER_MODE=ON".into(),
        "-DENABLE_REMOTE_INSPECTOR=ON".into(),
        "-DENABLE_FTL_JIT=ON".into(),
        "-DENABLE_STATIC_JSC=ON".into(),
        "-DUSE_THIN_ARCHIVES=OFF".into(),
    ]);

    if let Ok(extra_args) = env::var("RUST_JSC_CMAKE_ARGS") {
        args.extend(extra_args.split_whitespace().map(String::from));
    }

    run_command("cmake", &args, None);
}

fn build_jsc_targets(build_dir: &Path) {
    let jobs = env::var("RUST_JSC_JOBS").unwrap_or_else(|_| default_jobs());

    run_command(
        "cmake",
        &[
            "--build".into(),
            build_dir.display().to_string(),
            "--target".into(),
            "jsc".into(),
            "--parallel".into(),
            jobs,
        ],
        None,
    );
}

fn default_jobs() -> String {
    std::thread::available_parallelism()
        .map(|jobs| jobs.get().to_string())
        .unwrap_or_else(|_| "4".into())
}

fn command_exists(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn run_command(command: &str, args: &[String], cwd: Option<&Path>) {
    let mut cmd = Command::new(command);
    cmd.args(args);
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    let output = cmd
        .output()
        .unwrap_or_else(|error| panic!("Failed to run {command}: {error}"));

    if !output.status.success() {
        panic!(
            "Command failed: {command} {}\nstdout:\n{}\nstderr:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn find_framework_parent(path: &Path) -> Option<PathBuf> {
    if path.join("JavaScriptCore.framework").is_dir() {
        return Some(path.to_path_buf());
    }

    if path
        .file_name()
        .is_some_and(|name| name == "JavaScriptCore.framework")
    {
        return path.parent().map(Path::to_path_buf);
    }

    let lib_dir = path.join("lib");
    if lib_dir.join("JavaScriptCore.framework").is_dir() {
        return Some(lib_dir);
    }

    None
}

fn find_static_lib_dir(path: &Path) -> Option<PathBuf> {
    [path, &path.join("lib")]
        .into_iter()
        .find(|candidate| static_libs_exist(candidate))
        .map(Path::to_path_buf)
}

fn static_libs_exist(path: &Path) -> bool {
    ["libJavaScriptCore.a", "libWTF.a", "libbmalloc.a"]
        .iter()
        .all(|lib| path.join(lib).exists())
}

fn link_framework(framework_parent: &Path) {
    println!(
        "cargo:rustc-link-search=framework={}",
        framework_parent.display()
    );
    println!("cargo:rustc-link-lib=framework=JavaScriptCore");
    println!(
        "cargo:rustc-link-arg=-Wl,-rpath,{}",
        framework_parent.display()
    );
    link_common_system_libs();
}

fn link_static_libs(lib_dir: &Path) {
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=JavaScriptCore");
    if ensure_jsc_jit_static_archive(lib_dir) {
        println!("cargo:rustc-link-lib=static=JavaScriptCoreJIT");
    }
    println!("cargo:rustc-link-lib=static=WTF");
    println!("cargo:rustc-link-lib=static=bmalloc");
    link_platform_static_dependencies(lib_dir);
}

fn ensure_jsc_jit_static_archive(lib_dir: &Path) -> bool {
    let archive_path = lib_dir.join("libJavaScriptCoreJIT.a");
    if archive_path.exists() {
        return true;
    }

    let Some(build_dir) = lib_dir.parent() else {
        return false;
    };
    let jit_object_dir = build_dir
        .join("Source")
        .join("JavaScriptCore")
        .join("CMakeFiles")
        .join("JavaScriptCoreJIT.dir");

    if !jit_object_dir.is_dir() {
        return false;
    }

    let mut object_files = Vec::new();
    collect_object_files(&jit_object_dir, &mut object_files);
    object_files.sort();

    if object_files.is_empty() {
        return false;
    }

    let ar = env::var("AR").unwrap_or_else(|_| "ar".into());
    let mut command = Command::new(ar);
    command.arg("rcs").arg(&archive_path).args(&object_files);

    let output = command.output().unwrap_or_else(|error| {
        panic!(
            "Failed to create {} from JavaScriptCoreJIT objects: {error}",
            archive_path.display()
        )
    });

    if !output.status.success() {
        panic!(
            "Failed to create {}\nstdout:\n{}\nstderr:\n{}",
            archive_path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    true
}

fn collect_object_files(dir: &Path, object_files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("Failed to read {}: {error}", dir.display()));

    for entry in entries {
        let entry = entry.unwrap_or_else(|error| {
            panic!("Failed to read entry under {}: {error}", dir.display())
        });
        let path = entry.path();
        if path.is_dir() {
            collect_object_files(&path, object_files);
        } else if path.extension().is_some_and(|extension| extension == "o") {
            object_files.push(path);
        }
    }
}

fn archive_static_libs(lib_dir: &Path, archive_out: &Path) {
    if package_static_libs_with_script(lib_dir, archive_out) {
        return;
    }

    if let Some(parent) = archive_out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).unwrap_or_else(|error| {
                panic!(
                    "Failed to create archive output directory {}: {error}",
                    parent.display()
                )
            });
        }
    }

    let mut libs = fs::read_dir(lib_dir)
        .unwrap_or_else(|error| panic!("Failed to read {}: {error}", lib_dir.display()))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "a") {
                path.file_name().map(|name| name.to_os_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    libs.sort();

    if libs.is_empty() {
        panic!(
            "No static libraries found to archive under {}",
            lib_dir.display()
        );
    }

    let output = Command::new("tar")
        .arg("-czf")
        .arg(archive_out)
        .arg("-C")
        .arg(lib_dir)
        .args(&libs)
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "Failed to create JavaScriptCore archive {}: {error}",
                archive_out.display()
            )
        });

    if !output.status.success() {
        panic!(
            "Failed to create JavaScriptCore archive {}\nstdout:\n{}\nstderr:\n{}",
            archive_out.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn package_static_libs_with_script(lib_dir: &Path, archive_out: &Path) -> bool {
    let repo_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .expect("sys crate should have a parent directory")
        .to_path_buf();
    let script = repo_root.join("scripts").join("package_jsc_archive.py");
    if !script.is_file() {
        return false;
    }

    let mut command = Command::new("python3");
    command
        .arg(&script)
        .arg("--lib-dir")
        .arg(lib_dir)
        .arg("--archive-out")
        .arg(archive_out)
        .arg("--target-triple")
        .arg(target_triple_name())
        .arg("--repo-root")
        .arg(&repo_root)
        .arg("--webkit-dir")
        .arg(default_webkit_dir());

    if let Some(build_dir) = lib_dir.parent() {
        command.arg("--build-dir").arg(build_dir);
    }

    let output = command.output().unwrap_or_else(|error| {
        panic!(
            "Failed to run deterministic JavaScriptCore archive packager {}: {error}",
            script.display()
        )
    });

    if !output.status.success() {
        panic!(
            "Failed to package deterministic JavaScriptCore archive {}\nstdout:\n{}\nstderr:\n{}",
            archive_out.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    true
}

fn link_platform_static_dependencies(lib_dir: &Path) {
    if target_os() == "macos" {
        let lib_dir = env::var("SYSTEM_LIBS_PATH").unwrap_or_else(|_| "/usr/lib".into());
        println!("cargo:rustc-link-search={lib_dir}");
        link_common_system_libs();
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=dylib=objc");
        return;
    }

    if target_env() != "musl" && !static_system_libs_exist(lib_dir) {
        link_common_system_libs();
        println!("cargo:rustc-link-lib=dylib=icui18n");
        println!("cargo:rustc-link-lib=dylib=icuuc");
        println!("cargo:rustc-link-lib=dylib=icudata");
        println!("cargo:rustc-link-lib=dylib=atomic");
        return;
    }

    println!("cargo:rustc-link-lib=static=stdc++");
    println!("cargo:rustc-link-lib=static=icui18n");
    println!("cargo:rustc-link-lib=static=icuuc");
    println!("cargo:rustc-link-lib=static=icudata");
    println!("cargo:rustc-link-lib=static=atomic");

    if target_env() == "musl" && target_arch() == "aarch64" {
        println!("cargo:rustc-link-lib=gcc");
    }
}

fn static_system_libs_exist(lib_dir: &Path) -> bool {
    [
        "libstdc++.a",
        "libicui18n.a",
        "libicuuc.a",
        "libicudata.a",
        "libatomic.a",
    ]
    .iter()
    .all(|lib| lib_dir.join(lib).exists())
}

fn link_common_system_libs() {
    if target_os() == "macos" {
        println!("cargo:rustc-link-lib=dylib=c++");
        println!("cargo:rustc-link-lib=dylib=m");
        println!("cargo:rustc-link-lib=dylib=dl");
        println!("cargo:rustc-link-lib=dylib=icucore");
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
        println!("cargo:rustc-link-lib=dylib=m");
        println!("cargo:rustc-link-lib=dylib=dl");
        println!("cargo:rustc-link-lib=dylib=pthread");
    }
}

fn link_system_jsc() {
    if target_os() == "macos" {
        if let Some(framework_parent) = env_path("RUST_JSC_FRAMEWORK_PATH")
            .and_then(|path| find_framework_parent(&path))
        {
            link_framework(&framework_parent);
            warn("RUST_JSC_BUILD_MODE=system linked a framework path; stock JavaScriptCore usually lacks rust-jsc fork-only APIs");
            return;
        }
    }

    if let Some(path) = env_path("RUST_JSC_SYSTEM_LIBS_PATH") {
        println!("cargo:rustc-link-search=native={}", path.display());
    }

    let lib_name =
        env::var("RUST_JSC_SYSTEM_LIB_NAME").unwrap_or_else(|_| "JavaScriptCore".into());

    if target_os() == "macos" && lib_name == "JavaScriptCore" {
        println!("cargo:rustc-link-lib=framework=JavaScriptCore");
    } else {
        println!("cargo:rustc-link-lib=dylib={lib_name}");
    }

    link_common_system_libs();
    println!(
        "cargo:warning=RUST_JSC_BUILD_MODE=system may not provide rust-jsc's fork-only JavaScriptCore APIs"
    );
}
