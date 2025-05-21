FROM lukemathwalker/cargo-chef:latest-rust-1.85.0 AS chef
WORKDIR /app
RUN apt update && apt install lld clang -y

# Download the "non" static build of Litestream directly into the path & make it executable.
ADD https://github.com/benbjohnson/litestream/releases/download/v0.3.13/litestream-v0.3.13-linux-amd64.tar.gz /tmp/litestream.tar.gz
RUN tar -C /usr/local/bin -xzf /tmp/litestream.tar.gz

FROM chef AS planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
# Build our project
RUN cargo build --release --bin newzletter

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# Create the data directory and set it as a volume
RUN mkdir -p /app/data
VOLUME /app/data

COPY --from=builder /app/target/release/newzletter newzletter
COPY configuration configuration
# COPY migrations migrations
COPY --from=builder /usr/local/bin/litestream /usr/local/bin/litestream

# Ensure run.sh is executable
COPY scripts/run.sh /scripts/run.sh
RUN chmod +x /scripts/run.sh

# Copy Litestream configuration file
COPY etc/litestream.yml /etc/litestream.yml

# Copy frontend dir
COPY frontend frontend

ENV APP_ENVIRONMENT=production
# db is in docker's volume /app/data
ENV DATABASE_URL=sqlite:///app/data/newsletter.db

ENTRYPOINT ["/scripts/run.sh"]