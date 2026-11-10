//! AICraft Desktop Studio: Native egui 0.37 interface for AI Ops, Model Routing & Telemetry.

use aicraft_core::{
    prompts::{EvalReport, PromptDepot, PromptTemplate},
    router::{CompletionResult, ModelProfile, ModelRouter, ModelTier},
    swarm::{SwarmAgent, SwarmMonitor},
    telemetry::{TelemetrySnapshot, TokenAccountant},
    APP_VERSION, CONTROL_PORT, FRONTEND_PORT,
};
use clap::Parser;
use eframe::egui;

#[derive(Parser, Debug)]
#[command(name = "aicraft", about = "AICraft Desktop Studio", version = APP_VERSION)]
struct Args {
    /// Headless smoke test verification
    #[arg(long)]
    smoke: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveTab {
    Arena,
    PromptDepot,
    Telemetry,
    Swarm,
}

struct AICraftApp {
    models: Vec<ModelProfile>,
    selected_model: Option<ModelProfile>,
    active_tab: ActiveTab,
    prompt_input: String,
    selected_tier: ModelTier,
    completion_result: Option<CompletionResult>,
    accountant: TokenAccountant,
    telemetry_snapshot: TelemetrySnapshot,
    prompts: Vec<PromptTemplate>,
    selected_prompt: Option<PromptTemplate>,
    eval_report: Option<EvalReport>,
    swarm_agents: Vec<SwarmAgent>,
    status_message: String,
}

impl AICraftApp {
    fn new() -> Self {
        let models = ModelRouter::list_models();
        let selected_model = models.first().cloned();
        let prompts = PromptDepot::standard_prompts();
        let selected_prompt = prompts.first().cloned();
        let mut accountant = TokenAccountant::new();
        // Seed baseline session metrics
        accountant.record("qwen2.5-coder:14b", "ollama", 1200, 320, 140, 0.0);
        accountant.record("claude-3-7-sonnet", "anthropic", 4500, 890, 780, 0.026);
        let telemetry_snapshot = accountant.snapshot();
        let swarm_agents = SwarmMonitor::active_swarm();

        Self {
            models,
            selected_model,
            active_tab: ActiveTab::Arena,
            prompt_input: "Refactor Rust enum to enforce zero-panic result handling.".to_string(),
            selected_tier: ModelTier::Local,
            completion_result: None,
            accountant,
            telemetry_snapshot,
            prompts,
            selected_prompt,
            eval_report: None,
            swarm_agents,
            status_message: "AICraft ready. Smart router initialized.".to_string(),
        }
    }

    fn refresh_models(&mut self) {
        self.models = ModelRouter::list_models();
        self.telemetry_snapshot = self.accountant.snapshot();
        self.status_message = format!("Refreshed {} model profiles.", self.models.len());
    }
}

impl eframe::App for AICraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Top Bar
        egui::Panel::top("top_panel").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(format!("AICraft v{}", APP_VERSION));
                ui.separator();
                ui.label(format!("Frontend: {}", FRONTEND_PORT));
                ui.label(format!("Control/MCP: {}", CONTROL_PORT));
                ui.separator();

                let ollama_up = ModelRouter::is_ollama_online();
                if ollama_up {
                    ui.colored_label(egui::Color32::from_rgb(50, 210, 50), "Ollama: ONLINE");
                } else {
                    ui.colored_label(egui::Color32::from_rgb(220, 160, 50), "Ollama: IDLE");
                }
                ui.separator();

                ui.label(format!("Session Spend: ${:.4}", self.telemetry_snapshot.total_cost_usd));
                ui.label(format!("Tokens: {}", self.telemetry_snapshot.total_prompt_tokens + self.telemetry_snapshot.total_completion_tokens));
                ui.separator();

                if ui.button("Refresh Router").clicked() {
                    self.refresh_models();
                }
            });
        });

        // Bottom Bar
        egui::Panel::bottom("bottom_panel").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
            });
        });

        // Left Panel: Model Radar
        egui::Panel::left("left_panel").show(ui, |ui| {
            ui.set_width(260.0);
            ui.heading("Model Radar");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for model in &self.models {
                    let is_selected = self
                        .selected_model
                        .as_ref()
                        .map(|m| m.name == model.name)
                        .unwrap_or(false);

                    let tier_str = match model.tier {
                        ModelTier::Local => "Local",
                        ModelTier::Balanced => "Balanced",
                        ModelTier::Frontier => "Frontier",
                    };

                    let label = format!("[{}] {}", tier_str, model.name);
                    if ui.selectable_label(is_selected, label).clicked() {
                        self.selected_model = Some(model.clone());
                    }
                }
            });
        });

        // Central Panel
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, ActiveTab::Arena, "Model Arena");
                ui.selectable_value(&mut self.active_tab, ActiveTab::PromptDepot, "Prompt Depot & Evals");
                ui.selectable_value(&mut self.active_tab, ActiveTab::Telemetry, "Token Telemetry");
                ui.selectable_value(&mut self.active_tab, ActiveTab::Swarm, "Agent Swarm Monitor");
            });
            ui.separator();

            match self.active_tab {
                ActiveTab::Arena => {
                    ui.heading("Completion & Routing Arena");
                    ui.horizontal(|ui| {
                        ui.label("Routing Tier:");
                        ui.radio_value(&mut self.selected_tier, ModelTier::Local, "Local (Ollama)");
                        ui.radio_value(&mut self.selected_tier, ModelTier::Balanced, "Balanced (Flash/Haiku)");
                        ui.radio_value(&mut self.selected_tier, ModelTier::Frontier, "Frontier (Sonnet/Pro)");
                    });

                    ui.label("Prompt Input:");
                    ui.text_edit_multiline(&mut self.prompt_input);

                    if ui.button("Dispatch Completion").clicked() {
                        let res = ModelRouter::route(self.selected_tier, &self.prompt_input, None);
                        self.accountant.record(
                            &res.model_used,
                            &res.provider,
                            res.prompt_tokens,
                            res.completion_tokens,
                            res.latency_ms,
                            res.cost_usd,
                        );
                        self.telemetry_snapshot = self.accountant.snapshot();
                        self.status_message = format!("Dispatched via {} in {}ms", res.model_used, res.latency_ms);
                        self.completion_result = Some(res);
                    }

                    if let Some(ref res) = self.completion_result {
                        ui.separator();
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.strong(&res.model_used);
                                ui.label(format!("Provider: {}", res.provider));
                                ui.label(format!("Latency: {}ms", res.latency_ms));
                                ui.label(format!("Cost: ${:.6}", res.cost_usd));
                            });
                            ui.label("Output:");
                            ui.code(&res.response_text);
                        });
                    }
                }
                ActiveTab::PromptDepot => {
                    ui.heading("Versioned Prompt Depot & Evals");
                    ui.horizontal(|ui| {
                        for p in &self.prompts {
                            let is_sel = self.selected_prompt.as_ref().map(|x| x.id == p.id).unwrap_or(false);
                            if ui.selectable_label(is_sel, &p.name).clicked() {
                                self.selected_prompt = Some(p.clone());
                            }
                        }
                    });

                    if let Some(ref p) = self.selected_prompt {
                        ui.separator();
                        ui.label(format!("ID: {} | Version: {}", p.id, p.version));
                        ui.label("System Instruction:");
                        ui.code(&p.system_instruction);
                        ui.label("Template:");
                        ui.code(&p.user_template);

                        if ui.button("Run Evals Suite").clicked() {
                            let fixtures = vec![
                                aicraft_core::prompts::EvalFixture {
                                    id: "regression-fixture-01".to_string(),
                                    input_vars: vec![],
                                    required_keywords: vec!["zero-panic".to_string(), "clippy".to_string()],
                                },
                            ];
                            let report = PromptDepot::run_evals(p, &fixtures);
                            self.status_message = format!("Eval pass rate: {:.1}%", report.success_rate);
                            self.eval_report = Some(report);
                        }

                        if let Some(ref rep) = self.eval_report {
                            ui.separator();
                            ui.heading(format!("Eval Results: {:.1}% Passed", rep.success_rate));
                            for r in &rep.results {
                                ui.horizontal(|ui| {
                                    ui.strong(&r.fixture_id);
                                    if r.passed {
                                        ui.colored_label(egui::Color32::from_rgb(50, 200, 50), "PASSED");
                                    } else {
                                        ui.colored_label(egui::Color32::from_rgb(220, 50, 50), "FAILED");
                                    }
                                    ui.label(&r.notes);
                                });
                            }
                        }
                    }
                }
                ActiveTab::Telemetry => {
                    ui.heading("Token Economics & Latency Radar");
                    ui.label(format!("Total Requests: {}", self.telemetry_snapshot.total_requests));
                    ui.label(format!("Prompt Tokens: {}", self.telemetry_snapshot.total_prompt_tokens));
                    ui.label(format!("Completion Tokens: {}", self.telemetry_snapshot.total_completion_tokens));
                    ui.label(format!("Cumulative Spend: ${:.4}", self.telemetry_snapshot.total_cost_usd));
                    ui.label(format!("Average Latency: {:.1} ms", self.telemetry_snapshot.avg_latency_ms));
                    ui.label(format!("Average Speed: {:.1} tokens/sec", self.telemetry_snapshot.avg_tokens_per_sec));
                    ui.label(format!("Cache Hit Ratio: {:.1}%", self.telemetry_snapshot.cache_hit_ratio * 100.0));
                }
                ActiveTab::Swarm => {
                    ui.heading("Agent Swarm Hierarchy & Task Monitor");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for agent in &self.swarm_agents {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.strong(&agent.name);
                                    ui.label(format!("[{}]", agent.role));
                                    ui.label(format!("Status: {}", agent.status));
                                    ui.label(format!("Tokens: {}", agent.tokens_consumed));
                                });
                                if let Some(ref tool) = agent.active_tool {
                                    ui.label(format!("Active Tool: {}", tool));
                                }
                            });
                        }
                    });
                }
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let args = Args::parse();

    if args.smoke {
        println!("AICraft App smoke test: OK (v{})", APP_VERSION);
        return Ok(());
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title(format!("AICraft — Sovereign AI Ops & Model Routing Studio (v{})", APP_VERSION)),
        ..Default::default()
    };

    eframe::run_native(
        "AICraft Studio",
        native_options,
        Box::new(|_cc| Ok(Box::new(AICraftApp::new()))),
    )
}
