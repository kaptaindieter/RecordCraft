use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use egui::ColorImage;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaptureRect {
    pub x: i32,
    pub y: i32,
    pub src_w: i32,
    pub src_h: i32,
    pub dst_w: u32,
    pub dst_h: u32,
}

pub struct ScreenPreview {
    pub texture: Option<egui::TextureHandle>,
    shared_frame: Arc<Mutex<Option<ColorImage>>>,
    requested_rect: Arc<Mutex<CaptureRect>>,
    running: Arc<AtomicBool>,
    _worker: Option<JoinHandle<()>>,
}

impl ScreenPreview {
    pub fn new() -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let shared_frame = Arc::new(Mutex::new(None));
        let requested_rect = Arc::new(Mutex::new(CaptureRect {
            x: 0,
            y: 0,
            src_w: 3440,
            src_h: 1440,
            dst_w: 640,
            dst_h: 268,
        }));

        let r_flag = running.clone();
        let s_frame = shared_frame.clone();
        let r_rect = requested_rect.clone();

        // High-performance background capture worker thread (30 FPS)
        let worker = thread::spawn(move || {
            while r_flag.load(Ordering::Relaxed) {
                let rect = {
                    if let Ok(lock) = r_rect.lock() {
                        *lock
                    } else {
                        break;
                    }
                };

                if rect.src_w > 0 && rect.src_h > 0 && rect.dst_w > 0 && rect.dst_h > 0 {
                    if let Some(img) = capture_gdi_thumbnail(
                        rect.x,
                        rect.y,
                        rect.src_w,
                        rect.src_h,
                        rect.dst_w,
                        rect.dst_h,
                    ) {
                        if let Ok(mut lock) = s_frame.lock() {
                            *lock = Some(img);
                        }
                    }
                }

                thread::sleep(Duration::from_millis(33)); // ~30 FPS fluid preview
            }
        });

        Self {
            texture: None,
            shared_frame,
            requested_rect,
            running,
            _worker: Some(worker),
        }
    }

    /// Sets the source bounds and retrieves the latest frame smoothly on the UI thread.
    pub fn update(
        &mut self,
        ctx: &egui::Context,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) {
        // Compute destination preserving aspect ratio
        let max_w = 640u32;
        let aspect = if h > 0 { w as f32 / h as f32 } else { 16.0 / 9.0 };
        let dst_w = max_w;
        let dst_h = ((max_w as f32 / aspect).round() as u32).clamp(180, 420);

        if let Ok(mut lock) = self.requested_rect.lock() {
            if lock.x != x || lock.y != y || lock.src_w != w || lock.src_h != h || lock.dst_w != dst_w || lock.dst_h != dst_h {
                *lock = CaptureRect {
                    x,
                    y,
                    src_w: w,
                    src_h: h,
                    dst_w,
                    dst_h,
                };
            }
        }

        // Check if background thread produced a fresh frame
        if let Ok(mut lock) = self.shared_frame.try_lock() {
            if let Some(img) = lock.take() {
                if let Some(ref mut tex) = self.texture {
                    tex.set(img, egui::TextureOptions::LINEAR);
                } else {
                    self.texture = Some(ctx.load_texture(
                        "recordcraft_live_preview",
                        img,
                        egui::TextureOptions::LINEAR,
                    ));
                }
                ctx.request_repaint();
            }
        }
    }
}

impl Drop for ScreenPreview {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

pub fn capture_gdi_thumbnail(
    x: i32,
    y: i32,
    src_w: i32,
    src_h: i32,
    dst_w: u32,
    dst_h: u32,
) -> Option<ColorImage> {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::Graphics::Gdi::*;

        let hdc_screen = GetDC(std::ptr::null_mut());
        if hdc_screen.is_null() {
            return None;
        }

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        if hdc_mem.is_null() {
            ReleaseDC(std::ptr::null_mut(), hdc_screen);
            return None;
        }

        let hbm = CreateCompatibleBitmap(hdc_screen, dst_w as i32, dst_h as i32);
        if hbm.is_null() {
            DeleteDC(hdc_mem);
            ReleaseDC(std::ptr::null_mut(), hdc_screen);
            return None;
        }

        let old_bm = SelectObject(hdc_mem, hbm);
        SetStretchBltMode(hdc_mem, COLORONCOLOR as i32);
        StretchBlt(
            hdc_mem,
            0,
            0,
            dst_w as i32,
            dst_h as i32,
            hdc_screen,
            x,
            y,
            src_w,
            src_h,
            SRCCOPY,
        );

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: dst_w as i32,
                biHeight: -(dst_h as i32), // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: (dst_w * dst_h * 4) as u32,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };

        let mut bgra = vec![0u8; (dst_w * dst_h * 4) as usize];
        GetDIBits(
            hdc_mem,
            hbm,
            0,
            dst_h,
            bgra.as_mut_ptr() as *mut _,
            &mut bmi,
            DIB_RGB_COLORS,
        );

        SelectObject(hdc_mem, old_bm);
        DeleteObject(hbm);
        DeleteDC(hdc_mem);
        ReleaseDC(std::ptr::null_mut(), hdc_screen);

        let mut rgba = vec![255u8; (dst_w * dst_h * 4) as usize];
        for i in 0..(dst_w * dst_h) as usize {
            let b = bgra[i * 4];
            let g = bgra[i * 4 + 1];
            let r = bgra[i * 4 + 2];
            rgba[i * 4] = r;
            rgba[i * 4 + 1] = g;
            rgba[i * 4 + 2] = b;
            rgba[i * 4 + 3] = 255;
        }

        Some(ColorImage::from_rgba_unmultiplied(
            [dst_w as usize, dst_h as usize],
            &rgba,
        ))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (x, y, src_w, src_h, dst_w, dst_h);
        None
    }
}
