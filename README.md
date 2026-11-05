# RecordCraft

RecordCraft is a high-performance desktop screen recorder, multi-track audio studio, and gameplay capture application.

## 100% Original Ground-Up Architecture

RecordCraft was developed entirely from scratch for the Craft Suite. It is built in Rust using egui for high-efficiency rendering, native Windows Audio Session API (WASAPI) COM interfaces for audio loopback, and hardware-accelerated FFmpeg pipelines for video encoding.

## Technical Comparison

### In Comparison to Windows Xbox Game Bar:
- True Multi-Track Audio Isolation: Captures gameplay sound, system desktop audio, and microphone commentary onto independent audio tracks, allowing separate balancing during editing.
- Hardware Encoder Control: Direct access to GPU hardware encoders (NVIDIA NVENC, AMD AMF, Intel QuickSync, and x264/x265 CPU fallback) with configurable bitrates from 10 Mbps up to 100 Mbps and 10-bit color options.
- Container Selection: Flexible output to MP4 or MKV containers with faststart metadata optimizations.
- Per-App Audio Routing: Selective application audio capture via WASAPI loopback and Win32 process enumeration.

### In Comparison to OBS Studio:
- Zero Streaming Overhead: Eliminates complex scene collections, nested filter hierarchies, and RTMP broadcasting menus that are unnecessary for offline recording and gameplay archiving.
- Unified Single-Screen Dashboard: All controls (resolution, framerate, quality, audio routing, encoder settings) reside on a single, high-contrast dashboard with zero sub-menus.
- Direct CutCraft Integration: One-click export handoff automatically launches CutCraft with the newly recorded video and multi-track audio already placed on the timeline.
- Native BYOK AI Sidebar: Integrated assistant for technical recording optimization, hardware encoder advice, and bitrate recommendations.

## Core Features

- Real-Time Live Preview: Live studio canvas showing connected displays, virtual multi-monitor panorama spans, or isolated game windows.
- Dual VU-Meter Mixer: Smooth, hardware-polled volume meters for master output and microphone levels.
- Per-App Audio Session List: Dynamic detection of active Windows programs (Spotify, Discord, browsers, games) with independent volume controls and mute toggles.
- Microphones: Automatic hardware enumeration via Windows Multimedia (WinMM) without hardcoded device strings.
- Cloud Drive Sync: Top status indicator with configurable cloud storage directory selection.

## Building and Installation

### Prerequisites
- Rust 1.80 or later (cargo)
- Windows 10/11 (64-bit)
- FFmpeg available in system PATH or local tools directory

### Build Command
```bash
cargo build --release
```

The resulting binary will be located at `target/release/recordcraft.exe`.

## License

RecordCraft is licensed under the MIT License. See the LICENSE file for details.
