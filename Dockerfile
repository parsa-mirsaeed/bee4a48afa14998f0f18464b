# syntax=docker/dockerfile:1.7
ARG RUST_IMAGE=rust:1.96.1-trixie
FROM ${RUST_IMAGE} AS toolchain

ARG DIOXUS_CLI_VERSION=0.7.2
ARG CARGO_CHEF_VERSION=0.1.77
ARG TARGETARCH
# Pin official release archive digests for each supported architecture.
# Do not use cargo-binstall's rate-limited API/source fallback for build tools.
RUN set -eux; \
    apt-get update; \
    apt-get install --yes --no-install-recommends ca-certificates curl pkg-config libssl-dev xz-utils; \
    rm -rf /var/lib/apt/lists/*; \
    rustup target add wasm32-unknown-unknown; \
    case "${TARGETARCH:-$(uname -m)}" in \
      amd64|x86_64) \
        target=x86_64-unknown-linux-gnu; \
        chef_sha=ff390181459f53c787a496c4ee9a52d8c9c7975ab8d02c8dbd1a0fa1626171f4; \
        dx_sha=83a68cee967bba9e86aa6909c48544c8056be7df6af252517d8014b625314ce6 ;; \
      arm64|aarch64) \
        target=aarch64-unknown-linux-gnu; \
        chef_sha=64cd1b405894f587a461a0dd0f60127330b1ebd204fd46b1a36fb0ff6da157e0; \
        dx_sha=a59c71a4503514fc2ab1943baa157769353c4e83c81954223a8849c5461a1728 ;; \
      *) echo "Unsupported build architecture: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    curl --fail --silent --show-error --location --retry 6 --retry-connrefused \
      --proto '=https' --proto-redir '=https' --tlsv1.2 \
      "https://github.com/LukeMathWalker/cargo-chef/releases/download/v${CARGO_CHEF_VERSION}/cargo-chef-${target}.tar.xz" \
      --output /tmp/cargo-chef.tar.xz; \
    echo "${chef_sha}  /tmp/cargo-chef.tar.xz" | sha256sum --check --status; \
    chef_entry="$(tar -tf /tmp/cargo-chef.tar.xz | grep -E '(^|/)cargo-chef$' | head -n 1)"; \
    test -n "${chef_entry}"; \
    tar -xOJf /tmp/cargo-chef.tar.xz "${chef_entry}" > /usr/local/cargo/bin/cargo-chef; \
    chmod 0755 /usr/local/cargo/bin/cargo-chef; \
    curl --fail --silent --show-error --location --retry 6 --retry-connrefused \
      --proto '=https' --proto-redir '=https' --tlsv1.2 \
      "https://github.com/DioxusLabs/dioxus/releases/download/v${DIOXUS_CLI_VERSION}/dx-${target}.tar.gz" \
      --output /tmp/dx.tar.gz; \
    echo "${dx_sha}  /tmp/dx.tar.gz" | sha256sum --check --status; \
    dx_entry="$(tar -tzf /tmp/dx.tar.gz | grep -E '(^|/)dx$' | head -n 1)"; \
    test -n "${dx_entry}"; \
    tar -xOzf /tmp/dx.tar.gz "${dx_entry}" > /usr/local/cargo/bin/dx; \
    chmod 0755 /usr/local/cargo/bin/dx; \
    cargo chef --version; \
    dx --version; \
    rm -f /tmp/cargo-chef.tar.xz /tmp/dx.tar.gz

WORKDIR /workspace

FROM toolchain AS dev
CMD ["dx", "serve", "--web", "--package", "web", "--addr", "0.0.0.0", "--port", "8080"]

FROM toolchain AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# The current API/server graph still uses SQLx compile-time validation. Keep the
# minimum local PostgreSQL prerequisite in a shared build layer until a separately
# proven offline SQLx metadata change can remove it. Source compilation remains
# split below so Web/UI churn cannot invalidate the AI Gateway source layer.
FROM toolchain AS build-deps
RUN apt-get update \
    && apt-get install --yes --no-install-recommends postgresql postgresql-client \
    && rm -rf /var/lib/apt/lists/*
COPY --from=planner /workspace/recipe.json recipe.json
COPY vendor/dioxus-fullstack/ vendor/dioxus-fullstack/
RUN cargo chef cook --release --recipe-path recipe.json --package web --features server
RUN cargo chef cook --release --recipe-path recipe.json --package web --features web --target wasm32-unknown-unknown

# Build only the API-owned gateway source in this layer. Presentation files are
# intentionally absent so normal Web/UI source changes can reuse this result.
# BuildKit may execute this stage in parallel with web-builder on host networking,
# so its compile-only PostgreSQL instance uses a dedicated non-default port.
FROM build-deps AS gateway-builder
COPY packages/api/ packages/api/
COPY vendor/dioxus-fullstack/ vendor/dioxus-fullstack/
COPY migrations/ migrations/
COPY scripts/ci/apply_migrations.sh scripts/ci/apply_migrations.sh
RUN set -eux; \
    pg_version="$(pg_lsclusters --no-header | awk 'NR == 1 {print $1}')"; \
    sed -ri 's/^[# ]*port = [0-9]+/port = 55432/' "/etc/postgresql/${pg_version}/main/postgresql.conf"; \
    service postgresql start; \
    runuser -u postgres -- psql -p 55432 --set=ON_ERROR_STOP=1 \
        --command="ALTER USER postgres PASSWORD 'postgres'"; \
    runuser -u postgres -- createdb -p 55432 edutalent_build; \
    export DATABASE_URL='postgresql://postgres:postgres@127.0.0.1:55432/edutalent_build'; \
    bash scripts/ci/apply_migrations.sh; \
    cargo build --release --package api --features server --bin ai_gateway; \
    service postgresql stop

# The Web server feature links the API contract, so the complete source tree is
# required here. This layer remains parallel with gateway-builder and therefore
# uses a different compile-only PostgreSQL port to avoid host-network collisions.
FROM build-deps AS web-builder
COPY . .
RUN set -eux; \
    pg_version="$(pg_lsclusters --no-header | awk 'NR == 1 {print $1}')"; \
    sed -ri 's/^[# ]*port = [0-9]+/port = 55433/' "/etc/postgresql/${pg_version}/main/postgresql.conf"; \
    service postgresql start; \
    runuser -u postgres -- psql -p 55433 --set=ON_ERROR_STOP=1 \
        --command="ALTER USER postgres PASSWORD 'postgres'"; \
    runuser -u postgres -- createdb -p 55433 edutalent_build; \
    export DATABASE_URL='postgresql://postgres:postgres@127.0.0.1:55433/edutalent_build'; \
    bash scripts/ci/apply_migrations.sh; \
    bash scripts/package/build_web_release.sh; \
    service postgresql stop
RUN set -eux; \
    bundle_dir="target/dx/web/release/web"; \
    test -d "${bundle_dir}/public"; \
    mkdir -p /opt/edutalent-web; \
    cp -R "${bundle_dir}/public" /opt/edutalent-web/public; \
    if [ -x "${bundle_dir}/server" ]; then \
        cp "${bundle_dir}/server" /opt/edutalent-web/server; \
    elif [ -x "${bundle_dir}/web" ]; then \
        cp "${bundle_dir}/web" /opt/edutalent-web/server; \
    else \
        executable="$(find "${bundle_dir}" -maxdepth 1 -type f -perm /111 | head -n 1)"; \
        test -n "${executable}"; \
        cp "${executable}" /opt/edutalent-web/server; \
    fi; \
    if [ -d packages/web/assets/fonts ]; then \
        mkdir -p /opt/edutalent-web/public/fonts; \
        cp -R packages/web/assets/fonts/. /opt/edutalent-web/public/fonts/; \
    fi; \
    chmod +x /opt/edutalent-web/server

FROM debian:trixie-slim AS runtime
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl postgresql-client passwd \
    && groupadd --gid 65532 edutalent \
    && useradd --uid 65532 --gid 65532 --no-create-home --home-dir /nonexistent --shell /usr/sbin/nologin edutalent \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/edutalent
COPY --from=web-builder --chown=65532:65532 /opt/edutalent-web/ /opt/edutalent/
COPY --from=gateway-builder --chown=65532:65532 /workspace/target/release/ai_gateway /opt/edutalent/ai_gateway
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
