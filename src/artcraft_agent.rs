// Global Craft Suite AI Agent — 1:1 "Gemini in Google Workspace" Engine & Pure BYOK Architecture
// Synchronized across the Craft Suite via shared_custom_models.json.

use egui::{Color32, CornerRadius, FontFamily, FontId, Rect, RichText, ScrollArea, Stroke, TextureHandle, pos2, vec2};
use egui::text::{LayoutJob, TextFormat};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

pub const APP_NAME: &str = "RecordCraft";
pub const APP_ROLE: &str = "Capture & Studio Engineer";
pub const PANEL_ID: &str = "recordcraft_ai_sidebar";
pub const APP_COMMANDS_HINT: &str = "startRecording / recording.start, stopRecording / recording.stop, selectCaptureSource / capture.selectSource, setEncoderSettings / encoder.setPreset, setResolutionFps / encoder.setResolutionFps, muteMic / audio.setMicMute, setAudioGain / audio.setGain, addMarker, openInCutCraft / project.openInCutCraft, inspectStudio";

pub const QUICK_CHIPS: [(&str, &str, &str); 4] = [
    ("⏺️ Start Recording", "Begin live capture", "Start screen recording in RecordCraft now."),
    ("⏹️ Stop Recording", "End & save recording", "Stop the current recording and save it to the Google Drive project folder."),
    ("⚡ 60 FPS & HEVC", "Optimize hardware encoder", "Set recording to 60 FPS with HEVC hardware encoder."),
    ("🎬 Open in CutCraft", "Edit directly in editor", "Open the latest recording directly for editing in CutCraft."),
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomBot {
    pub id: String,
    pub name: String,
    #[serde(rename = "model_id", alias = "api_model_id", alias = "modelId", alias = "model", default)]
    pub api_model_id: String,
    #[serde(alias = "baseUrl", alias = "base_url", alias = "endpoint", default)]
    pub endpoint: String,
    #[serde(alias = "apiKey", alias = "api_key", default)]
    pub api_key: String,
    #[serde(default)]
    pub used_tokens: usize,
    #[serde(default)]
    pub max_quota_tokens: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SharedCustomBots {
    pub bots: Vec<CustomBot>,
}

fn shared_bots_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(user_profile) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        let p = PathBuf::from(&user_profile);
        paths.push(p.join(".craft-suite").join("shared-ai-config.json"));
        paths.push(p.join(".artcraft").join("shared_custom_models.json"));
    }
    if let Some(app_data) = std::env::var_os("APPDATA") {
        let p = PathBuf::from(app_data);
        paths.push(p.join("CraftSuite").join("shared-ai-config.json"));
        paths.push(p.join("ArtCraft").join("shared_custom_models.json"));
    }
    paths
}

pub fn load_shared_bots() -> Vec<CustomBot> {
    for p in shared_bots_paths() {
        if p.exists() {
            if let Ok(data) = std::fs::read_to_string(&p) {
                if let Ok(shared) = serde_json::from_str::<SharedCustomBots>(&data) {
                    if !shared.bots.is_empty() {
                        return shared.bots;
                    }
                }
                if let Ok(bots) = serde_json::from_str::<Vec<CustomBot>>(&data) {
                    if !bots.is_empty() {
                        return bots;
                    }
                }
            }
        }
    }
    Vec::new()
}

pub fn save_shared_bots(bots: &[CustomBot]) {
    let shared = SharedCustomBots { bots: bots.to_vec() };
    if let Ok(json) = serde_json::to_string_pretty(&shared) {
        for p in shared_bots_paths() {
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&p, &json);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing)]
    pub executed_commands: Vec<String>,
    #[serde(default, skip_serializing)]
    pub manually_inserted: bool,
}

pub struct ToolExecutionRequest {
    pub tool_name: String,
    pub params: Value,
    pub reply_sender: Sender<Result<Value, String>>,
}

pub enum AgentEvent {
    StatusUpdate(String),
    AssistantReply { text: String, executed_commands: Vec<String> },
    ToolExecutionLog { tool: String, args: String, result: String },
    TokensUsed { bot_id: String, tokens: usize },
    Error(String),
    Finished,
}

pub enum AgentWorkerCmd {
    RunPrompt {
        prompt: String,
        messages: Vec<ChatMessage>,
        bot: CustomBot,
        doc_text: String,
        project_dir_str: String,
    },
}

const RAW_AI: &[u8] = include_bytes!("../assets/ai_icons/AI_ICON.rgba");
const RAW_DEEPSEEK: &[u8] = include_bytes!("../assets/ai_icons/DEEPSEEK_ICON.rgba");
const RAW_GEMINI: &[u8] = include_bytes!("../assets/ai_icons/GEMINI_ICON.rgba");
const RAW_GPT: &[u8] = include_bytes!("../assets/ai_icons/GPT_ICON.rgba");
const RAW_LLAMA: &[u8] = include_bytes!("../assets/ai_icons/LLAMA_ICON.rgba");
const RAW_MISTRAL: &[u8] = include_bytes!("../assets/ai_icons/MISTRAL_ICON.rgba");
const RAW_QWEN: &[u8] = include_bytes!("../assets/ai_icons/QWEN_ICON.rgba");
const RAW_CLAUDE: &[u8] = include_bytes!("../assets/ai_icons/CLAUDE_ICON.rgba");

#[derive(Default)]
pub struct ModelIconCache {
    textures: HashMap<String, TextureHandle>,
}

impl ModelIconCache {
    pub fn get_for_str(&mut self, ctx: &egui::Context, id: &str) -> TextureHandle {
        let lower = id.to_lowercase();
        let bucket = if lower.contains("deepseek") {
            "deepseek"
        } else if lower.contains("gemini") || lower.contains("google") {
            "gemini"
        } else if lower.contains("openai") || lower.contains("gpt") || lower.contains("o1") || lower.contains("o3") {
            "gpt"
        } else if lower.contains("claude") || lower.contains("anthropic") {
            "claude"
        } else if lower.contains("llama") || lower.contains("meta") || lower.contains("ollama") || lower.contains("groq") {
            "llama"
        } else if lower.contains("mistral") || lower.contains("codestral") {
            "mistral"
        } else if lower.contains("qwen") {
            "qwen"
        } else {
            "ai_icon"
        };

        if let Some(tex) = self.textures.get(bucket) {
            return tex.clone();
        }

        let raw_bytes: &[u8] = match bucket {
            "deepseek" => RAW_DEEPSEEK,
            "gemini" => RAW_GEMINI,
            "gpt" => RAW_GPT,
            "llama" => RAW_LLAMA,
            "mistral" => RAW_MISTRAL,
            "qwen" => RAW_QWEN,
            "claude" => RAW_CLAUDE,
            _ => RAW_AI,
        };

        let img = egui::ColorImage::from_rgba_unmultiplied([128, 128], raw_bytes);
        let tex = ctx.load_texture(format!("craft_ai_provider_{bucket}"), img, egui::TextureOptions::LINEAR);
        self.textures.insert(bucket.to_string(), tex.clone());
        tex
    }
}

pub struct ArtCraftAgent {
    pub is_open: bool,
    pub is_busy: bool,
    pub status_message: String,
    pub input_text: String,
    pub messages: Vec<ChatMessage>,
    pub tool_execution_logs: Vec<String>,
    pub custom_bots: Vec<CustomBot>,
    pub selected_bot_id: Option<String>,
    pub show_add_api_modal: bool,
    pub show_manage_apis_modal: bool,
    pub show_model_popover: bool,
    pub editing_bot_id: Option<String>,
    pub new_bot_name: String,
    pub new_bot_model_id: String,
    pub new_bot_endpoint: String,
    pub new_bot_key: String,
    pub preset_hint_endpoint: String,
    pub preset_hint_model: String,
    pub show_api_key_plain: bool,
    pub test_connection_status: Option<(bool, String)>,
    pub copied_feedback: Option<(usize, std::time::Instant)>,
    pub applied_feedback: Option<(usize, String, std::time::Instant)>,
    pub icon_cache: ModelIconCache,
    worker_cmd_tx: Option<Sender<AgentWorkerCmd>>,
    event_rx: Option<Receiver<AgentEvent>>,
    tool_req_rx: Option<Receiver<ToolExecutionRequest>>,
    #[allow(dead_code)]
    direct_tool_tx: Option<Sender<ToolExecutionRequest>>,
}

impl Default for ArtCraftAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtCraftAgent {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = channel();
        let (event_tx, event_rx) = channel();
        let (tool_tx, tool_rx) = channel();
        let direct_tool_tx = tool_tx.clone();

        std::thread::Builder::new()
            .name("craft_ai_agent_worker".to_string())
            .spawn(move || {
                worker_loop(cmd_rx, tool_tx, event_tx);
            })
            .expect("failed to spawn craft ai agent worker thread");

        let custom_bots = load_shared_bots();
        let selected_bot_id = custom_bots.first().map(|b| b.id.clone());

        Self {
            is_open: false,
            is_busy: false,
            status_message: "Ready".to_string(),
            input_text: String::new(),
            messages: Vec::new(),
            tool_execution_logs: Vec::new(),
            custom_bots,
            selected_bot_id,
            show_add_api_modal: false,
            show_manage_apis_modal: false,
            show_model_popover: false,
            editing_bot_id: None,
            new_bot_name: String::new(),
            new_bot_model_id: String::new(),
            new_bot_endpoint: String::new(),
            new_bot_key: String::new(),
            preset_hint_endpoint: "https://api.deepseek.com/chat/completions".to_string(),
            preset_hint_model: "e.g. deepseek-chat, gpt-4o, gemini-2.5-flash, llama3.1".to_string(),
            show_api_key_plain: false,
            test_connection_status: None,
            copied_feedback: None,
            applied_feedback: None,
            icon_cache: ModelIconCache::default(),
            worker_cmd_tx: Some(cmd_tx),
            event_rx: Some(event_rx),
            tool_req_rx: Some(tool_rx),
            direct_tool_tx: Some(direct_tool_tx),
        }
    }

    pub fn active_bot(&self) -> Option<&CustomBot> {
        if let Some(ref id) = self.selected_bot_id {
            if let Some(b) = self.custom_bots.iter().find(|bot| &bot.id == id) {
                return Some(b);
            }
        }
        self.custom_bots.first()
    }

    #[allow(dead_code)]
    pub fn dispatch_direct_tool(&mut self, tool_name: &str, params: Value, msg_idx: usize, feedback_label: &str) {
        if let Some(ref tx) = self.direct_tool_tx {
            let (reply_tx, _reply_rx) = channel();
            let _ = tx.send(ToolExecutionRequest {
                tool_name: tool_name.to_string(),
                params,
                reply_sender: reply_tx,
            });
            self.applied_feedback = Some((msg_idx, feedback_label.to_string(), std::time::Instant::now()));
        }
    }

    pub fn take_tool_requests(&mut self) -> Vec<ToolExecutionRequest> {
        let mut list = Vec::new();
        if let Some(ref rx) = self.tool_req_rx {
            while let Ok(req) = rx.try_recv() {
                list.push(req);
            }
        }
        list
    }

    pub fn process_events(&mut self) {
        let mut finished = false;
        let mut save_needed = false;
        if let Some(ref rx) = self.event_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    AgentEvent::StatusUpdate(s) => {
                        self.status_message = s;
                    }
                    AgentEvent::AssistantReply { text, executed_commands } => {
                        self.messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            tool_calls: None,
                            tool_call_id: None,
                            name: None,
                            content: text,
                            executed_commands,
                            manually_inserted: false,
                        });
                    }
                    AgentEvent::ToolExecutionLog { tool, args, result } => {
                        self.tool_execution_logs.push(format!("⚡ {tool} ({args}) → {result}"));
                    }
                    AgentEvent::TokensUsed { bot_id, tokens } => {
                        if let Some(bot) = self.custom_bots.iter_mut().find(|b| b.id == bot_id) {
                            bot.used_tokens += tokens;
                            save_needed = true;
                        }
                    }
                    AgentEvent::Error(err) => {
                        self.messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            tool_calls: None,
                            tool_call_id: None,
                            name: None,
                            content: format!("⚠️ Error: {err}"),
                            executed_commands: Vec::new(),
                            manually_inserted: false,
                        });
                        finished = true;
                    }
                    AgentEvent::Finished => {
                        finished = true;
                    }
                }
            }
        }
        if save_needed {
            save_shared_bots(&self.custom_bots);
        }
        if finished {
            self.is_busy = false;
            self.status_message = "Ready".to_string();
        }
    }

    pub fn render_sidebar(&mut self, ui: &mut egui::Ui, summary: &str, project_dir_str: &str) {
        self.process_events();

        if !self.is_open {
            self.show_model_popover = false;
            return;
        }

        let mut pending_prompt_to_send: Option<String> = None;

        // Pre-clone active bot data to avoid borrow conflicts
        let active_bot_clone = self.active_bot().cloned();
        let bots_snapshot: Vec<CustomBot> = self.custom_bots.clone();

        let sidebar_frame = egui::Frame::new()
            .fill(Color32::from_rgb(16, 19, 28))
            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 20)))
            .inner_margin(egui::Margin::ZERO);

        let mut sidebar_rect = Rect::NOTHING;
        let mut prompt_box_top_y = 0.0f32;
        let mut pill_click_rect = Rect::NOTHING;

        egui::Panel::right(PANEL_ID)
            .resizable(true)
            .default_size(365.0)
            .min_size(335.0)
            .max_size(600.0)
            .frame(sidebar_frame)
            .show(ui, |ui| {
                sidebar_rect = ui.max_rect();

                // =============================================================
                // A. SLIM SIDEBAR HEADER (Top Docked inside Sidebar)
                // =============================================================
                egui::Panel::top(format!("{PANEL_ID}_header"))
                    .resizable(false)
                    .exact_size(54.0)
                    .frame(
                        egui::Frame::new()
                            .fill(Color32::from_rgb(19, 23, 34))
                            .inner_margin(egui::Margin::symmetric(14, 8))
                            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 16))),
                    )
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Left: ✨ Craft AI + App Role subtitle
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                ui.label(
                                    RichText::new("✨ Craft AI")
                                        .size(15.0)
                                        .strong()
                                        .color(Color32::from_rgb(241, 245, 249)),
                                );
                                ui.label(
                                    RichText::new(APP_ROLE)
                                        .size(11.5)
                                        .color(Color32::from_rgb(148, 163, 184)),
                                );
                            });

                            // Right: Action Icons (⚙️ APIs / Keys, 🗑️ Reset, ✕ Close)
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;

                                let close_btn = egui::Button::new(
                        RichText::new("✕")
                            .size(13.0)
                            .strong()
                            .color(Color32::from_rgb(180, 192, 212)),
                    )
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE)
                    .corner_radius(6.0)
                    .min_size(vec2(26.0, 26.0));
                    if ui.add(close_btn).on_hover_text("Close sidebar").clicked() {
                        self.is_open = false;
                        self.show_model_popover = false;
                    }

                                let clear_btn = egui::Button::new(
                                    RichText::new("🗑").size(13.0).color(Color32::from_rgb(180, 192, 212)),
                                )
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE)
                                .corner_radius(6.0)
                                .min_size(vec2(26.0, 26.0));
                                if ui.add(clear_btn).on_hover_text("Clear chat history").clicked() {
                                    self.messages.clear();
                                    self.tool_execution_logs.clear();
                                }

                                let api_btn = egui::Button::new(
                                    RichText::new("⚙ APIs / Keys")
                                        .size(11.5)
                                        .strong()
                                        .color(Color32::from_rgb(216, 226, 246)),
                                )
                                .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 12))
                                .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 26)))
                                .corner_radius(8.0)
                                .min_size(vec2(88.0, 26.0));
                                if ui
                                    .add(api_btn)
                                    .on_hover_text("Manage API Keys & Models")
                                    .clicked()
                                {
                                    self.custom_bots = load_shared_bots();
                                    self.show_manage_apis_modal = true;
                                    self.show_model_popover = false;
                                }
                            });
                        });
                    });

                // =============================================================
                // B. FIXED BOTTOM AREA: LIVE CONTEXT PILL + GEMINI PROMPT BOX
                // =============================================================
                egui::Panel::bottom(format!("{PANEL_ID}_bottom"))
                    .resizable(false)
                    .frame(
                        egui::Frame::new()
                            .fill(Color32::from_rgb(16, 19, 28))
                            .inner_margin(egui::Margin::symmetric(12, 10))
                            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 14))),
                    )
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 7.0;

                        // 1. Live-Context Chip ("Gemini Context Pill")
                        let context_first_line = summary
                            .lines()
                            .find(|l| !l.trim().is_empty())
                            .unwrap_or("Ready (Start screen — will automatically create a new document)");
                        let short_context = truncate_text(context_first_line.trim(), 46);

                        egui::Frame::new()
                            .fill(Color32::from_rgba_unmultiplied(99, 102, 241, 18))
                            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(129, 140, 248, 45)))
                            .corner_radius(999.0)
                            .inner_margin(egui::Margin::symmetric(10, 4))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    ui.label(
                                        RichText::new("●")
                                            .size(9.5)
                                            .color(Color32::from_rgb(74, 222, 128)),
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(format!("Context: {short_context}"))
                                                .size(11.0)
                                                .color(Color32::from_rgb(199, 210, 254)),
                                        )
                                        .truncate(),
                                    )
                                    .on_hover_text(format!("Live context ({APP_NAME}):\n{summary}"));
                                });
                            });

                        // 2. When messages are already in chat, show a compact single-row quick-action bar above the input
                        if !self.messages.is_empty() {
                            ScrollArea::horizontal()
                                .id_salt(format!("{PANEL_ID}_compact_chips"))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 6.0;
                                        for (chip_label, _chip_sub, chip_prompt) in QUICK_CHIPS {
                                            let chip_btn = egui::Button::new(
                                                RichText::new(chip_label)
                                                    .size(11.0)
                                                    .color(Color32::from_rgb(203, 213, 225)),
                                            )
                                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                                            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 20)))
                                            .corner_radius(999.0);

                                            if ui.add(chip_btn).on_hover_text(chip_prompt).clicked() {
                                                if active_bot_clone.is_some() {
                                                    pending_prompt_to_send = Some(chip_prompt.to_string());
                                                } else {
                                                    self.custom_bots = load_shared_bots();
                                                    self.show_manage_apis_modal = true;
                                                }
                                            }
                                        }
                                    });
                                });
                        }

                        // 3. Integrated Gemini Prompt Box
                        let prompt_frame_resp = egui::Frame::new()
                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 11))
                            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 32)))
                            .corner_radius(18.0)
                            .inner_margin(egui::Margin::symmetric(12, 10))
                            .show(ui, |ui| {
                                let placeholder_str = if active_bot_clone.is_none() {
                                    "Please connect a model first...".to_string()
                                } else {
                                    format!("Ask {APP_NAME} AI or enter a command...")
                                };

                                let text_edit_id = egui::Id::unique(format!("{PANEL_ID}_prompt_textarea"));
                                let has_focus = ui.ctx().memory(|m| m.has_focus(text_edit_id));

                                // Intercept Enter (without Shift) BEFORE TextEdit processes it:
                                // - Enter (no Shift): prevent newline insertion and send immediately!
                                // - Shift + Enter: let TextEdit insert a normal '\n' line break!
                                if has_focus {
                                    let enter_without_shift = ui.input(|i| {
                                        !i.modifiers.shift
                                            && !i.modifiers.ctrl
                                            && !i.modifiers.alt
                                            && i.events.iter().any(|ev| {
                                                matches!(
                                                    ev,
                                                    egui::Event::Key {
                                                        key: egui::Key::Enter,
                                                        pressed: true,
                                                        ..
                                                    }
                                                )
                                            })
                                    });

                                    if enter_without_shift {
                                        ui.input_mut(|i| {
                                            i.events.retain(|ev| {
                                                !matches!(
                                                    ev,
                                                    egui::Event::Key {
                                                        key: egui::Key::Enter,
                                                        pressed: true,
                                                        ..
                                                    }
                                                ) && !matches!(ev, egui::Event::Text(t) if t == "\n" || t == "\r" || t == "\r\n")
                                            });
                                        });
                                        let trimmed = self.input_text.trim().to_string();
                                        if !trimmed.is_empty() && !self.is_busy {
                                            pending_prompt_to_send = Some(trimmed);
                                        }
                                    }
                                }

                                // Auto-Resize Calculation: 1 row when compact, grows smoothly up to max-height 160px (~7 rows), then scrolls internally
                                let avail_w = ui.available_width().max(180.0);
                                let chars_per_line = ((avail_w / 7.4) as usize).max(22);
                                let visual_lines: usize = if self.input_text.is_empty() {
                                    1
                                } else {
                                    self.input_text
                                        .split('\n')
                                        .map(|line| {
                                            let len = line.chars().count();
                                            if len == 0 {
                                                1
                                            } else {
                                                (len + chars_per_line - 1) / chars_per_line
                                            }
                                        })
                                        .sum()
                                };
                                let desired_rows = visual_lines.clamp(1, 7);

                                ScrollArea::vertical()
                                    .id_salt(format!("{PANEL_ID}_textarea_scroll"))
                                    .max_height(160.0)
                                    .auto_shrink([false, true])
                                    .show(ui, |ui| {
                                        let text_edit = egui::TextEdit::multiline(&mut self.input_text)
                                            .id(text_edit_id)
                                            .hint_text(
                                                RichText::new(placeholder_str)
                                                    .size(13.5)
                                                    .color(Color32::from_rgb(125, 138, 165)),
                                            )
                                            .font(egui::FontId::proportional(13.5))
                                            .text_color(Color32::from_rgb(226, 232, 240))
                                            .desired_rows(desired_rows)
                                            .desired_width(f32::INFINITY)
                                            .frame(egui::Frame::NONE);

                                        ui.add(text_edit);
                                    });

                                ui.add_space(4.0);

                                // Integrated Bottom Action Row: Left = Model-Switcher Pill, Right = Round Send Button
                                ui.horizontal(|ui| {
                                    if let Some(ref bot) = active_bot_clone {
                                        let icon_key = format!("{} {} {}", bot.name, bot.api_model_id, bot.endpoint);
                                        let tex = self.icon_cache.get_for_str(ui.ctx(), &icon_key);
                                        let short_pill_name = truncate_text(&bot.name, 25);

                                        let pill_resp = ui
                                            .horizontal(|ui| {
                                                ui.spacing_mut().item_spacing.x = 5.0;
                                                render_ai_icon(ui, &tex, 15.0);
                                                let pill_btn = egui::Button::new(
                                                    RichText::new(format!("{short_pill_name} ▾"))
                                                        .size(12.0)
                                                        .strong()
                                                        .color(Color32::from_rgb(218, 228, 248)),
                                                )
                                                .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 14))
                                                .stroke(Stroke::new(
                                                    1.0,
                                                    if self.show_model_popover {
                                                        Color32::from_rgb(138, 112, 248)
                                                    } else {
                                                        Color32::from_rgba_unmultiplied(255, 255, 255, 28)
                                                    },
                                                ))
                                                .corner_radius(999.0);
                                                ui.add(pill_btn)
                                            })
                                            .inner;

                                        pill_click_rect = pill_resp.rect;
                                        if pill_resp.clicked() {
                                            self.custom_bots = load_shared_bots();
                                            self.show_model_popover = !self.show_model_popover;
                                        }
                                    } else {
                                        let no_model_pill = egui::Button::new(
                                            RichText::new("⚠ No Model (Add API) ▾")
                                                .size(11.5)
                                                .strong()
                                                .color(Color32::from_rgb(252, 185, 115)),
                                        )
                                        .fill(Color32::from_rgba_unmultiplied(245, 158, 11, 24))
                                        .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(245, 158, 11, 90)))
                                        .corner_radius(999.0);

                                        let pill_resp = ui.add(no_model_pill);
                                        pill_click_rect = pill_resp.rect;
                                        if pill_resp.clicked() {
                                            self.custom_bots = load_shared_bots();
                                            self.show_model_popover = !self.show_model_popover;
                                        }
                                    }

                                    // Right: Round Send Button (↑)
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let can_send = !self.input_text.trim().is_empty() && !self.is_busy;
                                        let send_fill = if can_send {
                                            Color32::from_rgb(112, 88, 228)
                                        } else {
                                            Color32::from_rgba_unmultiplied(255, 255, 255, 18)
                                        };
                                        let send_btn = egui::Button::new(
                                            RichText::new("↑")
                                                .size(15.0)
                                                .strong()
                                                .color(if can_send {
                                                    Color32::WHITE
                                                } else {
                                                    Color32::from_rgb(120, 130, 150)
                                                }),
                                        )
                                        .fill(send_fill)
                                        .stroke(Stroke::new(
                                            1.0,
                                            if can_send {
                                                Color32::from_rgb(165, 140, 255)
                                            } else {
                                                Color32::TRANSPARENT
                                            },
                                        ))
                                        .corner_radius(999.0)
                                        .min_size(vec2(30.0, 30.0));

                                        if ui
                                            .add(send_btn)
                                            .on_hover_text("Send message (Enter | Shift+Enter for new line)")
                                            .clicked()
                                        {
                                            let trimmed = self.input_text.trim().to_string();
                                            if !trimmed.is_empty() {
                                                pending_prompt_to_send = Some(trimmed);
                                            } else if active_bot_clone.is_none() {
                                                self.show_manage_apis_modal = true;
                                            }
                                        }
                                    });
                                });
                            });

                        prompt_box_top_y = prompt_frame_resp.response.rect.top();
                    });

                // =============================================================
                // C. SCROLLABLE MIDDLE CHAT & HERO EMPTY-STATE AREA
                // =============================================================
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::new()
                            .fill(Color32::from_rgb(16, 19, 28))
                            .inner_margin(egui::Margin::symmetric(14, 12)),
                    )
                    .show(ui, |ui| {
                        let panel_avail_h = ui.available_height();

                        ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .stick_to_bottom(!self.messages.is_empty())
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 10.0;

                                // STATE 1: No API Connected — Vertically Centered BYOK Onboarding Card
                                if self.custom_bots.is_empty() && self.messages.is_empty() {
                                    let card_est_h = 260.0;
                                    let top_pad = ((panel_avail_h - card_est_h) * 0.42).max(12.0);
                                    ui.add_space(top_pad);

                                    egui::Frame::new()
                                        .fill(Color32::from_rgb(22, 26, 38))
                                        .stroke(Stroke::new(1.0, Color32::from_rgb(56, 68, 98)))
                                        .corner_radius(16.0)
                                        .inner_margin(egui::Margin::symmetric(16, 18))
                                        .show(ui, |ui| {
                                            ui.vertical_centered(|ui| {
                                                paint_ambient_sparkle_emblem(ui, 48.0);
                                                ui.add_space(10.0);
                                                ui.label(
                                                    RichText::new(format!("Welcome to {APP_NAME} AI"))
                                                        .size(16.0)
                                                        .strong()
                                                        .color(Color32::from_rgb(248, 250, 252)),
                                                );
                                                ui.add_space(4.0);
                                                ui.label(
                                                    RichText::new("Connect your own API key or local model (Ollama) to activate the assistant.")
                                                        .size(12.5)
                                                        .color(Color32::from_rgb(148, 163, 184)),
                                                );
                                                ui.add_space(14.0);

                                                // Provider Icon Row (7 supported provider logos)
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.spacing_mut().item_spacing.x = 7.0;
                                                    for p_key in ["deepseek", "gemini", "gpt", "llama", "mistral", "qwen", "claude", "ai_icon"] {
                                                        let p_tex = self.icon_cache.get_for_str(ui.ctx(), p_key);
                                                        egui::Frame::new()
                                                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                                                            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 22)))
                                                            .corner_radius(8.0)
                                                            .inner_margin(egui::Margin::same(5))
                                                            .show(ui, |ui| {
                                                                render_ai_icon(ui, &p_tex, 20.0);
                                                            });
                                                    }
                                                });

                                                ui.add_space(14.0);
                                                let cta_btn = egui::Button::new(
                                                    RichText::new("＋ Add API Key / Model")
                                                        .size(13.0)
                                                        .strong()
                                                        .color(Color32::WHITE),
                                                )
                                                .fill(Color32::from_rgb(108, 84, 224))
                                                .stroke(Stroke::new(1.0, Color32::from_rgb(160, 135, 255)))
                                                .corner_radius(10.0)
                                                .min_size(vec2(ui.available_width(), 36.0));

                                                if ui.add(cta_btn).clicked() {
                                                    self.custom_bots = load_shared_bots();
                                                    self.editing_bot_id = None;
                                                    self.show_manage_apis_modal = true;
                                                }
                                            });
                                        });
                                } else if !self.custom_bots.is_empty() && self.messages.is_empty() {
                                    // STATE 2: API Connected & Empty Chat State — Centered Hero + 2x2 Action Cards!
                                    let hero_est_h = 310.0;
                                    let top_pad = ((panel_avail_h - hero_est_h) * 0.42).max(14.0);
                                    ui.add_space(top_pad);

                                    ui.vertical_centered(|ui| {
                                        paint_ambient_sparkle_emblem(ui, 48.0);
                                        ui.add_space(12.0);
                                        ui.label(
                                            RichText::new(format!("How can I help you in {APP_NAME} today?"))
                                                .size(16.0)
                                                .strong()
                                                .color(Color32::from_rgb(248, 250, 252)),
                                        );
                                        ui.add_space(4.0);
                                        ui.label(
                                            RichText::new(format!("{APP_ROLE} • Choose a quick action or enter your command below."))
                                                .size(12.0)
                                                .color(Color32::from_rgb(148, 163, 184)),
                                        );
                                    });

                                    ui.add_space(18.0);

                                    // 2x2 Hero Quick-Action Cards directly below the centered greeting
                                    let grid_w = ui.available_width();
                                    let card_w = ((grid_w - 8.0) * 0.5).max(115.0);
                                    let card_h = 58.0;

                                    for row_pair in QUICK_CHIPS.chunks(2) {
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 8.0;
                                            for (chip_title, chip_sub, chip_prompt) in row_pair {
                                                let (rect, resp) = ui.allocate_exact_size(
                                                    vec2(card_w, card_h),
                                                    egui::Sense::click(),
                                                );
                                                let hovered = resp.hovered();
                                                if hovered {
                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                }

                                                let bg_fill = if hovered {
                                                    Color32::from_rgba_unmultiplied(112, 88, 228, 34)
                                                } else {
                                                    Color32::from_rgba_unmultiplied(255, 255, 255, 10)
                                                };
                                                let border_col = if hovered {
                                                    Color32::from_rgb(148, 120, 255)
                                                } else {
                                                    Color32::from_rgba_unmultiplied(255, 255, 255, 24)
                                                };

                                                ui.painter().rect(
                                                    rect,
                                                    CornerRadius::same(12),
                                                    bg_fill,
                                                    Stroke::new(1.0, border_col),
                                                    egui::epaint::StrokeKind::Inside,
                                                );

                                                let inner = rect.shrink2(vec2(10.0, 8.0));
                                                let mut child_ui = ui.new_child(
                                                    egui::UiBuilder::new()
                                                        .max_rect(inner)
                                                        .layout(egui::Layout::top_down(egui::Align::Min)),
                                                );
                                                child_ui.spacing_mut().item_spacing.y = 3.0;
                                                child_ui.add(
                                                    egui::Label::new(
                                                        RichText::new(*chip_title)
                                                            .size(12.0)
                                                            .strong()
                                                            .color(if hovered {
                                                                Color32::WHITE
                                                            } else {
                                                                Color32::from_rgb(230, 236, 248)
                                                            }),
                                                    )
                                                    .truncate(),
                                                );
                                                child_ui.add(
                                                    egui::Label::new(
                                                        RichText::new(*chip_sub)
                                                            .size(10.5)
                                                            .color(Color32::from_rgb(148, 163, 184)),
                                                    )
                                                    .truncate(),
                                                );

                                                if resp.on_hover_text(*chip_prompt).clicked() {
                                                    pending_prompt_to_send = Some((*chip_prompt).to_string());
                                                }
                                            }
                                        });
                                        ui.add_space(6.0);
                                    }
                                }

                                // Render Chat Messages (Each bubble STRICTLY height: auto!)
                                let active_icon_id = active_bot_clone
                                    .as_ref()
                                    .map(|b| format!("{} {} {}", b.name, b.api_model_id, b.endpoint))
                                    .unwrap_or_else(|| "ai_icon".to_string());
                                let active_bot_label = active_bot_clone
                                    .as_ref()
                                    .map(|b| b.name.clone())
                                    .unwrap_or_else(|| "Craft AI".to_string());
                                let assistant_tex = self.icon_cache.get_for_str(ui.ctx(), &active_icon_id);

                                let messages_snapshot = self.messages.clone();
                                for (idx, msg) in messages_snapshot.iter().enumerate() {
                                    if msg.role == "tool" || msg.content.trim().is_empty() {
                                        continue;
                                    }
                                    let is_user = msg.role == "user";

                                    // Gemini-Style Tool-Chips above assistant responses that triggered verified app commands
                                    if !is_user && !msg.executed_commands.is_empty() {
                                        let count = msg.executed_commands.len();
                                        let joined = msg.executed_commands.join(", ");
                                        let chip_text = if count == 1 {
                                            format!("⚡ 1 app action executed: {joined}")
                                        } else {
                                            format!("⚡ {count} app actions executed: {joined}")
                                        };
                                        egui::Frame::new()
                                            .fill(Color32::from_rgba_unmultiplied(34, 197, 94, 24))
                                            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(74, 222, 128, 75)))
                                            .corner_radius(8.0)
                                            .inner_margin(egui::Margin::symmetric(10, 5))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(chip_text)
                                                        .size(11.5)
                                                        .strong()
                                                        .color(Color32::from_rgb(134, 239, 172)),
                                                );
                                            });
                                    }

                                    let bubble_fill = if is_user {
                                        Color32::from_rgb(30, 38, 58)
                                    } else {
                                        Color32::from_rgb(22, 26, 38)
                                    };
                                    let bubble_stroke = if is_user {
                                        Color32::from_rgb(62, 82, 128)
                                    } else {
                                        Color32::from_rgba_unmultiplied(255, 255, 255, 22)
                                    };

                                    egui::Frame::new()
                                        .fill(bubble_fill)
                                        .stroke(Stroke::new(1.0, bubble_stroke))
                                        .corner_radius(12.0)
                                        .inner_margin(egui::Margin::symmetric(12, 10))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing.x = 6.0;
                                                if is_user {
                                                    ui.label(
                                                        RichText::new("Du")
                                                            .size(11.5)
                                                            .strong()
                                                            .color(Color32::from_rgb(147, 197, 253)),
                                                    );
                                                } else {
                                                    render_ai_icon(ui, &assistant_tex, 15.0);
                                                    ui.label(
                                                        RichText::new(&active_bot_label)
                                                            .size(11.5)
                                                            .strong()
                                                            .color(Color32::from_rgb(196, 181, 253)),
                                                    );
                                                }
                                            });

                                            ui.add_space(4.0);
                                            if is_user {
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new(&msg.content)
                                                            .size(13.5)
                                                            .color(Color32::from_rgb(226, 232, 240)),
                                                    )
                                                    .wrap()
                                                    .selectable(true),
                                                );
                                            } else {
                                                render_markdown_message(ui, &msg.content, idx);
                                            }

                                            // IMPORTANT: Use `ui.horizontal_wrapped` (NEVER `ui.with_layout(right_to_left(Align::Center))`)
                                            // so the bubble height is strictly `height: auto` and never expands to fill the ScrollArea!
                                            if !is_user {
                                                ui.add_space(6.0);
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.spacing_mut().item_spacing.x = 6.0;
                                                    ui.spacing_mut().item_spacing.y = 4.0;

                                                    let is_recently_copied = self
                                                        .copied_feedback
                                                        .as_ref()
                                                        .is_some_and(|(i, t)| *i == idx && t.elapsed().as_secs_f32() < 2.0);
                                                    let copy_lbl = if is_recently_copied { "✓ Copied" } else { "📋 Copy" };
                                                    let copy_btn = egui::Button::new(
                                                        RichText::new(copy_lbl).size(11.0).color(if is_recently_copied {
                                                            Color32::from_rgb(74, 222, 128)
                                                        } else {
                                                            Color32::from_rgb(148, 163, 184)
                                                        }),
                                                    )
                                                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                                                    .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 18)))
                                                    .corner_radius(6.0)
                                                    .min_size(vec2(58.0, 22.0));

                                                    if ui.add(copy_btn).on_hover_text("Copy response to clipboard").clicked() {
                                                        ui.ctx().copy_text(msg.content.clone());
                                                        self.copied_feedback = Some((idx, std::time::Instant::now()));
                                                    }

                                                    

                                                    if !msg.executed_commands.is_empty() {
                                                        // Case A: Commands have already been executed directly in document
                                                        ui.label(
                                                            RichText::new("✅ Applied directly in document")
                                                                .size(11.0)
                                                                .strong()
                                                                .color(Color32::from_rgb(74, 222, 128)),
                                                        );

                                                        let undo_btn = egui::Button::new(
                                                            RichText::new("↩️ Undo / Discard")
                                                                .size(11.0)
                                                                .color(Color32::from_rgb(248, 113, 113)),
                                                        )
                                                        .fill(Color32::from_rgba_unmultiplied(239, 68, 68, 18))
                                                        .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(248, 113, 113, 60)))
                                                        .corner_radius(6.0)
                                                        .min_size(vec2(130.0, 22.0));

                                                        if ui.add(undo_btn).on_hover_text("Undo last action in document").clicked() {
                                                            self.dispatch_direct_tool("edit.undo", json!({}), idx, "↩️ Undone");
                                                        }
                                                    } else {
                                                        // Fall B: Reiner Textentwurf / Vorschau
                                                        if msg.manually_inserted {
                                                            let undo_insert_btn = egui::Button::new(
                                                                RichText::new("✓ Inserted (↩️ Undo)")
                                                                    .size(11.0)
                                                                    .strong()
                                                                    .color(Color32::from_rgb(74, 222, 128)),
                                                            )
                                                            .fill(Color32::from_rgba_unmultiplied(34, 197, 94, 26))
                                                            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(74, 222, 128, 80)))
                                                            .corner_radius(6.0)
                                                            .min_size(vec2(140.0, 22.0));

                                                            if ui.add(undo_insert_btn).on_hover_text("Undo insertion").clicked() {
                                                                self.dispatch_direct_tool("edit.undo", json!({}), idx, "↩️ Undone");
                                                                if let Some(m) = self.messages.get_mut(idx) {
                                                                    m.manually_inserted = false;
                                                                }
                                                            }
                                                        } else {
                                                            let insert_btn = egui::Button::new(
                                                                RichText::new("➕ Insert into document")
                                                                    .size(11.0)
                                                                    .strong()
                                                                    .color(Color32::from_rgb(196, 181, 253)),
                                                            )
                                                            .fill(Color32::from_rgba_unmultiplied(112, 88, 228, 28))
                                                            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(148, 120, 255, 70)))
                                                            .corner_radius(6.0)
                                                            .min_size(vec2(130.0, 22.0));

                                                            if ui.add(insert_btn).on_hover_text("Insert content of this response directly into document").clicked() {
                                                                self.dispatch_direct_tool("insert_from_chat", json!({"markdown": msg.content}), idx, "✓ Inserted into document");
                                                                if let Some(m) = self.messages.get_mut(idx) {
                                                                    m.manually_inserted = true;
                                                                }
                                                            }
                                                        }

                                                        let replace_btn = egui::Button::new(
                                                            RichText::new("🔄 Replace selection")
                                                                .size(11.0)
                                                                .color(Color32::from_rgb(180, 195, 220)),
                                                        )
                                                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                                                        .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 22)))
                                                        .corner_radius(6.0)
                                                        .min_size(vec2(110.0, 22.0));

                                                        if ui.add(replace_btn).on_hover_text("Replace selected area in document with this response").clicked() {
                                                            self.dispatch_direct_tool("replace_from_chat", json!({"markdown": msg.content}), idx, "✓ Selection replaced");
                                                            if let Some(m) = self.messages.get_mut(idx) {
                                                                m.manually_inserted = true;
                                                            }
                                                        }
                                                    }

                                                    if let Some((applied_idx, ref fb_lbl, ref ts)) = self.applied_feedback {
                                                        if applied_idx == idx && ts.elapsed().as_secs_f32() < 3.0 {
                                                            ui.label(
                                                                RichText::new(fb_lbl)
                                                                    .size(11.0)
                                                                    .strong()
                                                                    .color(Color32::from_rgb(74, 222, 128)),
                                                            );
                                                        }
                                                    }
                                                });
                                            }
                                        });
                                }

                                if self.is_busy {
                                    egui::Frame::new()
                                        .fill(Color32::from_rgba_unmultiplied(112, 88, 228, 22))
                                        .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(148, 120, 255, 60)))
                                        .corner_radius(10.0)
                                        .inner_margin(egui::Margin::symmetric(12, 8))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.spinner();
                                                ui.label(
                                                    RichText::new(&self.status_message)
                                                        .size(12.5)
                                                        .color(Color32::from_rgb(216, 226, 246)),
                                                );
                                            });
                                        });
                                }
                            });
                    });
            });

        // =====================================================================
        // E. STRICTLY BOUNDED MODEL-SWITCHER POPOVER (Inside Sidebar Bounds!)
        // =====================================================================
        if self.show_model_popover && sidebar_rect.width() > 50.0 {
            let popover_w = (sidebar_rect.width() - 24.0).clamp(220.0, 336.0);
            let anchor_x = sidebar_rect.left() + 12.0;
            let anchor_y = (prompt_box_top_y - 6.0).max(sidebar_rect.top() + 64.0);

            let area_resp = egui::Area::new(egui::Id::unique(format!("{PANEL_ID}_bounded_model_popover")))
                .order(egui::Order::Foreground)
                .pivot(egui::Align2::LEFT_BOTTOM)
                .fixed_pos(pos2(anchor_x, anchor_y))
                .show(ui.ctx(), |ui| {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(20, 24, 36))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(68, 82, 120)))
                        .corner_radius(12.0)
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            let inner_w = (popover_w - 20.0).max(180.0);
                            ui.set_min_width(inner_w);
                            ui.set_max_width(inner_w);
                            ui.spacing_mut().item_spacing.y = 4.0;

                            ui.label(
                                RichText::new("Saved Models")
                                    .size(11.0)
                                    .strong()
                                    .color(Color32::from_rgb(148, 163, 184)),
                            );

                            if bots_snapshot.is_empty() {
                                ui.label(
                                    RichText::new("No AI model connected yet.")
                                        .size(12.0)
                                        .color(Color32::from_rgb(148, 163, 184)),
                                );
                            } else {
                                ScrollArea::vertical()
                                    .id_salt(format!("{PANEL_ID}_model_popover_scroll"))
                                    .max_height(260.0)
                                    .auto_shrink([false, true])
                                    .show(ui, |ui| {
                                        ui.set_width(inner_w);
                                        for b in &bots_snapshot {
                                            let is_active = self.selected_bot_id.as_deref() == Some(&b.id)
                                                || (self.selected_bot_id.is_none()
                                                    && bots_snapshot.first().map(|f| f.id.as_str()) == Some(&b.id));
                                            let b_key = format!("{} {} {}", b.name, b.api_model_id, b.endpoint);
                                            let b_tex = self.icon_cache.get_for_str(ui.ctx(), &b_key);

                                            let row_h = 38.0;
                                            let (row_rect, row_resp) = ui.allocate_exact_size(
                                                vec2(inner_w, row_h),
                                                egui::Sense::click(),
                                            );
                                            let hovered = row_resp.hovered();
                                            if hovered || is_active {
                                                ui.painter().rect(
                                                    row_rect,
                                                    CornerRadius::same(8),
                                                    if is_active {
                                                        Color32::from_rgba_unmultiplied(112, 88, 228, 38)
                                                    } else {
                                                        Color32::from_rgba_unmultiplied(255, 255, 255, 12)
                                                    },
                                                    Stroke::new(
                                                        1.0,
                                                        if is_active {
                                                            Color32::from_rgb(138, 112, 248)
                                                        } else {
                                                            Color32::TRANSPARENT
                                                        },
                                                    ),
                                                    egui::epaint::StrokeKind::Inside,
                                                );
                                            }

                                            let content_rect = row_rect.shrink2(vec2(8.0, 4.0));
                                            let mut row_ui = ui.new_child(
                                                egui::UiBuilder::new()
                                                    .max_rect(content_rect)
                                                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                                            );
                                            row_ui.spacing_mut().item_spacing.x = 8.0;
                                            render_ai_icon(&mut row_ui, &b_tex, 18.0);

                                            let text_col_w = (content_rect.width() - 48.0).max(80.0);
                                            row_ui.allocate_ui_with_layout(
                                                vec2(text_col_w, 30.0),
                                                egui::Layout::top_down(egui::Align::Min),
                                                |col_ui| {
                                                    col_ui.spacing_mut().item_spacing.y = 1.0;
                                                    col_ui.add(
                                                        egui::Label::new(
                                                            RichText::new(truncate_text(&b.name, 34))
                                                                .size(12.5)
                                                                .strong()
                                                                .color(if is_active {
                                                                    Color32::from_rgb(196, 181, 253)
                                                                } else {
                                                                    Color32::from_rgb(241, 245, 249)
                                                                }),
                                                        )
                                                        .truncate(),
                                                    );
                                                    col_ui.add(
                                                        egui::Label::new(
                                                            RichText::new(truncate_text(&b.api_model_id, 40))
                                                                .size(11.0)
                                                                .color(Color32::from_rgb(148, 163, 184)),
                                                        )
                                                        .truncate(),
                                                    );
                                                },
                                            );

                                            if is_active {
                                                row_ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |r_ui| {
                                                        r_ui.label(
                                                            RichText::new("✓")
                                                                .size(13.0)
                                                                .strong()
                                                                .color(Color32::from_rgb(74, 222, 128)),
                                                        );
                                                    },
                                                );
                                            }

                                            if row_resp.clicked() {
                                                self.selected_bot_id = Some(b.id.clone());
                                                self.show_model_popover = false;
                                            }
                                        }
                                    });
                            }

                            ui.separator();
                            let action_btn_1 = egui::Button::new(
                                RichText::new("➕ Add New Model / API Key...")
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::from_rgb(165, 200, 255)),
                            )
                            .fill(Color32::TRANSPARENT)
                            .min_size(vec2(inner_w, 26.0));
                            if ui.add(action_btn_1).clicked() {
                                self.custom_bots = load_shared_bots();
                                self.editing_bot_id = None;
                                self.show_manage_apis_modal = true;
                                self.show_model_popover = false;
                            }

                            let action_btn_2 = egui::Button::new(
                                RichText::new("⚙ Manage / remove APIs...")
                                    .size(12.0)
                                    .color(Color32::from_rgb(210, 220, 238)),
                            )
                            .fill(Color32::TRANSPARENT)
                            .min_size(vec2(inner_w, 26.0));
                            if ui.add(action_btn_2).clicked() {
                                self.custom_bots = load_shared_bots();
                                self.show_manage_apis_modal = true;
                                self.show_model_popover = false;
                            }
                        });
                });

            // Close popover when clicking outside both the popover and the trigger pill
            if ui.input(|i| i.pointer.any_pressed()) {
                if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                    if !area_resp.response.rect.contains(pos) && !pill_click_rect.contains(pos) {
                        self.show_model_popover = false;
                    }
                }
            }
        }

        // =====================================================================
        // F. "MANAGE APIs & MODELS" MODAL (Unified Manage, Edit, Test & Add)
        // =====================================================================
        if self.show_manage_apis_modal || self.show_add_api_modal {
            self.render_manage_apis_modal(ui.ctx());
        }

        // (d) Doppeltes Absenden im selben Frame verhindern (Enter + Button):
        //     is_busy wird in send_prompt() gesetzt und erst nach Worker-Ende zurückgesetzt.
        if !self.is_busy {
            if let Some(prompt_to_send) = pending_prompt_to_send {
                self.input_text = prompt_to_send;
                self.send_prompt(summary.to_string(), project_dir_str.to_string());
            }
        }
    }

    fn render_manage_apis_modal(&mut self, ctx: &egui::Context) {
        let mut is_open = true;
        egui::Window::new("⚙ Manage API Keys & Models")
            .id(egui::Id::unique(format!("{PANEL_ID}_manage_apis_modal")))
            .open(&mut is_open)
            .collapsible(false)
            .resizable(true)
            .default_width(560.0)
            .min_width(500.0)
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(18, 22, 32))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(58, 72, 105)))
                    .corner_radius(14.0)
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
                ui.spacing_mut().button_padding = vec2(6.0, 3.0);
                ui.spacing_mut().interact_size = vec2(20.0, 22.0);
                ui.style_mut().text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(13.0));
                ui.style_mut().text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(12.5));
                ui.style_mut().text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(11.0));
                ui.spacing_mut().item_spacing.y = 10.0;

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("🌐 Synchronized across all Craft apps (BYOK)")
                            .size(12.0)
                            .color(Color32::from_rgb(74, 222, 128)),
                    );
                });

                // -------------------------------------------------------------
                // 1. SAVED CONNECTIONS LIST (Strict 2-Row Card Layout — Zero Overlap!)
                // -------------------------------------------------------------
                ui.label(
                    RichText::new(format!("Saved Connections ({})", self.custom_bots.len()))
                        .size(13.5)
                        .strong()
                        .color(Color32::from_rgb(226, 232, 240)),
                );

                let mut delete_id: Option<String> = None;
                let mut edit_bot: Option<CustomBot> = None;
                let mut activate_id: Option<String> = None;
                let mut test_bot: Option<CustomBot> = None;

                if self.custom_bots.is_empty() {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(24, 29, 42))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(45, 55, 78)))
                        .corner_radius(10.0)
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("No API connections saved yet. Use the form below to add your API key or local model.")
                                    .size(12.5)
                                    .color(Color32::from_rgb(160, 175, 200)),
                            );
                        });
                } else {
                    ScrollArea::vertical().max_height(190.0).show(ui, |ui| {
                        for bot in &self.custom_bots {
                            let is_active = self.selected_bot_id.as_deref() == Some(&bot.id)
                                || (self.selected_bot_id.is_none()
                                    && self.custom_bots.first().map(|f| f.id.as_str()) == Some(&bot.id));
                            let icon_str = format!("{} {} {}", bot.name, bot.api_model_id, bot.endpoint);
                            let tex = self.icon_cache.get_for_str(ui.ctx(), &icon_str);

                            egui::Frame::new()
                                .fill(if is_active {
                                    Color32::from_rgb(28, 35, 54)
                                } else {
                                    Color32::from_rgb(22, 27, 40)
                                })
                                .stroke(Stroke::new(
                                    1.0,
                                    if is_active {
                                        Color32::from_rgb(112, 88, 228)
                                    } else {
                                        Color32::from_rgb(46, 56, 80)
                                    },
                                ))
                                .corner_radius(10.0)
                                .inner_margin(egui::Margin::symmetric(12, 10))
                                .show(ui, |ui| {
                                    let card_avail_w = ui.available_width();
                                    ui.set_width(card_avail_w);
                                    ui.spacing_mut().item_spacing.y = 6.0;

                                    // ROW 1 (Card Header): Left = Icon + Display Name + Active Badge | Right = Compact Action Buttons
                                    let buttons_reserved_w = if is_active { 150.0 } else { 225.0 };
                                    let left_max_w = (card_avail_w - buttons_reserved_w).max(120.0);

                                    ui.horizontal(|ui| {
                                        ui.allocate_ui_with_layout(
                                            vec2(left_max_w, 24.0),
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |left_ui| {
                                                left_ui.spacing_mut().item_spacing.x = 6.0;
                                                render_ai_icon(left_ui, &tex, 18.0);
                                                let name_char_cap = if is_active { 24 } else { 20 };
                                                left_ui.add(
                                                    egui::Label::new(
                                                        RichText::new(truncate_text(&bot.name, name_char_cap))
                                                            .size(13.0)
                                                            .strong()
                                                            .color(Color32::WHITE),
                                                    )
                                                    .truncate(),
                                                );
                                                if is_active {
                                                    egui::Frame::new()
                                                        .fill(Color32::from_rgba_unmultiplied(34, 197, 94, 30))
                                                        .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(74, 222, 128, 90)))
                                                        .corner_radius(6.0)
                                                        .inner_margin(egui::Margin::symmetric(6, 2))
                                                        .show(left_ui, |b_ui| {
                                                            b_ui.label(
                                                                RichText::new("✓ Active")
                                                                    .size(10.5)
                                                                    .strong()
                                                                    .color(Color32::from_rgb(134, 239, 172)),
                                                            );
                                                        });
                                                }
                                            },
                                        );

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.spacing_mut().item_spacing.x = 6.0;

                                            let del_btn = egui::Button::new(
                                                RichText::new("🗑")
                                                    .size(12.0)
                                                    .color(Color32::from_rgb(252, 129, 129)),
                                            )
                                            .fill(Color32::from_rgb(52, 22, 28))
                                            .stroke(Stroke::new(1.0, Color32::from_rgb(140, 45, 58)))
                                            .corner_radius(6.0)
                                            .min_size(vec2(28.0, 24.0));
                                            if ui.add(del_btn).on_hover_text("Delete connection permanently").clicked() {
                                                delete_id = Some(bot.id.clone());
                                            }

                                            let test_btn = egui::Button::new(
                                                RichText::new("🔌 Test").size(11.5),
                                            )
                                            .corner_radius(6.0)
                                            .min_size(vec2(54.0, 24.0));
                                            if ui.add(test_btn).on_hover_text("Test connection").clicked() {
                                                test_bot = Some(bot.clone());
                                            }

                                            let edit_btn = egui::Button::new(
                                                RichText::new("✏").size(12.0),
                                            )
                                            .corner_radius(6.0)
                                            .min_size(vec2(28.0, 24.0));
                                            if ui.add(edit_btn).on_hover_text("Edit connection").clicked() {
                                                edit_bot = Some(bot.clone());
                                            }

                                            if !is_active {
                                                let act_btn = egui::Button::new(
                                                    RichText::new("Activate").size(11.5).strong(),
                                                )
                                                .corner_radius(6.0)
                                                .min_size(vec2(68.0, 24.0));
                                                if ui.add(act_btn).clicked() {
                                                    activate_id = Some(bot.id.clone());
                                                }
                                            }
                                        });
                                    });

                                    // ROW 2 (Card Metadata): Dedicated full-width bottom line for Model-ID, Base URL & Masked Key
                                    let meta_line = format!(
                                        "Model: {}   •   URL: {}   •   Key: {}",
                                        bot.api_model_id,
                                        bot.endpoint,
                                        mask_api_key(&bot.api_key)
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(meta_line)
                                                .size(11.5)
                                                .color(Color32::from_rgb(148, 163, 184)),
                                        )
                                        .truncate(),
                                    );
                                });
                            ui.add_space(4.0);
                        }
                    });
                }

                if let Some(act_id) = activate_id {
                    self.selected_bot_id = Some(act_id);
                }
                if let Some(b) = edit_bot {
                    self.editing_bot_id = Some(b.id);
                    self.new_bot_name = b.name;
                    self.new_bot_endpoint = b.endpoint;
                    self.new_bot_key = b.api_key;
                    self.new_bot_model_id = b.api_model_id;
                    self.test_connection_status = None;
                }
                if let Some(b) = test_bot {
                    self.test_connection_status = Some(test_api_endpoint(&b.endpoint, &b.api_key, &b.api_model_id));
                }
                if let Some(del) = delete_id {
                    self.custom_bots.retain(|b| b.id != del);
                    save_shared_bots(&self.custom_bots);
                    if self.selected_bot_id.as_deref() == Some(&del) {
                        self.selected_bot_id = self.custom_bots.first().map(|b| b.id.clone());
                    }
                    if self.editing_bot_id.as_deref() == Some(&del) {
                        self.editing_bot_id = None;
                    }
                }

                ui.separator();

                // -------------------------------------------------------------
                // 2. ADD / EDIT API CONNECTION FORM (Presets only set Name/Icon!)
                // -------------------------------------------------------------
                let form_title = if self.editing_bot_id.is_some() {
                    "✏ Edit Saved API Connection"
                } else {
                    "➕ Connect New API / Model"
                };
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(form_title)
                            .size(13.5)
                            .strong()
                            .color(Color32::from_rgb(226, 232, 240)),
                    );
                    if self.editing_bot_id.is_some() {
                        if ui.small_button("Cancel (Create New)").clicked() {
                            self.editing_bot_id = None;
                            self.new_bot_name.clear();
                            self.new_bot_endpoint.clear();
                            self.new_bot_key.clear();
                            self.new_bot_model_id.clear();
                        }
                    }
                });

                ui.label(
                    RichText::new("Quick Presets (selects icon & provider name):")
                        .size(11.5)
                        .color(Color32::from_rgb(148, 163, 184)),
                );

                let presets: [(&str, &str, &str, &str); 8] = [
                    ("OpenAI GPT", "gpt", "e.g. https://api.openai.com/v1/chat/completions", "e.g. gpt-4o"),
                    ("Gemini", "gemini", "e.g. https://generativelanguage.googleapis.com/v1beta/openai/chat/completions", "e.g. gemini-2.5-flash"),
                    ("DeepSeek", "deepseek", "e.g. https://api.deepseek.com/chat/completions", "e.g. deepseek-chat"),
                    ("Mistral", "mistral", "e.g. https://api.mistral.ai/v1/chat/completions", "e.g. mistral-large-latest"),
                    ("Qwen", "qwen", "e.g. https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions", "e.g. qwen-plus"),
                    ("Claude", "claude", "e.g. https://api.anthropic.com/v1/chat/completions", "e.g. claude-3-5-sonnet-latest"),
                    ("Local Ollama", "llama", "e.g. http://localhost:11434/v1/chat/completions", "e.g. llama3.1"),
                    ("Custom API", "ai_icon", "e.g. https://api.openai.com/api/v1/chat/completions", "e.g. llama-3.1-8b"),
                ];

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.spacing_mut().item_spacing.y = 6.0;
                    for (label, icon_id, hint_url, hint_model) in presets {
                        let p_tex = self.icon_cache.get_for_str(ui.ctx(), icon_id);
                        let is_selected_preset = self.new_bot_name.eq_ignore_ascii_case(label);
                        let resp = ui
                            .horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                render_ai_icon(ui, &p_tex, 14.0);
                                ui.add(
                                    egui::Button::new(
                                        RichText::new(label)
                                            .size(11.5)
                                            .color(Color32::from_rgb(226, 232, 240)),
                                    )
                                    .fill(if is_selected_preset {
                                        Color32::from_rgb(56, 46, 112)
                                    } else {
                                        Color32::from_rgb(28, 34, 50)
                                    })
                                    .stroke(Stroke::new(
                                        1.0,
                                        if is_selected_preset {
                                            Color32::from_rgb(138, 112, 248)
                                        } else {
                                            Color32::from_rgb(58, 72, 105)
                                        },
                                    ))
                                    .corner_radius(8.0),
                                )
                            })
                            .inner;
                        if resp.clicked() {
                            if label == "Custom API" {
                                self.new_bot_name.clear();
                            } else {
                                self.new_bot_name = label.to_string();
                            }
                            self.preset_hint_endpoint = hint_url.to_string();
                            self.preset_hint_model = hint_model.to_string();
                            self.test_connection_status = None;
                        }
                    }
                });

                egui::Grid::new(format!("{PANEL_ID}_api_form_grid"))
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .striped(false)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Provider / Display Name:").size(12.5).color(Color32::from_rgb(203, 213, 225)));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_bot_name)
                                .hint_text("e.g. DeepSeek R1, Custom Llama, OpenAI GPT-4o")
                                .desired_width(340.0),
                        );
                        ui.end_row();

                        ui.label(RichText::new("Base URL / Endpoint:").size(12.5).color(Color32::from_rgb(203, 213, 225)));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_bot_endpoint)
                                .hint_text(&self.preset_hint_endpoint)
                                .desired_width(340.0),
                        );
                        ui.end_row();

                        ui.label(RichText::new("API Key:").size(12.5).color(Color32::from_rgb(203, 213, 225)));
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            ui.add(
                                egui::TextEdit::singleline(&mut self.new_bot_key)
                                    .password(!self.show_api_key_plain)
                                    .hint_text("sk-... (leave blank for local Ollama)")
                                    .desired_width(308.0),
                            );
                            let eye = if self.show_api_key_plain { "🙈" } else { "👁" };
                            let eye_btn = egui::Button::new(RichText::new(eye).size(12.0))
                                .min_size(vec2(28.0, 22.0));
                            if ui.add(eye_btn).on_hover_text("Show/hide API key").clicked() {
                                self.show_api_key_plain = !self.show_api_key_plain;
                            }
                        });
                        ui.end_row();

                        ui.label(RichText::new("Model ID:").size(12.5).color(Color32::from_rgb(203, 213, 225)));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_bot_model_id)
                                .hint_text(&self.preset_hint_model)
                                .desired_width(340.0),
                        );
                        ui.end_row();
                    });

                if let Some((ok, ref msg)) = self.test_connection_status {
                    let bg = if ok {
                        Color32::from_rgba_unmultiplied(34, 197, 94, 28)
                    } else {
                        Color32::from_rgba_unmultiplied(239, 68, 68, 28)
                    };
                    let fg = if ok {
                        Color32::from_rgb(134, 239, 172)
                    } else {
                        Color32::from_rgb(252, 165, 165)
                    };
                    egui::Frame::new()
                        .fill(bg)
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin::symmetric(10, 6))
                        .show(ui, |ui| {
                            ui.label(RichText::new(msg).size(12.0).color(fg));
                        });
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let test_btn = egui::Button::new(
                        RichText::new("🔌 Test Connection")
                            .size(12.5)
                            .strong()
                            .color(Color32::from_rgb(216, 226, 246)),
                    )
                    .fill(Color32::from_rgb(30, 38, 56))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(72, 90, 130)))
                    .corner_radius(8.0)
                    .min_size(vec2(150.0, 32.0));

                    if ui.add(test_btn).clicked() {
                        let ep = self.new_bot_endpoint.trim();
                        let key = self.new_bot_key.trim();
                        let mid = self.new_bot_model_id.trim();
                        if ep.is_empty() || mid.is_empty() {
                            self.test_connection_status = Some((
                                false,
                                "❌ Please fill in Base URL / Endpoint and Model ID first.".to_string(),
                            ));
                        } else {
                            self.test_connection_status = Some(test_api_endpoint(ep, key, mid));
                        }
                    }

                    let save_btn = egui::Button::new(
                        RichText::new("💾 Save & Activate")
                            .size(12.5)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(108, 84, 224))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(160, 135, 255)))
                    .corner_radius(8.0)
                    .min_size(vec2(180.0, 32.0));

                    if ui.add(save_btn).clicked() {
                        let name = if self.new_bot_name.trim().is_empty() {
                            self.new_bot_model_id.trim().to_string()
                        } else {
                            self.new_bot_name.trim().to_string()
                        };
                        let endpoint = self.new_bot_endpoint.trim().to_string();
                        let model_id = self.new_bot_model_id.trim().to_string();
                        let api_key = self.new_bot_key.trim().to_string();

                        if !endpoint.is_empty() && !model_id.is_empty() {
                            if let Some(ref edit_id) = self.editing_bot_id.clone() {
                                if let Some(existing) = self.custom_bots.iter_mut().find(|b| &b.id == edit_id) {
                                    existing.name = name;
                                    existing.endpoint = endpoint;
                                    existing.api_model_id = model_id;
                                    existing.api_key = api_key;
                                }
                                self.selected_bot_id = Some(edit_id.clone());
                                self.editing_bot_id = None;
                            } else {
                                let new_id = format!(
                                    "custom_{}",
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map(|d| d.as_millis())
                                        .unwrap_or(1)
                                );
                                self.custom_bots.push(CustomBot {
                                    id: new_id.clone(),
                                    name,
                                    api_model_id: model_id,
                                    endpoint,
                                    api_key,
                                    used_tokens: 0,
                                    max_quota_tokens: 0,
                                });
                                self.selected_bot_id = Some(new_id);
                            }
                            save_shared_bots(&self.custom_bots);
                            self.new_bot_name.clear();
                            self.new_bot_endpoint.clear();
                            self.new_bot_key.clear();
                            self.new_bot_model_id.clear();
                            self.test_connection_status = Some((true, "✅ Model saved & activated!".to_string()));
                        } else {
                            self.test_connection_status = Some((
                                false,
                                "❌ Please provide Base URL / Endpoint and Model ID.".to_string(),
                            ));
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            self.show_manage_apis_modal = false;
                            self.show_add_api_modal = false;
                        }
                    });
                });
            });

        if !is_open {
            self.show_manage_apis_modal = false;
            self.show_add_api_modal = false;
        }
    }

    pub fn send_prompt(&mut self, summary: String, project_dir_str: String) {
        let prompt = std::mem::take(&mut self.input_text).trim().to_string();
        if prompt.is_empty() {
            return;
        }

        let Some(bot) = self.active_bot().cloned() else {
            // Fall 1: keine API aktiv -> genau EINE Meldung, kein Request, kein Fallback-Text.
            self.messages.push(ChatMessage {
                role: "assistant".to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                content: "⚠️ Bitte hinterlege zuerst eine API-Verbindung über „APIs / Keys“.".to_string(),
                executed_commands: Vec::new(),
                manually_inserted: false,
            });
            self.show_manage_apis_modal = true;
            return;
        };

        self.messages.push(ChatMessage {
            role: "user".to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            content: prompt.clone(),
            executed_commands: Vec::new(),
            manually_inserted: false,
        });

        self.is_busy = true;
        self.status_message = format!("{} denkt nach...", bot.name);

        if let Some(ref tx) = self.worker_cmd_tx {
            let _ = tx.send(AgentWorkerCmd::RunPrompt {
                prompt,
                messages: self.messages.clone(),
                bot,
                doc_text: summary,
                project_dir_str,
            });
        }
    }
}

// =============================================================================
// RICH MARKDOWN & STYLED TABLE RENDERER FOR CHAT MESSAGES
// =============================================================================

fn build_inline_markdown_job(text: &str, base_size: f32, base_color: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = 0;
    let mut buf = String::new();

    let flush_normal = |job: &mut LayoutJob, buf: &mut String| {
        if !buf.is_empty() {
            job.append(
                buf,
                0.0,
                TextFormat {
                    font_id: FontId::new(base_size, FontFamily::Proportional),
                    color: base_color,
                    ..Default::default()
                },
            );
            buf.clear();
        }
    };

    while i < n {
        // Inline code `...`
        if chars[i] == '`' {
            let mut end = i + 1;
            while end < n && chars[end] != '`' {
                end += 1;
            }
            if end < n {
                flush_normal(&mut job, &mut buf);
                let code_str: String = chars[i + 1..end].iter().collect();
                job.append(
                    &code_str,
                    0.0,
                    TextFormat {
                        font_id: FontId::new((base_size - 1.0).max(10.5), FontFamily::Monospace),
                        color: Color32::from_rgb(165, 243, 252),
                        background: Color32::from_rgba_unmultiplied(15, 23, 42, 210),
                        ..Default::default()
                    },
                );
                i = end + 1;
                continue;
            }
        }

        // Bold **...**
        if i + 1 < n && chars[i] == '*' && chars[i + 1] == '*' {
            let mut end = i + 2;
            while end + 1 < n && !(chars[end] == '*' && chars[end + 1] == '*') {
                end += 1;
            }
            if end + 1 < n {
                flush_normal(&mut job, &mut buf);
                let bold_str: String = chars[i + 2..end].iter().collect();
                job.append(
                    &bold_str,
                    0.0,
                    TextFormat {
                        font_id: FontId::new(base_size, FontFamily::Proportional),
                        color: Color32::WHITE,
                        ..Default::default()
                    },
                );
                i = end + 2;
                continue;
            }
        }

        // Italic *...*
        if chars[i] == '*' && (i + 1 < n && chars[i + 1] != ' ') {
            let mut end = i + 1;
            while end < n && chars[end] != '*' {
                end += 1;
            }
            if end < n {
                flush_normal(&mut job, &mut buf);
                let ital_str: String = chars[i + 1..end].iter().collect();
                job.append(
                    &ital_str,
                    0.0,
                    TextFormat {
                        font_id: FontId::new(base_size, FontFamily::Proportional),
                        color: Color32::from_rgb(203, 213, 225),
                        italics: true,
                        ..Default::default()
                    },
                );
                i = end + 1;
                continue;
            }
        }

        buf.push(chars[i]);
        i += 1;
    }

    flush_normal(&mut job, &mut buf);
    job
}

fn is_markdown_table_separator(line: &str) -> bool {
    let t = line.trim();
    if !t.starts_with('|') {
        return false;
    }
    let cells: Vec<&str> = t.trim_matches('|').split('|').collect();
    !cells.is_empty()
        && cells.iter().all(|c| {
            let ct = c.trim();
            !ct.is_empty() && ct.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')
        })
}

fn render_markdown_table(ui: &mut egui::Ui, table_lines: &[&str], msg_idx: usize, table_idx: usize) {
    let mut parsed_rows: Vec<Vec<String>> = Vec::new();
    for line in table_lines {
        if is_markdown_table_separator(line) {
            continue;
        }
        let cells: Vec<String> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim().replace("**", ""))
            .collect();
        if !cells.is_empty() {
            parsed_rows.push(cells);
        }
    }
    if parsed_rows.is_empty() {
        return;
    }

    let num_cols = parsed_rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);

    egui::Frame::new()
        .fill(Color32::from_rgb(15, 19, 29))
        .stroke(Stroke::new(1.0, Color32::from_rgb(56, 70, 104)))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::same(6))
        .show(ui, |ui| {
            ScrollArea::horizontal()
                .id_salt(format!("{PANEL_ID}_md_table_{msg_idx}_{table_idx}"))
                .show(ui, |ui| {
                    egui::Grid::new(format!("{PANEL_ID}_md_grid_{msg_idx}_{table_idx}"))
                        .num_columns(num_cols)
                        .spacing([8.0, 5.0])
                        .striped(true)
                        .show(ui, |ui| {
                            for (r_idx, row) in parsed_rows.iter().enumerate() {
                                for c_idx in 0..num_cols {
                                    let val = row.get(c_idx).map(|s| s.as_str()).unwrap_or("");
                                    if r_idx == 0 {
                                        egui::Frame::new()
                                            .fill(Color32::from_rgb(30, 41, 59))
                                            .corner_radius(4.0)
                                            .inner_margin(egui::Margin::symmetric(6, 3))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(val)
                                                        .size(11.5)
                                                        .strong()
                                                        .color(Color32::from_rgb(226, 232, 240)),
                                                );
                                            });
                                    } else {
                                        ui.label(
                                            RichText::new(val)
                                                .size(11.5)
                                                .color(Color32::from_rgb(203, 213, 225)),
                                        );
                                    }
                                }
                                ui.end_row();
                            }
                        });
                });
        });
}

fn render_markdown_message(ui: &mut egui::Ui, text: &str, msg_idx: usize) {
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    let mut table_counter = 0usize;

    ui.spacing_mut().item_spacing.y = 4.0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            ui.add_space(2.0);
            i += 1;
            continue;
        }

        // 1. Fenced code block ```...```
        if trimmed.starts_with("```") {
            let mut code_lines = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                code_lines.push(lines[i]);
                i += 1;
            }
            if i < lines.len() {
                i += 1;
            }
            let code_block = code_lines.join("\n");
            if !code_block.trim().is_empty() {
                egui::Frame::new()
                    .fill(Color32::from_rgb(13, 16, 24))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(45, 55, 78)))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.add(
                            egui::Label::new(
                                RichText::new(code_block)
                                    .font(FontId::monospace(11.5))
                                    .color(Color32::from_rgb(186, 230, 253)),
                            )
                            .wrap()
                            .selectable(true),
                        );
                    });
            }
            continue;
        }

        // 2. Markdown Table | A | B |
        if trimmed.starts_with('|') && trimmed.matches('|').count() >= 2 {
            let mut tbl_lines = Vec::new();
            while i < lines.len() {
                let tl = lines[i].trim();
                if tl.starts_with('|') && tl.matches('|').count() >= 2 {
                    tbl_lines.push(tl);
                    i += 1;
                } else {
                    break;
                }
            }
            if tbl_lines.len() >= 2 {
                table_counter += 1;
                render_markdown_table(ui, &tbl_lines, msg_idx, table_counter);
                continue;
            }
        }

        // 3. Horizontal separator ---
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            ui.separator();
            i += 1;
            continue;
        }

        // 4. Headings #, ##, ###, ####
        if let Some(h4) = trimmed.strip_prefix("#### ") {
            ui.label(
                RichText::new(h4.replace("**", ""))
                    .size(13.5)
                    .strong()
                    .color(Color32::from_rgb(196, 181, 253)),
            );
            i += 1;
            continue;
        }
        if let Some(h3) = trimmed.strip_prefix("### ") {
            ui.add_space(2.0);
            ui.label(
                RichText::new(h3.replace("**", ""))
                    .size(14.2)
                    .strong()
                    .color(Color32::from_rgb(216, 180, 254)),
            );
            i += 1;
            continue;
        }
        if let Some(h2) = trimmed.strip_prefix("## ") {
            ui.add_space(3.0);
            ui.label(
                RichText::new(h2.replace("**", ""))
                    .size(15.0)
                    .strong()
                    .color(Color32::from_rgb(241, 245, 249)),
            );
            i += 1;
            continue;
        }
        if let Some(h1) = trimmed.strip_prefix("# ") {
            ui.add_space(4.0);
            ui.label(
                RichText::new(h1.replace("**", ""))
                    .size(16.2)
                    .strong()
                    .color(Color32::WHITE),
            );
            i += 1;
            continue;
        }

        // 5. Blockquote / Callout > ...
        if let Some(quote) = trimmed.strip_prefix("> ") {
            egui::Frame::new()
                .fill(Color32::from_rgba_unmultiplied(99, 102, 241, 22))
                .stroke(Stroke::new(1.5, Color32::from_rgb(129, 140, 248)))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    let job = build_inline_markdown_job(quote, 12.8, Color32::from_rgb(224, 231, 255));
                    ui.add(egui::Label::new(job).wrap().selectable(true));
                });
            i += 1;
            continue;
        }

        // 6. Bullet lists (- , * , • )
        if let Some(bullet_body) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("• "))
        {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(
                    RichText::new("•")
                        .size(13.5)
                        .strong()
                        .color(Color32::from_rgb(165, 140, 255)),
                );
                let job = build_inline_markdown_job(bullet_body, 13.2, Color32::from_rgb(226, 232, 240));
                ui.add(egui::Label::new(job).wrap().selectable(true));
            });
            i += 1;
            continue;
        }

        // 7. Numbered list (1. , 2. , etc.)
        if let Some(dot_pos) = trimmed.find(". ") {
            let prefix = &trimmed[..dot_pos];
            if !prefix.is_empty() && prefix.len() <= 3 && prefix.chars().all(|c| c.is_ascii_digit()) {
                let body = &trimmed[dot_pos + 2..];
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(
                        RichText::new(format!("{prefix}."))
                            .size(13.0)
                            .strong()
                            .color(Color32::from_rgb(165, 140, 255)),
                    );
                    let job = build_inline_markdown_job(body, 13.2, Color32::from_rgb(226, 232, 240));
                    ui.add(egui::Label::new(job).wrap().selectable(true));
                });
                i += 1;
                continue;
            }
        }

        // 8. Standard paragraph with inline **bold**, *italic*, and `code`
        let job = build_inline_markdown_job(trimmed, 13.4, Color32::from_rgb(226, 232, 240));
        ui.add(egui::Label::new(job).wrap().selectable(true));
        i += 1;
    }
}

fn truncate_text(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        let head: String = chars[..max_chars.saturating_sub(1)].iter().collect();
        format!("{head}…")
    }
}

fn paint_ambient_sparkle_emblem(ui: &mut egui::Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(
        rect.expand(4.0),
        CornerRadius::same(16),
        Color32::from_rgba_unmultiplied(112, 88, 228, 30),
    );
    painter.rect(
        rect,
        CornerRadius::same(14),
        Color32::from_rgb(36, 30, 70),
        Stroke::new(1.5, Color32::from_rgb(148, 120, 255)),
        egui::epaint::StrokeKind::Inside,
    );
    let top_half = Rect::from_min_size(rect.min + vec2(2.0, 2.0), vec2(size - 4.0, (size - 4.0) * 0.45));
    painter.rect_filled(
        top_half,
        CornerRadius::same(12),
        Color32::from_rgba_unmultiplied(196, 181, 253, 24),
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "✨",
        egui::FontId::proportional(size * 0.46),
        Color32::WHITE,
    );
}

fn mask_api_key(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return "No Key (Local)".to_string();
    }
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= 8 {
        return "••••••••".to_string();
    }
    let prefix: String = chars[..4].iter().collect();
    let suffix: String = chars[chars.len() - 4..].iter().collect();
    format!("{prefix}...{suffix}")
}

fn normalize_endpoint(ep: &str) -> String {
    let trimmed = ep.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return "https://api.openai.com/v1/chat/completions".to_string();
    }
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else if trimmed.ends_with("/v1") || trimmed.ends_with("/v1beta/openai") {
        format!("{trimmed}/chat/completions")
    } else {
        trimmed.to_string()
    }
}

fn test_api_endpoint(endpoint: &str, api_key: &str, model_id: &str) -> (bool, String) {
    let url = normalize_endpoint(endpoint);
    let body = json!({
        "model": model_id,
        "messages": [{"role": "user", "content": "Ping"}],
        "max_tokens": 5
    });
    let agent = ureq::Agent::new_with_defaults();
    let mut req = agent.post(&url).header("Content-Type", "application/json");
    if !api_key.trim().is_empty() {
        req = req.header("Authorization", &format!("Bearer {}", api_key.trim()));
    }
    let body_str = serde_json::to_string(&body).unwrap_or_default();
    match req.send(&body_str) {
        Ok(mut resp) => {
            let mut buf = String::new();
            let _ = resp.body_mut().as_reader().read_to_string(&mut buf);
            if let Ok(v) = serde_json::from_str::<Value>(&buf) {
                if let Some(err) = v.get("error").and_then(|e| e.get("message")).and_then(Value::as_str) {
                    return (false, format!("❌ API Error: {err}"));
                }
                if v.get("choices").is_some() {
                    return (true, format!("✅ Connection successful ({model_id})!"));
                }
            }
            (true, "✅ Endpoint reachable!".to_string())
        }
        Err(e) => (false, format!("❌ Connection failed: {e}")),
    }
}

fn render_ai_icon(ui: &mut egui::Ui, tex: &TextureHandle, size: f32) {
    ui.add(egui::Image::new(tex).fit_to_exact_size(vec2(size, size)).corner_radius(CornerRadius::same(4)));
}

fn app_tools_definition() -> Value {
    crate::control::mcp_tool_definitions()
}

fn is_pure_greeting_or_smalltalk(prompt: &str) -> bool {
    let clean: String = prompt
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    matches!(
        clean.as_str(),
        "hi"
            | "hello"
            | "hallo"
            | "hey"
            | "moin"
            | "servus"
            | "guten tag"
            | "guten morgen"
            | "guten abend"
            | "wie gehts"
            | "wie geht es dir"
            | "wer bist du"
            | "was kannst du"
            | "test"
            | "ping"
    )
}

fn friendly_command_badge(tool_name: &str, args: &Value) -> String {
    if tool_name == "execute" {
        if let Some(cmd) = args.get("command").and_then(Value::as_str) {
            return cmd.to_string();
        }
    }
    tool_name.to_string()
}

fn extract_and_strip_json_actions(text: &str) -> (String, Vec<(String, Value)>) {
    let mut actions = Vec::new();
    let mut cleaned = String::new();
    let mut rest = text;

    while let Some(start) = rest.find("```") {
        cleaned.push_str(&rest[..start]);
        let after_ticks = &rest[start + 3..];
        if let Some(end) = after_ticks.find("```") {
            let block = &after_ticks[..end];
            let first_nl = block.find('\n').unwrap_or(0);
            let tag = block[..first_nl].trim().to_lowercase();
            let body = if first_nl > 0 { block[first_nl + 1..].trim() } else { block.trim() };

            let mut parsed_as_actions = false;
            if tag == "json_actions" || tag == "json" || tag.is_empty() {
                if let Ok(val) = serde_json::from_str::<Value>(body) {
                    if let Some(arr) = val.as_array() {
                        for item in arr {
                            if let Some(t) = item.get("tool").or_else(|| item.get("name")).or_else(|| item.get("command")).and_then(Value::as_str) {
                                let p = item.get("params").or_else(|| item.get("arguments")).cloned().unwrap_or_else(|| item.clone());
                                actions.push((t.to_string(), p));
                                parsed_as_actions = true;
                            }
                        }
                    } else if let Some(t) = val.get("tool").or_else(|| val.get("name")).and_then(Value::as_str) {
                        let p = val.get("params").or_else(|| val.get("arguments")).cloned().unwrap_or_else(|| val.clone());
                        actions.push((t.to_string(), p));
                        parsed_as_actions = true;
                    }
                }
            }

            if !parsed_as_actions {
                cleaned.push_str(&rest[start..start + 3 + end + 3]);
            }
            rest = &after_ticks[end + 3..];
        } else {
            cleaned.push_str(&rest[start..]);
            rest = "";
            break;
        }
    }
    cleaned.push_str(rest);
    (cleaned.trim().to_string(), actions)
}

fn build_rich_summary_for_executed_tools(executed_details: &[(String, Value, Value)]) -> String {
    let mut md = format!("### ✨ Executed in {APP_NAME}\n\n");
    for (tool, params, _res) in executed_details {
        match tool.as_str() {
            "deck.generatePresentation" => {
                let title = params.get("title").and_then(Value::as_str).unwrap_or("Presentation");
                let count = params.get("slides").and_then(Value::as_array).map(|a| a.len()).unwrap_or(1);
                md.push_str(&format!("- **Presentation created:** *{title}* ({count} slides)\n"));
                if let Some(slides) = params.get("slides").and_then(Value::as_array) {
                    for (idx, s) in slides.iter().enumerate() {
                        let st = s.get("title").and_then(Value::as_str).unwrap_or("Slide");
                        let lay = s.get("layout").and_then(Value::as_str).unwrap_or("bullets");
                        md.push_str(&format!("  {}. **{}** (`{}`)\n", idx + 1, st, lay));
                    }
                }
            }
            "deck.addRichSlide" | "add_slide" => {
                let title = params.get("title").and_then(Value::as_str).unwrap_or("New Slide");
                let lay = params.get("layout").and_then(Value::as_str).unwrap_or("bullets");
                md.push_str(&format!("- **New slide added:** *{title}* (Layout: `{lay}`)\n"));
            }
            "doc.insertStyledTable" => {
                let title = params.get("title").and_then(Value::as_str).unwrap_or("Table");
                let cols = params.get("headers").and_then(Value::as_array).map(|a| a.len()).unwrap_or(0);
                let rows = params.get("rows").and_then(Value::as_array).map(|a| a.len()).unwrap_or(0);
                md.push_str(&format!("- **Styled table inserted:** *{title}* ({cols} columns × {rows} rows)\n"));
            }
            "doc.insertRichLayout" | "doc.clearAndWrite" => {
                let title = params.get("title").and_then(Value::as_str).unwrap_or("Document Layout");
                md.push_str(&format!("- **Formatted document layout inserted:** *{title}*\n"));
            }
            "grid.createFancyTable" | "write_range" => {
                let start = params.get("startCell").and_then(Value::as_str).unwrap_or("A1");
                let rows = params.get("rows").or_else(|| params.get("values")).and_then(Value::as_array).map(|a| a.len()).unwrap_or(0);
                md.push_str(&format!("- **Table range starting at `{start}` populated & formatted** ({rows} data rows)\n"));
            }
            _ => {
                md.push_str(&format!("- **`{tool}`** applied successfully\n"));
            }
        }
    }
    md
}

fn worker_loop(
    cmd_rx: Receiver<AgentWorkerCmd>,
    tool_tx: Sender<ToolExecutionRequest>,
    event_tx: Sender<AgentEvent>,
) {
    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            AgentWorkerCmd::RunPrompt {
                prompt,
                messages,
                bot,
                doc_text,
                project_dir_str,
            } => {
                let actual_model_id = bot.api_model_id.as_str();
                let _ = event_tx.send(AgentEvent::StatusUpdate(format!("Querying {}...", bot.name)));

                let endpoint = normalize_endpoint(&bot.endpoint);

                let excerpt = if doc_text.len() > 8000 { &doc_text[..8000] } else { &doc_text };
                let system_prompt = format!(
                    "You are Craft AI (operating 1:1 like Google Gemini in Google Workspace), directly integrated into {APP_NAME} ({APP_ROLE}).\n\
                    Active Google Drive Project Directory: {project_dir_str}\n\
                    Live Context / Snapshot from {APP_NAME}:\n{excerpt}\n\n\
                    CORE BEHAVIORAL RULES:\n\
                    0. LANGUAGE & CONVERSATION POLICY (MANDATORY):\n\
                       - By default, communicate in English.\n\
                       - AUTOMATIC LANGUAGE MATCHING: You MUST ALWAYS detect the language used by the user in their prompt and reply in that EXACT same language (e.g. if the user prompts in German, reply in German; if in French, reply in French; if in Spanish, reply in Spanish; if in Japanese, reply in Japanese; etc.).\n\
                    1. TRUE CO-CREATOR & DIRECT APPLICATION CONTROL:\n\
                       - When the user requests creating, adding, editing, styling, or transforming content in {APP_NAME} (e.g. presentations/slides, documents/tables, spreadsheet formulas/ranges, PDF annotations, video timeline or studio recording), execute the appropriate high-level tools ({APP_COMMANDS_HINT}) with complete, rich parameters!\n\
                       - If {APP_NAME} is on the start / welcome screen, your tools automatically initialize a new active document / project.\n\
                       - You can invoke tools via native function-calling (`tool_calls`) OR append a ```json_actions\\n[{{\"tool\": \"...\", \"params\": {{...}}}}]\\n``` block at the very end of your response, which will be automatically executed and hidden from the chat text.\n\
                    2. CLEAN & STRUCTURED MARKDOWN IN CHAT:\n\
                       - Always format your chat responses clearly using Markdown (`###` headings, **bold**, lists, or Markdown tables `| Column A | Column B |`). Keep summaries concise when actions are executed directly in the document.\n\
                       - For pure greetings ('Hi', 'Hello') or questions, reply helpfully in the chat without triggering unnecessary tools.\n\
                    3. STRICT LENGTH & QUANTITY DISCIPLINE:\n\
                       - When the user specifies an exact count or duration (e.g. '3 pages', '5 slides', '10 rows', '30 seconds', 'short clip'), you MUST generate exactly that requested count/length—neither more nor less."
                );

                let mut api_messages: Vec<Value> = Vec::new();
                api_messages.push(json!({"role": "system", "content": system_prompt}));

                for m in &messages {
                    let mut obj = json!({"role": m.role, "content": m.content});
                    if let Some(ref tc) = m.tool_calls {
                        obj["tool_calls"] = tc.clone();
                    }
                    if let Some(ref tid) = m.tool_call_id {
                        obj["tool_call_id"] = json!(tid);
                    }
                    if let Some(ref name) = m.name {
                        obj["name"] = json!(name);
                    }
                    api_messages.push(obj);
                }

                let mut step = 0;
                let max_steps = 6;
                let allow_tools = !is_pure_greeting_or_smalltalk(&prompt);
                let tools = app_tools_definition();
                let mut has_emitted_reply = false;
                let mut fatal_error = false;
                let mut executed_badges: Vec<String> = Vec::new();
                let mut executed_details: Vec<(String, Value, Value)> = Vec::new();

                while step < max_steps {
                    step += 1;
                    let _ = event_tx.send(AgentEvent::StatusUpdate(format!("{} is working...", bot.name)));

                    let mut body = json!({
                        "model": actual_model_id,
                        "messages": api_messages,
                        "temperature": 0.4,
                    });

                    if allow_tools {
                        if let Some(t_arr) = tools.as_array() {
                            if !t_arr.is_empty() {
                                body["tools"] = tools.clone();
                                body["tool_choice"] = json!("auto");
                            }
                        }
                    }

                    let agent = ureq::Agent::new_with_defaults();
                    let body_str = serde_json::to_string(&body).unwrap_or_default();
                    let mut req = agent.post(&endpoint).header("Content-Type", "application/json");
                    if !bot.api_key.trim().is_empty() {
                        req = req.header("Authorization", &format!("Bearer {}", bot.api_key.trim()));
                    }

                    let mut response_res = req.send(&body_str);

                    // Fallback: if an endpoint rejects `tools` parameter, retry without `tools`
                    if response_res.is_err() && body.get("tools").is_some() {
                        let fallback_body = json!({
                            "model": actual_model_id,
                            "messages": api_messages,
                            "temperature": 0.5,
                        });
                        let fb_str = serde_json::to_string(&fallback_body).unwrap_or_default();
                        let mut fb_req = agent.post(&endpoint).header("Content-Type", "application/json");
                        if !bot.api_key.trim().is_empty() {
                            fb_req = fb_req.header("Authorization", &format!("Bearer {}", bot.api_key.trim()));
                        }
                        response_res = fb_req.send(&fb_str);
                    }

                    let response_val = match response_res {
                        Ok(mut r) => {
                            let mut buf = String::new();
                            let _ = r.body_mut().as_reader().read_to_string(&mut buf);
                            serde_json::from_str::<Value>(&buf)
                        }
                        Err(e) => {
                            fatal_error = true;
                            let _ = event_tx.send(AgentEvent::Error(format!("Network error: {e}")));
                            break;
                        }
                    };

                    let response_json = match response_val {
                        Ok(v) => v,
                        Err(e) => {
                            fatal_error = true;
                            let _ = event_tx.send(AgentEvent::Error(format!("Invalid API response (JSON): {e}")));
                            break;
                        }
                    };

                    if let Some(err_msg) = response_json.get("error").and_then(|e| e.get("message")).and_then(Value::as_str) {
                        fatal_error = true;
                        let _ = event_tx.send(AgentEvent::Error(err_msg.to_string()));
                        break;
                    }

                    if let Some(usage) = response_json.get("usage") {
                        let total = usage
                            .get("total_tokens")
                            .and_then(Value::as_u64)
                            .unwrap_or_else(|| {
                                let p = usage.get("prompt_tokens").and_then(Value::as_u64).unwrap_or(0);
                                let c = usage.get("completion_tokens").and_then(Value::as_u64).unwrap_or(0);
                                p + c
                            }) as usize;
                        if total > 0 {
                            let _ = event_tx.send(AgentEvent::TokensUsed {
                                bot_id: bot.id.clone(),
                                tokens: total,
                            });
                        }
                    }

                    let Some(choice) = response_json.get("choices").and_then(Value::as_array).and_then(|a| a.first()) else {
                        fatal_error = true;
                        let _ = event_tx.send(AgentEvent::Error("Empty response received from API endpoint.".to_string()));
                        break;
                    };

                    let message_obj = choice.get("message").cloned().unwrap_or(json!({}));
                    api_messages.push(message_obj.clone());

                    let raw_text_content = message_obj
                        .get("content")
                        .and_then(Value::as_str)
                        .or_else(|| message_obj.get("text").and_then(Value::as_str))
                        .unwrap_or("");

                    let (cleaned_text, inline_actions) = if allow_tools {
                        extract_and_strip_json_actions(raw_text_content)
                    } else {
                        (raw_text_content.trim().to_string(), Vec::new())
                    };

                    let tool_calls = message_obj.get("tool_calls").and_then(Value::as_array);
                    let has_native_tools = tool_calls.is_some_and(|tc| !tc.is_empty());

                    // Execute any inline `json_actions` blocks if no native tool_calls were emitted
                    if !has_native_tools && !inline_actions.is_empty() {
                        for (func_name, args_val) in inline_actions {
                            let badge = friendly_command_badge(&func_name, &args_val);
                            let _ = event_tx.send(AgentEvent::StatusUpdate(format!("Executing app command: {badge}...")));

                            let (reply_tx, reply_rx) = channel();
                            let _ = tool_tx.send(ToolExecutionRequest {
                                tool_name: func_name.clone(),
                                params: args_val.clone(),
                                reply_sender: reply_tx,
                            });

                            if let Ok(Ok(val)) = reply_rx.recv_timeout(Duration::from_secs(12)) {
                                let is_err = val.get("error").is_some()
                                    || val.get("status").and_then(Value::as_str) == Some("error");
                                if !is_err {
                                    executed_badges.push(badge);
                                    executed_details.push((func_name.clone(), args_val.clone(), val.clone()));
                                }
                                let result_str = serde_json::to_string(&val).unwrap_or_else(|_| "{}".to_string());
                                let _ = event_tx.send(AgentEvent::ToolExecutionLog {
                                    tool: func_name,
                                    args: serde_json::to_string(&args_val).unwrap_or_default(),
                                    result: result_str,
                                });
                            }
                        }
                    }

                    if !cleaned_text.is_empty() && !has_native_tools {
                        has_emitted_reply = true;
                        let _ = event_tx.send(AgentEvent::AssistantReply {
                            text: cleaned_text,
                            executed_commands: std::mem::take(&mut executed_badges),
                        });
                    }

                    if let Some(tool_calls_list) = tool_calls {
                        if tool_calls_list.is_empty() {
                            break;
                        }

                        for tc in tool_calls_list {
                            let call_id = tc.get("id").and_then(Value::as_str).unwrap_or("").to_string();
                            let func_name = tc.get("function").and_then(|f| f.get("name")).and_then(Value::as_str).unwrap_or("").to_string();
                            let args_str = tc.get("function").and_then(|f| f.get("arguments")).and_then(Value::as_str).unwrap_or("{}");
                            let args_val = serde_json::from_str::<Value>(args_str).unwrap_or(json!({}));

                            let badge = friendly_command_badge(&func_name, &args_val);
                            let _ = event_tx.send(AgentEvent::StatusUpdate(format!("Executing app command: {badge}...")));

                            let (reply_tx, reply_rx) = channel();
                            let _ = tool_tx.send(ToolExecutionRequest {
                                tool_name: func_name.clone(),
                                params: args_val.clone(),
                                reply_sender: reply_tx,
                            });

                            let result_val = reply_rx
                                .recv_timeout(Duration::from_secs(12))
                                .unwrap_or_else(|_| Err("Timeout executing command".to_string()));
                            let result_str = match result_val {
                                Ok(v) => {
                                    let is_err = v.get("error").is_some()
                                        || v.get("status").and_then(Value::as_str) == Some("error");
                                    if !is_err {
                                        executed_badges.push(badge.clone());
                                        executed_details.push((func_name.clone(), args_val.clone(), v.clone()));
                                    }
                                    serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string())
                                }
                                Err(e) => json!({"status": "error", "message": e}).to_string(),
                            };

                            let _ = event_tx.send(AgentEvent::ToolExecutionLog {
                                tool: func_name.clone(),
                                args: args_str.to_string(),
                                result: result_str.clone(),
                            });

                            api_messages.push(json!({
                                "role": "tool",
                                "tool_call_id": call_id,
                                "name": func_name,
                                "content": result_str,
                            }));
                        }
                    } else {
                        break;
                    }
                }

                if !has_emitted_reply && !fatal_error {
                    let fallback = if !executed_details.is_empty() {
                        build_rich_summary_for_executed_tools(&executed_details)
                    } else {
                        format!("How can I help you in {APP_NAME}?")
                    };
                    let _ = event_tx.send(AgentEvent::AssistantReply {
                        text: fallback,
                        executed_commands: executed_badges,
                    });
                }

                let _ = event_tx.send(AgentEvent::Finished);
            }
        }
    }
}
