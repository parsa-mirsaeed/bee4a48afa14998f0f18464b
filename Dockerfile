# syntax=docker/dockerfile:1.7
ARG RUST_IMAGE=rust:1.96.1-trixie
FROM ${RUST_IMAGE} AS toolchain

ARG DIOXUS_CLI_VERSION=0.7.2
ARG CARGO_CHEF_VERSION=0.1.77
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add wasm32-unknown-unknown \
    && curl -L --proto '=https' --tlsv1.2 -sSf \
        https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall "cargo-chef@${CARGO_CHEF_VERSION}" -y --force \
    && cargo binstall "dioxus-cli@${DIOXUS_CLI_VERSION}" -y --force

WORKDIR /workspace

FROM toolchain AS dev
CMD ["dx", "serve", "--web", "--package", "web", "--addr", "0.0.0.0", "--port", "8080"]

FROM toolchain AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Keep the AI gateway/API dependency graph independent from Web/WASM source and
# asset churn. The recipe still captures the exact workspace dependency graph.
FROM toolchain AS api-dependencies
COPY --from=planner /workspace/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --package api --features server

FROM toolchain AS web-dependencies
COPY --from=planner /workspace/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --package web --features server
RUN cargo chef cook --release --recipe-path recipe.json --package web --features web --target wasm32-unknown-unknown

FROM api-dependencies AS api-builder
COPY Cargo.toml Cargo.lock ./
COPY .cargo .cargo
COPY packages/api packages/api
COPY migrations migrations
RUN cargo build --release --package api --features server --bin ai_gateway

FROM web-dependencies AS web-builder
COPY . .
RUN dx bundle --web --release --package web

# Assemble the same runtime payload without carrying the Rust toolchain or a
# build-time PostgreSQL server into artifact compilation. Migration correctness
# is proven after the image exists, against the packaged migration entrypoint.
FROM debian:trixie-slim AS bundle
WORKDIR /workspace
COPY --from=web-builder /workspace/target/dx/web/release/web /tmp/web-bundle
COPY --from=api-builder /workspace/target/release/ai_gateway /tmp/ai_gateway
COPY packages/web/assets/fonts /tmp/fonts
RUN set -eux; \
    bundle_dir="/tmp/web-bundle"; \
    test -d "${bundle_dir}/public"; \
    mkdir -p /opt/edutalent-bundle; \
    cp -R "${bundle_dir}/public" /opt/edutalent-bundle/public; \
    if [ -x "${bundle_dir}/server" ]; then \
        cp "${bundle_dir}/server" /opt/edutalent-bundle/server; \
    elif [ -x "${bundle_dir}/web" ]; then \
        cp "${bundle_dir}/web" /opt/edutalent-bundle/server; \
    else \
        executable="$(find "${bundle_dir}" -maxdepth 1 -type f -perm /111 | head -n 1)"; \
        test -n "${executable}"; \
        cp "${executable}" /opt/edutalent-bundle/server; \
    fi; \
    test -x /tmp/ai_gateway; \
    cp /tmp/ai_gateway /opt/edutalent-bundle/ai_gateway; \
    mkdir -p /opt/edutalent-bundle/public/fonts; \
    cp -R /tmp/fonts/. /opt/edutalent-bundle/public/fonts/; \
    chmod +x /opt/edutalent-bundle/server /opt/edutalent-bundle/ai_gateway

FROM debian:trixie-slim AS runtime
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl postgresql-client passwd \
    && groupadd --gid 65532 edutalent \
    && useradd --uid 65532 --gid 65532 --no-create-home --home-dir /nonexistent --shell /usr/sbin/nologin edutalent \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/edutalent
COPY --from=bundle --chown=65532:65532 /opt/edutalent-bundle/ /opt/edutalent/
COPY --chown=65532:65532 packages/api/migration/migrations/ /opt/edutalent/packages/api/migration/migrations/
COPY --chown=65532:65532 migrations/ /opt/edutalent/migrations/
COPY --chown=65532:65532 scripts/ci/apply_migrations.sh /opt/edutalent/scripts/ci/apply_migrations.sh
COPY --chown=65532:65532 scripts/ci/configure_database_role.sh /opt/edutalent/scripts/ci/configure_database_role.sh
COPY docker/entrypoint.sh /usr/local/bin/edutalent-entrypoint
RUN chmod +x \
    /usr/local/bin/edutalent-entrypoint \
    /opt/edutalent/server \
    /opt/edutalent/ai_gateway \
    /opt/edutalent/scripts/ci/apply_migrations.sh \
    /opt/edutalent/scripts/ci/configure_database_role.sh

USER 65532:65532

ENV IP=0.0.0.0 \
    PORT=8080 \
    RUN_MIGRATIONS=true

EXPOSE 8080
HEALTHCHECK --interval=10s --timeout=3s --start-period=30s --retries=6 \
    CMD curl --fail --silent http://127.0.0.1:${PORT}/healthz >/dev/null || exit 1

ENTRYPOINT ["edutalent-entrypoint"]
CMD ["server"]
