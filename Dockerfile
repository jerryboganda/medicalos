# Production image for the Medical Learning OS API (§20.2).
# Built and published by .github/workflows/deploy.yml to GHCR.
# The production VPS runs ONLY this container + PostgreSQL + TLS proxy —
# nothing else, per the compute policy in AGENTS.md.

FROM rust:1-slim AS build
WORKDIR /app
COPY Cargo.toml rust-toolchain.toml ./
COPY .sqlx ./.sqlx
COPY crates ./crates
COPY apps/api ./apps/api
# Queries compile against the committed offline cache (generated in CI from
# the live schema — the same compile-time checks, no database needed).
# BUILD_SHA stamps /api/version.json so production E2E can prove the exact
# commit landed (passed as --build-arg by the deploy workflow).
ARG BUILD_SHA=dev
ENV SQLX_OFFLINE=true \
    MEDICALOS_BUILD_SHA=$BUILD_SHA
RUN cargo build --release -p api

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/api /usr/local/bin/api
ENV MIN_TIME_LIMIT_SECONDS=30 \
    FREE_DAILY_QUESTIONS=10
EXPOSE 8080
CMD ["api"]
