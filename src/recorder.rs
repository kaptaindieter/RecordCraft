use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Instant;
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

pub struct ActiveRecording {
    pub child: Child,
    pub start_time: Instant,
    pub output_path: PathBuf,
    pub project_name: String,
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

        let output_file = drive.raw_recordings_dir(&proj_name)
            .join(format!("{proj_name}.{}", config.container.extension()));

        if let Some(parent) = output_file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y");

        // 1. Video Input (Windows Desktop Duplication / GDI Grab)
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

        // 2. Audio Inputs (Multi-Track)
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

        // 3. Video Filters (Scaling / Resolution)
        if let Some(scale) = config.resolution.scale_filter() {
            cmd.arg("-vf").arg(scale);
        }

        // 4. Pixel Format
        if config.bit_depth_10bit {
            cmd.arg("-pix_fmt").arg("p010le");
        } else {
            cmd.arg("-pix_fmt").arg("yuv420p");
        }

        // 5. Video Codec
        cmd.arg("-c:v").arg(config.codec.ffmpeg_codec());

        // 6. Quality & Rate Control
        match config.quality {
            QualityPreset::Lossless => {
                match config.codec {
                    VideoCodec::HevcAmf | VideoCodec::H264Amf | VideoCodec::Av1Amf => {
                        cmd.arg("-rc").arg("cqp")
                            .arg("-qp_i").arg("14")
                            .arg("-qp_p").arg("14");
                    }
                    VideoCodec::HevcNvenc | VideoCodec::H264Nvenc | VideoCodec::Av1Nvenc => {
                        cmd.arg("-cq").arg("14")
                            .arg("-preset").arg("p7");
                    }
                    VideoCodec::HevcQsv | VideoCodec::H264Qsv => {
                        cmd.arg("-global_quality").arg("14");
                    }
                    VideoCodec::Libx264 | VideoCodec::Libx265 => {
                        cmd.arg("-crf").arg("14")
                            .arg("-preset").arg("veryfast");
                    }
                }
            }
            QualityPreset::Ultra100Mbps => {
                cmd.arg("-b:v").arg("100M")
                    .arg("-maxrate").arg("120M")
                    .arg("-bufsize").arg("200M");
            }
            QualityPreset::High50Mbps => {
                cmd.arg("-b:v").arg("50M")
                    .arg("-maxrate").arg("60M")
                    .arg("-bufsize").arg("100M");
            }
            QualityPreset::Custom => {
                let br = format!("{}M", config.custom_bitrate_mbps);
                cmd.arg("-b:v").arg(&br);
            }
        }

        // 7. Multi-Track Audio Mapping & Container Optimizations
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
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let child = cmd.spawn().map_err(|e| format!("Failed to launch FFmpeg: {e}"))?;

        self.highlight_markers.clear();
        self.active = Some(ActiveRecording {
            child,
            start_time: Instant::now(),
            output_path: output_file,
            project_name: proj_name,
        });

        Ok(())
    }

    /// Stops the active recording cleanly by sending 'q' to FFmpeg.
    pub fn stop_recording(&mut self, drive: &RecordCraftDrive) -> Result<(PathBuf, PathBuf), String> {
        let Some(mut active) = self.active.take() else {
            return Err("No active recording running!".to_string());
        };

        if let Some(ref mut stdin) = active.child.stdin {
            let _ = stdin.write_all(b"q\n");
            let _ = stdin.flush();
        }

        let _ = active.child.wait();

        let proj_file = drive.create_cutcraft_project(&active.project_name, &active.output_path)
            .map_err(|e| format!("Failed to generate CutCraft project: {e}"))?;

        self.last_recorded_file = Some(active.output_path.clone());
        self.last_created_project = Some(proj_file.clone());

        Ok((active.output_path, proj_file))
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
