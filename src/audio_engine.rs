#![allow(non_snake_case, non_camel_case_types)]

use std::collections::HashMap;
use windows_sys::core::{GUID, HRESULT};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Com::*;
use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
use windows_sys::Win32::System::Threading::*;

const CLSID_MMDEVICE_ENUMERATOR: GUID = GUID {
    data1: 0xbcde0395, data2: 0xe52f, data3: 0x467c,
    data4: [0x8e, 0x3d, 0xc4, 0x57, 0x92, 0x91, 0x69, 0x2e],
};
const IID_IMMDEVICE_ENUMERATOR: GUID = GUID {
    data1: 0xa95664d2, data2: 0x9614, data3: 0x4f35,
    data4: [0xa7, 0x46, 0xde, 0x8d, 0xb6, 0x36, 0x17, 0xe6],
};
const IID_IAUDIO_METER_INFORMATION: GUID = GUID {
    data1: 0xc02216f6, data2: 0x8c67, data3: 0x4b5b,
    data4: [0x9d, 0x00, 0xd0, 0x08, 0xe7, 0x3e, 0x00, 0x64],
};
const IID_IAUDIO_SESSION_MANAGER2: GUID = GUID {
    data1: 0x77aa99a0, data2: 0x1bd6, data3: 0x484f,
    data4: [0xbc, 0x0a, 0x7a, 0x15, 0xe9, 0xf1, 0xcc, 0xfe],
};
const IID_IAUDIO_SESSION_CONTROL2: GUID = GUID {
    data1: 0xbfb7ff88, data2: 0x7239, data3: 0x4fc9,
    data4: [0x8f, 0xa2, 0x07, 0xc9, 0x50, 0xbe, 0x9c, 0x6d],
};
const IID_ISIMPLE_AUDIO_VOLUME: GUID = GUID {
    data1: 0x87ce5498, data2: 0x68d6, data3: 0x44e5,
    data4: [0xac, 0x21, 0x42, 0xc0, 0xeb, 0x83, 0x07, 0x17],
};

const E_RENDER: i32 = 0;
const E_CAPTURE: i32 = 1;
const E_MULTIMEDIA: i32 = 1;

#[repr(C)]
struct IUnknownVtbl {
    pub QueryInterface: unsafe extern "system" fn(this: *mut IUnknown, riid: *const GUID, ppvObject: *mut *mut std::ffi::c_void) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(this: *mut IUnknown) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut IUnknown) -> u32,
}

#[repr(C)]
struct IUnknown {
    pub lpVtbl: *const IUnknownVtbl,
}

#[repr(C)]
struct IMMDeviceEnumeratorVtbl {
    pub base: IUnknownVtbl,
    pub EnumAudioEndpoints: unsafe extern "system" fn(this: *mut std::ffi::c_void, dataFlow: i32, dwStateMask: u32, ppDevices: *mut *mut std::ffi::c_void) -> HRESULT,
    pub GetDefaultAudioEndpoint: unsafe extern "system" fn(this: *mut std::ffi::c_void, dataFlow: i32, role: i32, ppEndpoint: *mut *mut std::ffi::c_void) -> HRESULT,
}

#[repr(C)]
struct IMMDeviceVtbl {
    pub base: IUnknownVtbl,
    pub Activate: unsafe extern "system" fn(this: *mut std::ffi::c_void, iid: *const GUID, dwClsCtx: u32, pActivationParams: *const std::ffi::c_void, ppInterface: *mut *mut std::ffi::c_void) -> HRESULT,
}

#[repr(C)]
struct IAudioMeterInformationVtbl {
    pub base: IUnknownVtbl,
    pub GetPeakValue: unsafe extern "system" fn(this: *mut std::ffi::c_void, pfPeak: *mut f32) -> HRESULT,
}

#[repr(C)]
struct IAudioSessionManager2Vtbl {
    pub base: IUnknownVtbl,
    pub GetAudioSessionControl: unsafe extern "system" fn(this: *mut std::ffi::c_void, AudioSessionGuid: *const GUID, StreamFlags: u32, SessionControl: *mut *mut std::ffi::c_void) -> HRESULT,
    pub GetSimpleAudioVolume: unsafe extern "system" fn(this: *mut std::ffi::c_void, AudioSessionGuid: *const GUID, StreamFlags: u32, AudioVolume: *mut *mut std::ffi::c_void) -> HRESULT,
    pub GetSessionEnumerator: unsafe extern "system" fn(this: *mut std::ffi::c_void, SessionEnum: *mut *mut std::ffi::c_void) -> HRESULT,
}

#[repr(C)]
struct IAudioSessionEnumeratorVtbl {
    pub base: IUnknownVtbl,
    pub GetCount: unsafe extern "system" fn(this: *mut std::ffi::c_void, SessionCount: *mut i32) -> HRESULT,
    pub GetSession: unsafe extern "system" fn(this: *mut std::ffi::c_void, SessionIndex: i32, Session: *mut *mut std::ffi::c_void) -> HRESULT,
}

#[repr(C)]
struct IAudioSessionControlVtbl {
    pub base: IUnknownVtbl,
    pub GetState: unsafe extern "system" fn(this: *mut std::ffi::c_void, pRetVal: *mut i32) -> HRESULT,
}

#[repr(C)]
struct IAudioSessionControl2Vtbl {
    pub base: IAudioSessionControlVtbl,
    pub _pad1: usize,
    pub _pad2: usize,
    pub _pad3: usize,
    pub _pad4: usize,
    pub _pad5: usize,
    pub _pad6: usize,
    pub _pad7: usize,
    pub _pad8: usize,
    pub _pad9: usize,
    pub _pad10: usize,
    pub GetProcessId: unsafe extern "system" fn(this: *mut std::ffi::c_void, pRetVal: *mut u32) -> HRESULT,
}

#[repr(C)]
struct ISimpleAudioVolumeVtbl {
    pub base: IUnknownVtbl,
    pub SetMasterVolume: unsafe extern "system" fn(this: *mut std::ffi::c_void, fLevel: f32, EventContext: *const GUID) -> HRESULT,
    pub GetMasterVolume: unsafe extern "system" fn(this: *mut std::ffi::c_void, pfLevel: *mut f32) -> HRESULT,
    pub SetMute: unsafe extern "system" fn(this: *mut std::ffi::c_void, bMute: i32, EventContext: *const GUID) -> HRESULT,
    pub GetMute: unsafe extern "system" fn(this: *mut std::ffi::c_void, pbMute: *mut i32) -> HRESULT,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioAppSession {
    pub pid: u32,
    pub pids: Vec<u32>,
    pub name: String,
    pub display_name: String,
    pub peak: f32,
    pub volume: f32,
    pub is_muted: bool,
    pub is_active: bool,
    pub has_audio_session: bool,
    pub is_selected: bool,
}

#[derive(Clone, Debug, Default)]
pub struct SystemAudioState {
    pub master_peak: f32,
    pub mic_peak: f32,
    pub apps: Vec<AudioAppSession>,
}

const DENYLIST_SUBSTRINGS: &[&str] = &[
    "crashpad", "helper", "handler", "proxy", "launcher",
    "browser_assistant", "cline-cli", "gameinput", "runtimebroker",
    "applicationframehost", "textinputhost", "searchhost",
    "startmenuexperience", "shellexperience",
];

pub fn is_denied_app(exe_or_display: &str) -> bool {
    let lower = exe_or_display.to_ascii_lowercase();
    if lower == "recordcraft.exe" || lower == "recordcraft" || lower.starts_with("app #") || lower.starts_with("process_") {
        return true;
    }
    DENYLIST_SUBSTRINGS.iter().any(|sub| lower.contains(sub))
}

pub fn sanitize_window_title(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|&c| {
            let code = c as u32;
            (code >= 0x20 && code <= 0x017F)
                || matches!(c, '–' | '—' | '…' | '•')
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.chars().count() > 40 {
        let mut s: String = trimmed.chars().take(39).collect();
        s.push('…');
        s
    } else {
        trimmed.to_string()
    }
}

pub fn clean_exe_display_name(exe_filename: &str) -> String {
    let base = exe_filename
        .strip_suffix(".exe")
        .or_else(|| exe_filename.strip_suffix(".EXE"))
        .unwrap_or(exe_filename);
    let mut chars = base.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

pub fn get_process_name_by_pid(pid: u32) -> (String, String) {
    if pid == 0 || pid == std::process::id() {
        return (String::new(), String::new());
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if !handle.is_null() {
            let mut buf = [0u16; 512];
            let mut size = 512u32;
            let res = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
            CloseHandle(handle);
            if res != 0 && size > 0 {
                let full = String::from_utf16_lossy(&buf[..size as usize]);
                if let Some(filename) = full.rsplit('\\').next() {
                    let friendly = clean_exe_display_name(filename);
                    return (filename.to_string(), friendly);
                }
            }
        }
    }
    (format!("Process_{pid}"), format!("App #{pid}"))
}

struct GuiAppEnum {
    apps: Vec<(u32, String, String)>,
}

unsafe extern "system" fn enum_gui_app_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state = &mut *(lparam as *mut GuiAppEnum);
    if windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0 {
        let mut pid = 0u32;
        windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, &mut pid);
        if pid != 0 && pid != std::process::id() {
            let mut buf = [0u16; 512];
            let len = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, buf.as_mut_ptr(), 512);
            let raw_title = if len > 0 {
                String::from_utf16_lossy(&buf[..len as usize]).trim().to_string()
            } else {
                String::new()
            };
            let title = sanitize_window_title(&raw_title);
            let title_lower = title.to_lowercase();
            if !title.is_empty()
                && title != "Program Manager"
                && title != "Settings"
                && title != "Windows Input Experience"
                && !title.starts_with("RecordCraft")
                && !title_lower.contains("geforce overlay")
            {
                let (proc_name, friendly) = get_process_name_by_pid(pid);
                if !proc_name.is_empty() && !is_denied_app(&proc_name) && !is_denied_app(&friendly) {
                    let display = if !title.is_empty() && !friendly.to_lowercase().contains(&title_lower) {
                        format!("{friendly} – {title}")
                    } else {
                        friendly
                    };
                    state.apps.push((pid, proc_name, display));
                }
            }
        }
    }
    1
}

pub fn enumerate_running_gui_apps() -> Vec<(u32, String, String)> {
    // Keyed by lowercase exe_name -> (representative_pid, exe_name, display_name, all_pids)
    let mut by_exe: HashMap<String, (u32, String, String, Vec<u32>)> = HashMap::new();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot != INVALID_HANDLE_VALUE && !snapshot.is_null() {
            let mut pe: PROCESSENTRY32W = std::mem::zeroed();
            pe.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            if Process32FirstW(snapshot, &mut pe) != 0 {
                loop {
                    let pid = pe.th32ProcessID;
                    if pid != 0 && pid != std::process::id() {
                        let name_len = pe.szExeFile.iter().position(|&c| c == 0).unwrap_or(pe.szExeFile.len());
                        let exe_name = String::from_utf16_lossy(&pe.szExeFile[..name_len]).trim().to_string();
                        let exe_lower = exe_name.to_ascii_lowercase();

                        let is_system_or_host = match exe_lower.as_str() {
                            "smss.exe" | "csrss.exe" | "wininit.exe" | "services.exe" | "lsaiso.exe" | "lsass.exe"
                            | "fontdrvhost.exe" | "dwm.exe" | "wudfhost.exe" | "atieclxx.exe" | "atiesrxx.exe"
                            | "amdfendrsr.exe" | "dashost.exe" | "spoolsv.exe" | "amdppkgsvc.exe" | "msmpeng.exe"
                            | "gamingservices.exe" | "gamingservicesnet.exe" | "aggregatorhost.exe" | "audiodg.exe"
                            | "midisrv.exe" | "msdtc.exe" | "winlogon.exe" | "dllhost.exe" | "taskhostw.exe"
                            | "sihost.exe" | "useroobebroker.exe" | "widgetboard.exe" | "winrtutil64.exe"
                            | "crashpad_handler.exe" | "applicationframehost.exe" | "backgroundtaskhost.exe"
                            | "crossdeviceresume.exe" | "ctfmon.exe" | "smartscreen.exe" | "phoneexperiencehost.exe"
                            | "shellhost.exe" | "startmenuexperiencehost.exe" | "systemsettings.exe" => true,
                            other => {
                                other.starts_with("svchost")
                                    || other.starts_with("conhost")
                                    || other.starts_with("wmi")
                                    || other.starts_with("antigravity")
                                    || other.starts_with("language_server")
                                    || other.ends_with("service.exe")
                                    || !other.ends_with(".exe")
                                    || is_denied_app(other)
                            }
                        };

                        if !is_system_or_host {
                            let friendly = clean_exe_display_name(&exe_name);
                            if !friendly.is_empty() && !is_denied_app(&friendly) {
                                let entry = by_exe.entry(exe_lower).or_insert_with(|| (pid, exe_name.clone(), friendly, Vec::new()));
                                if !entry.3.contains(&pid) {
                                    entry.3.push(pid);
                                }
                            }
                        }
                    }
                    if Process32NextW(snapshot, &mut pe) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);
        }
    }

    let mut win_state = GuiAppEnum { apps: Vec::new() };
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::EnumWindows(
            Some(enum_gui_app_cb),
            &mut win_state as *mut GuiAppEnum as isize,
        );
    }

    for (w_pid, w_proc, w_disp) in win_state.apps {
        let key = w_proc.to_ascii_lowercase();
        if is_denied_app(&key) || is_denied_app(&w_disp) {
            continue;
        }
        let entry = by_exe.entry(key).or_insert_with(|| (w_pid, w_proc, w_disp.clone(), Vec::new()));
        entry.2 = w_disp;
        if !entry.3.contains(&w_pid) {
            entry.3.push(w_pid);
        }
    }

    let mut result: Vec<(u32, String, String)> = by_exe
        .into_values()
        .map(|(pid, name, disp, _)| (pid, name, disp))
        .collect();
    result.sort_by(|a, b| a.2.to_lowercase().cmp(&b.2.to_lowercase()));
    result
}

pub fn poll_system_audio() -> SystemAudioState {
    let mut state = SystemAudioState::default();
    let mut app_map: HashMap<String, AudioAppSession> = HashMap::new();

    // Window titles map by exe_lower for clean title enrichment
    let mut win_state = GuiAppEnum { apps: Vec::new() };
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::EnumWindows(
            Some(enum_gui_app_cb),
            &mut win_state as *mut GuiAppEnum as isize,
        );
    }
    let mut title_by_exe: HashMap<String, String> = HashMap::new();
    for (_, w_proc, w_disp) in win_state.apps {
        title_by_exe.entry(w_proc.to_ascii_lowercase()).or_insert(w_disp);
    }

    unsafe {
        let _ = CoInitializeEx(std::ptr::null_mut(), COINIT_MULTITHREADED as u32);

        let mut enumerator: *mut std::ffi::c_void = std::ptr::null_mut();
        if CoCreateInstance(
            &CLSID_MMDEVICE_ENUMERATOR,
            std::ptr::null_mut(),
            CLSCTX_ALL,
            &IID_IMMDEVICE_ENUMERATOR,
            &mut enumerator as *mut _ as *mut _,
        ) == 0 && !enumerator.is_null()
        {
            let enum_vtbl = &**(enumerator as *mut *const IMMDeviceEnumeratorVtbl);

            let mut render_device: *mut std::ffi::c_void = std::ptr::null_mut();
            if (enum_vtbl.GetDefaultAudioEndpoint)(enumerator, E_RENDER, E_MULTIMEDIA, &mut render_device) == 0
                && !render_device.is_null()
            {
                let dev_vtbl = &**(render_device as *mut *const IMMDeviceVtbl);

                let mut meter: *mut std::ffi::c_void = std::ptr::null_mut();
                if (dev_vtbl.Activate)(
                    render_device,
                    &IID_IAUDIO_METER_INFORMATION,
                    CLSCTX_ALL,
                    std::ptr::null(),
                    &mut meter as *mut _ as *mut _,
                ) == 0 && !meter.is_null()
                {
                    let meter_vtbl = &**(meter as *mut *const IAudioMeterInformationVtbl);
                    let mut peak = 0.0f32;
                    let _ = (meter_vtbl.GetPeakValue)(meter, &mut peak);
                    state.master_peak = peak.clamp(0.0, 1.0);
                    let unk = meter as *mut IUnknown;
                    ((*(*unk).lpVtbl).Release)(unk);
                }

                let mut mgr: *mut std::ffi::c_void = std::ptr::null_mut();
                if (dev_vtbl.Activate)(
                    render_device,
                    &IID_IAUDIO_SESSION_MANAGER2,
                    CLSCTX_ALL,
                    std::ptr::null(),
                    &mut mgr as *mut _ as *mut _,
                ) == 0 && !mgr.is_null()
                {
                    let mgr_vtbl = &**(mgr as *mut *const IAudioSessionManager2Vtbl);
                    let mut sess_enum: *mut std::ffi::c_void = std::ptr::null_mut();
                    if (mgr_vtbl.GetSessionEnumerator)(mgr, &mut sess_enum) == 0 && !sess_enum.is_null() {
                        let s_enum_vtbl = &**(sess_enum as *mut *const IAudioSessionEnumeratorVtbl);
                        let mut count = 0i32;
                        let _ = (s_enum_vtbl.GetCount)(sess_enum, &mut count);

                        for i in 0..count {
                            let mut ctrl: *mut std::ffi::c_void = std::ptr::null_mut();
                            if (s_enum_vtbl.GetSession)(sess_enum, i, &mut ctrl) == 0 && !ctrl.is_null() {
                                let unk = ctrl as *mut IUnknown;

                                let mut ctrl2: *mut std::ffi::c_void = std::ptr::null_mut();
                                if ((*(*unk).lpVtbl).QueryInterface)(
                                    unk,
                                    &IID_IAUDIO_SESSION_CONTROL2,
                                    &mut ctrl2 as *mut _ as *mut _,
                                ) == 0 && !ctrl2.is_null()
                                {
                                    let ctrl2_vtbl = &**(ctrl2 as *mut *const IAudioSessionControl2Vtbl);
                                    let mut pid = 0u32;
                                    let _ = (ctrl2_vtbl.GetProcessId)(ctrl2, &mut pid);

                                    if pid != 0 && pid != std::process::id() {
                                        let (proc_name, base_friendly) = get_process_name_by_pid(pid);
                                        let key = proc_name.to_ascii_lowercase();

                                        if !proc_name.is_empty() && !is_denied_app(&key) && !is_denied_app(&base_friendly) {
                                            let mut app_peak = 0.0f32;
                                            let mut meter_iface: *mut std::ffi::c_void = std::ptr::null_mut();
                                            if ((*(*unk).lpVtbl).QueryInterface)(
                                                unk,
                                                &IID_IAUDIO_METER_INFORMATION,
                                                &mut meter_iface as *mut _ as *mut _,
                                            ) == 0 && !meter_iface.is_null()
                                            {
                                                let meter_vtbl = &**(meter_iface as *mut *const IAudioMeterInformationVtbl);
                                                let _ = (meter_vtbl.GetPeakValue)(meter_iface, &mut app_peak);
                                                let m_unk = meter_iface as *mut IUnknown;
                                                ((*(*m_unk).lpVtbl).Release)(m_unk);
                                            }

                                            let mut vol = 1.0f32;
                                            let mut muted = 0i32;
                                            let mut s_vol: *mut std::ffi::c_void = std::ptr::null_mut();
                                            if ((*(*unk).lpVtbl).QueryInterface)(
                                                unk,
                                                &IID_ISIMPLE_AUDIO_VOLUME,
                                                &mut s_vol as *mut _ as *mut _,
                                            ) == 0 && !s_vol.is_null()
                                            {
                                                let s_vol_vtbl = &**(s_vol as *mut *const ISimpleAudioVolumeVtbl);
                                                let _ = (s_vol_vtbl.GetMasterVolume)(s_vol, &mut vol);
                                                let _ = (s_vol_vtbl.GetMute)(s_vol, &mut muted);
                                                let v_unk = s_vol as *mut IUnknown;
                                                ((*(*v_unk).lpVtbl).Release)(v_unk);
                                            }

                                            let ctrl_vtbl = &**(ctrl as *mut *const IAudioSessionControlVtbl);
                                            let mut sess_state = 0i32;
                                            let _ = (ctrl_vtbl.GetState)(ctrl, &mut sess_state);
                                            let is_active = sess_state == 1 || app_peak > 0.005;

                                            let display_name = title_by_exe.get(&key).cloned().unwrap_or(base_friendly);

                                            let entry = app_map.entry(key).or_insert_with(|| AudioAppSession {
                                                pid,
                                                pids: vec![pid],
                                                name: proc_name,
                                                display_name,
                                                peak: 0.0,
                                                volume: vol,
                                                is_muted: muted != 0,
                                                is_active: false,
                                                has_audio_session: true,
                                                is_selected: true,
                                            });

                                            if !entry.pids.contains(&pid) {
                                                entry.pids.push(pid);
                                                entry.pids.sort_unstable();
                                            }
                                            entry.peak = entry.peak.max(app_peak.clamp(0.0, 1.0));
                                            entry.is_active = entry.is_active || is_active;
                                            entry.has_audio_session = true;
                                        }
                                    }

                                    let c2_unk = ctrl2 as *mut IUnknown;
                                    ((*(*c2_unk).lpVtbl).Release)(c2_unk);
                                }

                                ((*(*unk).lpVtbl).Release)(unk);
                            }
                        }

                        let se_unk = sess_enum as *mut IUnknown;
                        ((*(*se_unk).lpVtbl).Release)(se_unk);
                    }

                    let mgr_unk = mgr as *mut IUnknown;
                    ((*(*mgr_unk).lpVtbl).Release)(mgr_unk);
                }

                let dev_unk = render_device as *mut IUnknown;
                ((*(*dev_unk).lpVtbl).Release)(dev_unk);
            }

            let mut capture_device: *mut std::ffi::c_void = std::ptr::null_mut();
            if (enum_vtbl.GetDefaultAudioEndpoint)(enumerator, E_CAPTURE, E_MULTIMEDIA, &mut capture_device) == 0
                && !capture_device.is_null()
            {
                let cap_vtbl = &**(capture_device as *mut *const IMMDeviceVtbl);
                let mut mic_meter: *mut std::ffi::c_void = std::ptr::null_mut();
                if (cap_vtbl.Activate)(
                    capture_device,
                    &IID_IAUDIO_METER_INFORMATION,
                    CLSCTX_ALL,
                    std::ptr::null(),
                    &mut mic_meter as *mut _ as *mut _,
                ) == 0 && !mic_meter.is_null()
                {
                    let meter_vtbl = &**(mic_meter as *mut *const IAudioMeterInformationVtbl);
                    let mut peak = 0.0f32;
                    let _ = (meter_vtbl.GetPeakValue)(mic_meter, &mut peak);
                    state.mic_peak = peak.clamp(0.0, 1.0);
                    let m_unk = mic_meter as *mut IUnknown;
                    ((*(*m_unk).lpVtbl).Release)(m_unk);
                }

                let cap_unk = capture_device as *mut IUnknown;
                ((*(*cap_unk).lpVtbl).Release)(cap_unk);
            }

            let e_unk = enumerator as *mut IUnknown;
            ((*(*e_unk).lpVtbl).Release)(e_unk);
        }
    }

    let running_gui_apps = enumerate_running_gui_apps();
    for (pid, proc_name, display_name) in running_gui_apps {
        let key = proc_name.to_ascii_lowercase();
        if is_denied_app(&key) || is_denied_app(&display_name) {
            continue;
        }
        let entry = app_map.entry(key).or_insert_with(|| AudioAppSession {
            pid,
            pids: vec![pid],
            name: proc_name,
            display_name,
            peak: 0.0,
            volume: 1.0,
            is_muted: false,
            is_active: false,
            has_audio_session: false,
            is_selected: true,
        });
        if !entry.pids.contains(&pid) {
            entry.pids.push(pid);
            entry.pids.sort_unstable();
        }
    }

    let mut apps: Vec<AudioAppSession> = app_map.into_values().collect();
    apps.sort_by(|a, b| {
        b.has_audio_session
            .cmp(&a.has_audio_session)
            .then_with(|| b.is_active.cmp(&a.is_active))
            .then_with(|| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()))
    });
    state.apps = apps;
    state
}

pub fn set_app_volume_by_name(target_exe: &str, volume: f32, mute: bool) {
    let target_lower = target_exe.to_ascii_lowercase();
    unsafe {
        let _ = CoInitializeEx(std::ptr::null_mut(), COINIT_MULTITHREADED as u32);

        let mut enumerator: *mut std::ffi::c_void = std::ptr::null_mut();
        if CoCreateInstance(
            &CLSID_MMDEVICE_ENUMERATOR,
            std::ptr::null_mut(),
            CLSCTX_ALL,
            &IID_IMMDEVICE_ENUMERATOR,
            &mut enumerator as *mut _ as *mut _,
        ) != 0 || enumerator.is_null()
        {
            return;
        }

        let enum_vtbl = &**(enumerator as *mut *const IMMDeviceEnumeratorVtbl);
        let mut render_device: *mut std::ffi::c_void = std::ptr::null_mut();
        if (enum_vtbl.GetDefaultAudioEndpoint)(enumerator, E_RENDER, E_MULTIMEDIA, &mut render_device) == 0
            && !render_device.is_null()
        {
            let dev_vtbl = &**(render_device as *mut *const IMMDeviceVtbl);
            let mut mgr: *mut std::ffi::c_void = std::ptr::null_mut();
            if (dev_vtbl.Activate)(
                render_device,
                &IID_IAUDIO_SESSION_MANAGER2,
                CLSCTX_ALL,
                std::ptr::null(),
                &mut mgr as *mut _ as *mut _,
            ) == 0 && !mgr.is_null()
            {
                let mgr_vtbl = &**(mgr as *mut *const IAudioSessionManager2Vtbl);
                let mut sess_enum: *mut std::ffi::c_void = std::ptr::null_mut();
                if (mgr_vtbl.GetSessionEnumerator)(mgr, &mut sess_enum) == 0 && !sess_enum.is_null() {
                    let s_enum_vtbl = &**(sess_enum as *mut *const IAudioSessionEnumeratorVtbl);
                    let mut count = 0i32;
                    let _ = (s_enum_vtbl.GetCount)(sess_enum, &mut count);

                    for i in 0..count {
                        let mut ctrl: *mut std::ffi::c_void = std::ptr::null_mut();
                        if (s_enum_vtbl.GetSession)(sess_enum, i, &mut ctrl) == 0 && !ctrl.is_null() {
                            let unk = ctrl as *mut IUnknown;
                            let mut ctrl2: *mut std::ffi::c_void = std::ptr::null_mut();
                            if ((*(*unk).lpVtbl).QueryInterface)(
                                unk,
                                &IID_IAUDIO_SESSION_CONTROL2,
                                &mut ctrl2 as *mut _ as *mut _,
                            ) == 0 && !ctrl2.is_null()
                            {
                                let ctrl2_vtbl = &**(ctrl2 as *mut *const IAudioSessionControl2Vtbl);
                                let mut pid = 0u32;
                                let _ = (ctrl2_vtbl.GetProcessId)(ctrl2, &mut pid);
                                if pid != 0 {
                                    let (proc_name, _) = get_process_name_by_pid(pid);
                                    if proc_name.to_ascii_lowercase() == target_lower {
                                        let mut s_vol: *mut std::ffi::c_void = std::ptr::null_mut();
                                        if ((*(*unk).lpVtbl).QueryInterface)(
                                            unk,
                                            &IID_ISIMPLE_AUDIO_VOLUME,
                                            &mut s_vol as *mut _ as *mut _,
                                        ) == 0 && !s_vol.is_null()
                                        {
                                            let s_vol_vtbl = &**(s_vol as *mut *const ISimpleAudioVolumeVtbl);
                                            let _ = (s_vol_vtbl.SetMasterVolume)(s_vol, volume.clamp(0.0, 1.0), std::ptr::null());
                                            let _ = (s_vol_vtbl.SetMute)(s_vol, if mute { 1 } else { 0 }, std::ptr::null());
                                            let v_unk = s_vol as *mut IUnknown;
                                            ((*(*v_unk).lpVtbl).Release)(v_unk);
                                        }
                                    }
                                }
                                let c2_unk = ctrl2 as *mut IUnknown;
                                ((*(*c2_unk).lpVtbl).Release)(c2_unk);
                            }
                            ((*(*unk).lpVtbl).Release)(unk);
                        }
                    }
                    let se_unk = sess_enum as *mut IUnknown;
                    ((*(*se_unk).lpVtbl).Release)(se_unk);
                }
                let mgr_unk = mgr as *mut IUnknown;
                ((*(*mgr_unk).lpVtbl).Release)(mgr_unk);
            }
            let dev_unk = render_device as *mut IUnknown;
            ((*(*dev_unk).lpVtbl).Release)(dev_unk);
        }
        let e_unk = enumerator as *mut IUnknown;
        ((*(*e_unk).lpVtbl).Release)(e_unk);
    }
}

pub fn set_process_volume(target_pid: u32, volume: f32, mute: bool) {
    let (proc_name, _) = get_process_name_by_pid(target_pid);
    if !proc_name.is_empty() {
        set_app_volume_by_name(&proc_name, volume, mute);
    }
}
