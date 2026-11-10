//! AICraft Core Engine: Smart model routing, token economics telemetry,
//! prompt evals, and agent swarm inspection.
//!
//! Enforces zero-panic, zero-unwrap containment across all modules.

pub mod prompts;
pub mod router;
pub mod swarm;
pub mod telemetry;

pub const APP_VERSION: &str = "0.1.0";
pub const FRONTEND_PORT: u16 = 11260;
pub const CONTROL_PORT: u16 = 11261;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Resolve the fleet workspace root (`d:\Dev\repos`) whether called from root or inside `aicraft`.
pub fn resolve_workspace_root() -> PathBuf {
    if Path::new("mcp-central-docs").exists() {
        PathBuf::from(".")
    } else if Path::new("../mcp-central-docs").exists() {
        PathBuf::from("..")
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AICraftStatus {
    pub version: String,
    pub control_port: u16,
    pub frontend_port: u16,
    pub active_subsystems: Vec<String>,
}

impl Default for AICraftStatus {
    fn default() -> Self {
        Self {
            version: APP_VERSION.to_string(),
            control_port: CONTROL_PORT,
            frontend_port: FRONTEND_PORT,
            active_subsystems: vec![
                "model_router_and_failover".to_string(),
                "token_economics_telemetry".to_string(),
                "prompt_depot_and_evals".to_string(),
                "agent_swarm_monitor".to_string(),
            ],
        }
    }
}
