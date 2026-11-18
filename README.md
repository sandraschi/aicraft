# AICraft

![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)
![Rust 1.90+](https://img.shields.io/badge/rust-1.90%2B-orange?logo=rust)
![MCP stdio](https://img.shields.io/badge/MCP-stdio-purple)
![ArtCraft harness](https://img.shields.io/badge/ArtCraft-harness-green)

**Sovereign AI Ops, Model Routing, Shared Provider Client, and Token Telemetry Studio.**

Part of the **Sovereign Craft Suite** and registered in [CraftDeck](https://github.com/sandraschi/craft-launcher). Consolidates `craft-ai` (`crates/craft-ai-core`, merged 2026-11-18 per D2).

## Install

Tell Cursor, Claude, Antigravity, or OpenCode: **Install this.**

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-craft.ps1
```

Installs the Windows app, the CLI, and the MCP bundle.

---

## Architecture & Implementation Status

| Component | Status | Notes |
|---|---|---|
| `crates/craft-ai-core` | Implemented | Shared Rust AI client (`ProviderKind`, `CraftAiConfig`, Ollama/LM Studio/OpenAI/Anthropic/DeepSeek/Meta/OpenRouter provider table, DeepSeek completion/OCR/ASR/research parsers, 12 unit tests). Also used by `codercraft`, `travelcraft`, `writercraft`. |
| `crates/aicraft-core` | Implemented | Model router (`router.rs` with DeepSeek off-peak pricing test), prompt depot (`prompts.rs`), swarm inspector (`swarm.rs`), and token telemetry (`telemetry.rs`). |
| `crates/aicraft-mcp` | Implemented | CLI (`status`, `models`, `telemetry`, `eval`, `--smoke`) and MCP 2024-11-05 stdio server (`aicraft_route_completion`, `aicraft_get_telemetry`, `aicraft_inventory_models`, `aicraft_eval_prompt`). |
| `crates/aicraft-app` | Implemented | Native Rust + egui 0.37 desktop studio with `--smoke` parity. |
| Automated CI / GitHub Actions | Planned | Pending Phase 4 L-F fleet CI rollout. |

---

## Assigned Ports

- **Frontend Port**: `11260`
- **Backend / MCP Port**: `11261`
- Registered in: `mcp-central-docs/operations/WEBAPP_PORTS.md` *(adjacent to DevCraft `11258/11259`)*

---

## Quick Start

```powershell
# Headless smoke test
cargo run -p aicraft-app -- --smoke

# Run the native desktop studio
cargo run -p aicraft-app

# Run as FastMCP stdio server
cargo run -p aicraft-mcp -- --mcp

# Run CLI commands
cargo run -p aicraft-mcp -- status
cargo run -p aicraft-mcp -- models
cargo run -p aicraft-mcp -- telemetry
cargo run -p aicraft-mcp -- eval
```
