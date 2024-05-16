# Use an Ubuntu base image
FROM ubuntu:22.04 as builder

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
ENV CXX /usr/bin/g++

# Clone the WebKit repository
COPY ./WebKit ./WebKit

# Set the working directory to the WebKit directory
WORKDIR /usr/src/app/WebKit

# Ensure the build-webkit script is executable
RUN chmod +x Tools/Scripts/build-webkit

# Check Perl availability, version, and script shebang line
# RUN perl -v && head -n 1 Tools/Scripts/build-webkit

# Build JavaScriptCore only as a static library
RUN Tools/Scripts/build-webkit --jsc-only --cmakeargs="-DENABLE_STATIC_JSC=ON -DUSE_THIN_ARCHIVES=OFF"

# Move back to the main working directory
WORKDIR /usr/src/app

# Extract static libraries from JavaScriptCore
RUN mkdir libs && \
    cp /usr/src/app/WebKit/WebKitBuild/JSCOnly/Release/lib/*.a libs/

# RUN apt-get install -y libc6-dev

# Combine all static libraries into one
# RUN cd libs && \
#     ar -x /usr/lib/gcc/x86_64-linux-gnu/11/libstdc++.a && \
#     ar -x /usr/lib/x86_64-linux-gnu/libicui18n.a && \
#     ar -x /usr/lib/x86_64-linux-gnu/libmvec.a && \
#     ar -x /usr/lib/x86_64-linux-gnu/libdl.a && \
#     ar -x /usr/lib/x86_64-linux-gnu/libicuuc.a && \
#     ar -x /usr/lib/x86_64-linux-gnu/libicudata.a && \
#     ar -x /usr/lib/gcc/x86_64-linux-gnu/11/libatomic.a && \
#     ar -x libJavaScriptCore.a && \
#     ar -x libWTF.a && \
#     ar -x libbmalloc.a && \
#     ar -rcs ../librustjsc.a *.o && ranlib ../librustjsc.a

# Clean up object files
# RUN rm -rf libs

FROM scratch

# COPY --from=builder /usr/src/app/librustjsc.a /
COPY --from=builder /usr/src/app/libs/libJavaScriptCore.a /
COPY --from=builder /usr/src/app/libs/libWTF.a /
COPY --from=builder /usr/src/app/libs/libbmalloc.a /
COPY --from=builder /usr/lib/gcc/x86_64-linux-gnu/11/libstdc++.a /
# COPY --from=builder /usr/lib/x86_64-linux-gnu/libmvec.a /
COPY --from=builder /usr/lib/x86_64-linux-gnu/libdl.a /
COPY --from=builder /usr/lib/x86_64-linux-gnu/libicui18n.a /
COPY --from=builder /usr/lib/x86_64-linux-gnu/libicuuc.a /
COPY --from=builder /usr/lib/x86_64-linux-gnu/libicudata.a /
COPY --from=builder /usr/lib/gcc/x86_64-linux-gnu/11/libatomic.a /
