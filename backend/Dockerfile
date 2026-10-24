# syntax=docker/dockerfile:1 

# Pin the Rust toolchain version used in the build stage.
ARG RUST_VERSION=1.99.0

# Name of the compiled binary produced by Cargo (must match Cargo.toml package name).
ARG APP_NAME=stardate

################################################################################
# Build stage (DOI Rust image)
# This stage compiles the application.
################################################################################

FROM docker.io/library/rust:${RUST_VERSION}-alpine AS build

# Re-declare args inside the stage if you want to use them here.
ARG APP_NAME

# All build steps happen inside /app.
WORKDIR /app

# Install build dependencies needed to compile Rust crates on Alpine
# clang + musl-dev: sqlx compiles SQLite's C source into the binary
RUN apk add --no-cache musl-dev

# Build the application
# Migrations and build.rs are needed at compile time (migrate!() embeds the SQL)
RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=migrations,target=migrations \
    --mount=type=bind,source=build.rs,target=build.rs \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/var/cache/cargo \
    CARGO_HOME=/var/cache/cargo cargo build --locked --release && \
    cp ./target/release/$APP_NAME /bin/server

################################################################################
# Runtime stage (DOI Alpine image)
# This stage runs the already-compiled binary with minimal dependencies.
################################################################################

FROM docker.io/library/alpine:3.22 AS final

# Create a non-privileged user (recommended best practice)
ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser

# Must happen as root, before USER. A named volume inherits this ownership.
RUN mkdir /data && chown appuser:appuser /data

# Drop privileges for runtime.
USER appuser
WORKDIR /data

# Copy only the compiled binary from the build stage.
COPY --from=build /bin/server /bin/

EXPOSE 3000
VOLUME /data

# Start the application
CMD ["/bin/server"]