# Generate binding for JavaScriptCore
[![Crates.io](https://img.shields.io/crates/v/rust-jsc.svg)](https://crates.io/crates/rust-jsc)
[![Docs.rs](https://docs.rs/rust-jsc/badge.svg)](https://docs.rs/rust-jsc)

Rust-JSC is a Rust library that provides a low-level binding for the JavaScriptCore engine. It allows you to interact with JavaScript code from your Rust applications.

## MacOS
before running the following command, make sure you have the following environment variables set:

```bash
# JAVASCRIPTCORE_HEADERS_PATH: Path to the JavaScriptCore headers e.g: /Users/${user}/Documents/Projects/WebKit/Source/JavaScriptCore/API
JAVASCRIPTCORE_HEADERS_PATH = "/Users/${user}/Documents/Projects/WebKit/Source/JavaScriptCore/API"

# CORE_FOUNDATION_HEADERS_PATH: Path to the CoreFoundation headers e.g: /Library/Developer/CommandLineTools/SDKs/MacOSX14.4.sdk/System/Library/Frameworks/CoreFoundation.framework/Versions/A/Headers
CORE_FOUNDATION_HEADERS_PATH = "/Library/Developer/CommandLineTools/SDKs/MacOSX14.4.sdk/System/Library/Frameworks/CoreFoundation.framework/Versions/A/Headers"

# SYSTEM_FRAMEWORKS_PATH: Path to the System frameworks e.g: /Library/Developer/CommandLineTools/SDKs/MacOSX14.4.sdk/System/Library/Frameworks/
SYSTEM_FRAMEWORKS_PATH = "/Library/Developer/CommandLineTools/SDKs/MacOSX14.4.sdk/System/Library/Frameworks/"
```
