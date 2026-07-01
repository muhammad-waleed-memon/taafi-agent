# Copyright 2026 Muhammad Waleed
# Licensed under the Apache License, Version 2.0
# Author: Muhammad Waleed

FROM rust:1.78-slim AS builder
RUN apt-get update && apt-get install -y protobuf-compiler pkg-config libssl-dev cmake && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/taafi-agent /usr/local/bin/taafi-agent
EXPOSE 50052
ENTRYPOINT ["taafi-agent"]
CMD ["run"]
