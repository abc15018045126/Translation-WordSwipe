#![allow(unsafe_code)]

use dioxus::desktop::DesktopContext;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Threading::AttachThreadInput;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId,
    IsIconic, SetForegroundWindow, SetWindowPos, ShowWindow,
    SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_RESTORE, SW_SHOW,
};

/// 不固定模式（原始经典弹窗模式）
/// 划词时无条件唤醒还原窗口、显示窗口、强行弹出到最前台并获取焦点展示翻译结果
pub fn on_selection(win: &DesktopContext) {
    win.set_minimized(false);
    win.set_visible(true);
    win.set_focus();

    unsafe {
        let hwnd = crate::hook::find_app_window();
        if !hwnd.is_null() {
            // 1. 还原窗口状态
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_RESTORE);
            } else {
                ShowWindow(hwnd, SW_SHOW);
            }

            // 2. 突破 Windows 前台锁定，将窗口激活到最前台
            let fg_wnd = GetForegroundWindow();
            let cur_thread = windows_sys::Win32::System::Threading::GetCurrentThreadId();
            let fg_thread = if !fg_wnd.is_null() {
                GetWindowThreadProcessId(fg_wnd, std::ptr::null_mut())
            } else {
                0
            };

            if fg_thread != 0 && fg_thread != cur_thread {
                AttachThreadInput(cur_thread, fg_thread, 1);
                SetForegroundWindow(hwnd);
                BringWindowToTop(hwnd);
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                }
                AttachThreadInput(cur_thread, fg_thread, 0);
            } else {
                SetForegroundWindow(hwnd);
                BringWindowToTop(hwnd);
            }

            // 3. 不固定模式下保持常规层级（HWND_NOTOPMOST）
            SetWindowPos(
                hwnd,
                -2isize as HWND, // HWND_NOTOPMOST
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            );
        }
    }
}
