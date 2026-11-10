//! AICraft FastMCP Server & Universal CLI.
//!
//! Follows the MCP 2024-11-05 standard: stdout is 100% JSON-RPC, diagnostics go to stderr.

use aicraft_core::{
    prompts::{EvalFixture, PromptDepot},
    router::{ModelRouter, ModelTier},
    telemetry::TokenAccountant,
    AICraftStatus, APP_VERSION,
};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

#[derive(Parser)]
#[command(name = "aicraft-mcp")]
#[command(about = "AICraft: Sovereign AI Ops & Model Routing MCP Server", version = APP_VERSION)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    /// Run as MCP 2024-11-05 stdio server
    #[arg(long)]
    mcp: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Check AICraft status and active subsystems
    Status,
    /// List active model inventory across local and cloud providers
    Models,
    /// View token economics and latency telemetry
    Telemetry,
    /// Test smart model routing with a prompt
    Route {
        prompt: String,
        #[arg(long, default_value = "local")]
        tier: String,
    },
    /// Run standard prompt regression evaluations
    Eval,
}

fn main() {
    let cli = Cli::parse();

    if cli.mcp {
        run_mcp_server();
        return;
    }

    match cli.command {
        Some(Commands::Status) => {
            let status = AICraftStatus::default();
            println!("{}", serde_json::to_string_pretty(&status).unwrap_or_default());
        }
        Some(Commands::Models) => {
            let models = ModelRouter::list_models();
            println!("{}", serde_json::to_string_pretty(&models).unwrap_or_default());
        }
        Some(Commands::Telemetry) => {
            let mut accountant = TokenAccountant::new();
            accountant.record("qwen2.5-coder:14b", "ollama", 840, 160, 120, 0.0);
            accountant.record("claude-3-7-sonnet", "anthropic", 2400, 480, 850, 0.014);
            let snap = accountant.snapshot();
            println!("{}", serde_json::to_string_pretty(&snap).unwrap_or_default());
        }
        Some(Commands::Route { prompt, tier }) => {
            let model_tier = match tier.to_lowercase().as_str() {
                "frontier" => ModelTier::Frontier,
                "balanced" => ModelTier::Balanced,
                _ => ModelTier::Local,
            };
            let res = ModelRouter::route(model_tier, &prompt, None);
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Eval) => {
            let prompts = PromptDepot::standard_prompts();
            if let Some(first) = prompts.first() {
                let fixtures = vec![
                    EvalFixture {
                        id: "zero-panic-test".to_string(),
                        input_vars: vec![("CODE".to_string(), "fn test() {}".to_string())],
                        required_keywords: vec!["zero-panic".to_string(), "clippy".to_string()],
                    },
                ];
                let report = PromptDepot::run_evals(first, &fixtures);
                println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
            }
        }
        None => {
            eprintln!("AICraft MCP CLI v{}. Use --mcp for stdio server or --help.", APP_VERSION);
        }
    }
}

fn run_mcp_server() {
    eprintln!("AICraft FastMCP Server starting (MCP 2024-11-05 protocol)...");
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let mut accountant = TokenAccountant::new();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let err = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": { "code": -32700, "message": format!("Parse error: {}", e) }
                });
                let _ = writeln!(out, "{}", err);
                let _ = out.flush();
                continue;
            }
        };

        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

        let resp = match method {
            "initialize" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "serverInfo": {
                            "name": "aicraft-mcp",
                            "version": APP_VERSION
                        },
                        "capabilities": {
                            "tools": {}
                        }
                    }
                })
            }
            "notifications/initialized" => continue,
            "tools/list" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": [
                            {
                                "name": "aicraft_route_completion",
                                "description": "Route completion to optimal model tier (local Ollama weights, balanced cloud, or frontier reasoning) with automatic failover.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "prompt": { "type": "string", "description": "User prompt text" },
                                        "tier": { "type": "string", "enum": ["local", "balanced", "frontier"], "description": "Preferred model tier" },
                                        "system": { "type": "string", "description": "Optional system prompt instruction" }
                                    },
                                    "required": ["prompt"]
                                }
                            },
                            {
                                "name": "aicraft_get_telemetry",
                                "description": "Retrieve real-time token spend, latency, speed (tok/s), and cache hit ratios for active sessions.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {}
                                }
                            },
                            {
                                "name": "aicraft_inventory_models",
                                "description": "List all configured local Ollama models and cloud model providers with availability status.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {}
                                }
                            },
                            {
                                "name": "aicraft_eval_prompt",
                                "description": "Run automated regression tests on a prompt template against benchmark fixtures.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "prompt_id": { "type": "string", "description": "Prompt template identifier" }
                                    }
                                }
                            }
                        ]
                    }
                })
            }
            "tools/call" => {
                let params = req.get("params").cloned().unwrap_or(json!({}));
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                let result_value = match name {
                    "aicraft_route_completion" => {
                        let prompt = args.get("prompt").and_then(|p| p.as_str()).unwrap_or("");
                        let tier_str = args.get("tier").and_then(|t| t.as_str()).unwrap_or("local");
                        let sys = args.get("system").and_then(|s| s.as_str());

                        let tier = match tier_str {
                            "frontier" => ModelTier::Frontier,
                            "balanced" => ModelTier::Balanced,
                            _ => ModelTier::Local,
                        };

                        let res = ModelRouter::route(tier, prompt, sys);
                        accountant.record(&res.model_used, &res.provider, res.prompt_tokens, res.completion_tokens, res.latency_ms, res.cost_usd);
                        json!(res)
                    }
                    "aicraft_get_telemetry" => {
                        let snap = accountant.snapshot();
                        json!(snap)
                    }
                    "aicraft_inventory_models" => {
                        let models = ModelRouter::list_models();
                        json!(models)
                    }
                    "aicraft_eval_prompt" => {
                        let prompts = PromptDepot::standard_prompts();
                        let pid = args.get("prompt_id").and_then(|p| p.as_str()).unwrap_or("code-refactor");
                        let target = prompts.iter().find(|p| p.id == pid).unwrap_or(&prompts[0]);
                        let fixtures = vec![
                            EvalFixture {
                                id: "standard-eval".to_string(),
                                input_vars: vec![],
                                required_keywords: vec!["zero-panic".to_string()],
                            }
                        ];
                        let rep = PromptDepot::run_evals(target, &fixtures);
                        json!(rep)
                    }
                    _ => json!({ "error": format!("Unknown tool: {}", name) }),
                };

                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&result_value).unwrap_or_default()
                            }
                        ]
                    }
                })
            }
            _ => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32601, "message": format!("Method not found: {}", method) }
                })
            }
        };

        let _ = writeln!(out, "{}", resp);
        let _ = out.flush();
    }
}
