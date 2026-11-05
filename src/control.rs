//! RecordCraft Native MCP Server & JSON Control-Channel Bridge
//!
//! Provides programmatic control of RecordCraft via:
//! 1. In-process AI Agent Sidebar tool execution (`RecordCraftApp::execute_command`)
//! 2. TCP JSON-lines Control Channel (`recordcraft --control <port>`)
//! 3. Stdio Model Context Protocol (MCP) Server (`recordcraft mcp` / `recordcraft --mcp`)

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use serde_json::{Value, json};

use crate::app::RecordCraftApp;
use crate::recorder::{
    detect_system_monitors, get_active_windows, get_available_microphones,
    AudioIsolationMode, AudioRoutingMode, CaptureSourceChoice, ContainerFormat,
    QualityPreset, ResolutionMode, VideoCodec,
};

pub struct ControlRequest {
    pub method: String,
    pub params: Value,
    pub reply: Sender<Value>,
}

/// Starts a background TCP listener for `--control <port>` speaking JSON lines.
pub fn start_control_server(port: u16) -> Result<Receiver<ControlRequest>, String> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("Failed to bind RecordCraft control server on 127.0.0.1:{port}: {e}"))?;
    let (tx, rx) = mpsc::channel::<ControlRequest>();

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let tx_clone = tx.clone();
            std::thread::spawn(move || {
                handle_control_client(stream, tx_clone);
            });
        }
    });

    Ok(rx)
}

fn handle_control_client(stream: TcpStream, tx: Sender<ControlRequest>) {
    let Ok(mut writer) = stream.try_clone() else { return };
    let reader = BufReader::new(stream);
    for line in reader.lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parsed: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let _ = writeln!(writer, "{}", json!({"ok": false, "error": format!("Invalid JSON: {e}")}));
                continue;
            }
        };
        let method = parsed
            .get("method")
            .or_else(|| parsed.get("command"))
            .and_then(Value::as_str)
            .unwrap_or("studio.inspect")
            .to_string();
        let params = parsed.get("params").cloned().unwrap_or_else(|| parsed.clone());

        let (reply_tx, reply_rx) = mpsc::channel();
        if tx.send(ControlRequest { method, params, reply: reply_tx }).is_err() {
            break;
        }
        match reply_rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(resp) => {
                let _ = writeln!(writer, "{resp}");
                let _ = writer.flush();
            }
            Err(_) => {
                let _ = writeln!(writer, "{}", json!({"ok": false, "error": "Control request timed out"}));
                let _ = writer.flush();
            }
        }
    }
}

/// Returns the canonical MCP tool definitions for RecordCraft.
pub fn mcp_tool_definitions() -> Value {
    json!([
        {
            "name": "startRecording",
            "description": "Starts screen/gameplay video and multi-track audio recording immediately.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_name": {"type": "string", "description": "Optional recording or project name"}
                },
                "required": []
            }
        },
        {
            "name": "stopRecording",
            "description": "Stops the active recording, finalizes the video container, and syncs the project to Google Drive.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "selectCaptureSource",
            "description": "Selects the capture source: Primary/Specific Monitor, All Monitors, or a specific Game/Application Window.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": {"type": "string", "description": "'Monitor', 'AllMonitors', or 'GameWindow'"},
                    "monitor_index": {"type": "integer", "description": "0-based monitor index when source is 'Monitor'"},
                    "window_title": {"type": "string", "description": "Window title substring when source is 'GameWindow'"}
                },
                "required": ["source"]
            }
        },
        {
            "name": "setEncoderSettings",
            "description": "Configures hardware video encoder (AMD AMF, NVIDIA NVENC, Intel QSV, x264/x265), quality preset, bitrate, container format, and 10-bit HDR mode.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "codec": {"type": "string", "description": "HevcAmf, H264Amf, Av1Amf, HevcNvenc, H264Nvenc, Av1Nvenc, HevcQsv, H264Qsv, Libx264, Libx265"},
                    "quality": {"type": "string", "description": "Lossless, Ultra100Mbps, High50Mbps, Custom"},
                    "bitrate_mbps": {"type": "integer", "description": "Custom bitrate in Mbps (10..500)"},
                    "cqp": {"type": "integer", "description": "Constant Quantization Parameter (10..30)"},
                    "container": {"type": "string", "description": "Mp4 or Mkv"},
                    "bit_depth_10bit": {"type": "boolean", "description": "Enable 10-bit color depth"}
                },
                "required": []
            }
        },
        {
            "name": "setResolutionFps",
            "description": "Sets output video resolution (Native, 4K, 1440p, 1080p) and target framerate (30, 60, 120, 144, 240 FPS).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "resolution": {"type": "string", "description": "Native, 4K, 1440p, or 1080p"},
                    "framerate": {"type": "integer", "description": "Target FPS, e.g. 30, 60, 120, 144"}
                },
                "required": []
            }
        },
        {
            "name": "muteMic",
            "description": "Mutes or unmutes the microphone and/or desktop master audio channel.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "muted": {"type": "boolean", "description": "true to mute microphone, false to unmute"},
                    "master_muted": {"type": "boolean", "description": "Optional: true to mute desktop master audio"}
                },
                "required": ["muted"]
            }
        },
        {
            "name": "setAudioGain",
            "description": "Adjusts microphone volume gain, desktop system audio gain, multi-track separation, and audio isolation mode.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "mic_gain": {"type": "number", "description": "Microphone gain multiplier (0.0 to 2.0, where 1.0 = 100%)"},
                    "desktop_gain": {"type": "number", "description": "System/desktop audio gain multiplier (0.0 to 2.0, where 1.0 = 100%)"},
                    "multi_track_audio": {"type": "boolean", "description": "Separate game and mic into distinct audio tracks"},
                    "isolation_mode": {"type": "string", "description": "'MasterDesktop' or 'IsolatedGameApp'"}
                },
                "required": []
            }
        },
        {
            "name": "openInCutCraft",
            "description": "Opens the latest recorded video or project directly inside CutCraft for timeline editing.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "refreshDevices",
            "description": "Rescans and refreshes physical monitors, active game/application windows, and connected microphones.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "addMarker",
            "description": "Adds a timestamp highlight marker at the current recording position.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "inspectStudio",
            "description": "Returns the complete live status of RecordCraft Studio (recording state, encoder, resolution, FPS, audio gains, devices, and Drive path).",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }
    ])
}

/// Runs the stdio JSON-RPC 2.0 MCP server when `recordcraft mcp` or `recordcraft --mcp` is called.
pub fn run_mcp_stdio_server(control_port: Option<u16>) -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(req) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        let id = req.get("id").cloned();
        let method = req.get("method").and_then(Value::as_str).unwrap_or("");
        let params = req.get("params").cloned().unwrap_or_else(|| json!({}));

        if id.is_none() {
            // Notification (e.g. notifications/initialized)
            continue;
        }

        let result = match method {
            "initialize" => json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "recordcraft-mcp",
                    "version": "0.1.0"
                }
            }),
            "ping" => json!({}),
            "tools/list" => json!({
                "tools": mcp_tool_definitions()
            }),
            "tools/call" => {
                let tool_name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
                if let Some(port) = control_port {
                    if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
                        let payload = json!({"method": tool_name, "params": args});
                        let _ = writeln!(stream, "{payload}");
                        let mut reader = BufReader::new(stream);
                        let mut resp_line = String::new();
                        if reader.read_line(&mut resp_line).is_ok() {
                            json!({
                                "content": [{"type": "text", "text": resp_line.trim()}],
                                "isError": false
                            })
                        } else {
                            json!({
                                "content": [{"type": "text", "text": "No reply from RecordCraft control channel"}],
                                "isError": true
                            })
                        }
                    } else {
                        json!({
                            "content": [{"type": "text", "text": format!("Could not connect to RecordCraft on port {port}")}],
                            "isError": true
                        })
                    }
                } else {
                    json!({
                        "content": [{"type": "text", "text": json!({"tool": tool_name, "status": "acknowledged", "arguments": args}).to_string()}],
                        "isError": false
                    })
                }
            }
            _ => json!({}),
        };

        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });
        let _ = writeln!(stdout, "{response}");
        let _ = stdout.flush();
    }
    Ok(())
}

impl RecordCraftApp {
    /// Returns a rich JSON snapshot of the current RecordCraft Studio configuration and live state.
    pub fn studio_inspect_json(&self) -> Value {
        let source_str = match self.config.capture_source {
            CaptureSourceChoice::Monitor(i) => format!("Monitor({i})"),
            CaptureSourceChoice::AllMonitors => "AllMonitors".to_string(),
            CaptureSourceChoice::GameWindow => format!("GameWindow({})", self.config.selected_window_title),
        };
        json!({
            "app": "RecordCraft",
            "is_recording": self.recorder.is_recording(),
            "elapsed_seconds": self.recorder.elapsed_seconds(),
            "status_message": self.status_message,
            "project_name": self.config.project_name,
            "capture_source": source_str,
            "codec": self.config.codec.label(),
            "quality_preset": self.config.quality.label(),
            "bitrate_mbps": self.config.custom_bitrate_mbps,
            "cqp": self.config.custom_cqp,
            "resolution": self.config.resolution.label(),
            "framerate": self.config.framerate,
            "container": self.config.container.extension(),
            "bit_depth_10bit": self.config.bit_depth_10bit,
            "mic_muted": self.mic_muted,
            "master_muted": self.master_muted,
            "mic_gain": self.mic_audio_vol,
            "desktop_gain": self.system_audio_vol,
            "multi_track_audio": self.config.multi_track_audio,
            "audio_isolation": self.config.audio_isolation.label(),
            "highlight_markers": self.recorder.highlight_markers,
            "monitors_detected": self.monitors.len(),
            "windows_detected": self.available_windows.len(),
            "mics_detected": self.available_mics,
            "drive_folder": self.drive.cutcraft_base_dir().to_string_lossy(),
        })
    }

    /// Executes any RecordCraft MCP tool or JSON Control-Channel command directly on the live app state.
    pub fn execute_command(&mut self, command: &str, params: &Value) -> Result<Value, String> {
        match command {
            "execute" => {
                let inner_cmd = params.get("command").and_then(Value::as_str).unwrap_or("");
                let inner_params = params.get("params").cloned().unwrap_or_else(|| params.clone());
                if inner_cmd.is_empty() || inner_cmd == "execute" {
                    return Err("Missing inner command in execute".to_string());
                }
                self.execute_command(inner_cmd, &inner_params)
            }
            "startRecording" | "recording.start" | "record.start" => {
                if let Some(name) = params.get("project_name").and_then(Value::as_str) {
                    if !name.trim().is_empty() {
                        self.config.project_name = name.trim().to_string();
                    }
                }
                if self.recorder.is_recording() {
                    Ok(json!({"status": "already_recording", "elapsed_seconds": self.recorder.elapsed_seconds()}))
                } else {
                    self.recorder.start_recording(&self.config, &self.monitors, &self.drive)?;
                    self.status_message = "🔴 LIVE RECORDING started via AI Agent".to_string();
                    Ok(json!({
                        "status": "recording_started",
                        "project_name": self.config.project_name,
                        "codec": self.config.codec.label(),
                        "framerate": self.config.framerate
                    }))
                }
            }
            "stopRecording" | "recording.stop" | "record.stop" => {
                if !self.recorder.is_recording() {
                    Ok(json!({"status": "not_recording"}))
                } else {
                    let (file_path, proj_path) = self.recorder.stop_recording(&self.drive)?;
                    let msg = format!("Recording saved: {}", file_path.display());
                    self.status_message = msg.clone();
                    Ok(json!({
                        "status": "recording_stopped",
                        "message": msg,
                        "last_file": file_path.to_string_lossy().to_string(),
                        "project_file": proj_path.to_string_lossy().to_string()
                    }))
                }
            }
            "selectCaptureSource" | "source.select" | "capture.selectSource" => {
                let src = params.get("source").and_then(Value::as_str).unwrap_or("Monitor").to_lowercase();
                if src.contains("all") {
                    self.config.capture_source = CaptureSourceChoice::AllMonitors;
                    self.status_message = "Capture source set to All Monitors".to_string();
                } else if src.contains("win") || src.contains("game") {
                    self.config.capture_source = CaptureSourceChoice::GameWindow;
                    if let Some(title) = params.get("window_title").and_then(Value::as_str) {
                        if !title.trim().is_empty() {
                            self.config.selected_window_title = title.trim().to_string();
                        }
                    }
                    self.status_message = format!("Capture source set to Game Window: {}", self.config.selected_window_title);
                } else {
                    let idx = params.get("monitor_index").and_then(Value::as_u64).unwrap_or(0) as usize;
                    let clamped = idx.min(self.monitors.len().saturating_sub(1));
                    self.config.capture_source = CaptureSourceChoice::Monitor(clamped);
                    self.status_message = format!("Capture source set to Monitor #{}", clamped + 1);
                }
                Ok(json!({"status": "ok", "status_message": self.status_message}))
            }
            "setEncoderSettings" | "encoder.set" | "encoder.setPreset" => {
                if let Some(codec_str) = params.get("codec").and_then(Value::as_str) {
                    let c = codec_str.to_lowercase();
                    self.config.codec = if c.contains("av1") && c.contains("nvenc") {
                        VideoCodec::Av1Nvenc
                    } else if c.contains("av1") {
                        VideoCodec::Av1Amf
                    } else if c.contains("h264") && c.contains("nvenc") {
                        VideoCodec::H264Nvenc
                    } else if c.contains("nvenc") {
                        VideoCodec::HevcNvenc
                    } else if c.contains("h264") && c.contains("amf") {
                        VideoCodec::H264Amf
                    } else if c.contains("amf") {
                        VideoCodec::HevcAmf
                    } else if c.contains("h264") && c.contains("qsv") {
                        VideoCodec::H264Qsv
                    } else if c.contains("qsv") {
                        VideoCodec::HevcQsv
                    } else if c.contains("265") {
                        VideoCodec::Libx265
                    } else if c.contains("264") {
                        VideoCodec::Libx264
                    } else {
                        self.config.codec
                    };
                }
                if let Some(qual_str) = params.get("quality").and_then(Value::as_str) {
                    let q = qual_str.to_lowercase();
                    self.config.quality = if q.contains("lossless") {
                        QualityPreset::Lossless
                    } else if q.contains("ultra") || q.contains("100") {
                        QualityPreset::Ultra100Mbps
                    } else if q.contains("high") || q.contains("50") {
                        QualityPreset::High50Mbps
                    } else {
                        QualityPreset::Custom
                    };
                }
                if let Some(br) = params.get("bitrate_mbps").and_then(Value::as_u64) {
                    self.config.custom_bitrate_mbps = (br as u32).clamp(5, 500);
                }
                if let Some(cqp) = params.get("cqp").and_then(Value::as_u64) {
                    self.config.custom_cqp = (cqp as u32).clamp(10, 35);
                }
                if let Some(cont) = params.get("container").and_then(Value::as_str) {
                    if cont.to_lowercase().contains("mkv") {
                        self.config.container = ContainerFormat::Mkv;
                    } else {
                        self.config.container = ContainerFormat::Mp4;
                    }
                }
                if let Some(bit10) = params.get("bit_depth_10bit").and_then(Value::as_bool) {
                    self.config.bit_depth_10bit = bit10;
                }
                self.status_message = format!("Encoder updated: {} ({})", self.config.codec.short_badge(), self.config.quality.label());
                Ok(json!({
                    "status": "ok",
                    "codec": self.config.codec.label(),
                    "quality": self.config.quality.label(),
                    "bitrate_mbps": self.config.custom_bitrate_mbps,
                    "container": self.config.container.extension()
                }))
            }
            "setResolutionFps" | "video.set" | "encoder.setResolutionFps" => {
                if let Some(res_str) = params.get("resolution").and_then(Value::as_str) {
                    let r = res_str.to_lowercase();
                    self.config.resolution = if r.contains("4k") || r.contains("2160") || r.contains("uhd") {
                        ResolutionMode::Uhd4k
                    } else if r.contains("1440") || r.contains("qhd") || r.contains("2k") {
                        ResolutionMode::Qhd1440p
                    } else if r.contains("1080") || r.contains("fhd") {
                        ResolutionMode::Fhd1080p
                    } else {
                        ResolutionMode::Native
                    };
                }
                if let Some(fps) = params.get("framerate").or_else(|| params.get("fps")).and_then(Value::as_u64) {
                    self.config.framerate = (fps as u32).clamp(15, 240);
                }
                self.status_message = format!("Video set to {} @ {} FPS", self.config.resolution.label(), self.config.framerate);
                Ok(json!({
                    "status": "ok",
                    "resolution": self.config.resolution.label(),
                    "framerate": self.config.framerate
                }))
            }
            "muteMic" | "audio.muteMic" | "audio.setMicMute" => {
                if let Some(m) = params.get("muted").and_then(Value::as_bool) {
                    self.mic_muted = m;
                } else {
                    self.mic_muted = !self.mic_muted;
                }
                if let Some(mm) = params.get("master_muted").and_then(Value::as_bool) {
                    self.master_muted = mm;
                }
                self.status_message = if self.mic_muted {
                    "🔇 Microphone muted".to_string()
                } else {
                    "🎙️ Microphone active".to_string()
                };
                Ok(json!({"status": "ok", "mic_muted": self.mic_muted, "master_muted": self.master_muted}))
            }
            "setAudioGain" | "audio.setGain" | "balance_mic" => {
                if let Some(mg) = params.get("mic_gain").and_then(Value::as_f64) {
                    self.mic_audio_vol = (mg as f32).clamp(0.0, 2.0);
                }
                if let Some(dg) = params.get("desktop_gain").and_then(Value::as_f64) {
                    self.system_audio_vol = (dg as f32).clamp(0.0, 2.0);
                }
                if let Some(mt) = params.get("multi_track_audio").and_then(Value::as_bool) {
                    self.config.multi_track_audio = mt;
                }
                if let Some(iso) = params.get("isolation_mode").and_then(Value::as_str) {
                    if iso.to_lowercase().contains("isolated") || iso.to_lowercase().contains("game") {
                        self.config.audio_isolation = AudioIsolationMode::IsolatedGameApp;
                        self.config.audio_routing = AudioRoutingMode::PerAppIsolation;
                    } else {
                        self.config.audio_isolation = AudioIsolationMode::MasterDesktop;
                        self.config.audio_routing = AudioRoutingMode::AllSources;
                    }
                }
                self.status_message = format!(
                    "Audio gain updated: Mic {:.0}%, Desktop {:.0}%",
                    self.mic_audio_vol * 100.0,
                    self.system_audio_vol * 100.0
                );
                Ok(json!({
                    "status": "ok",
                    "mic_gain": self.mic_audio_vol,
                    "desktop_gain": self.system_audio_vol,
                    "multi_track_audio": self.config.multi_track_audio
                }))
            }
            "openInCutCraft" | "project.openInCutCraft" => {
                self.open_in_cutcraft();
                Ok(json!({"status": "launched_cutcraft", "message": self.status_message}))
            }
            "refreshDevices" | "devices.refresh" => {
                self.monitors = detect_system_monitors();
                self.available_windows = get_active_windows();
                self.available_mics = get_available_microphones();
                self.status_message = format!(
                    "Devices refreshed: {} monitors, {} windows, {} mics",
                    self.monitors.len(),
                    self.available_windows.len(),
                    self.available_mics.len()
                );
                Ok(json!({
                    "status": "ok",
                    "monitors": self.monitors.len(),
                    "windows": self.available_windows,
                    "microphones": self.available_mics
                }))
            }
            "addMarker" | "recording.addMarker" | "mark_highlights" => {
                let added = self.recorder.add_highlight();
                Ok(json!({
                    "status": "ok",
                    "added_at_sec": added,
                    "markers": self.recorder.highlight_markers
                }))
            }
            "inspectStudio" | "studio.inspect" | "inspect_document" => {
                Ok(self.studio_inspect_json())
            }
            "listCommands" | "engine.commands" => {
                Ok(mcp_tool_definitions())
            }
            other => Err(format!("Unknown RecordCraft command: {other}")),
        }
    }
}
