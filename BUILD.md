# HexagonDB Build Tools

## Quick Start

```bash
# Using Make
make build-all    # Build server + CLI
make run          # Run server
make run-cli      # Run CLI

# Using Just (modern alternative)
just build-all    # Build server + CLI
just run          # Run server
just run-cli      # Run CLI
```

## Makefile Commands

- `make build` - Build server (debug)
- `make build-release` - Build server (release)
- `make build-cli` - Build CLI (debug)
- `make build-cli-release` - Build CLI (release)
- `make build-all` - Build everything (release) ⭐
- `make test` - Run tests
- `make clean` - Clean build artifacts
- `make run` - Run server on port 6379
- `make run-cli` - Run CLI
- `make install` - Install to ~/.cargo/bin
- `make check` - Run clippy
- `make fmt` - Format code
- `make help` - Show help

## Justfile Commands (More Features!)

Install just: `cargo install just`

- `just build-all` - Build everything
- `just dev` - Format + Check + Test (development workflow)
- `just ci` - Format check + Clippy + Test (CI workflow)
- `just watch` - Watch for changes and rebuild
- `just watch-test` - Watch and run tests
- `just bench` - Run benchmarks
- `just doc` - Build and open documentation
- `just list` - Show all commands

## Examples

```bash
# Development workflow
make fmt && make check && make test

# Or with just
just dev

# Build and run
make build-all && make run

# Install globally
make install
hexagondb  # Run from anywhere
hexagondb-cli
```

## CI/CD

```yaml
# GitHub Actions example
- run: make check
- run: make test
- run: make build-all
```
