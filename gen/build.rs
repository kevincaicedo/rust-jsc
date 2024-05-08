extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    // Path to the JavaScriptCore headers
    let jsc_headers_path = env::var("JAVASCRIPTCORE_HEADERS_PATH").unwrap();
    // let framework_dir = env::var("CUSTOM_BUILD_FRAMEWORK_PATH").unwrap();
    let system_frameworks_path = env::var("SYSTEM_FRAMEWORKS_PATH").unwrap();
    let core_foundation_headers_path = env::var("CORE_FOUNDATION_HEADERS_PATH").unwrap();

    let bindings = bindgen::Builder::default()
        // Specify the JavaScriptCore header files
        .header(format!("{}/JavaScriptCore.h", jsc_headers_path))
        .clang_arg(format!("-I{}", jsc_headers_path))
        .clang_arg("-DWEBKIT_DIRECTORIES")
        .clang_arg(format!("-F{}", system_frameworks_path))
        // .clang_arg(format!("-F{}", framework_dir))
        .clang_arg(format!("-I{}", core_foundation_headers_path))
        .clang_arg("-DJS_EXPORT_PRIVATE=1")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from("../sys/src");
    bindings
        .write_to_file(out_path.join("lib.rs"))
        .expect("Couldn't write bindings!");

    // Path to the build JavaScriptCore framework
    // println!("cargo:rustc-link-search=framework={}", framework_dir);
    // println!("cargo:rustc-link-lib=framework=JavaScriptCore");
}
