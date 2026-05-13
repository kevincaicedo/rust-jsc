# Use an Ubuntu base image
FROM ubuntu:22.04 AS builder

# Set the working directory
WORKDIR /usr/src/app

# Install software-properties-common to add PPAs
RUN apt-get update && apt-get install -y software-properties-common

RUN apt-get install -y \
    cmake \
    ninja-build

RUN apt-get install -y \
    libicu-dev \
    libc-dev \
    gcc-12 \
    g++-12 \
    make \
    python3 \
    libatomic1 \
    libstdc++-12-dev \
    ruby \
    bison \
    flex \
    perl \
    file \
    && update-alternatives --install /usr/bin/gcc gcc /usr/bin/gcc-12 100 \
    && update-alternatives --install /usr/bin/g++ g++ /usr/bin/g++-12 100

# Set environment variable for C++ compiler
ENV CC=/usr/bin/gcc
ENV CXX=/usr/bin/g++
ENV JSC_BUILD_DIR=/usr/src/app/WebKit/WebKitBuild/RustJSC/JSCOnly/Release-Static

# Clone the WebKit repository
COPY ./WebKit ./WebKit

# Build JavaScriptCore directly with CMake/Ninja. The WebKit wrapper is kept out
# of Docker so CI uses the same static build contract as the Makefile path.
RUN cmake -S WebKit -B "${JSC_BUILD_DIR}" -G Ninja \
    -DPORT=JSCOnly \
    -DCMAKE_BUILD_TYPE=Release \
    -DSHOW_BINDINGS_GENERATION_PROGRESS=1 \
    -DDEVELOPER_MODE=ON \
    -DENABLE_REMOTE_INSPECTOR=ON \
    -DENABLE_FTL_JIT=ON \
    -DENABLE_EXPERIMENTAL_FEATURES=OFF \
    -DENABLE_STATIC_JSC=ON \
    -DUSE_THIN_ARCHIVES=OFF \
    -DCMAKE_C_COMPILER="${CC}" \
    -DCMAKE_CXX_COMPILER="${CXX}" && \
    cmake --build "${JSC_BUILD_DIR}" --target jsc --parallel "$(nproc)" && \
    if [ -d "${JSC_BUILD_DIR}/Source/JavaScriptCore/CMakeFiles/JavaScriptCoreJIT.dir" ]; then \
        find "${JSC_BUILD_DIR}/Source/JavaScriptCore/CMakeFiles/JavaScriptCoreJIT.dir" -type f -name '*.o' \
        -exec ar rcs "${JSC_BUILD_DIR}/lib/libJavaScriptCoreJIT.a" {} +; \
    else \
        echo "JavaScriptCoreJIT object directory not produced; libJavaScriptCore.a is self-contained for this target"; \
    fi

# Move back to the main working directory
WORKDIR /usr/src/app

# Extract static libraries from JavaScriptCore and system dependencies
# Uses dynamic paths so this Dockerfile works on both x86_64 and aarch64
RUN mkdir libs && \
    cp "${JSC_BUILD_DIR}"/lib/*.a libs/ && \
    TRIPLET=$(gcc -dumpmachine) && \
    GCC_VERSION=$(gcc -dumpversion | cut -d. -f1) && \
    cp /usr/lib/gcc/${TRIPLET}/${GCC_VERSION}/libstdc++.a libs/ && \
    (cp /usr/lib/${TRIPLET}/libdl.a libs/ 2>/dev/null || true) && \
    cp /usr/lib/${TRIPLET}/libicui18n.a libs/ && \
    cp /usr/lib/${TRIPLET}/libicuuc.a libs/ && \
    cp /usr/lib/${TRIPLET}/libicudata.a libs/ && \
    cp /usr/lib/gcc/${TRIPLET}/${GCC_VERSION}/libatomic.a libs/

FROM scratch

COPY --from=builder /usr/src/app/libs/ /
