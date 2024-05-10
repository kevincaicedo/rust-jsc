use std::env;
extern crate pkg_config;

#[cfg(target_os = "macos")]
#[cfg(feature = "patches")]
fn main() {
    let webkit_path = env::var("WEBKIT_PATH").unwrap();
    let lib_dir = format!("{}/WebKitBuild/JSCOnly/Release/lib", webkit_path);

    println!("cargo:rustc-link-lib=dylib=c++");
    println!("cargo:rustc-link-lib=dylib=m");
    println!("cargo:rustc-link-lib=dylib=dl");

    println!("cargo:rustc-link-lib=dylib=icucore.A");
    println!("cargo:rustc-link-search=/usr/lib");
    // println!("cargo:rustc-link-lib=dylib=icui18n");
    // println!("cargo:rustc-link-lib=dylib=icuuc");
    // println!("cargo:rustc-link-lib=dylib=icudata");
    // println!("cargo:rustc-link-lib=dylib=atomic");

    println!("cargo:rustc-link-search=native={}", lib_dir);
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
fn main() {
    let lib_path = env::var("JSC_LIBS_PATH").unwrap();
    println!("cargo:rustc-link-search=native={}", lib_path);
    
    // dylib
    println!("cargo:rustc-link-lib=static=stdc++");
    // println!("cargo:rustc-link-lib=static=mvec");
    
    println!("cargo:rustc-link-lib=static=icui18n");
    println!("cargo:rustc-link-lib=static=icuuc");
    println!("cargo:rustc-link-lib=static=icudata");
    println!("cargo:rustc-link-lib=static=atomic");

    println!("cargo:rustc-link-lib=static=JavaScriptCore");
    println!("cargo:rustc-link-lib=static=WTF");
    println!("cargo:rustc-link-lib=static=bmalloc");
}
