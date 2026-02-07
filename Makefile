CARGO := /home/kithkui/.cargo/bin/cargo
BINARY := target/release/anky
PID_FILE := data/.anky.pid

.PHONY: run dev build release stop restart logs clean test-write test-health test-generate db-shell help

# Default target
help:
	@echo "anky server commands:"
	@echo "  make run          - build release and start server"
	@echo "  make dev          - build debug and start with RUST_LOG=debug"
	@echo "  make build        - build release binary"
	@echo "  make stop         - stop running server"
	@echo "  make restart      - stop + run"
	@echo "  make logs         - tail server logs"
	@echo ""
	@echo "testing:"
	@echo "  make test-health  - hit /health endpoint"
	@echo "  make test-write   - submit a test writing session"
	@echo "  make test-generate - generate a test anky (Rumi)"
	@echo ""
	@echo "utils:"
	@echo "  make db-shell     - open sqlite3 on anky.db"
	@echo "  make clean        - cargo clean"
	@echo "  make train        - trigger training manually"

# Build release
build:
	$(CARGO) build --release

# Build debug
build-debug:
	$(CARGO) build

# Run release server (foreground)
run: build
	@mkdir -p data data/images data/streams data/training_runs data/lora_weights
	./$(BINARY)

# Run in background
run-bg: build
	@mkdir -p data data/images data/streams data/training_runs data/lora_weights
	@if [ -f $(PID_FILE) ] && kill -0 $$(cat $(PID_FILE)) 2>/dev/null; then \
		echo "server already running (pid $$(cat $(PID_FILE)))"; \
	else \
		./$(BINARY) > data/anky.log 2>&1 & echo $$! > $(PID_FILE); \
		echo "server started (pid $$(cat $(PID_FILE)))"; \
	fi

# Dev mode with debug logging
dev: build-debug
	@mkdir -p data data/images data/streams data/training_runs data/lora_weights
	RUST_LOG=anky=debug,tower_http=debug ./target/debug/anky

# Stop background server
stop:
	@if [ -f $(PID_FILE) ] && kill -0 $$(cat $(PID_FILE)) 2>/dev/null; then \
		kill $$(cat $(PID_FILE)); \
		rm -f $(PID_FILE); \
		echo "server stopped"; \
	else \
		pkill -f "target/release/anky" 2>/dev/null || pkill -f "target/debug/anky" 2>/dev/null || true; \
		rm -f $(PID_FILE); \
		echo "server stopped"; \
	fi

# Restart
restart: stop run-bg

# Tail logs
logs:
	@tail -f data/anky.log

# Clean build artifacts
clean:
	$(CARGO) clean

# Test endpoints
test-health:
	@curl -s http://localhost:8889/health | python3 -m json.tool

test-write:
	@curl -s -X POST http://localhost:8889/write \
		-H 'Content-Type: application/json' \
		-d '{"text":"stream of consciousness test writing, the words flow like water through the cracks of my mind, what am i doing here, what is this place, the keyboard clicks and the thoughts emerge","duration":120}' \
		| python3 -m json.tool

test-generate:
	@echo "generating anky for Rumi (takes ~30s)..."
	@curl -s -X POST http://localhost:8889/api/generate \
		-H 'Content-Type: application/json' \
		-d '{"thinker_name":"Rumi","moment":"the night Shams disappeared, alone in the courtyard, whirling for the first time without his beloved teacher"}' \
		| python3 -m json.tool

# Open database shell
db-shell:
	sqlite3 data/anky.db

# Trigger training manually
train:
	@curl -s -X POST http://localhost:8889/api/generate \
		-H 'Content-Type: application/json' \
		-d '{"thinker_name":"test","moment":"training trigger"}' > /dev/null
	@echo "check /poiesis for progress"
