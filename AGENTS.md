# AICraft — Sovereign AI Ops & Model Routing Studio

## Overview
AICraft is the AI operations and model routing cockpit of the Sovereign Craft Suite. It unifies local inference weights (Ollama, vLLM, llama.cpp), cloud frontier APIs (Claude, Gemini, DeepSeek), token economics telemetry, and prompt eval regression testing into a native Rust + egui 0.37 desktop studio and compound FastMCP server.

It acts as the cognitive intelligence counterpart to DevCraft's workspace infrastructure.

## Architecture & Crates
- `crates/aicraft-core`:
  - `router`: Probing local inference engines (`http://127.0.0.1:11434`), smart model tiering (Local, Balanced, Frontier), and automated failover.
  - `telemetry`: Token consumption tracking, tokens/sec rate, latency, and cost calculations across providers.
  - `prompts`: Local versioned prompt storage and fixture-based eval test runner.
  - `swarm`: Agent hierarchy visualizer and tool-call trace recorder.
- `crates/aicraft-mcp`: FastMCP stdio JSON-RPC 2.0 server (`aicraft-mcp --mcp`) exposing compound AI tools.
- `crates/aicraft-app`: Native immediate-mode egui 0.37 desktop workstation with Model Radar, Arena, Telemetry, and Prompt Depot panes.

## Assigned Ports
- Frontend Port: `11260`
- Backend / MCP Control Port: `11261`
- Registered in: `mcp-central-docs/operations/WEBAPP_PORTS.md`

## Commands
```powershell
# Headless smoke test
cargo run -p aicraft-app -- --smoke

# Run Desktop Studio
cargo run -p aicraft-app

# Start FastMCP stdio server
cargo run -p aicraft-mcp -- --mcp
```
