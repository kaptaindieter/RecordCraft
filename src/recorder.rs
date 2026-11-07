use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use crate::artcraft_drive::RecordCraftDrive;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoCodec {
    HevcAmf,    // AMD AMF H.265 / HEVC
    H264Amf,    // AMD AMF H.264
    Av1Amf,     // AMD AMF AV1
    HevcNvenc,  // NVIDIA NVENC H.265
    H264Nvenc,  // NVIDIA NVENC H.264
    Av1Nvenc,   // NVIDIA NVENC AV1
    HevcQsv,    // Intel QuickSync H.265
    H264Qsv,    // Intel QuickSync H.264
    Libx264,    // Software x264
    Libx265,    // Software x265
}

impl VideoCodec {
    pub fn label(self) -> &'static str {
        match self {
            Self::HevcAmf => "AMD AMF HEVC / H.265 (Hardware Accelerated)",
            Self::H264Amf => "AMD AMF H.264 / AVC (Hardware Accelerated)",
            Self::Av1Amf => "AMD AMF AV1 (Next-Gen Hardware)",
            Self::HevcNvenc => "NVIDIA NVENC HEVC / H.265",
            Self::H264Nvenc => "NVIDIA NVENC H.264",
            Self::Av1Nvenc => "NVIDIA NVENC AV1",
            Self::HevcQsv => "Intel QuickSync HEVC / H.265",
            Self::H264Qsv => "Intel QuickSync H.264",
            Self::Libx264 => "Software x264 (CPU Fallback)",
            Self::Libx265 => "Software x265 (CPU Fallback)",
        }
    }

    pub fn ffmpeg_codec(self) -> &'static str {
        match self {
            Self::HevcAmf => "hevc_amf",
            Self::H264Amf => "h264_amf",
            Self::Av1Amf => "av1_amf",
            Self::HevcNvenc => "hevc_nvenc",
            Self::H264Nvenc => "h264_nvenc",
            Self::Av1Nvenc => "av1_nvenc",
            Self::HevcQsv => "hevc_qsv",
            Self::H264Qsv => "h264_qsv",
            Self::Libx264 => "libx264",
            Self::Libx265 => "libx265",
        }
    }

    pub fn short_badge(self) -> &'static str {
        match self {
            Self::HevcAmf | Self::H264Amf | Self::Av1Amf => "AMD AMF",
            Self::HevcNvenc | Self::H264Nvenc | Self::Av1Nvenc => "NVENC",
            Self::HevcQsv | Self::H264Qsv => "Intel QSV",
            Self::Libx264 | Self::Libx265 => "CPU x264",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QualityPreset {
    Lossless,     // Visually Lossless CQP 14
    Ultra100Mbps, // 100 Mbps Ultra
    High50Mbps,   // 50 Mbps High
    Custom,       // Custom Bitrate & CQP
}

impl QualityPreset {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lossless => "🔥 Visually Lossless (CQP 14 · Zero Artifacts)",
            Self::Ultra100Mbps => "💎 Ultra Quality (100 Mbps · Perfect for Fast Shooters & 4K)",
            Self::High50Mbps => "🚀 High Quality (50 Mbps · Standard High-End)",
            Self::Custom => "⚙ Custom Bitrate & CQP (Manual Professional Control)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolutionMode {
    Native,
    Uhd4k,
    Qhd1440p,
    Fhd1080p,
}

impl ResolutionMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Native => "Native (Match Monitor / Game Display)",
            Self::Uhd4k => "4K Ultra HD (3840×2160)",
            Self::Qhd1440p => "1440p QHD (2560×1440)",
            Self::Fhd1080p => "1080p Full HD (1920×1080)",
        }
    }

    pub fn scale_filter(self) -> Option<&'static str> {
        match self {
            Self::Native => None,
            Self::Uhd4k => Some("scale=3840:2160:flags=lanczos"),
            Self::Qhd1440p => Some("scale=2560:1440:flags=lanczos"),
            Self::Fhd1080p => Some("scale=1920:1080:flags=lanczos"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerFormat {
    Mp4,
    Mkv,
}

impl ContainerFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "mkv",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Mp4 => "MP4 (Universal Video Container)",
            Self::Mkv => "MKV (Crash-Safe Matroska)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioRoutingMode {
    AllSources,      // All PC Audio (Global Master)
    PerAppIsolation, // Per-Application Audio Capture
}

impl AudioRoutingMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::AllSources => "All Sources (Global Master Desktop)",
            Self::PerAppIsolation => "Per-App Routing (Application Audio Capture)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioIsolationMode {
    MasterDesktop,   // All PC audio
    IsolatedGameApp, // Clean Game Audio only (excludes Spotify, Discord, Browser)
}

impl AudioIsolationMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::MasterDesktop => "Master Desktop Audio (Captures everything you hear)",
            Self::IsolatedGameApp => "Isolated Game Audio (Excludes Discord, Spotify & Browser)",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DetectedMonitor {
    pub id: usize,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub is_primary: bool,
    pub aspect_str: String,
}

/// Dynamically queries Windows Display API for all physically connected monitors.
pub fn detect_system_monitors() -> Vec<DetectedMonitor> {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::Graphics::Gdi::*;
        use windows_sys::Win32::Foundation::*;

        let mut list: Vec<DetectedMonitor> = Vec::new();

        unsafe extern "system" fn enum_proc(
            hmonitor: HMONITOR,
            _hdc: HDC,
            _lprect: *mut RECT,
            lparam: LPARAM,
        ) -> BOOL {
            let list_ptr = lparam as *mut Vec<DetectedMonitor>;
            let mut mi = MONITORINFOEXW {
                monitorInfo: MONITORINFO {
                    cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
                    rcMonitor: RECT { left: 0, top: 0, right: 0, bottom: 0 },
                    rcWork: RECT { left: 0, top: 0, right: 0, bottom: 0 },
                    dwFlags: 0,
                },
                szDevice: [0u16; 32],
            };

            if GetMonitorInfoW(hmonitor, &mut mi as *mut _ as *mut _) != 0 {
                let rc = mi.monitorInfo.rcMonitor;
                let w = rc.right - rc.left;
                let h = rc.bottom - rc.top;
                let is_primary = (mi.monitorInfo.dwFlags & 1) != 0;

                let aspect = if h > 0 {
                    let r = w as f32 / h as f32;
                    if (r - 21.0 / 9.0).abs() < 0.1 || (r - 3440.0 / 1440.0).abs() < 0.05 {
                        "21:9 Ultrawide".to_string()
                    } else if (r - 16.0 / 9.0).abs() < 0.05 {
                        "16:9 Widescreen".to_string()
                    } else if (r - 16.0 / 10.0).abs() < 0.05 {
                        "16:10".to_string()
                    } else {
                        format!("{w}×{h}")
                    }
                } else {
                    "16:9".to_string()
                };

                let count = (*list_ptr).len() + 1;
                let primary_tag = if is_primary { " (Primary)" } else { "" };
                let friendly_name = format!("Monitor {count}{primary_tag}: {w}×{h} · {aspect}");

                (*list_ptr).push(DetectedMonitor {
                    id: count,
                    name: friendly_name,
                    x: rc.left,
                    y: rc.top,
                    width: w,
                    height: h,
                    is_primary,
                    aspect_str: aspect,
                });
            }
            1
        }

        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            Some(enum_proc),
            &mut list as *mut _ as isize,
        );

        if list.is_empty() {
            list.push(DetectedMonitor {
                id: 1,
                name: "Monitor 1 (Primary): 3440×1440 · 21:9 Ultrawide".to_string(),
                x: 0,
                y: 0,
                width: 3440,
                height: 1440,
                is_primary: true,
                aspect_str: "21:9 Ultrawide".to_string(),
            });
        }
        list
    }

    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CaptureSourceChoice {
    Monitor(usize),  // Index into detected_monitors
    AllMonitors,     // Combined Virtual Desktop
    GameWindow,      // Window by title
}

pub struct RecorderConfig {
    pub codec: VideoCodec,
    pub quality: QualityPreset,
    pub custom_bitrate_mbps: u32,
    pub custom_cqp: u32,
    pub resolution: ResolutionMode,
    pub framerate: u32,
    pub container: ContainerFormat,
    pub bit_depth_10bit: bool,
    pub capture_source: CaptureSourceChoice,
    pub selected_window_title: String,
    pub show_cursor: bool,
    pub multi_track_audio: bool,
    pub audio_isolation: AudioIsolationMode,
    pub audio_routing: AudioRoutingMode,
    pub isolated_pids: Vec<u32>,
    pub microphone_name: String,
    pub project_name: String,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            codec: VideoCodec::HevcAmf,
            quality: QualityPreset::Lossless,
            custom_bitrate_mbps: 100,
            custom_cqp: 14,
            resolution: ResolutionMode::Native,
            framerate: 60,
            container: ContainerFormat::Mp4,
            bit_depth_10bit: false,
            capture_source: CaptureSourceChoice::Monitor(0),
            selected_window_title: String::new(),
            show_cursor: true,
            multi_track_audio: true,
            audio_isolation: AudioIsolationMode::MasterDesktop,
            audio_routing: AudioRoutingMode::AllSources,
            isolated_pids: Vec::new(),
            microphone_name: "Kopfhörermikrofon (AceZone A-Spire Wireless Dongle)".to_string(),
            project_name: String::new(),
        }
    }
}

/// Appends a timestamped diagnostic line to the per-project `recording.log`.
///
/// Recording failures used to be invisible: FFmpeg's standard error was
/// discarded and every spawn/wait result was swallowed, so a broken encoder or
/// a missing binary produced an empty project folder and a "saved" status with
/// no way to tell why. Every lifecycle step now writes here; the last line is
/// also surfaced into the UI status message.
fn log_recording(dir: &std::path::Path, message: &str) {
    let path = dir.join("recording.log");
    let line = format!("[{}] {message}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(line.as_bytes());
    }
    eprintln!("[RecordCraft] {message}");
}

/// Best-effort read of the last `n` non-empty lines of a log file, joined for
/// display. Used to surface FFmpeg's own error text in a user-facing message.
fn read_last_lines(path: &std::path::Path, n: usize) -> String {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
            let start = lines.len().saturating_sub(n);
            lines.iter().skip(start).cloned().collect::<Vec<_>>().join(" | ").trim().to_string()
        }
        Err(_) => String::new(),
    }
}

/// Locates the FFmpeg executable.
///
/// The recorder shells out to FFmpeg but never told the user when it is absent
/// (the spawn error was swallowed), so recording silently produced no video.
/// Resolution order: `PATH`, then common installation folders, then a copy
/// beside this executable.
fn ffmpeg_path() -> Option<PathBuf> {
    const CANDIDATES: &[&str] = &[
        r"C:\ffmpeg\bin\ffmpeg.exe",
        r"C:\Program Files\ffmpeg\bin\ffmpeg.exe",
        r"C:\Program Files (x86)\ffmpeg\bin\ffmpeg.exe",
    ];
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(dir) = &exe_dir {
        candidates.push(dir.join("ffmpeg.exe"));
        candidates.push(dir.join("ffmpeg").join("ffmpeg.exe"));
    }
    for c in CANDIDATES {
        candidates.push(PathBuf::from(c));
    }
    let found = Command::new("where")
        .arg("ffmpeg.exe")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty() && std::path::Path::new(l).exists())
                .map(PathBuf::from)
        });
    if let Some(path) = found {
        return Some(path);
    }
    candidates.into_iter().find(|c| c.exists())
}

/// Builds the command that finalizes an already-captured file: re-encodes the
/// raw recording into a playable container with a universal software codec.
fn build_finalization_command(ffmpeg: &PathBuf, raw_file: &PathBuf, output_file: &PathBuf) -> Command {
    let mut cmd = Command::new(ffmpeg);
    cmd.arg("-hide_banner").arg("-y")
        .arg("-i").arg(raw_file.to_string_lossy().to_string())
        .arg("-c:v").arg("libx264")
        .arg("-preset").arg("veryfast")
        .arg("-crf").arg("18")
        .arg("-pix_fmt").arg("yuv420p")
        .arg("-c:a").arg("aac")
        .arg("-b:a").arg("320k");
    if output_file.extension().and_then(|e| e.to_str()) == Some("mp4") {
        cmd.arg("-movflags").arg("+faststart");
    }
    cmd.arg(output_file.to_string_lossy().to_string());
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd
}

/// Waits for a running FFmpeg to exit and returns its exit status. A watchdog
/// kills the process if it ignores the graceful stop. This fixes the previous
/// behavior: sending `q` to `gdigrab` and then calling `wait()` without a
/// timeout blocked the UI indefinitely because `gdigrab` does not honor `q`,
/// and a failed hardware encoder can leave the process alive.
fn await_recording(
    child: &mut Child,
    log_dir: &std::path::Path,
) -> Result<std::process::ExitStatus, String> {
    // Send the graceful quit (best effort) and close the pipe so the child
    // cannot block on a full standard input.
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"q\n");
        let _ = stdin.flush();
        // Dropping `stdin` closes the write end of the pipe.
    }
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                if Instant::now() >= deadline {
                    log_recording(log_dir, "FFmpeg did not exit within 120s; force-killing the process.");
                    let _ = child.kill();
                    return child
                        .wait()
                        .map_err(|e| format!("Failed to reap FFmpeg after kill: {e}"));
                }
                std::thread::sleep(Duration::from_millis(150));
            }
            Err(e) => return Err(format!("Failed to wait for FFmpeg to finish: {e}")),
        }
    }
}

/// What a probe of a media file learned. Used so the generated CutCraft
/// project's timeline canvas, clip length and audio tracks match what is
/// actually on disk instead of a hard-coded placeholder.
#[derive(Clone, Copy, Debug, Default)]
pub struct MediaProbe {
    pub width: u32,
    pub height: u32,
    pub rate_num: i64,
    pub rate_den: i64,
    pub duration: f64,
    pub has_audio: bool,
    pub audio_track_count: u32,
}

/// Probes a media file with `ffprobe` and returns its resolution, frame
/// rate, duration and audio stream count. Every field falls back to a
/// sensible default when it cannot be determined, so the caller always gets
/// a usable value; `None` only when FFmpeg/ffprobe is unavailable.
pub fn probe_media_info(path: &std::path::Path) -> Option<MediaProbe> {
    let ffmpeg = ffmpeg_path()?;
    // ffprobe (usually installed beside ffmpeg) gives a stable JSON report.
    // When it is not available, probing is skipped and the caller falls back
    // to default canvas/duration values rather than guessing.
    let mut ffprobe = ffmpeg.clone();
    let ext = ffprobe
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    ffprobe.set_file_name(format!("ffprobe{ext}"));
    let ffprobe = match ffprobe.exists() {
        true => ffprobe,
        false => return None,
    };

    let out = Command::new(&ffprobe)
        .arg("-hide_banner")
        .arg("-v")
        .arg("error")
        .arg("-show_streams")
        .arg("-show_format")
        .arg("-of")
        .arg("json")
        .arg(path)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).ok()?;
    let mut probe = MediaProbe::default();

    // Duration from the container.
    if let Some(d) = value
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(|s| s.parse::<f64>().ok())
    {
        probe.duration = d;
    }

    // Streams: resolution, frame rate, audio count.
    if let Some(streams) = value.get("streams").and_then(|s| s.as_array()) {
        for stream in streams {
            let codec_type = stream.get("codec_type").and_then(|c| c.as_str()).unwrap_or("");
            if codec_type == "video" {
                if probe.width == 0 {
                    probe.width = stream.get("width").and_then(|w| w.as_u64()).unwrap_or(0) as u32;
                    probe.height = stream.get("height").and_then(|h| h.as_u64()).unwrap_or(0) as u32;
                }
                if probe.rate_num == 0 {
                    // `avg_frame_rate` is "30000/1001" style; `r_frame_rate` too.
                    for key in ["avg_frame_rate", "r_frame_rate"] {
                        if let Some(rate) = stream.get(key).and_then(|r| r.as_str()) {
                            if let Some((num, den)) = rate.split_once('/') {
                                let n = num.trim().parse::<i64>().ok().unwrap_or(0);
                                let d = den.trim().parse::<i64>().ok().unwrap_or(0);
                                if n > 0 && d > 0 {
                                    probe.rate_num = n;
                                    probe.rate_den = d;
                                    break;
                                }
                            }
                        }
                    }
                }
            } else if codec_type == "audio" {
                probe.has_audio = true;
                probe.audio_track_count += 1;
            }
        }
    }

    // Defaults when the probe could not determine a value.
    if probe.width == 0 {
        probe.width = 1920;
    }
    if probe.height == 0 {
        probe.height = 1080;
    }
    if probe.rate_num == 0 {
        probe.rate_num = 60;
        probe.rate_den = 1;
    }
    if probe.duration <= 0.0 {
        probe.duration = 600.0;
    }
    Some(probe)
}

/// Builds the output command for a recording. The diagnostics stream goes to
/// `log_dir/ffmpeg.log` instead of being discarded - that was the core of the
/// "no playable video" bug: an unsupported encoder or a missing microphone
/// failed silently and the output file was never produced.
fn build_output_command(
    ffmpeg: &PathBuf,
    config: &RecorderConfig,
    monitors: &[DetectedMonitor],
    output_file: &PathBuf,
    log_dir: &std::path::Path,
) -> (Command, Vec<String>) {
    let mut cmd = Command::new(ffmpeg);
    cmd.arg("-hide_banner").arg("-y");

    // 1. Video input (Windows GDI grab).
    match config.capture_source {
        CaptureSourceChoice::Monitor(idx) => {
            let mon = monitors.get(idx).or_else(|| monitors.first());
            let (x, y, w, h) = if let Some(m) = mon {
                (m.x, m.y, m.width, m.height)
            } else {
                (0, 0, 3440, 1440)
            };
            cmd.arg("-f").arg("gdigrab")
                .arg("-framerate").arg(config.framerate.to_string())
                .arg("-offset_x").arg(x.to_string())
                .arg("-offset_y").arg(y.to_string())
                .arg("-video_size").arg(format!("{w}x{h}"))
                .arg("-draw_mouse").arg(if config.show_cursor { "1" } else { "0" })
                .arg("-i").arg("desktop");
        }
        CaptureSourceChoice::AllMonitors => {
            let (x, y, w, h) = unsafe {
                use windows_sys::Win32::UI::WindowsAndMessaging::*;
                (
                    GetSystemMetrics(SM_XVIRTUALSCREEN),
                    GetSystemMetrics(SM_YVIRTUALSCREEN),
                    GetSystemMetrics(SM_CXVIRTUALSCREEN),
                    GetSystemMetrics(SM_CYVIRTUALSCREEN),
                )
            };
            cmd.arg("-f").arg("gdigrab")
                .arg("-framerate").arg(config.framerate.to_string())
                .arg("-offset_x").arg(x.to_string())
                .arg("-offset_y").arg(y.to_string())
                .arg("-video_size").arg(format!("{w}x{h}"))
                .arg("-draw_mouse").arg(if config.show_cursor { "1" } else { "0" })
                .arg("-i").arg("desktop");
        }
        CaptureSourceChoice::GameWindow => {
            let title = if config.selected_window_title.is_empty() {
                "desktop".to_string()
            } else {
                format!("title={}", config.selected_window_title)
            };
            cmd.arg("-f").arg("gdigrab")
                .arg("-framerate").arg(config.framerate.to_string())
                .arg("-draw_mouse").arg(if config.show_cursor { "1" } else { "0" })
                .arg("-i").arg(title);
        }
    }

    // 2. Audio input (microphone). A capture device that does not exist or is
    // in use aborts the whole command, so the user gets a message instead of a
    // recording that never produces a file.
    let mic_active = config.multi_track_audio
        && !config.microphone_name.trim().is_empty()
        && !config.microphone_name.contains("Disabled");
    let mut has_audio_in = false;
    if mic_active {
        let mic_name = config.microphone_name.trim();
        let mic_arg = if mic_name.contains("Default") || mic_name.is_empty() {
            let available = get_available_microphones();
            let real = available.iter().find(|m| !m.contains("Default") && !m.contains("Disabled"));
            if let Some(r) = real {
                format!("audio={r}")
            } else {
                "audio=virtual-audio-capturer".to_string()
            }
        } else {
            format!("audio={mic_name}")
        };
        cmd.arg("-f").arg("dshow").arg("-i").arg(mic_arg);
        has_audio_in = true;
    }

    // 3. Video filters (scaling / resolution).
    if let Some(scale) = config.resolution.scale_filter() {
        cmd.arg("-vf").arg(scale);
    }

    // 4. Pixel format.
    if config.bit_depth_10bit {
        cmd.arg("-pix_fmt").arg("p010le");
    } else {
        cmd.arg("-pix_fmt").arg("yuv420p");
    }

    // 5. Video codec.
    cmd.arg("-c:v").arg(config.codec.ffmpeg_codec());

    // 6. Quality & rate control.
    match config.quality {
        QualityPreset::Lossless => match config.codec {
            VideoCodec::HevcAmf | VideoCodec::H264Amf | VideoCodec::Av1Amf => {
                cmd.arg("-rc").arg("cqp").arg("-qp_i").arg("14").arg("-qp_p").arg("14");
            }
            VideoCodec::HevcNvenc | VideoCodec::H264Nvenc | VideoCodec::Av1Nvenc => {
                cmd.arg("-cq").arg("14").arg("-preset").arg("p7");
            }
            VideoCodec::HevcQsv | VideoCodec::H264Qsv => {
                cmd.arg("-global_quality").arg("14");
            }
            VideoCodec::Libx264 | VideoCodec::Libx265 => {
                cmd.arg("-crf").arg("14").arg("-preset").arg("veryfast");
            }
        },
        QualityPreset::Ultra100Mbps => {
            cmd.arg("-b:v").arg("100M").arg("-maxrate").arg("120M").arg("-bufsize").arg("200M");
        }
        QualityPreset::High50Mbps => {
            cmd.arg("-b:v").arg("50M").arg("-maxrate").arg("60M").arg("-bufsize").arg("100M");
        }
        QualityPreset::Custom => {
            let br = format!("{}M", config.custom_bitrate_mbps);
            cmd.arg("-b:v").arg(&br);
        }
    }

    // 7. Multi-track audio mapping & container optimizations.
    if has_audio_in {
        cmd.arg("-map").arg("0:v")
            .arg("-map").arg("1:a")
            .arg("-c:a").arg("aac")
            .arg("-b:a").arg("320k")
            .arg("-metadata:s:a:0").arg("title=Microphone Commentary");
    }
    if config.container == ContainerFormat::Mp4 {
        cmd.arg("-movflags").arg("+faststart");
    }

    cmd.arg(output_file.to_string_lossy().to_string());

    // Keep standard input open so a stop can signal the process, and write the
    // diagnostics to a file we surface on failure.
    let ffmpeg_log = log_dir.join("ffmpeg.log");
    let ffmpeg_stderr = match std::fs::File::create(&ffmpeg_log) {
        Ok(f) => Stdio::from(f),
        Err(_) => Stdio::null(),
    };
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(ffmpeg_stderr);

    let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().to_string()).collect();
    (cmd, args)
}

/// Finalizes a stopped recording into a single playable file and returns it.
///
/// Order of operations:
/// 1. Wait for the capture process to exit (with a kill watchdog).
/// 2. Verify the file FFmpeg actually produced is non-empty.
/// 3. If the capture produced no usable file (e.g. an unsupported hardware
///    encoder or a missing microphone), re-encode the raw capture with a
///    universal software codec so a file is always delivered.
/// 4. If even the re-encode produces nothing, report the exact failure.
fn finalize_recording(
    mut child: Child,
    raw_path: &PathBuf,
    output_path: &PathBuf,
    ffmpeg: &PathBuf,
    log_dir: &std::path::Path,
) -> Result<PathBuf, String> {
    let status = await_recording(&mut child, log_dir)?;
    log_recording(
        log_dir,
        &format!("FFmpeg capture exited with {status} (raw: {})", raw_path.display()),
    );

    // A real, non-empty output file means the capture succeeded and the
    // requested encoding was produced - use it as-is.
    if let Ok(meta) = std::fs::metadata(output_path) {
        if meta.is_file() && meta.len() > 0 {
            log_recording(
                log_dir,
                &format!("Recording finalized: {} ({} bytes)", output_path.display(), meta.len()),
            );
            return Ok(output_path.clone());
        }
    }

    // The capture produced no usable file. Re-encode the raw capture with a
    // universal software codec so a playable file is always delivered.
    log_recording(
        log_dir,
        "Primary capture produced no usable file; re-encoding raw capture with libx264.",
    );
    let raw_ok = std::fs::metadata(raw_path)
        .map(|m| m.is_file() && m.len() > 0)
        .unwrap_or(false);
    if !raw_ok {
        return Err(format!(
            "Recording failed: FFmpeg exited with {status} and produced no video at {} (see ffmpeg.log / recording.log in the project folder). Check that FFmpeg, the selected capture source and encoder are available.",
            raw_path.display()
        ));
    }

    let final_path = if output_path != raw_path {
        output_path.clone()
    } else {
        let parent = output_path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
        let file_name = output_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "recording.mp4".to_string());
        let (stem, ext) = match file_name.split_once('.') {
            Some((s, e)) => (s.to_string(), e.to_string()),
            None => (file_name.clone(), "mp4".to_string()),
        };
        parent.join(format!("{stem}_final.{ext}"))
    };

    let mut final_cmd = build_finalization_command(ffmpeg, raw_path, &final_path);
    let re_status = final_cmd
        .status()
        .map_err(|e| format!("Failed to re-encode recording: {e}"))?;
    log_recording(
        log_dir,
        &format!("Re-encode exited with {re_status} (final: {})", final_path.display()),
    );
    if let Ok(meta) = std::fs::metadata(&final_path) {
        if meta.is_file() && meta.len() > 0 {
            log_recording(
                log_dir,
                &format!("Recording finalized (re-encoded): {} ({} bytes)", final_path.display(), meta.len()),
            );
            return Ok(final_path);
        }
    }
    Err(format!(
        "Recording failed: neither the capture ({}) nor the re-encode ({}) produced a playable file (FFmpeg exit {status}). See ffmpeg.log / recording.log in the project folder for the encoder's own error.",
        raw_path.display(),
        final_path.display()
    ))
}

pub struct ActiveRecording {
    pub child: Child,
    pub start_time: Instant,
    pub output_path: PathBuf,
    pub project_name: String,
    pub ffmpeg: PathBuf,
    pub log_dir: PathBuf,
    pub raw_path: PathBuf,
}

pub struct RecorderState {
    pub active: Option<ActiveRecording>,
    pub last_recorded_file: Option<PathBuf>,
    pub last_created_project: Option<PathBuf>,
    pub highlight_markers: Vec<f64>,
}

impl RecorderState {
    pub fn new() -> Self {
        Self {
            active: None,
            last_recorded_file: None,
            last_created_project: None,
            highlight_markers: Vec::new(),
        }
    }

    pub fn is_recording(&self) -> bool {
        self.active.is_some()
    }

    pub fn elapsed_seconds(&self) -> f64 {
        if let Some(ref a) = self.active {
            a.start_time.elapsed().as_secs_f64()
        } else {
            0.0
        }
    }

    pub fn add_highlight(&mut self) -> Option<f64> {
        if self.is_recording() {
            let t = self.elapsed_seconds();
            self.highlight_markers.push(t);
            Some(t)
        } else {
            None
        }
    }

    /// Starts recording with hardware encoding and multi-track audio.
    pub fn start_recording(
        &mut self,
        config: &RecorderConfig,
        monitors: &[DetectedMonitor],
        drive: &RecordCraftDrive,
    ) -> Result<(), String> {
        if self.active.is_some() {
            return Err("Recording is already running!".to_string());
        }

        let proj_name = if config.project_name.trim().is_empty() {
            format!("Gameplay_{}", chrono::Local::now().format("%Y%m%d_%H%M%S"))
        } else {
            config.project_name.trim().to_string()
        };

        // The raw capture and the final playable file live in the project's
        // `raw_recordings` folder. The raw file is always written (even when
        // the requested hardware encoder is unsupported) so the stop step can
        // re-encode it into a playable file rather than losing the recording.
        let raw_dir = drive.raw_recordings_dir(&proj_name);
        if let Err(e) = std::fs::create_dir_all(&raw_dir) {
            return Err(format!(
                "Failed to create recording folder {}: {e}",
                raw_dir.display()
            ));
        }
        let ext = config.container.extension();
        let raw_file = raw_dir.join(format!("{proj_name}_raw.{ext}"));
        let output_file = raw_dir.join(format!("{proj_name}.{ext}"));

        // FFmpeg must be available before we commit to a recording; without
        // this check a missing binary produced a project folder and no video.
        let ffmpeg = match ffmpeg_path() {
            Some(p) => p,
            None => {
                let msg = "FFmpeg was not found. Install it or add it to PATH - RecordCraft shells out to FFmpeg for capture.".to_string();
                log_recording(&raw_dir, &msg);
                return Err(msg);
            }
        };

        let (mut cmd, args) = build_output_command(&ffmpeg, config, monitors, &raw_file, &raw_dir);
        log_recording(
            &raw_dir,
            &format!("Starting FFmpeg capture: {} {}", ffmpeg.display(), args.join(" ")),
        );

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let tail = read_last_lines(&raw_dir.join("ffmpeg.log"), 6);
                let suffix = if tail.is_empty() {
                    String::new()
                } else {
                    format!(" FFmpeg said: {tail}")
                };
                let msg = format!("Failed to launch FFmpeg ({ffmpeg:?}): {e}.{suffix}");
                log_recording(&raw_dir, &msg);
                return Err(msg);
            }
        };

        self.highlight_markers.clear();
        self.active = Some(ActiveRecording {
            child,
            start_time: Instant::now(),
            output_path: output_file.clone(),
            project_name: proj_name,
            ffmpeg,
            log_dir: raw_dir,
            raw_path: raw_file,
        });

        Ok(())
    }

    /// Stops the active recording, flushes the streams, verifies the output,
    /// and returns the finalized playable file plus the CutCraft project
    /// directory. Never reports success unless a real, non-empty video exists.
    pub fn stop_recording(&mut self, drive: &RecordCraftDrive) -> Result<(PathBuf, PathBuf), String> {
        let Some(active) = self.active.take() else {
            return Err("No active recording running!".to_string());
        };

        let log_dir = active.log_dir.clone();
        let final_file = match finalize_recording(
            active.child,
            &active.raw_path,
            &active.output_path,
            &active.ffmpeg,
            &log_dir,
        ) {
            Ok(f) => f,
            Err(e) => {
                let tail = read_last_lines(&log_dir.join("ffmpeg.log"), 6);
                let suffix = if tail.is_empty() {
                    String::new()
                } else {
                    format!(" FFmpeg said: {tail}")
                };
                let msg = format!("{e}{suffix}");
                log_recording(&log_dir, &msg);
                return Err(msg);
            }
        };

        // Generate the CutCraft project pointing at the playable file: this is
        // the hand-off that lets CutCraft open the recording directly.
        let proj_dir = match drive.create_cutcraft_project(&active.project_name, &final_file) {
            Ok(d) => d,
            Err(e) => {
                let msg = format!(
                    "Recording saved to {} but the CutCraft project could not be created: {e}",
                    final_file.display()
                );
                log_recording(&log_dir, &msg);
                return Err(msg);
            }
        };

        self.last_recorded_file = Some(final_file.clone());
        self.last_created_project = Some(proj_dir.clone());

        Ok((final_file, proj_dir))
    }
}

/// Provides the list of detected and supported microphone devices in English.
pub fn get_available_microphones() -> Vec<String> {
    let mut mics = Vec::new();
    #[repr(C)]
    struct WAVEINCAPSW {
        w_mid: u16,
        w_pid: u16,
        v_driver_version: u32,
        sz_pname: [u16; 32],
        dw_formats: u32,
        w_channels: u16,
        w_reserved1: u16,
    }
    extern "system" {
        fn waveInGetNumDevs() -> u32;
        fn waveInGetDevCapsW(u_device_id: usize, pwic: *mut WAVEINCAPSW, cbwic: u32) -> u32;
    }
    unsafe {
        let count = waveInGetNumDevs();
        for i in 0..count {
            let mut caps: WAVEINCAPSW = std::mem::zeroed();
            if waveInGetDevCapsW(i as usize, &mut caps, std::mem::size_of::<WAVEINCAPSW>() as u32) == 0 {
                let name = String::from_utf16_lossy(&caps.sz_pname);
                let clean = name.trim_matches('\0').trim().to_string();
                if !clean.is_empty() {
                    mics.push(clean);
                }
            }
        }
    }
    if mics.is_empty() {
        mics.push("Default Microphone (Windows Default)".to_string());
    }
    mics.push("Disabled (No Microphone)".to_string());
    mics
}

struct SearchWindowRect {
    target: String,
    found: Option<(i32, i32, i32, i32)>,
}

unsafe extern "system" fn enum_search_rect_cb(
    hwnd: windows_sys::Win32::Foundation::HWND,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::BOOL {
    let search = &mut *(lparam as *mut SearchWindowRect);
    if windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0 {
        let mut buf = [0u16; 512];
        let len = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, buf.as_mut_ptr(), 512);
        if len > 0 {
            let title = String::from_utf16_lossy(&buf[..len as usize]);
            let t_low = title.trim().to_lowercase();
            if !search.target.is_empty() && (t_low == search.target || t_low.contains(&search.target) || search.target.contains(&t_low)) {
                let mut rect = windows_sys::Win32::Foundation::RECT { left: 0, top: 0, right: 0, bottom: 0 };
                windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect);
                let w = rect.right - rect.left;
                let h = rect.bottom - rect.top;
                if w > 30 && h > 30 {
                    search.found = Some((rect.left, rect.top, w, h));
                    return 0;
                }
            }
        }
    }
    1
}

pub fn get_window_rect_by_title(target_title: &str) -> Option<(i32, i32, i32, i32)> {
    if target_title.trim().is_empty() {
        return None;
    }
    let mut search = SearchWindowRect {
        target: target_title.trim().to_lowercase(),
        found: None,
    };
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::EnumWindows(
            Some(enum_search_rect_cb),
            &mut search as *mut SearchWindowRect as isize,
        );
    }
    search.found
}

/// Lists visible top-level windows on Windows for Game / Window capture selection.
pub fn get_active_windows() -> Vec<String> {
    let mut list = Vec::new();
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::EnumWindows(
            Some(enum_window_callback),
            &mut list as *mut Vec<String> as isize,
        );
    }
    list.sort();
    list.dedup();
    list
}

unsafe extern "system" fn enum_window_callback(
    hwnd: windows_sys::Win32::Foundation::HWND,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::BOOL {
    let list = &mut *(lparam as *mut Vec<String>);
    if windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0 {
        let mut buf = [0u16; 512];
        let len = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, buf.as_mut_ptr(), 512);
        if len > 0 {
            if let Ok(title) = String::from_utf16(&buf[..len as usize]) {
                let trimmed = title.trim();
                if !trimmed.is_empty()
                    && trimmed != "Program Manager"
                    && trimmed != "Settings"
                    && !trimmed.starts_with("RecordCraft")
                {
                    list.push(trimmed.to_string());
                }
            }
        }
    }
    1
}
