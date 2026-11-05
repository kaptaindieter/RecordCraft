use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyEvent {
    ToggleRecord,
    HighlightMarker,
}

pub struct HotkeyManager {
    pub _enabled: Arc<AtomicBool>,
    pub rx: Receiver<HotkeyEvent>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let enabled = Arc::new(AtomicBool::new(true));
        let enabled_clone = enabled.clone();

        thread::spawn(move || {
            Self::hotkey_worker_loop(tx, enabled_clone);
        });

        Self { _enabled: enabled, rx }
    }

    fn hotkey_worker_loop(tx: Sender<HotkeyEvent>, enabled: Arc<AtomicBool>) {
        const VK_F9: i32 = 0x78;
        const VK_F10: i32 = 0x79;

        let mut f9_was_down = false;
        let mut f10_was_down = false;

        loop {
            thread::sleep(Duration::from_millis(40));

            if !enabled.load(Ordering::Relaxed) {
                continue;
            }

            unsafe {
                let f9_down = (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(VK_F9) as u16 & 0x8000) != 0;
                let f10_down = (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(VK_F10) as u16 & 0x8000) != 0;

                if f9_down && !f9_was_down {
                    let _ = tx.send(HotkeyEvent::ToggleRecord);
                }
                f9_was_down = f9_down;

                if f10_down && !f10_was_down {
                    let _ = tx.send(HotkeyEvent::HighlightMarker);
                }
                f10_was_down = f10_down;
            }
        }
    }

    pub fn poll_events(&self) -> Vec<HotkeyEvent> {
        let mut events = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            events.push(ev);
        }
        events
    }
}
