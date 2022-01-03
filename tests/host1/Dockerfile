FROM rust

WORKDIR /mrbgpdv2
COPY . .
RUN rustup default nightly
RUN cargo build
RUN apt-get update
RUN apt-get -y install iputils-ping
CMD ./target/debug/mrbgpdv2 "64512 10.200.100.2 64513 10.200.100.3 active"
