FROM rust:1.92 AS builder

WORKDIR /app
COPY . .

# Build the release binary
RUN cargo build --release -p template-api

FROM debian:bookworm-slim

WORKDIR /app

# Install OpenSSL (required for many Rust networking crates) and CA certificates
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/template-api .


# Create data directory for SQLite
RUN mkdir -p /app/data

ENV DATABASE_URL=sqlite:///app/data/template.db
ENV SESSION_SECRET=supersecretchangeinproduction

EXPOSE 3000

CMD ["./template-api"]
