FROM --platform=linux/arm64 dustynv/jetson-inference:r32.7.1

ENV DEBIAN_FRONTEND=noninteractive
ENV SHELL /bin/bash

WORKDIR /workspace

RUN apt-key adv --keyserver keyserver.ubuntu.com --recv-keys 42D5A192B819C5DA
RUN apt-get update

# Install Rust with specific version and target
RUN apt-get -y install curl
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.92.0 --target aarch64-unknown-linux-gnu
ENV PATH="/root/.cargo/bin:${PATH}"

ENV LD_LIBRARY_PATH="/usr/lib:${LD_LIBRARY_PATH}"
COPY lib/jetson_nano/stt/*.so /usr/lib/

# Configure the microphone.
RUN apt-get -y install libportaudio2 libasound2-dev
ENV PA_ALSA_PLUGHW=1

# Copy project files
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build the project
RUN cargo build --release
RUN cp target/release/jetson-stt /usr/local/bin/jetson-stt

ENTRYPOINT ["/usr/local/bin/jetson-stt"]
