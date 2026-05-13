use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BuildMode {
    Auto,
    Download,
    Framework,
    Source,
    Static,
    System,
}

fn main() {
    if env::var_os("DOCS_RS").is_some() {
        return;
    }

    emit_rerun_directives();
    check_supported_platform();

    match build_mode() {
        BuildMode::Auto => setup_auto(),
        BuildMode::Download => {
            let lib_dir = setup_downloaded_static_libs();
            link_static_libs(&lib_dir);
        }
        BuildMode::Framework => {
            let path = framework_path_from_env();
            link_framework(&path);
        }
        BuildMode::Source => {
            let build_dir = build_jsc_from_source();
            link_from_build_path(&build_dir);
        }
        BuildMode::Static => {
            let lib_dir = env_path("RUST_JSC_CUSTOM_BUILD_PATH")
                .unwrap_or_else(setup_downloaded_static_libs);
            link_static_libs(&lib_dir);
        }
        BuildMode::System => link_system_jsc(),
    }
}

fn build_mode() -> BuildMode {
    if env_truthy("RUST_JSC_FROM_SOURCE") {
        return BuildMode::Source;
    }

    if env::var_os("RUST_JSC_FRAMEWORK_PATH").is_some() {
        return BuildMode::Framework;
    }

    match env::var("RUST_JSC_BUILD_MODE")
        .unwrap_or_else(|_| "auto".into())
        .to_ascii_lowercase()
        .as_str()
    {
        "auto" | "custom" => BuildMode::Auto,
        "download" | "archive" => BuildMode::Download,
        "framework" => BuildMode::Framework,
        "source" => BuildMode::Source,
        "static" => BuildMode::Static,
        "system" => BuildMode::System,
        other => panic!(
            "Unsupported RUST_JSC_BUILD_MODE={other}. Use auto, download, framework, source, static, or system."
        ),
    }
}

fn setup_auto() {
    if let Some(custom_path) = env_path("RUST_JSC_CUSTOM_BUILD_PATH") {
        link_from_build_path(&custom_path);
        return;
    }

    let lib_dir = setup_downloaded_static_libs();
    link_static_libs(&lib_dir);
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
        "RUST_JSC_CUSTOM_ARCHIVE",
        "RUST_JSC_CUSTOM_BUILD_PATH",
        "RUST_JSC_FRAMEWORK_PATH",
        "RUST_JSC_FROM_SOURCE",
        "RUST_JSC_WEBKIT_DIR",
        "RUST_JSC_BUILD_DIR",
        "RUST_JSC_BUILD_PROFILE",
        "RUST_JSC_CMAKE_GENERATOR",
        "RUST_JSC_JOBS",
        "RUST_JSC_STATIC",
        "RUST_JSC_FORCE_SOURCE_BUILD",
        "RUST_JSC_FORCE_CMAKE_CONFIGURE",
        "RUST_JSC_MIRROR",
        "RUST_JSC_SYSTEM_LIBS_PATH",
        "RUST_JSC_SYSTEM_LIB_NAME",
        "SYSTEM_LIBS_PATH",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
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

fn static_lib_file() -> String {
    let target_arch = target_arch();
    let target_os = target_os();
    let target_env = target_env();
    let platform = match (
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
    };
    format!("libjsc-{platform}.a.gz")
}

fn static_lib_url() -> String {
    if let Ok(custom_archive) = env::var("RUST_JSC_CUSTOM_ARCHIVE") {
        return custom_archive;
    }

    let default_base = "https://github.com/kevincaicedo/rust-jsc/releases/download";
    let base = env::var("RUST_JSC_MIRROR").unwrap_or_else(|_| default_base.into());
    let version = env::var("CARGO_PKG_VERSION").unwrap();

    format!("{base}/sys-v{version}/{}", static_lib_file())
}

fn downloaded_lib_dir() -> PathBuf {
    PathBuf::from(env::var("OUT_DIR").unwrap())
        .join(env::var("CARGO_PKG_VERSION").unwrap())
}

fn setup_downloaded_static_libs() -> PathBuf {
    let output_path = downloaded_lib_dir();
    let filename = static_lib_file();
    let archive_path = output_path.join(&filename);

    if !archive_path.exists() {
        fetch_static_lib(&output_path, &filename);
    }

    if !static_libs_exist(&output_path) {
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

fn fetch_static_lib(output_path: &Path, filename: &str) {
    let url = static_lib_url();

    let output = Command::new("python3")
        .arg("scripts/download_file.py")
        .arg(url.clone())
        .arg(output_path)
        .arg(filename)
        .output()
        .unwrap_or_else(|error| {
            panic!("Failed to download static library from {url}: {error}")
        });

    if !output.status.success() {
        panic!(
            "Failed to download static library from {url}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn extract_static_lib(archive_path: &Path, output_path: &Path) {
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

fn build_jsc_from_source() -> PathBuf {
    let webkit_dir = env_path("RUST_JSC_WEBKIT_DIR").unwrap_or_else(default_webkit_dir);
    let profile = env::var("RUST_JSC_BUILD_PROFILE").unwrap_or_else(|_| "Release".into());
    let build_dir = env_path("RUST_JSC_BUILD_DIR").unwrap_or_else(|| {
        webkit_dir
            .join("WebKitBuild")
            .join("RustJSC")
            .join("JSCOnly")
            .join(&profile)
    });

    let generator = env::var("RUST_JSC_CMAKE_GENERATOR")
        .ok()
        .or_else(|| command_exists("ninja").then(|| "Ninja".into()));

    let force_configure = env_truthy("RUST_JSC_FORCE_CMAKE_CONFIGURE");
    let force_build = env_truthy("RUST_JSC_FORCE_SOURCE_BUILD");

    if force_configure || !build_dir.join("CMakeCache.txt").exists() {
        configure_jsc(&webkit_dir, &build_dir, &profile, generator.as_deref());
    }

    if force_build || !has_jsc_artifact(&build_dir) {
        build_jsc_targets(&build_dir, generator.as_deref());
    }

    build_dir
}

fn default_webkit_dir() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .expect("sys crate should have a parent directory")
        .join("WebKit")
}

fn configure_jsc(
    webkit_dir: &Path,
    build_dir: &Path,
    profile: &str,
    generator: Option<&str>,
) {
    let mut args = vec![
        "-S".into(),
        webkit_dir.display().to_string(),
        "-B".into(),
        build_dir.display().to_string(),
    ];

    if let Some(generator) = generator {
        args.push("-G".into());
        args.push(generator.into());
    }

    args.extend([
        "-DPORT=JSCOnly".into(),
        format!("-DCMAKE_BUILD_TYPE={profile}"),
        "-DSHOW_BINDINGS_GENERATION_PROGRESS=1".into(),
        "-DDEVELOPER_MODE=ON".into(),
        "-DENABLE_REMOTE_INSPECTOR=ON".into(),
        "-DENABLE_FTL_JIT=ON".into(),
    ]);

    if env_truthy("RUST_JSC_STATIC") {
        args.push("-DENABLE_STATIC_JSC=ON".into());
        args.push("-DUSE_THIN_ARCHIVES=OFF".into());
    }

    run_command("cmake", &args, None);
}

fn build_jsc_targets(build_dir: &Path, generator: Option<&str>) {
    let jobs = env::var("RUST_JSC_JOBS").unwrap_or_else(|_| default_jobs());

    if generator != Some("Ninja") {
        run_command(
            "cmake",
            &[
                "--build".into(),
                build_dir.display().to_string(),
                "--target".into(),
                "JavaScriptCoreJIT".into(),
                "--parallel".into(),
                jobs.clone(),
            ],
            None,
        );
    }

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

fn link_from_build_path(path: &Path) {
    if target_os() == "macos" {
        if let Some(framework_parent) = find_framework_parent(path) {
            link_framework(&framework_parent);
            return;
        }
    }

    if let Some(static_dir) = find_static_lib_dir(path) {
        link_static_libs(&static_dir);
        return;
    }

    if let Some(dylib_dir) = find_dylib_dir(path) {
        link_dylibs(&dylib_dir);
        return;
    }

    panic!(
        "Could not find JavaScriptCore artifacts under {}. Expected JavaScriptCore.framework, libJavaScriptCore.a, or libJavaScriptCore.*.",
        path.display()
    );
}

fn framework_path_from_env() -> PathBuf {
    env_path("RUST_JSC_FRAMEWORK_PATH")
        .or_else(|| env_path("RUST_JSC_CUSTOM_BUILD_PATH"))
        .and_then(|path| find_framework_parent(&path))
        .unwrap_or_else(|| {
            panic!(
                "RUST_JSC_BUILD_MODE=framework requires RUST_JSC_FRAMEWORK_PATH or RUST_JSC_CUSTOM_BUILD_PATH pointing to JavaScriptCore.framework or its parent directory"
            )
        })
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

fn has_jsc_artifact(path: &Path) -> bool {
    find_framework_parent(path).is_some()
        || find_static_lib_dir(path).is_some()
        || find_dylib_dir(path).is_some()
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

fn find_dylib_dir(path: &Path) -> Option<PathBuf> {
    [path, &path.join("lib")]
        .into_iter()
        .find(|candidate| dynamic_jsc_exists(candidate))
        .map(Path::to_path_buf)
}

fn dynamic_jsc_exists(path: &Path) -> bool {
    if target_os() == "macos" {
        path.join("libJavaScriptCore.dylib").exists()
    } else {
        path.join("libJavaScriptCore.so").exists()
    }
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

fn link_dylibs(lib_dir: &Path) {
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    link_common_system_libs();
    println!("cargo:rustc-link-lib=dylib=JavaScriptCore");

    if lib_dir.join(dynamic_lib_name("WTF")).exists() {
        println!("cargo:rustc-link-lib=dylib=WTF");
    }
    if lib_dir.join(dynamic_lib_name("bmalloc")).exists() {
        println!("cargo:rustc-link-lib=dylib=bmalloc");
    }
}

fn dynamic_lib_name(name: &str) -> String {
    if target_os() == "macos" {
        format!("lib{name}.dylib")
    } else {
        format!("lib{name}.so")
    }
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
