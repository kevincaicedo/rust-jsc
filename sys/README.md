# Env Variables

## MacOS
```bash
export WEBKIT_PATH=/Users/${user}/Documents/Projects/WebKit
```

## Linux
```bash
export JSC_LIBS_PATH=/Users/${user}/Documents/Projects/libs
```

The default JSC LIBS path is relative to the library path. If you have a different path, you can set the `JSC_LIBS_PATH` environment variable to the correct path.

# Throuble shooting

if you encounter any problem, generating binary trying setting the following environment variables:

```bash
# for macOS
# path to the JavaScriptCore library
DYLD_LIBRARY_PATH=/Users/${user}/Documents/Projects/WebKit/WebKitBuild/JSCOnly/Release/lib:$DYLD_LIBRARY_PATH
```

```bash
# for linux
# path to the JavaScriptCore library
LD_LIBRARY_PATH=/Users/${user}/Documents/Projects/WebKit/WebKitBuild/JSCOnly/Release/lib:$LD_LIBRARY_PATH
```
