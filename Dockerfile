FROM ubuntu:20.04
ENV PATH="/root/.cargo/bin:${PATH}"
RUN apt update && apt install -y curl build-essential
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain stable -y
RUN rustup update && rustup toolchain install stable
COPY . /root/src
WORKDIR /root/src
RUN cargo build --release

FROM ubuntu:20.04
COPY --from=0 /root/src/target/release/dwarf-writer /usr/local/bin/dwarf-writer
RUN apt update && apt install -y binutils-multiarch
ENTRYPOINT ["/usr/local/bin/dwarf-writer"]
