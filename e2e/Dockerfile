# syntax=docker/dockerfile:1
FROM rust:slim-bookworm as builder
ARG FEATURES=""
WORKDIR /app
COPY . /app
# RUN --mount=type=bind,target=. --mount=type=cache,target=/root/.cargo cargo fetch
# RUN --mount=type=bind,rw,target=. --mount=type=cache,target=/root/.cargo --mount=type=cache,target=target,id=target cargo install --path=e2e --features=${FEATURES} --bin=job
RUN cargo fetch
RUN cargo install --path=e2e --features=${FEATURES} --bin=job

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /usr/local/cargo/bin/job /app/
CMD ["/app/job"]
