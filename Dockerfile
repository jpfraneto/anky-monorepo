FROM rust:1.93-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true

COPY src/ src/
COPY static/ static/
COPY templates/ templates/
COPY prompts/ prompts/
COPY agent-skills/ agent-skills/
COPY flux/ flux/
COPY migrations/ migrations/
COPY PROMPT.md MANIFESTO.md SOUL.md skills.md ./

RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/anky ./anky
COPY --from=builder /app/templates ./templates
COPY --from=builder /app/static ./static
COPY --from=builder /app/prompts ./prompts
COPY --from=builder /app/agent-skills ./agent-skills
COPY --from=builder /app/flux ./flux
COPY --from=builder /app/migrations ./migrations

RUN mkdir -p \
    data/images \
    data/anky-images \
    data/writings \
    data/videos \
    data/generations \
    data/training-images \
    data/training_runs \
    data/mirrors \
    data/classes \
    videos

EXPOSE 8889
ENV ANKY_MODE=web

CMD ["./anky"]
