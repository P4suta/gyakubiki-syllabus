# syntax=docker/dockerfile:1
#
# Dev Container image for gyakubiki-syllabus — a Rust/WASM + Bun toolchain kept
# entirely inside the container so the host stays clean.
#
# Design notes:
# - Toolchains are baked read-only into the image; every *writable* cache lives
#   under /home/dev (backed by named volumes in compose.yaml), so nothing heavy
#   lands on the bind-mounted workspace.
# - The container runs as a non-root user whose UID/GID matches the host
#   (rootful Docker), so files written back to the bind mount stay host-owned.

# ---- Pinned upstreams, copied in (reproducible, no curl-from-the-internet) ----
FROM oven/bun:1.2 AS bun

# ---- Dev image ----
FROM rust:1.95.0-bookworm

# OS deps: TLS, git, and jq (handy in scripts); build-essential ships with the
# full rust image already (needed to link proc-macros).
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl git jq \
 && rm -rf /var/lib/apt/lists/*

# Bun — frontend runtime + Vite.
COPY --from=bun /usr/local/bin/bun /usr/local/bin/bun
RUN ln -sf /usr/local/bin/bun /usr/local/bin/bunx

# just (task runner) + wasm-pack (WASM bundler): pinned prebuilt binaries into
# /usr/local/bin (root-owned, always present regardless of volume mounts).
ARG JUST_VERSION=1.36.0
ARG WASM_PACK_VERSION=0.13.1
RUN set -eux; arch="$(uname -m)"; \
    curl -fsSL "https://github.com/casey/just/releases/download/${JUST_VERSION}/just-${JUST_VERSION}-${arch}-unknown-linux-musl.tar.gz" \
      | tar -xz -C /usr/local/bin just; \
    curl -fsSL "https://github.com/rustwasm/wasm-pack/releases/download/v${WASM_PACK_VERSION}/wasm-pack-v${WASM_PACK_VERSION}-${arch}-unknown-linux-musl.tar.gz" \
      | tar -xz --strip-components=1 -C /usr/local/bin "wasm-pack-v${WASM_PACK_VERSION}-${arch}-unknown-linux-musl/wasm-pack"

# Non-root user matching the host (UID/GID 1000) → clean bind-mount ownership.
ARG USER=dev
ARG UID=1000
ARG GID=1000
RUN groupadd -g ${GID} ${USER} \
 && useradd -m -u ${UID} -g ${GID} -s /bin/bash ${USER}

# Keep toolchains read-only & baked (RUSTUP_HOME); redirect every writable cache
# under /home/dev where compose mounts named volumes.
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/home/${USER}/.cargo \
    CARGO_TARGET_DIR=/home/${USER}/target \
    PATH=/home/${USER}/.cargo/bin:/usr/local/cargo/bin:${PATH}

# Bake the wasm32 target and rustfmt/clippy/llvm-tools into the image so the first
# `cargo build`/lint/coverage is offline-fast and reproducible — crucial for the
# ephemeral `docker compose run --rm` containers the git hooks use.
# (image is 1.95.0, matching rust-toolchain.toml.)
RUN rustup component add rustfmt clippy llvm-tools-preview \
 && rustup target add wasm32-unknown-unknown

# cargo-llvm-cov so the pre-push hook can run the same coverage gate as CI.
# Installed to /usr/local (root-owned) so the runtime cargo-cache volume mount
# over /home/dev/.cargo doesn't hide it.
RUN cargo install cargo-llvm-cov --locked --root /usr/local

# Pre-create the writable cache dirs owned by dev so that *fresh* named volumes
# mounted here inherit dev ownership (Docker seeds an empty volume from the
# image dir's content & permissions). No runtime chown/entrypoint needed.
RUN mkdir -p /home/${USER}/.cargo /home/${USER}/target /home/${USER}/.bun/install/cache \
 && chown -R ${UID}:${GID} /home/${USER}

USER ${USER}
WORKDIR /workspace
