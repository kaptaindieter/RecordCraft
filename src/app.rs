use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use egui::{Color32, CornerRadius, Rect, RichText, Stroke, pos2, vec2};

use crate::artcraft_agent::ArtCraftAgent;
use crate::artcraft_drive::RecordCraftDrive;
use crate::artcraft_theme::{paint_aurora_border, AppTheme};
use crate::audio_engine::{poll_system_audio, set_app_volume_by_name, set_process_volume, AudioAppSession, SystemAudioState};
use crate::hotkeys::{HotkeyEvent, HotkeyManager};
use crate::preview::ScreenPreview;
use crate::recorder::{
    detect_system_monitors, get_active_windows, get_available_microphones,
    AudioIsolationMode, AudioRoutingMode, CaptureSourceChoice, ContainerFormat, DetectedMonitor,
    QualityPreset, RecorderConfig, RecorderState, ResolutionMode, VideoCodec,
};

pub struct RecordCraftApp {
    pub config: RecorderConfig,
    pub recorder: RecorderState,
    pub drive: RecordCraftDrive,
    pub hotkeys: HotkeyManager,
    pub agent: ArtCraftAgent,
    pub preview: ScreenPreview,
    pub monitors: Vec<DetectedMonitor>,
    pub available_windows: Vec<String>,
    pub available_mics: Vec<String>,
    pub status_message: String,
    pub system_audio_vol: f32,
    pub mic_audio_vol: f32,
    pub anim_phase: f32,
    pub show_audio_guide: bool,
    pub audio_state: SystemAudioState,
    pub selected_app_pids: HashSet<u32>,
    pub selected_app_names: HashSet<String>,
    pub show_all_running_apps: bool,
    pub muted_during_recording: Vec<(String, f32)>,
    pub smoothed_mic_peak: f32,
    pub smoothed_master_peak: f32,
    pub smoothed_app_peaks: HashMap<String, f32>,
    pub mic_muted: bool,
    pub master_muted: bool,
    pub theme: AppTheme,
    pub control_rx: Option<std::sync::mpsc::Receiver<crate::control::ControlRequest>>,
}

pub struct Palette {
    pub bg: Color32,
    pub panel_bg: Color32,
    pub card_bg: Color32,
    pub card_border: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub input_bg: Color32,
    pub input_border: Color32,
    pub is_aurora: bool,
    pub is_light: bool,
}

impl RecordCraftApp {
    pub fn palette(&self) -> Palette {
        match self.theme {
            AppTheme::Aurora => Palette {
                bg: Color32::from_rgb(9, 11, 20),
                panel_bg: Color32::from_rgb(15, 19, 34),
                card_bg: Color32::from_rgb(22, 27, 48),
                card_border: Color32::from_rgba_unmultiplied(139, 92, 246, 56),
                text: Color32::from_rgb(241, 243, 249),
                text_dim: Color32::from_rgb(148, 163, 184),
                accent: Color32::from_rgb(168, 85, 247),
                accent_hover: Color32::from_rgb(192, 132, 252),
                input_bg: Color32::from_rgb(22, 27, 48),
                input_border: Color32::from_rgba_unmultiplied(139, 92, 246, 56),
                is_aurora: true,
                is_light: false,
            },
            AppTheme::Light => Palette {
                bg: Color32::from_rgb(245, 247, 250),
                panel_bg: Color32::from_rgb(240, 243, 248),
                card_bg: Color32::from_rgb(255, 255, 255),
                card_border: Color32::from_rgb(215, 222, 235),
                text: Color32::from_rgb(24, 28, 36),
                text_dim: Color32::from_rgb(95, 105, 125),
                accent: Color32::from_rgb(30, 95, 220),
                accent_hover: Color32::from_rgb(50, 115, 240),
                input_bg: Color32::from_rgb(250, 252, 255),
                input_border: Color32::from_rgb(195, 205, 220),
                is_aurora: false,
                is_light: true,
            },
            AppTheme::Dark => Palette {
                bg: Color32::from_rgb(18, 18, 20),
                panel_bg: Color32::from_rgb(24, 24, 27),
                card_bg: Color32::from_rgb(32, 32, 36),
                card_border: Color32::from_rgb(39, 39, 42),
                text: Color32::from_rgb(237, 237, 237),
                text_dim: Color32::from_rgb(161, 161, 170),
                accent: Color32::from_rgb(80, 120, 240),
                accent_hover: Color32::from_rgb(100, 140, 255),
                input_bg: Color32::from_rgb(32, 32, 36),
                input_border: Color32::from_rgb(39, 39, 42),
                is_aurora: false,
                is_light: false,
            },
        }
    }

    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let monitors = detect_system_monitors();
        let windows = get_active_windows();
        let first_window = windows.first().cloned().unwrap_or_default();
        let mics = get_available_microphones();
        let first_mic = mics.first().cloned().unwrap_or_default();

        let mut config = RecorderConfig::default();
        config.selected_window_title = first_window;
        config.microphone_name = first_mic;
        config.capture_source = CaptureSourceChoice::Monitor(0);

        Self {
            config,
            recorder: RecorderState::new(),
            drive: RecordCraftDrive::new(),
            hotkeys: HotkeyManager::new(),
            agent: ArtCraftAgent::new(),
            preview: ScreenPreview::new(),
            monitors,
            available_windows: windows,
            available_mics: mics,
            status_message: "Ready to record".to_string(),
            system_audio_vol: 1.0,
            mic_audio_vol: 1.0,
            anim_phase: 0.0,
            show_audio_guide: false,
            audio_state: SystemAudioState::default(),
            selected_app_pids: HashSet::new(),
            selected_app_names: HashSet::new(),
            show_all_running_apps: false,
            muted_during_recording: Vec::new(),
            smoothed_mic_peak: 0.0,
            smoothed_master_peak: 0.0,
            smoothed_app_peaks: HashMap::new(),
            mic_muted: false,
            master_muted: false,
            theme: AppTheme::load_global(),
            control_rx: None,
        }
    }

    pub fn apply_per_app_recording_mute(&mut self) {
        self.restore_per_app_recording_mute();
        if self.config.audio_routing == AudioRoutingMode::PerAppIsolation {
            for app in &self.audio_state.apps {
                let key = app.name.to_ascii_lowercase();
                if !self.selected_app_names.contains(&key) && !app.is_muted {
                    set_app_volume_by_name(&app.name, app.volume, true);
                    self.muted_during_recording.push((app.name.clone(), app.volume));
                }
            }
        }
    }

    pub fn restore_per_app_recording_mute(&mut self) {
        for (exe_name, vol) in self.muted_during_recording.drain(..) {
            set_app_volume_by_name(&exe_name, vol, false);
        }
    }

    /// Refresh display hardware list from Windows API.
    pub fn refresh_displays(&mut self) {
        self.monitors = detect_system_monitors();
        self.available_windows = get_active_windows();
        self.available_mics = get_available_microphones();
        self.audio_state = poll_system_audio();
        self.status_message = format!(
            "Refreshed devices: {} monitors, {} windows, {} audio apps",
            self.monitors.len(),
            self.available_windows.len(),
            self.audio_state.apps.len()
        );
    }

    /// Handles opening the generated project or recording directly inside CutCraft
    pub fn open_in_cutcraft(&mut self) {
        let (media_file, proj_name) = if let Some(ref f) = self.recorder.last_recorded_file {
            let name = f.file_stem().and_then(|s| s.to_str()).unwrap_or("Recording").to_string();
            (f.clone(), name)
        } else if let Some(ref p) = self.recorder.last_created_project {
            let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("Recording").to_string();
            (p.clone(), name)
        } else {
            // Check raw recordings folder for newest file or prompt user
            let raw_dir = self.drive.raw_recordings_dir(&self.config.project_name);
            let mut newest_video: Option<PathBuf> = None;
            let mut newest_time = std::time::UNIX_EPOCH;
            if raw_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&raw_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if ext.eq_ignore_ascii_case("mp4") || ext.eq_ignore_ascii_case("mkv") || ext.eq_ignore_ascii_case("mov") {
                                if let Ok(meta) = entry.metadata() {
                                    if let Ok(mtime) = meta.modified() {
                                        if mtime > newest_time {
                                            newest_time = mtime;
                                            newest_video = Some(path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(f) = newest_video {
                let name = f.file_stem().and_then(|s| s.to_str()).unwrap_or("Recording").to_string();
                (f, name)
            } else if let Some(picked) = rfd::FileDialog::new()
                .add_filter("Video Recordings", &["mp4", "mkv", "mov"])
                .set_title("Select Video to Edit in CutCraft")
                .pick_file()
            {
                let name = picked.file_stem().and_then(|s| s.to_str()).unwrap_or("Recording").to_string();
                (picked, name)
            } else {
                self.status_message = "No recording found to open in CutCraft.".to_string();
                return;
            }
        };

        let proj_dir = match self.drive.create_cutcraft_project(&proj_name, &media_file) {
            Ok(dir) => dir,
            Err(_) => self.drive.cutcraft_base_dir().join(&proj_name),
        };

        // Write cross-app handoff file
        if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            let handoff_dir = PathBuf::from(user_profile).join(".craft-suite");
            let _ = std::fs::create_dir_all(&handoff_dir);
            let handoff_file = handoff_dir.join("cutcraft-import-handoff.json");
            let handoff_data = serde_json::json!({
                "projectName": proj_name,
                "videoFilePath": media_file.to_string_lossy(),
                "projectDir": proj_dir.to_string_lossy(),
                "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
            });
            let _ = std::fs::write(&handoff_file, serde_json::to_string_pretty(&handoff_data).unwrap_or_default());
        }

        let candidates = [
            PathBuf::from("../cutcraft/target/release/cutcraft.exe"),
            PathBuf::from("../cutcraft/src/target/release/concat.exe"),
        ];

        let mut launched = false;
        for c in candidates {
            if c.exists() {
                let _ = Command::new(&c).arg("--import-handoff").arg("--project").arg(&proj_dir).spawn();
                self.status_message = format!("Opened in CutCraft: {}", proj_dir.display());
                launched = true;
                break;
            }
        }

        if !launched {
            let _ = Command::new("cargo")
                .arg("run")
                .arg("-p")
                .arg("concat")
                .arg("--manifest-path")
                .arg("../cutcraft/src/Cargo.toml")
                .arg("--")
                .arg("--import-handoff")
                .arg("--project")
                .arg(&proj_dir)
                .spawn();
            self.status_message = format!("Launched CutCraft via Cargo: {}", proj_dir.display());
        }
    }


    fn poll_hotkeys(&mut self) {
        for ev in self.hotkeys.poll_events() {
            match ev {
                HotkeyEvent::ToggleRecord => {
                    if self.recorder.is_recording() {
                        self.restore_per_app_recording_mute();
                            if let Ok((file, proj)) = self.recorder.stop_recording(&self.drive) {
                            self.status_message = format!(
                                "Recording stopped & saved: {}",
                                file.file_name().and_then(|n| n.to_str()).unwrap_or("")
                            );
                            let _ = proj;
                        }
                    } else {
                        if let Err(e) = self.recorder.start_recording(&self.config, &self.monitors, &self.drive) {
                            self.status_message = format!("Failed to start recording: {e}");
                        } else {
                            self.apply_per_app_recording_mute();
                                self.status_message = "Recording in progress (Press F9 to stop)...".to_string();
                        }
                    }
                }
                HotkeyEvent::HighlightMarker => {
                    if let Some(t) = self.recorder.add_highlight() {
                        let mins = (t / 60.0).floor() as u32;
                        let secs = (t % 60.0) as u32;
                        self.status_message = format!("Highlight marker placed at {mins:02}:{secs:02}");
                    }
                }
            }
        }
    }

    fn poll_agent(&mut self) {
        let mut ext_reqs = Vec::new();
        if let Some(ref rx) = self.control_rx {
            while let Ok(req) = rx.try_recv() {
                ext_reqs.push(req);
            }
        }
        for req in ext_reqs {
            let res = match self.execute_command(&req.method, &req.params) {
                Ok(v) => serde_json::json!({"ok": true, "result": v}),
                Err(e) => serde_json::json!({"ok": false, "error": e}),
            };
            let _ = req.reply.send(res);
        }

        let reqs = self.agent.take_tool_requests();
        for req in reqs {
            let res = self.execute_command(&req.tool_name, &req.params);
            let _ = req.reply_sender.send(res);
        }
        self.agent.process_events();
    }
}

impl eframe::App for RecordCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_hotkeys();
        self.poll_agent();
        self.anim_phase += 0.08;

        // Force regular repaints for smooth live preview & VU meter animations
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));

        // Poll Real-Time System Audio from WASAPI
        let real_audio = poll_system_audio();
        let alpha_attack = 0.55f32;
        let alpha_decay = 0.07f32;

        let update_smooth = |cur: &mut f32, target: f32| {
            if target > *cur {
                *cur = (*cur + (target - *cur) * alpha_attack).min(1.0);
            } else {
                *cur = (*cur - alpha_decay).max(0.0);
            }
        };

        update_smooth(&mut self.smoothed_master_peak, if self.master_muted { 0.0 } else { real_audio.master_peak });
        update_smooth(&mut self.smoothed_mic_peak, if self.mic_muted { 0.0 } else { real_audio.mic_peak });

        for app in &real_audio.apps {
            let key = app.name.to_ascii_lowercase();
            if !self.selected_app_names.contains(&key) && self.audio_state.apps.iter().all(|a| a.name.to_ascii_lowercase() != key) {
                self.selected_app_names.insert(key.clone());
            }
            let entry = self.smoothed_app_peaks.entry(key).or_insert(0.0);
            let target = if app.is_muted { 0.0 } else { app.peak };
            update_smooth(entry, target);
        }
        self.audio_state = real_audio;

        // Dynamic System Visuals (Dark, Light, Aurora)
        let pal = self.palette();
        let mut visuals = if pal.is_light { egui::Visuals::light() } else { egui::Visuals::dark() };
        visuals.panel_fill = pal.panel_bg;
        visuals.window_fill = pal.bg;
        visuals.override_text_color = Some(pal.text);
        visuals.widgets.inactive.weak_bg_fill = pal.card_bg;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, pal.input_border);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, pal.text);
        visuals.widgets.hovered.weak_bg_fill = if pal.is_light { Color32::from_rgb(225, 232, 244) } else if pal.is_aurora { Color32::from_rgb(32, 38, 58) } else { Color32::from_rgb(34, 42, 60) };
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, pal.accent);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, if pal.is_light { Color32::BLACK } else { Color32::WHITE });
        visuals.widgets.active.weak_bg_fill = if pal.is_light { Color32::from_rgb(205, 218, 235) } else { Color32::from_rgb(46, 56, 88) };
        visuals.selection.bg_fill = pal.accent;
        visuals.selection.stroke = Stroke::new(1.0, pal.accent_hover);
        ui.ctx().set_visuals(visuals);

        // Update live preview capture in background
        let (src_x, src_y, src_w, src_h) = match self.config.capture_source {
            CaptureSourceChoice::Monitor(idx) => {
                let mon = self.monitors.get(idx).or_else(|| self.monitors.first());
                if let Some(m) = mon {
                    (m.x, m.y, m.width, m.height)
                } else {
                    (0, 0, 3440, 1440)
                }
            }
            CaptureSourceChoice::AllMonitors => {
                unsafe {
                    use windows_sys::Win32::UI::WindowsAndMessaging::*;
                    (
                        GetSystemMetrics(SM_XVIRTUALSCREEN),
                        GetSystemMetrics(SM_YVIRTUALSCREEN),
                        GetSystemMetrics(SM_CXVIRTUALSCREEN),
                        GetSystemMetrics(SM_CYVIRTUALSCREEN),
                    )
                }
            }
            CaptureSourceChoice::GameWindow => {
                if let Some((x, y, w, h)) = crate::recorder::get_window_rect_by_title(&self.config.selected_window_title) {
                    (x, y, w, h)
                } else {
                    (0, 0, 1920, 1080)
                }
            }
        };

        self.preview.update(ui.ctx(), src_x, src_y, src_w, src_h);

        // =====================================================================
        // TOP STUDIO HEADER BAR
        // =====================================================================
        egui::Panel::top("top_header")
            .exact_size(40.0)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(10.0);
                    // Studio Logo & Title
                    ui.label(RichText::new("RecordCraft").strong().size(18.0).color(pal.text));
                    ui.label(RichText::new("STUDIO PRO").strong().size(12.5).color(if pal.is_aurora { Color32::from_rgb(180, 150, 240) } else { pal.text_dim }));

                    // Engine Badge with vector status dot
                    ui.add_space(14.0);
                    let (status_text, dot_color) = if self.recorder.is_recording() {
                        ("LIVE RECORDING", Color32::from_rgb(255, 45, 70))
                    } else {
                        ("READY TO RECORD", Color32::from_rgb(35, 205, 120))
                    };

                    let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.5, dot_color);
                    ui.label(RichText::new(status_text).size(12.0).strong().color(dot_color));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(10.0);
                        let drive_active = self.drive.is_drive_detected();
                        let proj_name = if self.config.project_name.trim().is_empty() {
                            "RecordCraft Studio".to_string()
                        } else {
                            self.config.project_name.trim().to_string()
                        };
                        let proj_dir = self.drive.cutcraft_base_dir();
                        crate::artcraft_theme::render_top_right_action_bar(
                            ui,
                            &mut self.drive.auto_save_enabled,
                            drive_active,
                            &proj_name,
                            &proj_dir,
                            &mut self.agent.is_open,
                        );
                    });
                });
            });

        // =====================================================================
        // RIGHT DOCK: AI AGENT SIDEBAR (Google Docs Gemini Theme)
        // =====================================================================
        if self.agent.is_open {
            let rec_summary = self.studio_inspect_json().to_string();
            let drive_dir = self.drive.cutcraft_base_dir().to_string_lossy().to_string();
            self.agent.render_sidebar(ui, &rec_summary, &drive_dir);
        }

        // =====================================================================
        // BOTTOM STATUS BAR (28px Global Footer)
        // =====================================================================
        egui::Panel::bottom("bottom_bar")
            .exact_size(28.0)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(12.0);
                    ui.label(RichText::new(&self.status_message).size(12.0).color(pal.text_dim));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(12.0);
                        let mut cur_theme = self.theme;
                        if crate::artcraft_theme::render_theme_switcher_dropdown(ui, &mut cur_theme) {
                            self.theme = cur_theme;
                        }
                        ui.add_space(10.0);
                        let markers_count = self.recorder.highlight_markers.len();
                        ui.label(RichText::new(format!("Markers: {markers_count}")).size(11.5).color(pal.text_dim));
                        ui.label(RichText::new("â€¢").color(pal.card_border));
                        ui.label(RichText::new(format!("{} Â· {} FPS", self.config.codec.short_badge(), self.config.framerate)).size(11.5).color(pal.text_dim));
                    });
                });
            });

        // =====================================================================
        // CENTRAL BODY: LIVE PREVIEW + OBS STUDIO RECORDING CONTROLS
        // =====================================================================
        egui::CentralPanel::default().show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(12.0, 12.0);

            let avail_w = ui.available_width();
            let preview_h = 280.0f32; // Generous, cinematic studio canvas

            // 1. LIVE PREVIEW CANVAS
            egui::Frame::new()
                .fill(Color32::from_rgb(10, 12, 16))
                .stroke(Stroke::new(1.0, Color32::from_rgb(34, 40, 56)))
                .corner_radius(10.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    let canvas_rect = Rect::from_min_size(ui.cursor().min, vec2(avail_w - 20.0, preview_h));
                    ui.allocate_rect(canvas_rect, egui::Sense::hover());

                    // Render texture if available
                    if let Some(ref tex) = self.preview.texture {
                        let tex_size = tex.size_vec2();
                        let aspect = if tex_size.y > 0.0 { tex_size.x / tex_size.y } else { 16.0 / 9.0 };
                        let mut draw_w = canvas_rect.width();
                        let mut draw_h = draw_w / aspect;
                        if draw_h > canvas_rect.height() {
                            draw_h = canvas_rect.height();
                            draw_w = draw_h * aspect;
                        }
                        let draw_x = canvas_rect.min.x + (canvas_rect.width() - draw_w) * 0.5;
                        let draw_y = canvas_rect.min.y + (canvas_rect.height() - draw_h) * 0.5;
                        let draw_rect = Rect::from_min_size(pos2(draw_x, draw_y), vec2(draw_w, draw_h));

                        ui.painter().rect_filled(draw_rect, 4.0, Color32::BLACK);
                        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                        ui.painter().image(tex.id(), draw_rect, uv, Color32::WHITE);
                        ui.painter().rect(draw_rect, 4.0, Color32::TRANSPARENT, Stroke::new(1.0, Color32::from_rgb(45, 52, 70)), egui::StrokeKind::Inside);
                    } else {
                        ui.painter().rect_filled(canvas_rect, 6.0, Color32::from_rgb(14, 16, 22));
                        ui.painter().text(
                            canvas_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "Live Studio Monitor Preview\nClick or select source below to activate feed",
                            egui::FontId::proportional(14.0),
                            Color32::from_rgb(130, 145, 175),
                        );
                    }

                    // Overlay 1: Live Status Badge (Top Left) â€” CLEAN, NO DUPLICATE DOT!
                    let badge_bg = Rect::from_min_size(canvas_rect.min + vec2(12.0, 12.0), vec2(110.0, 26.0));
                    ui.painter().rect(badge_bg, 5.0, Color32::from_black_alpha(200), Stroke::new(1.0, Color32::from_rgb(55, 65, 85)), egui::StrokeKind::Inside);

                    let dot_center = badge_bg.min + vec2(14.0, 13.0);
                    let (live_text, live_color) = if self.recorder.is_recording() {
                        ("LIVE REC", Color32::from_rgb(255, 50, 75))
                    } else {
                        ("PREVIEW", Color32::from_rgb(40, 215, 130))
                    };
                    ui.painter().circle_filled(dot_center, 4.5, live_color);
                    ui.painter().text(
                        badge_bg.min + vec2(27.0, 6.0),
                        egui::Align2::LEFT_TOP,
                        live_text,
                        egui::FontId::proportional(12.0),
                        Color32::WHITE,
                    );

                    // Overlay 2: Source Info Badge (Top Right) â€” DYNAMICALLY SIZED TO PREVENT ANY OVERFLOW!
                    let aspect_str = match self.config.capture_source {
                        CaptureSourceChoice::Monitor(idx) => {
                            self.monitors.get(idx).map(|m| m.aspect_str.clone()).unwrap_or_else(|| "21:9".to_string())
                        }
                        CaptureSourceChoice::AllMonitors => "37:10 Panorama".to_string(),
                        CaptureSourceChoice::GameWindow => "16:9".to_string(),
                    };
                    let info_text = format!(
                        "{} Â· {} Â· {} FPS",
                        aspect_str,
                        self.config.resolution.label().split('(').next().unwrap_or("Native").trim(),
                        self.config.framerate
                    );

                    let font_info = egui::FontId::proportional(12.0);
                    let measured_w = ui.painter().layout_no_wrap(info_text.clone(), font_info.clone(), Color32::WHITE).size().x;
                    let badge_w = measured_w + 24.0;
                    let info_bg = Rect::from_min_size(
                        pos2(canvas_rect.max.x - badge_w - 12.0, canvas_rect.min.y + 12.0),
                        vec2(badge_w, 26.0)
                    );
                    ui.painter().rect(info_bg, 5.0, Color32::from_black_alpha(200), Stroke::new(1.0, Color32::from_rgb(55, 65, 85)), egui::StrokeKind::Inside);
                    ui.painter().text(
                        info_bg.center(),
                        egui::Align2::CENTER_CENTER,
                        info_text,
                        font_info,
                        Color32::from_rgb(225, 235, 250),
                    );

                });

            // 2. DEDICATED LIVE VU-METER & PER-APP AUDIO MIXER (Separated from Preview Canvas!)
            egui::Frame::new()
                .fill(pal.card_bg)
                .stroke(Stroke::new(1.0, pal.card_border))
                .corner_radius(10.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    // Header Bar
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("ðŸ”Š Live Audio VU-Meter & Mixer").strong().size(14.0).color(pal.text));
                        ui.label(RichText::new("â€¢ Real-Time Peak Visualizer").size(11.5).color(pal.text_dim));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Routing Mode Toggle Pills
                            let is_per_app = self.config.audio_routing == AudioRoutingMode::PerAppIsolation;
                            let per_app_btn = egui::Button::new(
                                RichText::new("ðŸŽ›ï¸ Per-App Routing")
                                    .size(11.5)
                                    .color(if is_per_app { Color32::WHITE } else { pal.text_dim })
                            )
                            .fill(if is_per_app { pal.accent } else { pal.input_bg })
                            .corner_radius(6.0);

                            if ui.add(per_app_btn).clicked() {
                                self.config.audio_routing = AudioRoutingMode::PerAppIsolation;
                            }

                            let is_all = self.config.audio_routing == AudioRoutingMode::AllSources;
                            let all_btn = egui::Button::new(
                                RichText::new("ðŸ“» All Sources (Global Master)")
                                    .size(11.5)
                                    .color(if is_all { Color32::WHITE } else { pal.text_dim })
                            )
                            .fill(if is_all { pal.accent } else { pal.input_bg })
                            .corner_radius(6.0);

                            if ui.add(all_btn).clicked() {
                                self.config.audio_routing = AudioRoutingMode::AllSources;
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Dynamic VU Meter Rows
                    // Row A: Microphone (Always present & prominent)
                    let mic_lvl = if self.mic_muted { 0.0 } else { self.smoothed_mic_peak * self.mic_audio_vol };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("ðŸŽ™ï¸ Voice Mic").strong().size(12.5).color(Color32::from_rgb(210, 225, 250)));
                        
                        // Mute button
                        let mute_label = if self.mic_muted { "ðŸ”‡ Unmute" } else { "ðŸ”Š Mute" };
                        let mute_col = if self.mic_muted { Color32::from_rgb(230, 50, 60) } else { Color32::from_rgb(32, 38, 54) };
                        if ui.add(egui::Button::new(RichText::new(mute_label).size(11.0).color(Color32::WHITE)).fill(mute_col).corner_radius(4.0)).clicked() {
                            self.mic_muted = !self.mic_muted;
                        }

                        // Gain slider
                        ui.add(egui::Slider::new(&mut self.mic_audio_vol, 0.0..=2.0).show_value(false));
                        ui.label(RichText::new(format!("{:.0}%", self.mic_audio_vol * 100.0)).size(11.0).color(Color32::from_rgb(140, 160, 195)));

                        // Dynamic VU Meter Bar
                        let bar_w = (ui.available_width() - 20.0).max(100.0);
                        let (rect, _) = ui.allocate_exact_size(vec2(bar_w, 14.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 3.0, Color32::from_rgb(14, 16, 24));
                        
                        let fill_w = bar_w * mic_lvl.clamp(0.0, 1.0);
                        if fill_w > 0.0 {
                            let fill_rect = Rect::from_min_size(rect.min, vec2(fill_w, 14.0));
                            let col = if mic_lvl > 0.85 { Color32::from_rgb(255, 55, 65) } else if mic_lvl > 0.65 { Color32::from_rgb(250, 185, 30) } else { Color32::from_rgb(35, 205, 120) };
                            ui.painter().rect_filled(fill_rect, 3.0, col);
                        }
                    });

                    ui.add_space(6.0);

                    // Row B: System Audio
                    match self.config.audio_routing {
                        AudioRoutingMode::AllSources => {
                            let master_lvl = if self.master_muted { 0.0 } else { self.smoothed_master_peak * self.system_audio_vol };
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("ðŸ”Š Master Desktop").strong().size(12.5).color(Color32::from_rgb(210, 225, 250)));

                               // Mute button
                                let mute_label = if self.master_muted { "ðŸ”‡ Unmute" } else { "ðŸ”Š Mute" };
                                let mute_col = if self.master_muted { Color32::from_rgb(230, 50, 60) } else { Color32::from_rgb(32, 38, 54) };
                                if ui.add(egui::Button::new(RichText::new(mute_label).size(11.0).color(Color32::WHITE)).fill(mute_col).corner_radius(4.0)).clicked() {
                                    self.master_muted = !self.master_muted;
                                }

                                // Volume slider
                                ui.add(egui::Slider::new(&mut self.system_audio_vol, 0.0..=1.5).show_value(false));
                                ui.label(RichText::new(format!("{:.0}%", self.system_audio_vol * 100.0)).size(11.0).color(Color32::from_rgb(140, 160, 195)));

                                // Dynamic Master VU Meter Bar
                                let bar_w = (ui.available_width() - 20.0).max(100.0);
                                let (rect, _) = ui.allocate_exact_size(vec2(bar_w, 14.0), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 3.0, Color32::from_rgb(14, 16, 24));

                                let fill_w = bar_w * master_lvl.clamp(0.0, 1.0);
                                if fill_w > 0.0 {
                                    let fill_rect = Rect::from_min_size(rect.min, vec2(fill_w, 14.0));
                                    let col = if master_lvl > 0.85 { Color32::from_rgb(255, 55, 65) } else if master_lvl > 0.65 { Color32::from_rgb(250, 185, 30) } else { Color32::from_rgb(45, 120, 255) };
                                    ui.painter().rect_filled(fill_rect, 3.0, col);
                                }
                            });
                        }
                        AudioRoutingMode::PerAppIsolation => {
                            if self.audio_state.apps.is_empty() {
                                ui.label(RichText::new("â„¹ï¸ No active audio applications detected on Windows. Start playing audio in Discord, Spotify, or a Game to see live meters!").size(12.0).color(Color32::from_rgb(150, 165, 195)));
                            } else {
                                ui.label(RichText::new("Active Applications (Toggle checkbox to include in recording):").size(11.5).color(Color32::from_rgb(160, 175, 205)));
                                
                                for app in &self.audio_state.apps {
                                    let pid = app.pid;
                                    let mut is_sel = self.selected_app_pids.contains(&pid);
                                    let app_peak = self.smoothed_app_peaks.get(&pid.to_string()).copied().unwrap_or(0.0);
                                    let is_active = app.is_active || app_peak > 0.01;

                                    ui.horizontal(|ui| {
                                        if ui.checkbox(&mut is_sel, "").changed() {
                                            if is_sel {
                                                self.selected_app_pids.insert(pid);
                                            } else {
                                                self.selected_app_pids.remove(&pid);
                                            }
                                        }

                                        let name_col = if is_sel { Color32::WHITE } else { Color32::GRAY };
                                        let active_dot = if is_active { "ðŸŸ¢ " } else { "âšª " };
                                        ui.label(RichText::new(format!("{active_dot}{}", app.display_name)).strong().size(12.0).color(name_col));

                                        // Mute toggle
                                        let mute_label = if app.is_muted { "ðŸ”‡" } else { "ðŸ”Š" };
                                        if ui.button(mute_label).on_hover_text("Toggle Mute in Windows Mixer").clicked() {
                                            set_process_volume(pid, app.volume, !app.is_muted);
                                        }

                                        // Volume slider
                                        let mut vol = app.volume;
                                        if ui.add(egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false)).changed() {
                                            set_process_volume(pid, vol, app.is_muted);
                                        }

                                        // Live Meter Bar
                                        let bar_w = (ui.available_width() - 10.0).max(80.0);
                                        let (rect, _) = ui.allocate_exact_size(vec2(bar_w, 12.0), egui::Sense::hover());
                                        ui.painter().rect_filled(rect, 2.0, Color32::from_rgb(14, 16, 24));

                                        if is_sel && !app.is_muted && app_peak > 0.005 {
                                            let fill_w = bar_w * app_peak.clamp(0.0, 1.0);
                                            let fill_rect = Rect::from_min_size(rect.min, vec2(fill_w, 12.0));
                                            let col = if app_peak > 0.85 { Color32::from_rgb(255, 55, 65) } else if app_peak > 0.65 { Color32::from_rgb(250, 185, 30) } else { Color32::from_rgb(50, 140, 255) };
                                            ui.painter().rect_filled(fill_rect, 2.0, col);
                                        }
                                    });
                                }
                            }
                        }
                    }
                });

            // 2. HERO RECORDING CONTROLLER (Generous, High-Contrast Bar)
            egui::Frame::new()
                .fill(if self.recorder.is_recording() { Color32::from_rgb(32, 16, 22) } else { Color32::from_rgb(22, 26, 36) })
                .stroke(Stroke::new(1.0, if self.recorder.is_recording() { Color32::from_rgb(200, 40, 65) } else { Color32::from_rgb(42, 50, 72) }))
                .corner_radius(10.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    if self.recorder.is_recording() {
                        let elapsed = self.recorder.elapsed_seconds();
                        let mins = (elapsed / 60.0).floor() as u32;
                        let secs = (elapsed % 60.0) as u32;
                        let millis = ((elapsed - elapsed.floor()) * 10.0) as u32;
                        let timer_str = format!("REC  {mins:02}:{secs:02}.{millis}");

                        ui.horizontal(|ui| {
                            let (dot_rect, _) = ui.allocate_exact_size(vec2(20.0, 20.0), egui::Sense::hover());
                            ui.painter().circle_filled(dot_rect.center(), 8.0, Color32::from_rgb(255, 45, 75));

                            ui.label(RichText::new(timer_str).monospace().size(26.0).strong().color(Color32::from_rgb(255, 65, 90)));

                            ui.add_space(20.0);
                            ui.label(RichText::new(format!("{} FPS", self.config.framerate)).size(13.0).color(Color32::from_rgb(130, 175, 245)));
                            ui.label(RichText::new("â€¢").color(Color32::from_rgb(60, 75, 95)));
                            ui.label(RichText::new(self.config.codec.label()).size(13.0).color(Color32::from_rgb(180, 195, 220)));

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Stop Recording Button
                                let stop_btn = egui::Button::new(RichText::new("â–  STOP RECORDING (F9)").strong().size(14.0).color(Color32::WHITE))
                                    .fill(Color32::from_rgb(215, 35, 60))
                                    .corner_radius(8.0)
                                    .min_size(vec2(220.0, 44.0));

                                if ui.add(stop_btn).clicked() {
                                    if let Ok((file, _)) = self.recorder.stop_recording(&self.drive) {
                                        self.status_message = format!("Saved to: {}", file.display());
                                    }
                                }

                                ui.add_space(10.0);
                                // Highlight Marker Button
                                let mark_btn = egui::Button::new(RichText::new("â˜… Highlight Marker (F10)").strong().size(13.0).color(Color32::BLACK))
                                    .fill(Color32::from_rgb(250, 190, 20))
                                    .corner_radius(8.0)
                                    .min_size(vec2(180.0, 44.0));

                                if ui.add(mark_btn).clicked() {
                                    if let Some(t) = self.recorder.add_highlight() {
                                        let mins = (t / 60.0).floor() as u32;
                                        let secs = (t % 60.0) as u32;
                                        self.status_message = format!("Highlight marker placed at {mins:02}:{secs:02}");
                                    }
                                }
                            });
                        });
                    } else {
                        ui.horizontal(|ui| {
                            // Massive Glowing START RECORDING Button
                            let rec_btn = egui::Button::new(
                                RichText::new("â— START RECORDING (F9)")
                                    .size(16.0)
                                    .strong()
                                    .color(Color32::WHITE)
                            )
                            .fill(Color32::from_rgb(230, 35, 65))
                            .corner_radius(8.0)
                            .min_size(vec2(240.0, 48.0));

                            if ui.add(rec_btn).clicked() {
                                if let Err(e) = self.recorder.start_recording(&self.config, &self.monitors, &self.drive) {
                                    self.status_message = format!("Failed to start recording: {e}");
                                } else {
                                    self.apply_per_app_recording_mute();
                                        self.status_message = "Recording started...".to_string();
                                }
                            }

                            ui.add_space(16.0);
                            ui.vertical(|ui| {
                                ui.label(RichText::new("DirectX 12 / WGC Desktop Duplication (Zero Latency)").size(13.5).strong().color(Color32::WHITE));
                                let src_name = match self.config.capture_source {
                                    CaptureSourceChoice::Monitor(idx) => {
                                        self.monitors.get(idx).map(|m| m.name.clone()).unwrap_or_else(|| "Primary Display".to_string())
                                    }
                                    CaptureSourceChoice::AllMonitors => "All Displays Combined".to_string(),
                                    CaptureSourceChoice::GameWindow => format!("Game Window: {}", self.config.selected_window_title),
                                };
                                ui.label(
                                    RichText::new(format!(
                                        "Target: {} Â· {} Â· {} FPS",
                                        src_name,
                                        self.config.resolution.label(),
                                        self.config.framerate,
                                    ))
                                    .size(12.0)
                                    .color(Color32::from_rgb(160, 175, 200))
                                );
                            });
                        });
                    }
                });

            // Post-Recording Banner: "Directly Open & Edit in CutCraft"
            if let Some(ref proj_path) = self.recorder.last_created_project {
                let mut should_open = false;
                egui::Frame::new()
                    .fill(Color32::from_rgb(22, 28, 42))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(60, 95, 160)))
                    .corner_radius(8.0)
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("ðŸŽ‰ Recording saved & synced to Google Drive!").strong().size(14.0).color(Color32::WHITE));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let edit_btn = egui::Button::new(RichText::new("ðŸŽ¬ Directly Open & Edit in CutCraft").color(Color32::WHITE).strong().size(13.5))
                                    .fill(Color32::from_rgb(50, 110, 240))
                                    .corner_radius(6.0)
                                    .min_size(vec2(320.0, 36.0));

                                if ui.add(edit_btn).clicked() {
                                    should_open = true;
                                }
                            });
                        });
                        ui.add_space(2.0);
                        ui.label(RichText::new(format!("Project file: {}", proj_path.display())).size(11.5).color(Color32::from_rgb(150, 185, 230)));
                    });

                if should_open {
                    self.open_in_cutcraft();
                }
            }

            // =================================================================
            // =================================================================
            // 3. STUDIO SETTINGS & CONFIGURATION PANELS (3-Column Bento Grid)
            // =================================================================
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.columns(3, |cols| {
                    // COLUMN 1: Capture Source, Display & Framerate
                    let ui = &mut cols[0];
                    egui::Frame::new()
                        .fill(pal.card_bg)
                        .stroke(Stroke::new(1.0, pal.card_border))
                        .corner_radius(12.0)
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("ðŸ–¥ï¸ Capture Source & Display").strong().size(14.0).color(pal.text));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("ðŸ”„").on_hover_text("Re-query connected monitors and windows").clicked() {
                                        self.refresh_displays();
                                    }
                                });
                            });
                            ui.separator();

                            ui.label(RichText::new("Select Display or Window:").size(12.0).color(pal.text_dim));

                            let current_label = match self.config.capture_source {
                                CaptureSourceChoice::Monitor(idx) => {
                                    self.monitors.get(idx).map(|m| m.name.clone()).unwrap_or_else(|| "Display 1".to_string())
                                }
                                CaptureSourceChoice::AllMonitors => "All Displays Combined".to_string(),
                                CaptureSourceChoice::GameWindow => "Specific Window".to_string(),
                            };

                            egui::ComboBox::from_id_salt("display_source_select_3col")
                                .selected_text(&current_label)
                                .width(ui.available_width())
                                .show_ui(ui, |ui| {
                                    for (i, m) in self.monitors.iter().enumerate() {
                                        ui.selectable_value(
                                            &mut self.config.capture_source,
                                            CaptureSourceChoice::Monitor(i),
                                            &m.name,
                                        );
                                    }
                                    ui.selectable_value(
                                        &mut self.config.capture_source,
                                        CaptureSourceChoice::AllMonitors,
                                        "All Displays Combined (Virtual Panorama)",
                                    );
                                    ui.selectable_value(
                                        &mut self.config.capture_source,
                                        CaptureSourceChoice::GameWindow,
                                        "Specific Game / App Window",
                                    );
                                });

                            if self.config.capture_source == CaptureSourceChoice::GameWindow {
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Window:").size(11.5));
                                    egui::ComboBox::from_id_salt("window_select_box_3col")
                                        .selected_text(if self.config.selected_window_title.is_empty() { "Select window..." } else { &self.config.selected_window_title })
                                        .width(ui.available_width() - 30.0)
                                        .show_ui(ui, |ui| {
                                            for w in &self.available_windows {
                                                ui.selectable_value(&mut self.config.selected_window_title, w.clone(), w);
                                            }
                                        });

                                    if ui.button("ðŸ”„").clicked() {
                                        self.available_windows = get_active_windows();
                                    }
                                });
                            }

                            ui.add_space(8.0);
                            ui.checkbox(&mut self.config.show_cursor, "Capture mouse cursor");

                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(8.0);

                            // Resolution & Framerate
                            ui.label(RichText::new("Resolution Mode:").size(12.0).color(pal.text_dim));
                            egui::ComboBox::from_id_salt("res_select_box_3col")
                                .selected_text(self.config.resolution.label())
                                .width(ui.available_width())
                                .show_ui(ui, |ui| {
                                    for r in [ResolutionMode::Native, ResolutionMode::Uhd4k, ResolutionMode::Qhd1440p, ResolutionMode::Fhd1080p] {
                                        ui.selectable_value(&mut self.config.resolution, r, r.label());
                                    }
                                });

                            ui.add_space(8.0);
                            ui.label(RichText::new("Framerate:").size(12.0).color(pal.text_dim));
                            egui::ComboBox::from_id_salt("fps_select_box_3col")
                                .selected_text(format!("{} FPS", self.config.framerate))
                                .width(ui.available_width())
                                .show_ui(ui, |ui| {
                                    for fps in [60, 120, 144, 240] {
                                        ui.selectable_value(&mut self.config.framerate, fps, format!("{fps} FPS"));
                                    }
                                });
                        });

                    // COLUMN 2: Audio Mixer & Multi-Track Separation
                    let ui = &mut cols[1];
                    egui::Frame::new()
                        .fill(pal.card_bg)
                        .stroke(Stroke::new(1.0, pal.card_border))
                        .corner_radius(12.0)
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("ðŸŽ™ï¸ Audio Mixer & Multi-Track").strong().size(14.0).color(pal.text));
                            ui.separator();

                            ui.checkbox(&mut self.config.multi_track_audio, "Enable Multi-Track Audio (Game + Mic)");

                            ui.add_space(8.0);
                            ui.label(RichText::new("Routing Mode:").size(12.0).color(pal.text_dim));
                            ui.horizontal(|ui| {
                                let is_all = self.config.audio_routing == AudioRoutingMode::AllSources;
                                if ui.selectable_label(is_all, "ðŸ“» Master").clicked() {
                                    self.config.audio_routing = AudioRoutingMode::AllSources;
                                }
                                let is_iso = self.config.audio_routing == AudioRoutingMode::PerAppIsolation;
                                if ui.selectable_label(is_iso, "ðŸŽ›ï¸ Per-App").clicked() {
                                    self.config.audio_routing = AudioRoutingMode::PerAppIsolation;
                                }
                            });

                            ui.add_space(8.0);

                            // Mic selector
                            ui.label(RichText::new("Microphone Device:").size(12.0).color(pal.text_dim));
                            egui::ComboBox::from_id_salt("mic_select_box_3col")
                                .selected_text(&self.config.microphone_name)
                                .width(ui.available_width())
                                .show_ui(ui, |ui| {
                                    for mic in &self.available_mics {
                                        ui.selectable_value(&mut self.config.microphone_name, mic.clone(), mic);
                                    }
                                });

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Mic Gain:").size(11.5));
                                ui.add(egui::Slider::new(&mut self.mic_audio_vol, 0.0..=2.0).show_value(false));
                                ui.label(RichText::new(format!("{:.0}%", self.mic_audio_vol * 100.0)).size(11.0).color(pal.text_dim));
                            });

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Desktop Vol:").size(11.5));
                                ui.add(egui::Slider::new(&mut self.system_audio_vol, 0.0..=1.5).show_value(false));
                                ui.label(RichText::new(format!("{:.0}%", self.system_audio_vol * 100.0)).size(11.0).color(pal.text_dim));
                            });

                            if self.config.audio_routing == AudioRoutingMode::PerAppIsolation {
                                ui.add_space(8.0);
                                ui.label(RichText::new("Active Applications:").size(11.5).strong().color(Color32::from_rgb(180, 200, 235)));
                                for app in self.audio_state.apps.iter().take(4) {
                                    let pid = app.pid;
                                    let mut is_sel = self.selected_app_pids.contains(&pid);
                                    ui.horizontal(|ui| {
                                        if ui.checkbox(&mut is_sel, &app.display_name).changed() {
                                            if is_sel {
                                                self.selected_app_pids.insert(pid);
                                            } else {
                                                self.selected_app_pids.remove(&pid);
                                            }
                                        }
                                    });
                                }
                            }
                        });

                    // COLUMN 3: Hardware Encoder & CutCraft Direct Export
                    let ui = &mut cols[2];
                    egui::Frame::new()
                        .fill(pal.card_bg)
                        .stroke(Stroke::new(1.0, pal.card_border))
                        .corner_radius(12.0)
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("âš¡ Hardware Encoder & CutCraft").strong().size(14.0).color(pal.text));
                            ui.separator();

                            ui.label(RichText::new("GPU Hardware Encoder:").size(12.0).color(pal.text_dim));
                            egui::ComboBox::from_id_salt("codec_select_box_3col")
                                .selected_text(self.config.codec.label())
                                .width(ui.available_width())
                                .show_ui(ui, |ui| {
                                    for c in [VideoCodec::HevcNvenc, VideoCodec::H264Nvenc, VideoCodec::HevcAmf, VideoCodec::H264Amf, VideoCodec::HevcQsv, VideoCodec::H264Qsv, VideoCodec::Libx264] {
                                        ui.selectable_value(&mut self.config.codec, c, c.label());
                                    }
                                });

                            ui.add_space(6.0);
                            ui.label(RichText::new("Quality Preset:").size(12.0).color(pal.text_dim));
                            egui::ComboBox::from_id_salt("quality_select_box_3col")
                                .selected_text(self.config.quality.label())
                                .width(ui.available_width())
                                .show_ui(ui, |ui| {
                                    for p in [QualityPreset::Lossless, QualityPreset::Ultra100Mbps, QualityPreset::High50Mbps, QualityPreset::Custom] {
                                        ui.selectable_value(&mut self.config.quality, p, p.label());
                                    }
                                });

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Format:").size(11.5).color(pal.text_dim));
                                egui::ComboBox::from_id_salt("container_select_box_3col")
                                    .selected_text(self.config.container.label())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.config.container, ContainerFormat::Mp4, ContainerFormat::Mp4.label());
                                        ui.selectable_value(&mut self.config.container, ContainerFormat::Mkv, ContainerFormat::Mkv.label());
                                    });
                                ui.checkbox(&mut self.config.bit_depth_10bit, "10-Bit HDR");
                            });

                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(8.0);

                            ui.label(RichText::new("Project / Recording Name:").size(12.0).color(pal.text_dim));
                            ui.text_edit_singleline(&mut self.config.project_name);

                            ui.add_space(10.0);
                            let open_btn = egui::Button::new(
                                RichText::new("ðŸŽ¬ Open Project in CutCraft").strong().color(Color32::WHITE).size(13.5)
                            )
                            .fill(Color32::from_rgb(50, 105, 230))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(99, 140, 248)))
                            .corner_radius(8.0)
                            .min_size(vec2(ui.available_width(), 42.0));

                            if ui.add(open_btn).on_hover_text("Open recording directly in CutCraft editor").clicked() {
                                self.open_in_cutcraft();
                            }
                        });
                });
            });
        });
    }
}

impl Drop for RecordCraftApp {
    fn drop(&mut self) {
        self.restore_per_app_recording_mute();
    }
}


