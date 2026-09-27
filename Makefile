# Trellis Protocol — Build Automation
#
# Available targets:
#   make help          — list all targets
#   make build         — build everything (contract WASM + frontend)
#   make test          — run all tests (contract + CLI + frontend)
#   make lint          — run all linters (clippy + oxlint)
#   make fmt           — run cargo fmt on the workspace
#   make typecheck-frontend — run tsc typecheck on the frontend
#   make setup         — install prerequisites (rustup target + npm deps)
#   make deploy        — deploy contract to testnet (see DEPLOYMENT.md)
#   make clean         — remove all build artifacts
#   make build-contract — build only the contract WASM
#   make build-cli     — build only the CLI binary
#   make build-frontend — build only the frontend bundle
#   make test-contract — run only contract tests
#   make test-cli      — run only CLI tests
#   make test-snapshots-update — regenerate Soroban test snapshots (commit the result)
#   make test-frontend — run only frontend tests
#   make lint-contract — run only clippy on contract and CLI
#   make lint-frontend — run only oxlint on frontend
#   make changelog     — regenerate CHANGELOG.md from git history (git-cliff)

.PHONY: help build test lint fmt deploy clean setup
.PHONY: build-contract build-cli build-frontend
.PHONY: test-contract test-cli test-snapshots-update test-frontend
.PHONY: lint-contract lint-frontend typecheck-frontend
.PHONY: changelog

help:
	@echo "Trellis Protocol — Build Targets"
	@echo ""
	@grep -E '^[a-z-]+:' $(MAKEFILE_LIST) | sort | while IFS=: read -r target _; do \
		desc=$$(grep -A1 "^$$target:" $(MAKEFILE_LIST) | tail -1 | sed 's/# //'); \
		printf "  %-20s %s\n" "$$target" "$$desc"; \
	done

# ── Build ──────────────────────────────────────────────────────────────────

build: build-contract build-cli build-frontend

build-contract:
	cargo build --frozen --manifest-path contracts/trellis_core/Cargo.toml --target wasm32-unknown-unknown --release

build-cli:
	cargo build --frozen --manifest-path cli/trellis_cli/Cargo.toml --release

build-frontend:
	# npm ci (not npm install) installs exact versions from package-lock.json
	# for reproducible builds.
	cd frontend && npm ci && npm run build

# ── Test ───────────────────────────────────────────────────────────────────

test: test-contract test-cli test-frontend

test-contract:
	cargo test --frozen --manifest-path contracts/trellis_core/Cargo.toml

# CI does not shell out to make: the `verify` job in
# .github/workflows/contract-ci.yml runs `cargo test --workspace`, which
# already covers cli/trellis_cli. This target keeps `make test` in parity.
test-cli:
	cargo test --frozen --manifest-path cli/trellis_cli/Cargo.toml

test-snapshots-update:
	SOROBAN_TEST_SNAPSHOT_FILE=overwrite cargo test --manifest-path contracts/trellis_core/Cargo.toml
	@echo "Snapshots regenerated. Review the diff with: git diff contracts/trellis_core/test_snapshots/"

test-frontend:
	cd frontend && npm ci && npm test

# ── Lint ───────────────────────────────────────────────────────────────────

lint: lint-contract lint-frontend

lint-contract:
	cargo clippy --frozen --manifest-path contracts/trellis_core/Cargo.toml -- -D warnings
	cargo clippy --frozen --manifest-path cli/trellis_cli/Cargo.toml -- -D warnings

lint-frontend:
	cd frontend && npm ci && npm run lint

fmt:
	cargo fmt --all

typecheck-frontend:
	cd frontend && npm run typecheck

# ── Changelog ──────────────────────────────────────────────────────────────

# Requires git-cliff: https://github.com/orhun/git-cliff
changelog:
	# Regenerate the committed CHANGELOG.md from conventional commits.
	@command -v git-cliff >/dev/null 2>&1 || { \
		echo "git-cliff not found. Install it with: cargo install git-cliff"; \
		exit 1; \
	}
	git-cliff --config cliff.toml --output CHANGELOG.md
	@echo "CHANGELOG.md regenerated from git history."

# ── Deploy ─────────────────────────────────────────────────────────────────

DEPLOYER_IDENTITY ?= trellis-deployer

deploy:
	@echo "=== Deploying Trellis to Stellar Testnet ==="
	@echo "Ensure you have:"
	@echo "  1. A funded testnet identity: stellar keys fund <name> --network testnet"
	@echo "  2. The contract WASM built: make build-contract"
	@echo ""
	@echo "Running: stellar contract deploy ..."
	stellar contract deploy \
		--wasm target/wasm32-unknown-unknown/release/trellis_core.wasm \
		--source $(DEPLOYER_IDENTITY) \
		--network testnet

# ── Clean ──────────────────────────────────────────────────────────────────

clean:
	cargo clean
	cd frontend && rm -rf dist node_modules
	rm -rf target

# ── Setup ──────────────────────────────────────────────────────────────────

setup:
	rustup target add wasm32-unknown-unknown
	cd frontend && npm ci