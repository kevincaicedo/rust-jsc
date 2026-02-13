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
    g++-11 \
    make \
    python2 \
    libatomic1 \
    libstdc++-11-dev \
    ruby \
    bison \
    flex \
    perl \
    file \
    && update-alternatives --install /usr/bin/gcc gcc /usr/bin/gcc-11 100 \
    && update-alternatives --install /usr/bin/g++ g++ /usr/bin/g++-11 100

# Set environment variable for C++ compiler
ENV CXX=/usr/bin/g++

# Clone the WebKit repository
COPY ./WebKit ./WebKit

# Set the working directory to the WebKit directory
WORKDIR /usr/src/app/WebKit

# Ensure the build-webkit script is executable
RUN chmod +x Tools/Scripts/build-webkit

# Build JavaScriptCore only as a static library
RUN Tools/Scripts/build-webkit --jsc-only --cmakeargs="-DENABLE_STATIC_JSC=ON -DUSE_THIN_ARCHIVES=OFF -DENABLE_REMOTE_INSPECTOR=ON -DENABLE_EXPERIMENTAL_FEATURES=OFF"

# Move back to the main working directory
WORKDIR /usr/src/app

# Extract static libraries from JavaScriptCore and system dependencies
# Uses dynamic paths so this Dockerfile works on both x86_64 and aarch64
RUN mkdir libs && \
    cp /usr/src/app/WebKit/WebKitBuild/JSCOnly/Release/lib/*.a libs/ && \
    TRIPLET=$(gcc -dumpmachine) && \
    GCC_VERSION=$(gcc -dumpversion | cut -d. -f1) && \
    cp /usr/lib/gcc/${TRIPLET}/${GCC_VERSION}/libstdc++.a libs/ && \
    cp /usr/lib/${TRIPLET}/libdl.a libs/ 2>/dev/null || true && \
    cp /usr/lib/${TRIPLET}/libicui18n.a libs/ && \
    cp /usr/lib/${TRIPLET}/libicuuc.a libs/ && \
    cp /usr/lib/${TRIPLET}/libicudata.a libs/ && \
    cp /usr/lib/gcc/${TRIPLET}/${GCC_VERSION}/libatomic.a libs/

FROM scratch

COPY --from=builder /usr/src/app/libs/ /
