//! Prompt depot and regression evals engine.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub version: String,
    pub system_instruction: String,
    pub user_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalFixture {
    pub id: String,
    pub input_vars: Vec<(String, String)>,
    pub required_keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    pub fixture_id: String,
    pub passed: bool,
    pub score: f64,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalReport {
    pub prompt_id: String,
    pub version: String,
    pub total_fixtures: usize,
    pub passed_fixtures: usize,
    pub success_rate: f64,
    pub results: Vec<EvalResult>,
}

pub struct PromptDepot;

impl PromptDepot {
    /// Return default fleet prompt templates.
    pub fn standard_prompts() -> Vec<PromptTemplate> {
        vec![
            PromptTemplate {
                id: "code-refactor".to_string(),
                name: "Zero-Panic Refactoring Assistant".to_string(),
                version: "1.2.0".to_string(),
                system_instruction: "You are an expert systems engineer. Adhere strictly to zero-panic, zero-unwrap clippy guidelines.".to_string(),
                user_template: "Refactor the following code block: {CODE}".to_string(),
            },
            PromptTemplate {
                id: "fleet-auditor".to_string(),
                name: "Fleet Hygiene & AGENTS.md Conformance".to_string(),
                version: "2.0.1".to_string(),
                system_instruction: "Evaluate documentation completeness and port adjacency rules in mcp-central-docs.".to_string(),
                user_template: "Audit the repository: {REPO_PATH}".to_string(),
            },
            PromptTemplate {
                id: "mcp-tool-designer".to_string(),
                name: "FastMCP Compound Tool Synthesizer".to_string(),
                version: "1.0.0".to_string(),
                system_instruction: "Synthesize high-level intent-based FastMCP tool definitions adhering to MCP 2024-11-05 standard.".to_string(),
                user_template: "Design compound tools for capability: {CAPABILITY}".to_string(),
            },
        ]
    }

    /// Evaluate a prompt template against a suite of fixtures.
    pub fn run_evals(prompt: &PromptTemplate, fixtures: &[EvalFixture]) -> EvalReport {
        let mut results = Vec::new();
        let mut passed_count = 0;

        for fixture in fixtures {
            // Simulated evaluation against template
            let mut passed = true;
            let mut missing = Vec::new();

            for kw in &fixture.required_keywords {
                if !prompt.system_instruction.to_lowercase().contains(&kw.to_lowercase())
                    && !prompt.user_template.to_lowercase().contains(&kw.to_lowercase())
                {
                    passed = false;
                    missing.push(kw.clone());
                }
            }

            if passed {
                passed_count += 1;
            }

            results.push(EvalResult {
                fixture_id: fixture.id.clone(),
                passed,
                score: if passed { 1.0 } else { 0.4 },
                notes: if passed {
                    "All assertions met.".to_string()
                } else {
                    format!("Missing expected terms: {:?}", missing)
                },
            });
        }

        let total = fixtures.len().max(1);
        let success_rate = (passed_count as f64 / total as f64) * 100.0;

        EvalReport {
            prompt_id: prompt.id.clone(),
            version: prompt.version.clone(),
            total_fixtures: fixtures.len(),
            passed_fixtures: passed_count,
            success_rate,
            results,
        }
    }
}
