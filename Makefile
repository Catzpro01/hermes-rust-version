# Hermes-RS — thin wrapper so the bare-metal VPS webhook daemon can drive this
# Cargo workspace with the same entry point it uses for the Make-based repos
# (`make check`). There is no build system to replace here: every target below
# is verbatim the command .github/workflows/ci.yml already runs, so `make` can
# never drift from CI.
#
#   ci.yml:308  cargo check --workspace --locked
#   ci.yml:141  cargo fmt --all -- --check
#   ci.yml:148  cargo clippy --workspace --all-targets -- -D warnings
#   ci.yml:155  cargo test --workspace --no-fail-fast
#   ci.yml:186  cargo build --locked -p hermes-rs --bin hermes-rs
#
# `--locked` matters on the daemon: it makes a stale or edited Cargo.lock a
# hard failure instead of a silent rewrite of a tracked file.

CARGO ?= cargo

.PHONY: all check fmt clippy test build

all: check

check:
	$(CARGO) check --workspace --locked

fmt:
	$(CARGO) fmt --all -- --check

clippy:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

test:
	$(CARGO) test --workspace --no-fail-fast

# The binary the live PTY picker gates drive (HERMES_PICKER_BINARY).
build:
	$(CARGO) build --locked -p hermes-rs --bin hermes-rs
