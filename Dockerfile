FROM rust:1.98-bookworm AS rust-build
WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

FROM node:24-bookworm-slim AS web-build
WORKDIR /app/web
COPY web/package.json web/pnpm-lock.yaml web/pnpm-workspace.yaml ./
RUN corepack enable && pnpm install --frozen-lockfile
COPY web ./
RUN pnpm build

FROM mcr.microsoft.com/playwright:v1.65.1-noble
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp \
    && chmod +x /usr/local/bin/yt-dlp \
    && rm -rf /var/lib/apt/lists/*
COPY --from=rust-build /app/target/release/ainews /usr/local/bin/ainews
COPY --from=web-build /app/web/dist /app/web/dist
COPY config /app/config
COPY browser-worker /app/browser-worker
RUN cd /app/browser-worker && npm install --omit=dev
COPY .env.example /app/.env.example
ENV RUST_LOG=ainews=info
ENV PLAYWRIGHT_CHANNEL=
EXPOSE 8080
VOLUME ["/app/data"]
CMD ["ainews", "serve"]
