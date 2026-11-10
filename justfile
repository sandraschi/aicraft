default:
    @just --list

# Run headless smoke test
smoke:
    cargo run -p aicraft-app -- --smoke

# Run desktop studio
run:
    cargo run -p aicraft-app

# Run MCP stdio server
mcp:
    cargo run -p aicraft-mcp -- --mcp

# Build all workspace targets
build:
    cargo build --workspace

# Run tests
test:
    cargo test --workspace

# Check clippy lints
lint:
    cargo clippy --workspace --all-targets -- -D warnings
