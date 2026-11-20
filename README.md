# AICraft

Local AI ops, model routing, shared provider client, and token telemetry studio for the Craft fleet.

![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)
![Rust 1.90+](https://img.shields.io/badge/rust-1.90%2B-orange?logo=rust)
![MCP stdio](https://img.shields.io/badge/MCP-stdio-purple)
![ArtCraft harness](https://img.shields.io/badge/ArtCraft-harness-green)

Consolidates `craft-ai` (`crates/craft-ai-core`, merged 2026-11-18).

## What You Can Do

- **Unified Provider Client (`crates/craft-ai-core`)**: Connect to local (Ollama, LM Studio, vLLM, `llama.cpp`) and cloud (OpenAI, Anthropic, DeepSeek, Meta, OpenRouter) providers via `ProviderKind` and `CraftAiConfig`.
- **Model Routing & Off-Peak Pricing (`crates/aicraft-core`)**: Route completions, manage the prompt depot (`prompts.rs`), inspect swarms (`swarm.rs`), and track token telemetry (`telemetry.rs`).
- **Desktop Studio & CLI (`crates/aicraft-app`, `crates/aicraft-mcp`)**: Native `egui 0.37` desktop workstation with `--smoke` parity, CLI (`status`, `models`, `telemetry`, `eval`), and MCP `2024-11-05` stdio server (`aicraft_route_completion`, `aicraft_get_telemetry`, `aicraft_inventory_models`, `aicraft_eval_prompt`).

## Install

1. **Windows Installer (Recommended):** Download `aicraft-0.1.0-setup.exe` from [GitHub Releases](https://github.com/sandraschi/aicraft/releases).
2. **Local Install Script:**

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-craft.ps1
```

## Quick Start

```powershell
cargo run -p aicraft-app -- --smoke
cargo run -p aicraft-app
cargo run -p aicraft-mcp -- --mcp
cargo run -p aicraft-mcp -- status
```

## Workspace

| Component | Role |
|---|---|
| `crates/craft-ai-core` | Shared Rust AI client (`ProviderKind`, `CraftAiConfig`, DeepSeek parsers, 12 unit tests) |
| `crates/aicraft-core` | Model router, prompt depot, swarm inspector, and token telemetry |
| `crates/aicraft-mcp` | CLI and MCP `2024-11-05` stdio server |
| `crates/aicraft-app` | Native `egui 0.37` desktop studio |
