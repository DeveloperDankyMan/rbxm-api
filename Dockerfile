# ---- build stage ----
FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

# ---- run stage (small image) ----
FROM debian:bookworm-slim
RUN useradd --no-create-home --shell /usr/sbin/nologin app
COPY --from=build /app/target/release/rbxm-api /usr/local/bin/rbxm-api
USER app
# The server reads PORT (Render sets it automatically) and defaults to 8080.
ENV PORT=8080
EXPOSE 8080
CMD ["rbxm-api"]