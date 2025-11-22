# HexagonDB Justfile
# Modern build tool alternative to Make
# Install: cargo install just
# Usage: just <command>

# Default recipe (runs when you type 'just')
default: build

# Build server in debug mode
build:
    @echo "🔨 Building HexagonDB server (debug)..."
    cargo build
    @echo "✅ Server built: target/debug/hexagondb"

# Build server in release mode
build-release:
    @echo "🔨 Building HexagonDB server (release)..."
    cargo build --release
    @echo "✅ Server built: target/release/hexagondb"

# Build CLI in debug mode
build-cli:
    @echo "🔨 Building HexagonDB CLI (debug)..."
    cd cli && cargo build
    @echo "✅ CLI built: cli/target/debug/hexagondb-cli"

# Build CLI in release mode
build-cli-release:
    @echo "🔨 Building HexagonDB CLI (release)..."
    cd cli && cargo build --release
    @echo "✅ CLI built: cli/target/release/hexagondb-cli"

# Build everything (server + CLI) in release mode
build-all: build-release build-cli-release
    @echo "🎉 All components built successfully!"

# Run tests
test:
    @echo "🧪 Running tests..."
    cargo test
    @echo "✅ All tests passed!"

# Run tests with output
test-verbose:
    @echo "🧪 Running tests (verbose)..."
    cargo test -- --nocapture

# Clean build artifacts
clean:
    @echo "🧹 Cleaning build artifacts..."
    cargo clean
    cd cli && cargo clean
    @echo "✅ Clean complete!"

# Run server
run:
    @echo "🚀 Starting HexagonDB server on port 6379..."
    cargo run --release

# Run server in debug mode
run-debug:
    @echo "🚀 Starting HexagonDB server (debug)..."
    cargo run

# Run CLI
run-cli:
    @echo "🚀 Starting HexagonDB CLI..."
    cd cli && cargo run --release

# Run CLI in debug mode
run-cli-debug:
    @echo "🚀 Starting HexagonDB CLI (debug)..."
    cd cli && cargo run

# Install binaries to ~/.cargo/bin
install: build-all
    @echo "📦 Installing binaries..."
    cargo install --path .
    cargo install --path cli
    @echo "✅ Installed to ~/.cargo/bin"

# Check code with clippy
check:
    @echo "🔍 Running clippy..."
    cargo clippy --all-targets --all-features -- -D warnings
    cd cli && cargo clippy --all-targets --all-features -- -D warnings
    @echo "✅ Clippy checks passed!"

# Format code
fmt:
    @echo "🎨 Formatting code..."
    cargo fmt --all
    cd cli && cargo fmt --all
    @echo "✅ Code formatted!"

# Check formatting without changing files
fmt-check:
    @echo "🔍 Checking code formatting..."
    cargo fmt --all -- --check
    cd cli && cargo fmt --all -- --check

# Run benchmarks
bench:
    @echo "⚡ Running benchmarks..."
    cargo bench

# Update dependencies
update:
    @echo "📦 Updating dependencies..."
    cargo update
    cd cli && cargo update
    @echo "✅ Dependencies updated!"

# Build documentation
doc:
    @echo "📚 Building documentation..."
    cargo doc --no-deps --open

# Development workflow: format, check, test
dev: fmt check test
    @echo "✅ Development checks complete!"

# CI workflow: format check, clippy, test
ci: fmt-check check test
    @echo "✅ CI checks complete!"

# Show all available commands
list:
    @just --list

# Watch for changes and rebuild
watch:
    @echo "👀 Watching for changes..."
    cargo watch -x build

# Watch and run tests on changes
watch-test:
    @echo "👀 Watching for changes and running tests..."
    cargo watch -x test
