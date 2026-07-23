FROM rust:1.97 as builder

# Set the working directory inside the container
WORKDIR /usr/src/app

# Install build dependencies if needed (e.g., git for cloning)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl3 \
    ca-certificates \
    openssh-client git \
    && rm -rf /var/lib/apt/lists/*

# Create .ssh/ directory for internal dependencies
RUN mkdir -p -m 0700 ~/.ssh && \
    echo "Host git.kundeng.us" >> ~/.ssh/config && \
    echo "    User git" >> ~/.ssh/config && \
    chmod 600 ~/.ssh/config

RUN ssh-keyscan git.kundeng.us >> ~/.ssh/known_hosts

# Copy Cargo manifests
COPY Cargo.toml Cargo.lock ./

RUN --mount=type=ssh mkdir src && \
    echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs && \
    cargo build --release --quiet && \
    rm -rf src target/release/deps/soaricarus_api*

# Copy the actual source code
COPY src ./src
COPY .env ./.env
COPY migrations ./migrations
COPY scripts/init-garage.sh /scripts/init-garage.sh

# Make it executable
RUN chmod +x /scripts/init-garage.sh

RUN --mount=type=ssh \
    cargo build --release --quiet

FROM debian:trixie-slim

# Install runtime dependencies if needed (e.g., SSL certificates)
RUN apt-get update && apt-get install -y ca-certificates libssl-dev libssl3 && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /usr/local/bin

COPY --from=builder /usr/src/app/target/release/soaricarus_api .

COPY --from=builder /usr/src/app/.env .
COPY --from=builder /usr/src/app/migrations ./migrations

EXPOSE 8000

CMD ["./soaricarus_api"]
