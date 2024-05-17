use std::env;
extern crate pkg_config;

fn check_supported_platform() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    if target_os != "linux" && target_os != "macos" {
        panic!("Unsupported target OS: {}", target_os);
    }

    if target_arch != "x86_64" && target_arch != "aarch64" {
        panic!("Unsupported target architecture: {}", target_arch);
    }
}

fn static_lib_file() -> String {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let platform = match (target_os.as_ref(), target_arch.as_ref()) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        // TODO: Support windows
        // ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        // ("windows", "i686") => "i686-pc-windows-msvc",
        _ => panic!("Unsupported target OS or architecture: {}-{}", target_os, target_arch),
    };
    format!("libjsc-{}.a.gz", platform)
}

fn static_lib_url() -> String {
    if let Ok(custom_archive) = env::var("RUST_JSC_CUSTOM_ARCHIVE") {
        return custom_archive;
    }

    check_supported_platform();

    let default_base = "https://github.com/kevincaicedo/rust-jsc/releases/download";
    let base = env::var("RUST_JSC_MIRROR").unwrap_or_else(|_| default_base.into());
    let version = env::var("CARGO_PKG_VERSION").unwrap();

    let platform_file = static_lib_file();
    format!("{}/v{}/{}", base, version, platform_file)
}

// use a python script that receives the URL and downloads the file also passing the output path
fn fetch_static_lib() {
    let url = static_lib_url();
    let output_path = env::var("OUT_DIR").unwrap();
    let filename = static_lib_file();
    let version = env::var("CARGO_PKG_VERSION").unwrap();

    let output_path = format!("{}/{}", output_path, version);

    let output = std::process::Command::new("python3")
        .arg("scripts/download_file.py")
        .arg(url.clone())
        .arg(output_path)
        .arg(filename)
        .output();

    if let Err(e) = output {
        // panic and show the error and url
        panic!("Failed to download static library: {}\n{}", e, url);
    }

    let output = output.unwrap();
    if !output.status.success() {
        panic!("Failed to download static library: {:?}", output);
    }
}

fn extract_static_lib() {
    let output_path = env::var("OUT_DIR").unwrap();
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let output_path = format!("{}/{}", output_path, version);
    let filename = static_lib_file();

    let output = std::process::Command::new("tar")
        .arg("-xvf")
        .arg(format!("{}/{}", output_path, filename))
        .arg("-C")
        .arg(output_path)
        .output()
        .expect("Failed to extract static library");

    if !output.status.success() {
        panic!("Failed to extract static library: {:?}", output);
    }
}

#[cfg(target_os = "macos")]
#[cfg(feature = "patches")]
fn main() {
    // if custom path for the static lib is set use it, otherwise download the static lib
    if let Ok(custom_build_path) = env::var("RUST_JSC_CUSTOM_BUILD_PATH") {
        println!("cargo:rustc-link-search=native={}", custom_build_path);
    } else {
        let output_path = env::var("OUT_DIR").unwrap();
        let version = env::var("CARGO_PKG_VERSION").unwrap();
        let output_path = format!("{}/{}", output_path, version);
        let static_lib_file = static_lib_file();

        // if archive file is not found in outdir, download it
        if !std::path::Path::new(&format!("{}/{}", output_path, static_lib_file)).exists() {
            fetch_static_lib();
            extract_static_lib();
        }

        // set search native path to the output directory
        println!("cargo:rustc-link-search=native={}", output_path);
    }

    let lib_dir = env::var("SYSTEM_LIBS_PATH").unwrap_or_else(|_| "/usr/lib".into());
    println!("cargo:rustc-link-search={}", lib_dir);

    // dylib
    println!("cargo:rustc-link-lib=dylib=c++");
    println!("cargo:rustc-link-lib=dylib=m");
    println!("cargo:rustc-link-lib=dylib=dl");
    println!("cargo:rustc-link-lib=dylib=icucore");

    // static libs
    println!("cargo:rustc-link-lib=static=JavaScriptCore");
    println!("cargo:rustc-link-lib=static=WTF");
    println!("cargo:rustc-link-lib=static=bmalloc");
}

// is macOS and not using custom JavaScriptCore framework
#[cfg(target_os = "macos")]
#[cfg(not(feature = "patches"))]
fn main() {
    println!("cargo:rustc-link-lib=framework=JavaScriptCore");
}

#[cfg(target_os = "linux")]
#[cfg(feature = "patches")]
fn main() {
    // if custom path for the static lib is set use it, otherwise download the static lib
    if let Ok(custom_build_path) = env::var("RUST_JSC_CUSTOM_BUILD_PATH") {
        println!("cargo:rustc-link-search=native={}", custom_build_path);
    } else {
        let output_path = env::var("OUT_DIR").unwrap();
        let version = env::var("CARGO_PKG_VERSION").unwrap();
        let output_path = format!("{}/{}", output_path, version);
        let static_lib_file = static_lib_file();

        // if archive file is not found in outdir, download it
        if !std::path::Path::new(&format!("{}/{}", output_path, static_lib_file)).exists()
        {
            fetch_static_lib();
            extract_static_lib();
        }

        // set search native path to the output directory
        println!("cargo:rustc-link-search=native={}", output_path);
    }

    // static libs
    println!("cargo:rustc-link-lib=static=stdc++");
    println!("cargo:rustc-link-lib=static=icui18n");
    println!("cargo:rustc-link-lib=static=icuuc");
    println!("cargo:rustc-link-lib=static=icudata");
    println!("cargo:rustc-link-lib=static=atomic");

    println!("cargo:rustc-link-lib=static=JavaScriptCore");
    println!("cargo:rustc-link-lib=static=WTF");
    println!("cargo:rustc-link-lib=static=bmalloc");
}

#[cfg(target_os = "linux")]
#[cfg(not(feature = "patches"))]
fn main() {
    pkg_config::probe_library("javascriptcoregtk-6.0").unwrap();
}
