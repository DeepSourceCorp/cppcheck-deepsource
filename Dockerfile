# -----------------------------------------------------------
# Base Image with LLVM
# -----------------------------------------------------------
FROM ubuntu:22.04 as ubuntu_llvm
ENV DEBIAN_FRONTEND=noninteractive

# update the system and install any dependencies
RUN add-apt-repository ppa:deadsnakes/ppa\
    && apt-get update \
    && apt-get upgrade -y libksba-dev \
    && apt-get install -y git cmake build-essential byacc libpcre3 libpcre3-dev grep lsb-release wget software-properties-common gnupg libcurl4-openssl-dev unzip lcov python3.11 --no-install-recommends # skipcq: DOK-DL3018

# Get LLVM
ARG LLVM_VER=16
RUN wget --no-verbose https://apt.llvm.org/llvm.sh
RUN chmod +x ./llvm.sh \
  && ./llvm.sh ${LLVM_VER} \
  && apt-get -y install libclang-${LLVM_VER}-dev libclang-cpp${LLVM_VER}-dev --no-install-recommends \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/* \
  && ln -s /usr/lib/x86_64-linux-gnu/libclang-16.so.16.0.6 /usr/lib/x86_64-linux-gnu/libclang-16.so.16

# Add environment variables for build
ENV PATH="$PATH:/usr/lib/llvm-${LLVM_VER}/bin"
ENV LLVM_INSTALL_DIR "/usr/lib/llvm-${LLVM_VER}"
ENV SENTRY_INSTALL_DIR="/usr/lib/sentry-sdk"

# Get Sentry
ARG SENTRY_TAG=0.6.3
RUN mkdir /sentry-sdk \
  && cd /sentry-sdk \
  && wget --no-verbose "https://github.com/getsentry/sentry-native/releases/download/${SENTRY_TAG}/sentry-native.zip" \
  && unzip sentry-native.zip \
  && cmake -B ./build \
  && cmake --build ./build --parallel \
  && cmake --install ./build --prefix "${SENTRY_INSTALL_DIR}"

# Install spdlog
RUN git clone --depth=1 --branch v1.11.0 https://github.com/gabime/spdlog.git \
  && cd spdlog \
  && cmake -B build \
  && cmake --build build --parallel \
  && cd build && make install

# Install cppcheck
RUN git clone --depth=1 --branch 2.10.3 https://github.com/danmar/cppcheck.git \
  && cd cppcheck \
  && cmake -B build -DHAVE_RULES=ON -DUSE_MATCHCOMPILER=ON -DCMAKE_BUILD_TYPE=RELEASE \
  && cmake --build build --parallel 4 \
  && cd build && make install

# -----------------------------------------------------------
# End
# -----------------------------------------------------------

FROM rust:slim-bookworm AS rs_builder

RUN mkdir -p /code
ADD . /code
WORKDIR /code

RUN cargo b --release

FROM ubuntu_llvm

RUN mkdir -p /toolbox
COPY --from=rs_builder /code/target/release/cppcheck-deepsource /toolbox/
