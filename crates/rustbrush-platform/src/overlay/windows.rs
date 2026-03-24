//! Windows overlay implementation using a layered, click-through window.
//!
//! Creates a fullscreen transparent window with WS_EX_TRANSPARENT (click-through)
//! and WS_EX_LAYERED with a color key for transparency. Draws a green rectangle
//! border to show the current selection.

use std::sync::{Arc, Mutex};
use std::thread;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, InvalidateRect, PAINTSTRUCT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetSystemMetrics, PostMessageW, RegisterClassExW, SetLayeredWindowAttributes, ShowWindow,
    CS_HREDRAW, CS_VREDRAW, LWA_ALPHA, LWA_COLORKEY, MSG, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_SHOWNOACTIVATE, WNDCLASSEXW,
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

use super::SelectionOverlay;

/// Magenta color key — pixels of this color become transparent.
const TRANSPARENT_COLOR: u32 = 0x00FF00FF; // COLORREF is 0x00BBGGRR, so this is RGB(FF, 00, FF)

/// Green border color for the selection rectangle.
const BORDER_COLOR: u32 = 0x0000FF00; // RGB(0, 255, 0) in COLORREF

/// Black outline color for contrast.
const OUTLINE_COLOR: u32 = 0x00000000; // RGB(0, 0, 0) in COLORREF

/// Semi-transparent fill color for the selection interior (dark green tint).
const FILL_COLOR: u32 = 0x00004000; // RGB(0, 64, 0) in COLORREF — visible through alpha

/// Overall window opacity (0-255). Applied to all non-colorkey pixels.
/// Lower = more see-through. 160 ≈ 63% opacity.
const WINDOW_ALPHA: u8 = 160;

/// Border thickness in pixels.
const BORDER_WIDTH: i32 = 3;

/// Outline thickness (drawn outside the green border).
const OUTLINE_WIDTH: i32 = 1;

/// Custom message ID to trigger repaint.
const WM_UPDATE_RECT: u32 = 0x0400 + 1; // WM_USER + 1

/// Shared state between the update caller and the window paint handler.
struct OverlayState {
    /// Current selection rectangle (x1, y1, x2, y2) in screen coords.
    rect: Option<(i32, i32, i32, i32)>,
}

/// Thread-safe pointer to shared state, stored as window user data.
static mut GLOBAL_STATE: Option<Arc<Mutex<OverlayState>>> = None;

pub struct WindowsOverlay {
    hwnd: HWND,
    thread_handle: Option<thread::JoinHandle<()>>,
    state: Arc<Mutex<OverlayState>>,
}

// HWND is a raw pointer but we only send it across threads for PostMessage,
// which is safe to call from any thread.
unsafe impl Send for WindowsOverlay {}

impl WindowsOverlay {
    pub fn new() -> Option<Self> {
        let state = Arc::new(Mutex::new(OverlayState { rect: None }));
        let state_clone = state.clone();

        // Channel to receive the HWND from the window thread.
        let (tx, rx) = std::sync::mpsc::channel();

        let handle = thread::spawn(move || {
            unsafe {
                GLOBAL_STATE = Some(state_clone);
                let hwnd = create_overlay_window();
                let _ = tx.send(hwnd);
                if hwnd != 0 {
                    run_message_loop();
                }
                GLOBAL_STATE = None;
            }
        });

        match rx.recv() {
            Ok(hwnd) if hwnd != 0 => Some(WindowsOverlay {
                hwnd,
                thread_handle: Some(handle),
                state,
            }),
            _ => None,
        }
    }
}

impl SelectionOverlay for WindowsOverlay {
    fn update(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        if let Ok(mut s) = self.state.lock() {
            s.rect = Some((x1, y1, x2, y2));
        }
        // Trigger a repaint on the window thread.
        unsafe {
            PostMessageW(self.hwnd, WM_UPDATE_RECT, 0, 0);
        }
    }

    fn destroy(&mut self) {
        unsafe {
            // WM_CLOSE will cause the message loop to exit.
            PostMessageW(
                self.hwnd,
                windows_sys::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                0,
                0,
            );
        }
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for WindowsOverlay {
    fn drop(&mut self) {
        if self.thread_handle.is_some() {
            self.destroy();
        }
    }
}

/// Create the overlay window and show it.
unsafe fn create_overlay_window() -> HWND {
    let hinstance = GetModuleHandleW(std::ptr::null());

    // Register window class.
    let class_name: Vec<u16> = "RustBrushOverlay\0".encode_utf16().collect();
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: 0,
        hCursor: 0,
        hbrBackground: 0,
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: 0,
    };
    RegisterClassExW(&wc);

    // Get virtual screen dimensions (covers all monitors).
    let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
    let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
    let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
    let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);

    let ex_style = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW;

    let hwnd = CreateWindowExW(
        ex_style,
        class_name.as_ptr(),
        std::ptr::null(), // no title
        WS_POPUP,
        vx,
        vy,
        vw,
        vh,
        0,
        0,
        hinstance,
        std::ptr::null(),
    );

    if hwnd == 0 {
        return 0;
    }

    // Color key: magenta pixels become fully transparent.
    // Alpha: all other pixels rendered at WINDOW_ALPHA opacity so the game shows through.
    SetLayeredWindowAttributes(hwnd, TRANSPARENT_COLOR, WINDOW_ALPHA, LWA_COLORKEY | LWA_ALPHA);

    // Show without activating (don't steal focus from game).
    ShowWindow(hwnd, SW_SHOWNOACTIVATE);

    hwnd
}

/// Window message loop — runs until WM_CLOSE / WM_DESTROY.
unsafe fn run_message_loop() {
    let mut msg: MSG = std::mem::zeroed();
    while GetMessageW(&mut msg, 0, 0, 0) > 0 {
        DispatchMessageW(&msg);
    }
}

/// Window procedure handling paint and close messages.
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_UPDATE_RECT => {
            // Invalidate the entire window to trigger WM_PAINT.
            InvalidateRect(hwnd, std::ptr::null(), 1);
            0
        }
        windows_sys::Win32::UI::WindowsAndMessaging::WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);

            // Fill entire window with the transparent color key.
            let transparent_brush = CreateSolidBrush(TRANSPARENT_COLOR);
            FillRect(hdc, &ps.rcPaint, transparent_brush);
            DeleteObject(transparent_brush);

            // Draw the selection rectangle if we have one.
            if let Some(ref state) = GLOBAL_STATE {
                if let Ok(s) = state.lock() {
                    if let Some((x1, y1, x2, y2)) = s.rect {
                        // Convert to screen-relative coords (window is at virtual screen origin).
                        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
                        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
                        let left = x1.min(x2) - vx;
                        let top = y1.min(y2) - vy;
                        let right = x1.max(x2) - vx;
                        let bottom = y1.max(y2) - vy;

                        let total = BORDER_WIDTH + OUTLINE_WIDTH;

                        // Fill the selection interior with a tinted color (semi-transparent
                        // via the window's LWA_ALPHA so the game is visible underneath).
                        let fill_brush = CreateSolidBrush(FILL_COLOR);
                        let fill_rect = windows_sys::Win32::Foundation::RECT {
                            left, top, right, bottom,
                        };
                        FillRect(hdc, &fill_rect, fill_brush);
                        DeleteObject(fill_brush);

                        // Draw black outline (slightly larger rectangle).
                        let outline_brush = CreateSolidBrush(OUTLINE_COLOR);
                        draw_rect_border(hdc, outline_brush, left - OUTLINE_WIDTH, top - OUTLINE_WIDTH, right + OUTLINE_WIDTH, bottom + OUTLINE_WIDTH, total);
                        DeleteObject(outline_brush);

                        // Draw green border inside the outline.
                        let border_brush = CreateSolidBrush(BORDER_COLOR);
                        draw_rect_border(hdc, border_brush, left, top, right, bottom, BORDER_WIDTH);
                        DeleteObject(border_brush);
                    }
                }
            }

            EndPaint(hwnd, &ps);
            0
        }
        windows_sys::Win32::UI::WindowsAndMessaging::WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        windows_sys::Win32::UI::WindowsAndMessaging::WM_DESTROY => {
            windows_sys::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Draw a rectangle border (4 filled rectangles forming the frame).
unsafe fn draw_rect_border(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    brush: windows_sys::Win32::Graphics::Gdi::HBRUSH,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    width: i32,
) {
    use windows_sys::Win32::Foundation::RECT;

    // Top edge
    let r = RECT { left, top, right, bottom: top + width };
    FillRect(hdc, &r, brush);

    // Bottom edge
    let r = RECT { left, top: bottom - width, right, bottom };
    FillRect(hdc, &r, brush);

    // Left edge (between top and bottom bars)
    let r = RECT { left, top: top + width, right: left + width, bottom: bottom - width };
    FillRect(hdc, &r, brush);

    // Right edge (between top and bottom bars)
    let r = RECT { left: right - width, top: top + width, right, bottom: bottom - width };
    FillRect(hdc, &r, brush);
}
