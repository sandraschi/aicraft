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


## Documentation preservation and hooks

On 2026-11-20 a README "standardization" sweep across the fleet deleted project descriptions,
status sections and user warnings in nine repos. Full rules:
`mcp-central-docs/standards/CONTENT_PRESERVATION_STANDARD.md`.

- Applying any standard or template restructures; it never deletes. Move content (within the file
  or to `docs/` with a link), do not drop it. Never remove what the project is, what it is based on,
  its status (what works, what does not, what is not built) or warnings to users.
- Remove content only when Sandra explicitly asks for that removal, in this repo.
- Never add claims you have not verified (release downloads, versions, test counts, features).
- Only standards with `status: active` may be applied across repos. A change across several repos is
  done on one repo first and approved by Sandra; its commits name the tool and carry a `Sweep-Id:` line.
- `.githooks/commit-msg` blocks a commit that deletes a Markdown file, removes 20% of its lines
  (15 or more) or removes 3 or more headings. A removal Sandra asked for passes with
  `Docs-Removal-Approved: <who and why>` in the message. Enable once per clone:
  `git config core.hooksPath .githooks`. Never use `--no-verify`; if a hook blocks you, stop and ask.
