//! Craft Suite Global Shared Theme & Header/Footer Action Components
//!
//! Synchronizes Light, Dark, and Aurora themes plus uniform Top-Right Action Bar
//! and Bottom-Right Theme Switcher across DeckCraft, WordCraft, GridCraft, PdfCraft, FilmCraft, and RecordCraft.

use egui::{Color32, Stroke, pos2, vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AppTheme {
    #[default]
    Dark,
    Light,
    Aurora,
}

impl AppTheme {
    pub const ALL: [AppTheme; 3] = [AppTheme::Aurora, AppTheme::Dark, AppTheme::Light];

    pub fn code(&self) -> &'static str {
        match self {
            AppTheme::Dark => "dark",
            AppTheme::Light => "light",
            AppTheme::Aurora => "aurora",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AppTheme::Dark => "Dark",
            AppTheme::Light => "Light",
            AppTheme::Aurora => "Aurora",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            AppTheme::Dark => "Dark",
            AppTheme::Light => "Light",
            AppTheme::Aurora => "Aurora",
        }
    }

    pub fn from_code(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "dark" => Some(AppTheme::Dark),
            "light" => Some(AppTheme::Light),
            "aurora" | "prism" => Some(AppTheme::Aurora),
            _ => None,
        }
    }

    pub fn load_global() -> Self {
        for p in shared_theme_paths() {
            if p.exists() {
                if let Ok(data) = std::fs::read_to_string(&p) {
                    if let Ok(cfg) = serde_json::from_str::<ThemeConfig>(&data) {
                        return cfg.theme;
                    }
                }
            }
        }
        AppTheme::Dark
    }

    pub fn save_global(theme: AppTheme) {
        let cfg = ThemeConfig { theme };
        if let Ok(json) = serde_json::to_string_pretty(&cfg) {
            for p in shared_theme_paths() {
                if let Some(parent) = p.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&p, &json);
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ThemeConfig {
    theme: AppTheme,
}

pub fn shared_theme_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    if let Some(prof) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        let base = std::path::PathBuf::from(prof);
        let craft_dir = base.join(".craft-suite");
        let _ = std::fs::create_dir_all(&craft_dir);
        paths.push(craft_dir.join("artcraft_theme.json"));

        let dir = base.join(".artcraft");
        let _ = std::fs::create_dir_all(&dir);
        paths.push(dir.join("artcraft_theme.json"));
    }
    paths
}

// =========================================================================
// Spectacular Gemini Rainbow Aurora Color Scheme Constants
// =========================================================================
pub const GEMINI_CYAN: Color32 = Color32::from_rgb(56, 189, 248);     // #38bdf8
pub const GEMINI_BLUE: Color32 = Color32::from_rgb(66, 133, 244);     // #4285f4
pub const GEMINI_PURPLE: Color32 = Color32::from_rgb(168, 85, 247);   // #a855f7
pub const GEMINI_VIOLET: Color32 = Color32::from_rgb(192, 132, 252);  // #c084fc
pub const GEMINI_MAGENTA: Color32 = Color32::from_rgb(236, 72, 153);  // #ec4899
pub const GEMINI_AMBER: Color32 = Color32::from_rgb(251, 146, 60);    // #fb923c

pub const AURORA_BG: Color32 = Color32::from_rgb(9, 11, 20);         // #090b14 Deep cosmic midnight-indigo         // Deep cosmic nebula space
pub const AURORA_PANEL: Color32 = Color32::from_rgb(15, 19, 34);      // #0f1322 Dark nebula-navy panel      // Twilight indigo panel
pub const AURORA_CARD: Color32 = Color32::from_rgb(22, 27, 48);       // #161b30 Iridescent indigo-slate card       // Dark iridescent card fill
pub const AURORA_BORDER: Color32 = Color32::from_rgba_premultiplied(31, 20, 54, 56); // rgba(139, 92, 246, 0.22) Specular violet     // Subtle cosmic border
pub const AURORA_TEXT: Color32 = Color32::from_rgb(241, 243, 249);    // #f1f3f9 Crisp starlight white    // Crisp starlight white
pub const AURORA_TEXT_MUTED: Color32 = Color32::from_rgb(148, 163, 184);// #94a3b8 Muted stardust slate// Muted stardust blue

/// Renders a horizontal Gemini iridescent rainbow gradient line [Cyan -> Blue -> Purple -> Magenta -> Amber]
pub fn paint_gemini_rainbow_line(painter: &egui::Painter, rect: egui::Rect) {
    let w = rect.width();
    let segments = [
        (0.0, 0.20, GEMINI_CYAN, GEMINI_BLUE),
        (0.20, 0.45, GEMINI_BLUE, GEMINI_PURPLE),
        (0.45, 0.70, GEMINI_PURPLE, GEMINI_MAGENTA),
        (0.70, 1.0, GEMINI_MAGENTA, GEMINI_AMBER),
    ];
    for (f_start, f_end, c1, c2) in segments {
        let seg_rect = egui::Rect::from_min_max(
            pos2(rect.min.x + w * f_start, rect.min.y),
            pos2(rect.min.x + w * f_end, rect.max.y),
        );
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(seg_rect.left_top(), c1);
        mesh.colored_vertex(seg_rect.right_top(), c2);
        mesh.colored_vertex(seg_rect.right_bottom(), c2);
        mesh.colored_vertex(seg_rect.left_bottom(), c1);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(egui::Shape::mesh(mesh));
    }
}

/// Renders a delicate iridescent gradient glow stroke around a card
pub fn paint_aurora_card_glow(painter: &egui::Painter, rect: egui::Rect, radius: f32) {
    let glow = Color32::from_rgba_unmultiplied(168, 85, 247, 18);
    painter.rect_filled(rect, radius, glow);
    painter.rect_stroke(rect, radius, Stroke::new(1.0, Color32::from_rgb(95, 105, 180)), egui::StrokeKind::Inside);
}

/// Renders a delicate 1px gradient stroke around a rect with subtle glow background
pub fn paint_aurora_border(painter: &egui::Painter, rect: egui::Rect, corner_radius: f32) {
    let glow = Color32::from_rgba_unmultiplied(79, 172, 254, 18);
    painter.rect_filled(rect, corner_radius, glow);
    painter.rect_stroke(rect, corner_radius, Stroke::new(1.0, GEMINI_VIOLET), egui::StrokeKind::Inside);
}

/// Renders a GPU-interpolated 135deg Aurora gradient rectangle
pub fn paint_aurora_gradient_rect(painter: &egui::Painter, rect: egui::Rect) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), GEMINI_CYAN);
    mesh.colored_vertex(rect.right_top(), GEMINI_PURPLE);
    mesh.colored_vertex(rect.right_bottom(), GEMINI_AMBER);
    mesh.colored_vertex(rect.left_bottom(), GEMINI_MAGENTA);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

/// Universal uniform theme switcher dropdown for 28px Footers across all 6 applications
/// Order: 1. ✨ Aurora, 2. 🌙 Dark, 3. ☀️ Light
pub fn render_theme_switcher_dropdown(ui: &mut egui::Ui, current_theme: &mut AppTheme) -> bool {
    let mut changed = false;
    let (icon_text, label_text) = match *current_theme {
        AppTheme::Aurora => ("✨", "Aurora"),
        AppTheme::Dark => ("🌙", "Dark"),
        AppTheme::Light => ("☀️", "Light"),
    };
    let button_text = format!("{icon_text} {label_text} ▾");

    egui::ComboBox::from_id_salt("craft_universal_footer_theme_dropdown")
        .selected_text(egui::RichText::new(&button_text).size(11.5))
        .width(105.0)
        .show_ui(ui, |ui| {
            for theme in &[AppTheme::Aurora, AppTheme::Dark, AppTheme::Light] {
                let (t_icon, t_name) = match theme {
                    AppTheme::Aurora => ("✨", "Aurora"),
                    AppTheme::Dark => ("🌙", "Dark"),
                    AppTheme::Light => ("☀️", "Light"),
                };
                let is_selected = *current_theme == *theme;
                let item_text = format!("{t_icon} {t_name}");
                if ui.selectable_label(is_selected, egui::RichText::new(item_text).size(12.0)).clicked() {
                    if *current_theme != *theme {
                        *current_theme = *theme;
                        AppTheme::save_global(*theme);
                        changed = true;
                    }
                }
            }
        });
    changed
}

/// Universal uniform AI Agent Pill Button ("✨ AI Agent", height 28px, border-radius: 9999px)
pub fn render_ai_agent_button(ui: &mut egui::Ui, is_open: bool) -> egui::Response {
    let desired_size = vec2(106.0, 28.0);
    let (rect, resp) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let hovered = resp.hovered();
    let is_dark = ui.visuals().dark_mode;

    let painter = ui.painter();
    let radius = 14.0; // 28px height -> 14px radius = 9999px full pill!

    if is_open {
        let bg_fill = if hovered {
            Color32::from_rgb(109, 72, 235)
        } else {
            Color32::from_rgb(94, 60, 218)
        };
        painter.rect_filled(rect, radius, bg_fill);
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.2, Color32::from_rgb(185, 145, 255)),
            egui::StrokeKind::Inside,
        );
    } else {
        let (bg_fill, stroke_col) = if is_dark {
            let fill = if hovered {
                Color32::from_rgba_unmultiplied(255, 255, 255, 26)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 14)
            };
            let stroke = if hovered {
                Color32::from_rgba_unmultiplied(168, 85, 247, 140)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 36)
            };
            (fill, stroke)
        } else {
            let fill = if hovered {
                Color32::from_rgba_unmultiplied(15, 23, 42, 22)
            } else {
                Color32::from_rgba_unmultiplied(15, 23, 42, 10)
            };
            let stroke = if hovered {
                Color32::from_rgb(124, 58, 237)
            } else {
                Color32::from_rgba_unmultiplied(15, 23, 42, 42)
            };
            (fill, stroke)
        };
        painter.rect_filled(rect, radius, bg_fill);
        painter.rect_stroke(rect, radius, Stroke::new(1.0, stroke_col), egui::StrokeKind::Inside);
    }

    let text_col = if is_open {
        Color32::WHITE
    } else if is_dark {
        Color32::from_rgb(230, 232, 245)
    } else {
        Color32::from_rgb(30, 41, 59)
    };

    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "✨ AI Agent",
        egui::FontId::proportional(12.5),
        text_col,
    );

    resp.on_hover_text("Toggle AI Agent Sidebar")
}

/// Renders the standardized Top-Right Action Bar inside a `right_to_left` layout so that
/// from left to right visually the order is:
/// 1. `✓ Auto-Save` (28px height, 12px font)
/// 2. `● Google Drive: Active (G:)` (28px height, 12px font)
/// 3. `✨ AI Agent` (Pill Button, 28px height, 14px radius, 12.5px font)
pub fn render_top_right_action_bar(
    ui: &mut egui::Ui,
    auto_save_enabled: &mut bool,
    drive_detected: bool,
    proj_name: &str,
    proj_dir: &std::path::Path,
    agent_is_open: &mut bool,
) {
    let is_dark = ui.visuals().dark_mode;

    // 3. Far-Right (first in right_to_left): AI-Sidebar Toggle Pill Button ("✨ AI Agent")
    if render_ai_agent_button(ui, *agent_is_open).clicked() {
        *agent_is_open = !*agent_is_open;
    }

    ui.add_space(8.0);

    // 2. Middle: Cloud-Status Badge (Dynamic Provider & User-Configurable Folder Modal)
    let modal_id = ui.make_persistent_id("craft_suite_cloud_sync_modal_open");
    let cfg_id = ui.make_persistent_id("craft_suite_cloud_sync_modal_cfg");
    let mut cloud_modal_open = ui
        .ctx()
        .data_mut(|d| d.get_temp::<bool>(modal_id).unwrap_or(false));
    let mut cloud_cfg = ui
        .ctx()
        .data_mut(|d| d.get_temp::<CraftSuiteCloudConfig>(cfg_id))
        .unwrap_or_else(CraftSuiteCloudConfig::load);

    let active_sync_ok = drive_detected || std::path::Path::new(&cloud_cfg.sync_path).exists();
    let dot_col = if active_sync_ok {
        Color32::from_rgb(34, 197, 94) // #22c55e green indicator dot
    } else {
        Color32::from_rgb(148, 163, 184)
    };
    let text_col = if is_dark {
        Color32::from_rgb(215, 222, 235)
    } else {
        Color32::from_rgb(51, 65, 85)
    };
    let provider_name = if cloud_cfg.provider.trim().is_empty() {
        "Cloud Sync"
    } else {
        cloud_cfg.provider.trim()
    };
    let status_str = if active_sync_ok {
        format!("{provider_name}: Active")
    } else {
        format!("{provider_name}: Offline")
    };

    let mut job = egui::text::LayoutJob::default();
    job.append(
        "● ",
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(12.0),
            color: dot_col,
            ..Default::default()
        },
    );
    job.append(
        &status_str,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(12.0),
            color: text_col,
            ..Default::default()
        },
    );

    let drive_btn = egui::Button::new(job)
        .min_size(vec2(0.0, 28.0))
        .corner_radius(6.0)
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::NONE);

    let drive_tooltip = format!(
        "{}: {}\nProject: {}\nSync Folder: {}\n\nLeft-click: Configure Cloud & Sync Folder\nRight-click: Open project folder in Explorer",
        provider_name,
        if active_sync_ok { "Active" } else { "Offline" },
        proj_name,
        proj_dir.display()
    );
    let drive_resp = ui.add(drive_btn).on_hover_text(drive_tooltip);
    if drive_resp.clicked() {
        cloud_cfg = CraftSuiteCloudConfig::load();
        cloud_cfg.auto_save = *auto_save_enabled;
        cloud_modal_open = !cloud_modal_open;
    } else if drive_resp.secondary_clicked() {
        let _ = std::fs::create_dir_all(proj_dir);
        let _ = std::process::Command::new("explorer").arg(proj_dir).spawn();
    }

    if cloud_modal_open {
        let prev_path = cloud_cfg.sync_path.clone();
        let prev_prov = cloud_cfg.provider.clone();
        let prev_auto = cloud_cfg.auto_save;
        render_cloud_sync_modal(ui.ctx(), &mut cloud_modal_open, &mut cloud_cfg, Some(proj_dir));
        if cloud_cfg.auto_save != prev_auto || cloud_cfg.sync_path != prev_path || cloud_cfg.provider != prev_prov {
            *auto_save_enabled = cloud_cfg.auto_save;
        }
    }
    ui.ctx().data_mut(|d| {
        d.insert_temp(modal_id, cloud_modal_open);
        d.insert_temp(cfg_id, cloud_cfg);
    });

    ui.add_space(8.0);

    // 1. Leftmost of the 3: Auto-Save Toggle/Status ("✓ Auto-Save")
    let auto_label = if *auto_save_enabled {
        "✓ Auto-Save"
    } else {
        "○ Auto-Save"
    };
    let auto_col = if *auto_save_enabled {
        if is_dark {
            Color32::from_rgb(203, 213, 225)
        } else {
            Color32::from_rgb(51, 65, 85)
        }
    } else {
        Color32::from_rgb(120, 130, 148)
    };
    let auto_btn = egui::Button::new(egui::RichText::new(auto_label).size(12.0).color(auto_col))
        .min_size(vec2(0.0, 28.0))
        .corner_radius(6.0)
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::NONE);

    if ui
        .add(auto_btn)
        .on_hover_text("Auto-Save: Automatically save changes to the Google Drive project folder")
        .clicked()
    {
        *auto_save_enabled = !*auto_save_enabled;
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CraftSuiteCloudConfig {
    pub provider: String,
    pub sync_path: String,
    pub auto_save: bool,
}

impl Default for CraftSuiteCloudConfig {
    fn default() -> Self {
        let (provider, sync_path) = if std::path::Path::new(r"G:").exists() {
            ("Google Drive".to_string(), r"G:".to_string())
        } else if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            let docs = std::path::PathBuf::from(user_profile).join("Documents");
            ("Local Folder".to_string(), docs.to_string_lossy().to_string())
        } else {
            ("Local Folder".to_string(), r"C:\CraftSuite_Sync".to_string())
        };
        Self {
            provider,
            sync_path,
            auto_save: true,
        }
    }
}

impl CraftSuiteCloudConfig {
    pub fn config_file() -> Option<std::path::PathBuf> {
        if let Some(user_profile) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
            Some(std::path::PathBuf::from(user_profile).join(".craft-suite").join("cloud-config.json"))
        } else {
            None
        }
    }

    pub fn load() -> Self {
        if let Some(p) = Self::config_file() {
            if p.exists() {
                if let Ok(d) = std::fs::read_to_string(&p) {
                    if let Ok(cfg) = serde_json::from_str::<CraftSuiteCloudConfig>(&d) {
                        return cfg;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(p) = Self::config_file() {
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(d) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(&p, d);
            }
        }
    }
}

pub fn render_cloud_sync_modal(
    ctx: &egui::Context,
    open: &mut bool,
    config: &mut CraftSuiteCloudConfig,
    proj_dir: Option<&std::path::Path>,
) {
    if !*open {
        return;
    }

    let mut should_close = false;
    let mut should_save = false;

    egui::Window::new("☁️ Cloud & Auto-Save Sync")
        .open(open)
        .collapsible(false)
        .resizable(false)
        .default_width(440.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Configure your cloud drive provider or local synchronization folder.").size(12.0).color(Color32::from_rgb(160, 175, 205)));
            ui.add_space(10.0);

            ui.label(egui::RichText::new("Cloud Provider:").strong().size(12.5));
            egui::ComboBox::from_id_salt("craft_cloud_provider_dropdown")
                .selected_text(&config.provider)
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for p in &["Google Drive", "OneDrive", "Nextcloud", "Dropbox", "Local Folder"] {
                        ui.selectable_value(&mut config.provider, p.to_string(), *p);
                    }
                });

            ui.add_space(10.0);
            ui.label(egui::RichText::new("Sync Path / Drive Root:").strong().size(12.5));
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut config.sync_path).desired_width(ui.available_width() - 130.0));
                if ui.button("📂 Choose Folder").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        config.sync_path = folder.to_string_lossy().to_string();
                    }
                }
            });

            ui.add_space(12.0);
            ui.checkbox(&mut config.auto_save, "Enable automatic background project saving");

            if let Some(pd) = proj_dir {
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Active Directory:").size(11.5).color(Color32::GRAY));
                    ui.label(egui::RichText::new(pd.to_string_lossy()).size(11.0).monospace());
                });
                if ui.button("📁 Open in Windows Explorer").clicked() {
                    let _ = std::fs::create_dir_all(pd);
                    let _ = std::process::Command::new("explorer").arg(pd).spawn();
                }
            }

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("Save & Apply").strong().color(Color32::WHITE)).clicked() {
                    should_save = true;
                    should_close = true;
                }
                if ui.button("Cancel").clicked() {
                    should_close = true;
                }
            });
        });

    if should_save {
        config.save();
    }
    if should_close {
        *open = false;
    }
}
