# rust-jsc-sys

This crate provides the raw bindings to the JavaScriptCore library. The bindings are generated using the `bindgen` crate. 
This crate for now only supports macOS and linux. and use a custom version of [WebKit](https://github.com/kevincaicedo/Kedo-WebKit) to generate the bindings. 

## Usage

Add the following to your `Cargo.toml` file:

```toml
[dependencies]
rust_jsc_sys = { features = ["patches"], version = "0.1.2" }
```

## Custom static libs

For custom static libs, you can set the following environment variable:

```bash 
export RUST_JSC_CUSTOM_BUILD_PATH=/Users/${user}/Documents/path/to/static/libs
```

or set the env variable in .cargo/config file.=

by default this library will try to donwload the static libraries from github mirror. If you want to build the static libraries yourself, 
you can build the docker image from the Dockerfile it will build the static libraries for you. and copy the static libraries to the provide path `DOCKER_BUILDKIT=1 docker build -o ./.libs -t $(IMAGE_NAME) .`
this will only works on linux for macos you should build the Javascript Core static libraries running this command from the makefile `make build-jsc` or `WebKit/Tools/Scripts/build-webkit --jsc-only --cmakeargs="-DENABLE_STATIC_JSC=ON -DUSE_THIN_ARCHIVES=OFF"` then set the `RUST_JSC_CUSTOM_BUILD_PATH` to the path of the static libraries.

Make commands:
- `make build-docker-jsc` - build the static libraries for linux
- `make build-jsc` - build the static libraries for macos

# Throuble shooting

if you encounter any problem, linking the static libs trying setting the following environment variables:

```bash
# for macOS
# Example path to the JavaScriptCore static libraries
DYLD_LIBRARY_PATH=/Users/${user}/Documents/Projects/WebKit/WebKitBuild/JSCOnly/Release/lib:$DYLD_LIBRARY_PATH
```

```bash
# for linux
# Example path to the JavaScriptCore static libraries
LD_LIBRARY_PATH=/Users/${user}/Documents/Projects/WebKit/WebKitBuild/JSCOnly/Release/lib:$LD_LIBRARY_PATH
```
