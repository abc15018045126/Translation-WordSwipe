#![allow(unsafe_code)] // Required for Windows Win32 low-level mouse hook, GetClipboardSequenceNumber, and keybd_event FFI

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP, VK_CONTROL};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, EnumWindows,
    GA_ROOT, GetAncestor, GetMessageW, GetParent, GetWindowLongW, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    SetWindowPos, SetWindowsHookExW, ShowWindow, UnhookWindowsHookEx,
    WindowFromPoint, GWL_STYLE, HHOOK, MSG, MSLLHOOKSTRUCT, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    SW_RESTORE, SW_SHOWNOACTIVATE, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP, WS_MINIMIZE,
};

static IS_ENABLED: AtomicBool = AtomicBool::new(true);

#[derive(Clone, Copy)]
struct MouseState {
    is_down: bool,
    down_point: POINT,
    down_wnd: isize,
    down_top_wnd: isize,
    down_top_rect: RECT,
}

static STATE: Mutex<MouseState> = Mutex::new(MouseState {
    is_down: false,
    down_point: POINT { x: 0, y: 0 },
    down_wnd: 0,
    down_top_wnd: 0,
    down_top_rect: RECT { left: 0, top: 0, right: 0, bottom: 0 },
});

static LAST_SELECTED_TEXT: Mutex<String> = Mutex::new(String::new());
static HOOK_HANDLE: AtomicIsize = AtomicIsize::new(0);
static SENDER: OnceLock<UnboundedSender<(String, i32, i32)>> = OnceLock::new();

const DRAG_THRESHOLD: i32 = 15; // Exact threshold from original C# program: prevents click triggers

static UI_PID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

pub fn set_ui_pid(pid: u32) {
    UI_PID.store(pid, Ordering::SeqCst);
}

pub unsafe fn find_app_window() -> HWND {
    let daemon_pid = GetCurrentProcessId();
    let ui_pid = UI_PID.load(Ordering::Relaxed);
    struct EnumData {
        daemon_pid: u32,
        ui_pid: u32,
        hwnd: HWND,
        fallback_hwnd: HWND,
    }
    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> windows_sys::Win32::Foundation::BOOL {
        let data = &mut *(lparam as *mut EnumData);
        let mut wnd_pid = 0;
        GetWindowThreadProcessId(hwnd, &mut wnd_pid);
        let is_target_pid = wnd_pid == data.daemon_pid || (data.ui_pid != 0 && wnd_pid == data.ui_pid);
        if is_target_pid {
            let mut title_buf = [0u16; 256];
            let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 256);
            if len > 0 {
                let title = String::from_utf16_lossy(&title_buf[..len as usize]);
                if title.contains("Translation WordSwipe") || title.contains("智能划词翻译") {
                    data.hwnd = hwnd;
                    return 0; // 精确匹配到标题
                }
            }
            if data.fallback_hwnd.is_null() && GetParent(hwnd).is_null() {
                data.fallback_hwnd = hwnd;
            }
        }
        1
    }
    let mut data = EnumData {
        daemon_pid,
        ui_pid,
        hwnd: std::ptr::null_mut(),
        fallback_hwnd: std::ptr::null_mut(),
    };
    EnumWindows(Some(enum_proc), &mut data as *mut _ as LPARAM);
    if !data.hwnd.is_null() {
        data.hwnd
    } else {
        data.fallback_hwnd
    }
}

#[allow(dead_code)]
pub fn is_window_minimized() -> bool {
    unsafe {
        let hwnd = find_app_window();
        if !hwnd.is_null() {
            if IsIconic(hwnd) != 0 {
                return true;
            }
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            if (style & WS_MINIMIZE) != 0 {
                return true;
            }
            false
        } else {
            false
        }
    }
}

#[allow(dead_code)]
pub fn set_window_topmost(topmost: bool) {
    unsafe {
        let hwnd = find_app_window();
        if !hwnd.is_null() {
            let insert_after = if topmost {
                -1isize as HWND // HWND_TOPMOST
            } else {
                -2isize as HWND // HWND_NOTOPMOST
            };
            SetWindowPos(
                hwnd,
                insert_after,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            );
        }
    }
}

#[allow(dead_code)]
pub fn bring_window_to_front(topmost: bool) {
    unsafe {
        let hwnd = find_app_window();
        if !hwnd.is_null() {
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_RESTORE);
            } else {
                ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }

            let insert_after = if topmost {
                -1isize as HWND // HWND_TOPMOST
            } else {
                -2isize as HWND // HWND_NOTOPMOST
            };
            SetWindowPos(
                hwnd,
                insert_after,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }
}

pub struct SelectionHook;

impl SelectionHook {
    pub fn set_enabled(enabled: bool) {
        IS_ENABLED.store(enabled, Ordering::SeqCst);
    }

    #[allow(dead_code)]
    pub fn is_enabled() -> bool {
        IS_ENABLED.load(Ordering::SeqCst)
    }

    pub fn start(sender: UnboundedSender<(String, i32, i32)>) {
        let _ = SENDER.set(sender);

        thread::spawn(|| {
            unsafe {
                let hinstance = GetModuleHandleW(std::ptr::null());
                let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), hinstance, 0);
                if hook.is_null() {
                    eprintln!("Failed to install mouse hook");
                    return;
                }

                HOOK_HANDLE.store(hook as isize, Ordering::SeqCst);

                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    DispatchMessageW(&msg);
                }

                let prev_hook = HOOK_HANDLE.swap(0, Ordering::SeqCst) as HHOOK;
                if !prev_hook.is_null() {
                    UnhookWindowsHookEx(prev_hook);
                }
            }
        });
    }
}

unsafe fn is_our_process(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == GetCurrentProcessId() {
        return true;
    }
    let ui_pid = UI_PID.load(Ordering::Relaxed);
    if ui_pid != 0 && pid == ui_pid {
        return true;
    }
    let mut title_buf = [0u16; 64];
    let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 64);
    if len > 0 {
        let title = String::from_utf16_lossy(&title_buf[..len as usize]);
        if title.contains("Translation WordSwipe") || title.contains("智能划词翻译") {
            return true;
        }
    }
    false
}

unsafe extern "system" fn mouse_hook_proc(code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let hook_val = HOOK_HANDLE.load(Ordering::Relaxed) as HHOOK;

    if code >= 0 && IS_ENABLED.load(Ordering::Relaxed) && l_param != 0 {
        let msg = w_param as u32;
        let hook_struct = *(l_param as *const MSLLHOOKSTRUCT);
        let pt = hook_struct.pt;
        let wnd_under_cursor = WindowFromPoint(pt);

        // 获取光标所在顶层宿主窗口
        let cursor_root = if !wnd_under_cursor.is_null() {
            let root = GetAncestor(wnd_under_cursor, GA_ROOT);
            if !root.is_null() { root } else { wnd_under_cursor }
        } else {
            std::ptr::null_mut()
        };

        // 1. 如果光标所在的窗口或其宿主顶层窗口属于我们自身程序，完全忽略钩子（避免在自身界面选词时误触发）
        if is_our_process(wnd_under_cursor) || is_our_process(cursor_root) {
            if let Ok(mut state) = STATE.lock() {
                state.is_down = false;
            }
            return CallNextHookEx(hook_val, code, w_param, l_param);
        }

        if msg == WM_LBUTTONDOWN {
            // 记录按下时的鼠标坐标、按下处顶层窗口句柄以及该窗口的位置（用于检测窗口移动）
            let mut top_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            if !cursor_root.is_null() {
                GetWindowRect(cursor_root, &mut top_rect);
            }

            if let Ok(mut state) = STATE.lock() {
                state.is_down = true;
                state.down_point = pt;
                state.down_wnd = wnd_under_cursor as isize;
                state.down_top_wnd = cursor_root as isize;
                state.down_top_rect = top_rect;
            }
        } else if msg == WM_LBUTTONUP {
            let old_state = if let Ok(mut state) = STATE.lock() {
                let s = *state;
                state.is_down = false;
                state.down_point = POINT { x: 0, y: 0 };
                state.down_wnd = 0;
                state.down_top_wnd = 0;
                state.down_top_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                s
            } else {
                return CallNextHookEx(hook_val, code, w_param, l_param);
            };

            // 1. 必须此前按下了鼠标
            if !old_state.is_down {
                return CallNextHookEx(hook_val, code, w_param, l_param);
            }

            // 2. 检测是否属于屏幕 OCR 工具窗口（如 PowerToys 文本提取器、截图识图工具等）
            let target_root = old_state.down_top_wnd as HWND;
            let is_ocr = crate::ocr::is_ocr_window(target_root)
                || crate::ocr::is_ocr_window(wnd_under_cursor)
                || crate::ocr::is_ocr_window(cursor_root);

            // 若属于 OCR 工具且用户在全局设置中选择了【普通 (OCR不触发)】，直接跳过不触发！
            if is_ocr && !crate::ocr::is_ocr_trigger_enabled() {
                return CallNextHookEx(hook_val, code, w_param, l_param);
            }

            // 3. 【核心过滤】检测按下时的目标顶层窗口是否被拖动移位了
            // （普通窗口被拖动移位时跳过，避免将移动窗口误判为划词；OCR 遮罩层通常固定全屏不移动）
            let down_rect = old_state.down_top_rect;
            let has_down_rect = down_rect.left != 0 || down_rect.top != 0 || down_rect.right != 0 || down_rect.bottom != 0;
            if !is_ocr && !target_root.is_null() && has_down_rect {
                let mut rect_now = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                if GetWindowRect(target_root, &mut rect_now) != 0
                    && (rect_now.left != down_rect.left || rect_now.top != down_rect.top)
                {
                    return CallNextHookEx(hook_val, code, w_param, l_param); // 窗口移动了，跳过！
                }
            }

            // 4. 拖拽位移阈值过滤：必须有实质性拖动（>15px），防止普通点击误触发
            let dx = (pt.x - old_state.down_point.x).abs();
            let dy = (pt.y - old_state.down_point.y).abs();
            if dx < DRAG_THRESHOLD && dy < DRAG_THRESHOLD {
                return CallNextHookEx(hook_val, code, w_param, l_param);
            }

            let click_x = pt.x;
            let click_y = pt.y;
            thread::spawn(move || {
                capture_selection_via_clipboard(click_x, click_y, is_ocr);
            });
        }
    }

    CallNextHookEx(hook_val, code, w_param, l_param)
}

fn get_clipboard_text_with_retry(max_retries: usize, delay_ms: u64) -> Option<String> {
    for i in 0..max_retries {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            if let Ok(text) = cb.get_text() {
                return Some(text);
            }
        }
        if i + 1 < max_retries {
            thread::sleep(Duration::from_millis(delay_ms));
        }
    }
    None
}

fn set_clipboard_text_with_retry(text: &str, max_retries: usize, delay_ms: u64) {
    for i in 0..max_retries {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            if cb.set_text(text.to_string()).is_ok() {
                return;
            }
        }
        if i + 1 < max_retries {
            thread::sleep(Duration::from_millis(delay_ms));
        }
    }
}

fn capture_selection_via_clipboard(cursor_x: i32, cursor_y: i32, is_ocr: bool) {
    // 微小延时，确保宿主应用完成 MouseUp 消息分发和选区划定
    thread::sleep(Duration::from_millis(35));

    // 1. Record Win32 clipboard sequence number and previous content before Ctrl+C
    let seq_before = unsafe { GetClipboardSequenceNumber() };
    let text_before = get_clipboard_text_with_retry(3, 20);

    // 2. 对于普通划词合成 Ctrl+C；对 OCR 工具（如 PowerToys 文本提取器），自身完成识别后会直接写剪贴板
    if !is_ocr {
        unsafe {
            keybd_event(VK_CONTROL as u8, 0, 0, 0);
            keybd_event(0x43, 0, 0, 0); // 'C'
            keybd_event(0x43, 0, KEYEVENTF_KEYUP, 0);
            keybd_event(VK_CONTROL as u8, 0, KEYEVENTF_KEYUP, 0);
        }
    }

    // 3. 等待剪贴板序列号变化：
    // - 普通划词：宿主应用响应 Ctrl+C 大约 40~120ms
    // - OCR 截图识图：OCR 引擎完成屏幕像素识别并写入剪贴板通常需要 100~450ms
    let max_wait_ms = if is_ocr { 450 } else { 120 };
    let step_ms = 25;
    let mut waited = 0;
    let mut seq_after = seq_before;

    while waited < max_wait_ms {
        thread::sleep(Duration::from_millis(step_ms));
        waited += step_ms;
        let s = unsafe { GetClipboardSequenceNumber() };
        if s != seq_before {
            seq_after = s;
            break;
        }
    }

    // 4. 如果剪贴板序列号未改变，说明没有产生新的复制或 OCR 文本，退出
    if seq_after == seq_before {
        return;
    }

    // 5. 读取最新剪贴板文本
    let text_after = get_clipboard_text_with_retry(5, 30);
    let selected = match text_after {
        Some(t) => t,
        None => return,
    };

    let trimmed = selected.trim();
    if trimmed.is_empty() {
        return;
    }

    // 6. 记录最后一次选中的文本
    if let Ok(mut last) = LAST_SELECTED_TEXT.lock() {
        *last = trimmed.to_string();
    }

    // 7. 剪贴板保护与恢复：
    // - 普通划词：还原用户原有的剪贴板内容，避免划词破坏剪贴板
    // - OCR 划选：保留用户 OCR 提取到的新内容（用户使用 OCR 就是为了复制提取的文字）
    if !is_ocr {
        if let Some(prev) = text_before {
            thread::sleep(Duration::from_millis(40));
            set_clipboard_text_with_retry(&prev, 3, 20);
        }
    }

    // 8. 派发划词翻译事件至 UI 界面展示
    if let Some(sender) = SENDER.get() {
        let _ = sender.send((trimmed.to_string(), cursor_x, cursor_y));
    }
}
