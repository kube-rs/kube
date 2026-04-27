# syntax=docker/dockerfile:1
FROM rust:bookworm AS builder
ARG FEATURES=""
WORKDIR /app
COPY . /app
# NB: need rw even on fetch because we do not have a lockfile checked in
RUN --mount=type=bind,rw,target=. \
    --mount=type=cache,target=/root/.cargo \
    cargo fetch
RUN --mount=type=bind,rw,target=. \
    --mount=type=cache,target=/root/.cargo \
    --mount=type=cache,target=target,id=target \
    cargo install --path=e2e --features=${FEATURES} --bin=job

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /usr/local/cargo/bin/job /app/
CMD ["/app/job"]
