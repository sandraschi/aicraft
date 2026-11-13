# AICraft 🧠

![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)
![Rust 1.90+](https://img.shields.io/badge/rust-1.90%2B-orange?logo=rust)
![MCP stdio](https://img.shields.io/badge/MCP-stdio-purple)
![ArtCraft harness](https://img.shields.io/badge/ArtCraft-harness-green)

**Sovereign AI Ops, Model Routing & Token Telemetry Studio.**

Part of the **Sovereign Craft Suite** and registered in [CraftDeck](file:///d:/Dev/repos/craft-deck).

---

## What is AICraft?

AICraft is the cognitive operations studio of the Sovereign Craft Suite. Working hand-in-hand with [DevCraft](file:///d:/Dev/repos/devcraft), AICraft consolidates local inference engines, cloud frontier models, token accounting, and prompt regression testing into:

1. **A Native Desktop Studio (`crates/aicraft-app`)**:
   - Built in **Rust + egui 0.37** with zero-panic / zero-unwrap containment.
   - **Model Radar**: Probes local weights (Ollama `127.0.0.1:11434`, vLLM, llama.cpp) alongside cloud endpoints (Claude, Gemini, DeepSeek).
   - **Model Arena**: Side-by-side prompt synthesis with tier-based failover (Local $\rightarrow$ Balanced $\rightarrow$ Frontier).
   - **Token Economics & Latency Radar**: Live metrics for $t_{\text{first-token}}$, generation speed (tok/s), context-window meters, and cumulative session spend.
   - **Prompt Depot & Evals**: Versioned local prompt repository with automated regression fixtures.
   - **Swarm Monitor**: Visual hierarchy of active agents, subagents, and tool-call traces.
2. **A Compound FastMCP Server (`crates/aicraft-mcp`)**:
   - Implements the MCP 2024-11-05 protocol over stdio.
   - Exposes high-level intent tools: `aicraft_route_completion`, `aicraft_get_telemetry`, `aicraft_inventory_models`, `aicraft_eval_prompt`.

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
