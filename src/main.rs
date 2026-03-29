//! server-checker — A Win32 GUI application that fetches the HTTP `Server`
//! header from any URL the user enters.
//!
//! Stack:
//!   • `windows` crate   – native Win32 GUI (window, textbox, button, message box)
//!   • `ureq`             – HTTP client
//!   • `rustls`           – TLS back-end (pulled in by ureq's `tls` feature)
//!   • MinGW              – GNU cross-linker for Windows targets
//!
//! Build:
//!   rustup target add x86_64-pc-windows-gnu
//!   cargo build --target x86_64-pc-windows-gnu --release

// Suppress the console window on Windows (works with MSVC and MinGW).
#![windows_subsystem = "windows"]

use std::mem::zeroed;

use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::Controls::*,
        UI::WindowsAndMessaging::*,
    },
};

// ── Control IDs ─────────────────────────────────────────────────────────────
const IDC_URL_EDIT: i32 = 101; // Text-box for the URL
const IDC_SUBMIT_BTN: i32 = 102; // "Check Server" button

// ── Window procedure ────────────────────────────────────────────────────────
extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            // ── WM_CREATE: build the child controls ─────────────────────────
            WM_CREATE => {
                let hinstance = GetModuleHandleW(None).unwrap_or_default();

                // --- URL label ---
                let _ = CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("STATIC"),
                    w!("Enter URL:"),
                    WS_CHILD | WS_VISIBLE,
                    20,   // x
                    20,   // y
                    360,  // width
                    20,   // height
                    hwnd,
                    HMENU(0 as _),
                    hinstance,
                    None,
                );

                // --- URL text-box (Edit control) ---
                let _ = CreateWindowExW(
                    WS_EX_CLIENTEDGE,
                    w!("EDIT"),
                    w!("https://"),
                    WS_CHILD
                        | WS_VISIBLE
                        | WS_TABSTOP
                        | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                    20,   // x
                    48,   // y
                    360,  // width
                    28,   // height
                    hwnd,
                    HMENU(IDC_URL_EDIT as _),
                    hinstance,
                    None,
                );

                // --- Submit button (default push button so Enter triggers it) ---
                let _ = CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("BUTTON"),
                    w!("Check Server"),
                    WS_CHILD
                        | WS_VISIBLE
                        | WS_TABSTOP
                        | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
                    130,  // x
                    90,   // y
                    140,  // width
                    32,   // height
                    hwnd,
                    HMENU(IDC_SUBMIT_BTN as _),
                    hinstance,
                    None,
                );

                // Tell the dialog manager that IDC_SUBMIT_BTN is the default
                // button, so pressing Enter anywhere in the window activates it.
                SendMessageW(hwnd, DM_SETDEFID, WPARAM(IDC_SUBMIT_BTN as _), LPARAM(0));

                // Enable visual-styles for a modern look (Common Controls v6).
                let mut icc: INITCOMMONCONTROLSEX = zeroed();
                icc.dwSize = std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32;
                icc.dwICC = ICC_STANDARD_CLASSES;
                let _ = InitCommonControlsEx(&icc);

                LRESULT(0)
            }

            // ── WM_COMMAND: handle button click or Enter key ────────────
            WM_COMMAND => {
                let control_id = (wparam.0 & 0xFFFF) as i32;
                let notification = ((wparam.0 >> 16) & 0xFFFF) as u32;

                // Trigger on direct button click OR when IsDialogMessageW
                // sends IDOK because Enter was pressed with a default button.
                let clicked_submit =
                    control_id == IDC_SUBMIT_BTN && notification == BN_CLICKED as u32;
                let pressed_enter =
                    control_id == IDOK.0 as i32;

                if clicked_submit || pressed_enter {
                    // Read the URL from the text-box.
                    let h_edit = GetDlgItem(hwnd, IDC_URL_EDIT).ok().unwrap_or_default();
                    let mut buf = [0u16; 2048];
                    let len = GetWindowTextW(h_edit, &mut buf) as usize;
                    let url = String::from_utf16_lossy(&buf[..len]);

                    // Perform the HTTPS request and show the result.
                    let server = fetch_server_header(&url);
                    let message: Vec<u16> = server
                        .encode_utf16()
                        .chain(std::iter::once(0))
                        .collect();
                    let title = w!("Server Header");

                    MessageBoxW(
                        hwnd,
                        PCWSTR(message.as_ptr()),
                        title,
                        MB_OK | MB_ICONINFORMATION,
                    );
                }

                LRESULT(0)
            }

            // ── WM_CTLCOLORSTATIC: paint label background to match window ──
            WM_CTLCOLORSTATIC => {
                let hdc = HDC(wparam.0 as _);
                SetBkMode(hdc, TRANSPARENT);
                LRESULT(GetStockObject(NULL_BRUSH).0 as _)
            }

            // ── WM_DESTROY: quit the message loop ───────────────────────────
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }

            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

// ── HTTP(S) fetch using ureq + rustls ───────────────────────────────────────
/// Sends a HEAD request to `url` and returns the value of the `Server`
/// header, or a user-friendly error / "Unknown Server" string.
fn fetch_server_header(url: &str) -> String {
    // Ensure the URL has a scheme.
    let url = if !url.starts_with("http://") && !url.starts_with("https://") {
        format!("https://{url}")
    } else {
        url.to_string()
    };

    // ureq is compiled with the `tls` feature which uses rustls under
    // the hood — no OpenSSL required.
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(2))
        .timeout(std::time::Duration::from_secs(10))
        .build();

    match agent.head(&url).call() {
        Ok(response) => match response.header("server") {
            Some(value) => format!("Server: {value}"),
            None => "Unknown Server".to_string(),
        },
        Err(ureq::Error::Status(code, response)) => {
            // Even error responses can carry a Server header.
            match response.header("server") {
                Some(value) => format!("Server (HTTP {code}): {value}"),
                None => format!("HTTP {code} — Unknown Server"),
            }
        }
        Err(e) => format!("Request failed:\n{e}"),
    }
}

// ── Entry point ─────────────────────────────────────────────────────────────
fn main() -> Result<()> {
    unsafe {
        let hinstance = GetModuleHandleW(None)?;
        let class_name = w!("ServerCheckerClass");

        // Register the window class.
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as _),
            lpszClassName: class_name,
            ..zeroed()
        };
        RegisterClassExW(&wc);

        // Centre the window on the primary monitor.
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let win_w = 420;
        let win_h = 180;

        // Create the top-level window.
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("HTTPS Server Checker"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            (screen_w - win_w) / 2,
            (screen_h - win_h) / 2,
            win_w,
            win_h,
            None,
            None,
            hinstance,
            None,
        )?;

        let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
        let _ = UpdateWindow(hwnd);

        // Standard Win32 message loop.
        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            // Allow Tab key navigation between controls.
            if IsDialogMessageW(hwnd, &msg).as_bool() {
                continue;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    Ok(())
}
