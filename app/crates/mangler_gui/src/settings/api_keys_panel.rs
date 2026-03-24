/// API keys settings panel — a modal window for entering and saving
/// API keys used by AI operations (e.g. OpenAI).

use eframe::egui;
use crate::config::AppConfig;

/// State for the API keys panel window.
pub struct ApiKeysPanel {
    /// Whether the panel window is open.
    pub open: bool,

    /// The OpenAI API key text field value.
    openai_key: String,

    /// Whether to show the key in plain text or masked.
    show_key: bool,

    /// Per-session AI cost limit text field (edited as string, parsed to f64).
    cost_limit_text: String,

    /// Status message shown after save.
    status_message: Option<String>,
}

impl ApiKeysPanel {
    /// Create a new panel, loading current keys from config.
    pub fn new() -> Self {
        let config = AppConfig::load();
        let cost_limit_text = if config.ai_cost_limit > 0.0 {
            format!("{:.2}", config.ai_cost_limit)
        } else {
            String::new()
        };
        Self {
            open: false,
            openai_key: config.api_keys.openai,
            show_key: false,
            cost_limit_text,
            status_message: None,
        }
    }

    /// Show the API keys window. Call this each frame from the main app.
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        let mut still_open = self.open;

        egui::Window::new("API Keys")
            .open(&mut still_open)
            .resizable(false)
            .collapsible(false)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.label("Configure API keys for AI operations. Keys are stored locally in your app config file.");
                ui.add_space(16.0);

                // OpenAI section
                ui.heading("OpenAI");
                ui.add_space(4.0);
                ui.label("Used for DALL-E image generation, AI edit, and AI variation nodes.");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("API Key:");
                    if self.show_key {
                        // Show key in plain text.
                        ui.text_edit_singleline(&mut self.openai_key);
                    } else {
                        // Show masked version — use a password field.
                        let mut masked = self.openai_key.clone();
                        let response = ui.add(egui::TextEdit::singleline(&mut masked).password(true));
                        if response.changed() {
                            self.openai_key = masked;
                        }
                    }
                });

                ui.add_space(4.0);
                ui.checkbox(&mut self.show_key, "show key");

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                // Cost limit section
                ui.heading("Cost Limit");
                ui.add_space(4.0);
                ui.label("Maximum AI spend per session (USD). Leave empty for no limit.");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Limit: $");
                    ui.add(egui::TextEdit::singleline(&mut self.cost_limit_text).desired_width(80.0));
                });

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    // Save button
                    if ui.button("save").clicked() {
                        let mut config = AppConfig::load();
                        config.api_keys.openai = self.openai_key.trim().to_string();

                        // Parse cost limit — empty or invalid means 0 (no limit).
                        config.ai_cost_limit = self.cost_limit_text.trim().parse::<f64>().unwrap_or(0.0).max(0.0);

                        config.save();
                        config.apply_api_keys_to_env();
                        config.apply_ai_cost_limit();
                        self.status_message = Some("Saved.".to_string());
                    }

                    // Show status message
                    if let Some(msg) = &self.status_message {
                        ui.label(msg);
                    }
                });

                ui.add_space(4.0);
                ui.label("You can also set the OPENAI_API_KEY environment variable instead.");
            });

        self.open = still_open;
    }
}
