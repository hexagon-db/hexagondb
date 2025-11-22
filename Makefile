# HexagonDB Makefile
# Build both server and CLI easily

.PHONY: all build build-release build-cli build-cli-release test clean run run-cli install help

# Default target
all: build

# Build server in debug mode
build:
	@echo "🔨 Building HexagonDB server (debug)..."
	@cargo build
	@echo "✅ Server built: target/debug/hexagondb"

# Build server in release mode
build-release:
	@echo "🔨 Building HexagonDB server (release)..."
	@cargo build --release
	@echo "✅ Server built: target/release/hexagondb"

# Build CLI in debug mode
build-cli:
	@echo "🔨 Building HexagonDB CLI (debug)..."
	@cd cli && cargo build
	@echo "✅ CLI built: cli/target/debug/hexagondb-cli"

# Build CLI in release mode
build-cli-release:
	@echo "🔨 Building HexagonDB CLI (release)..."
	@cd cli && cargo build --release
	@echo "✅ CLI built: cli/target/release/hexagondb-cli"

# Build everything (server + CLI) in release mode
build-all: build-release build-cli-release
	@echo "🎉 All components built successfully!"

# Run tests
test:
	@echo "🧪 Running tests..."
	@cargo test
	@echo "✅ All tests passed!"

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
	@cd cli && cargo clean
	@echo "✅ Clean complete!"

# Run server
run:
	@echo "🚀 Starting HexagonDB server on port 6379..."
	@cargo run --release

# Run CLI
run-cli:
	@echo "🚀 Starting HexagonDB CLI..."
	@cd cli && cargo run --release

# Install binaries to ~/.cargo/bin
install: build-all
	@echo "📦 Installing binaries..."
	@cargo install --path .
	@cargo install --path cli
	@echo "✅ Installed to ~/.cargo/bin"

# Install globally and setup PATH
install-global: install
	@echo "🔧 Setting up global access..."
	@if ! grep -q 'export PATH="$$HOME/.cargo/bin:$$PATH"' ~/.zshrc 2>/dev/null; then \
		echo 'export PATH="$$HOME/.cargo/bin:$$PATH"' >> ~/.zshrc; \
		echo "✅ Added ~/.cargo/bin to PATH in ~/.zshrc"; \
	else \
		echo "✓ PATH already configured in ~/.zshrc"; \
	fi
	@echo ""
	@echo "🎉 Installation complete!"
	@echo "Run: source ~/.zshrc"
	@echo "Then use: hexagondb  or  hexagondb-cli"

# Uninstall binaries
uninstall:
	@echo "🗑️  Uninstalling binaries..."
	@cargo uninstall hexagondb hexagondb-cli || true
	@echo "✅ Uninstalled!"

# Check code with clippy
check:
	@echo "🔍 Running clippy..."
	@cargo clippy --all-targets --all-features -- -D warnings
	@cd cli && cargo clippy --all-targets --all-features -- -D warnings
	@echo "✅ Clippy checks passed!"

# Format code
fmt:
	@echo "🎨 Formatting code..."
	@cargo fmt --all
	@cd cli && cargo fmt --all
	@echo "✅ Code formatted!"

# Show help
help:
	@echo "HexagonDB Build Commands:"
	@echo ""
	@echo "  make build              - Build server (debug)"
	@echo "  make build-release      - Build server (release)"
	@echo "  make build-cli          - Build CLI (debug)"
	@echo "  make build-cli-release  - Build CLI (release)"
	@echo "  make build-all          - Build everything (release)"
	@echo "  make test               - Run tests"
	@echo "  make clean              - Clean build artifacts"
	@echo "  make run                - Run server"
	@echo "  make run-cli            - Run CLI"
	@echo "  make install            - Install binaries"
	@echo "  make check              - Run clippy"
	@echo "  make fmt                - Format code"
	@echo "  make help               - Show this help"
