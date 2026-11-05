#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod artcraft_agent;
mod artcraft_drive;
pub mod artcraft_theme;
pub mod audio_engine;
pub mod control;
mod hotkeys;
mod preview;
mod recorder;

use app::RecordCraftApp;

fn app_icon() -> egui::IconData {
    let rgba = include_bytes!("../assets/app-icon/recordcraft_rgba.bin").to_vec();
    egui::IconData {
        rgba,
        width: 256,
        height: 256,
    }
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Check for Windows Segoe UI Emoji font to completely prevent tofu boxes (▯)
    let emoji_paths = [
        r"C:\Windows\Fonts\seguiemj.ttf",
        r"C:\Windows\Fonts\seguisym.ttf",
    ];

    for path in emoji_paths {
        if let Ok(data) = std::fs::read(path) {
            let font_name = if path.contains("seguiemj") { "SegoeEmoji" } else { "SegoeSymbol" };
            fonts.font_data.insert(
                font_name.to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(data)),
            );
            if let Some(prop) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                prop.push(font_name.to_owned());
            }
            if let Some(mono) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                mono.push(font_name.to_owned());
            }
            break;
        }
    }

    ctx.set_fonts(fonts);
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Check for --mcp or mcp subcommand
    let mut is_mcp = false;
    let mut control_port: u16 = 9550;
    let mut start_control_tcp = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mcp" | "mcp" => {
                is_mcp = true;
            }
            "--port" => {
                if let Some(p) = args.get(i + 1).and_then(|s| s.parse::<u16>().ok()) {
                    control_port = p;
                    i += 1;
                }
            }
            "--control" => {
                start_control_tcp = true;
                if let Some(p) = args.get(i + 1).and_then(|s| s.parse::<u16>().ok()) {
                    control_port = p;
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    if is_mcp {
        let _ = control::run_mcp_stdio_server(Some(control_port));
        return Ok(());
    }

    let control_rx = if start_control_tcp {
        control::start_control_server(control_port).ok()
    } else {
        // Also try binding default control port non-fatally so MCP clients can connect anytime
        control::start_control_server(control_port).ok()
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RecordCraft - Craft Suite (High-End Gameplay & Screen Recorder)")
            .with_inner_size([1260.0, 820.0])
            .with_min_inner_size([1040.0, 680.0])
            .with_icon(app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "RecordCraft",
        options,
        Box::new(move |cc| {
            configure_fonts(&cc.egui_ctx);
            let mut app = RecordCraftApp::new(cc);
            app.control_rx = control_rx;
            Ok(Box::new(app))
        }),
    )
}
