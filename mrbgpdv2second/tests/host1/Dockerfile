FROM rust

RUN apt install git
RUN git clone https://github.com/Miyoshi-Ryota/mrbgpdv2.git
WORKDIR /mrbgpdv2/mrbgpdv2second
RUN cargo build
CMD RUST_LOG=info ./target/debug/mrbgpdv2second "64512 10.200.100.2 64513 10.200.100.3 active"
