
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
