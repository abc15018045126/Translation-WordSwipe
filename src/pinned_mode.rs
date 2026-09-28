#![allow(unsafe_code)]

use dioxus::desktop::DesktopContext;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Threading::AttachThreadInput;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId,
    SetForegroundWindow, SetWindowPos, ShowWindow,
    SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_SHOW,
};

/// 切换固定置顶状态
pub fn set_pinned(win: &DesktopContext, pinned: bool) {
    win.set_always_on_top(pinned);
    unsafe {
        let hwnd = crate::hook::find_app_window();
        if !hwnd.is_null() {
            let insert_after = if pinned {
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

/// 固定模式下的划词响应逻辑：
/// 1. 最小化到任务栏或已关闭：100% 保持在后台，绝不弹窗、不恢复最小化，静默在后台更新翻译
/// 2. 在前台开启中（哪怕点击其他软件沉底了）：立即跳起来置顶到最前面！
pub fn on_selection(win: &DesktopContext) {
    // 检查是否在后台（最小化或隐藏关闭）
    let is_minimized = win.is_minimized() || crate::hook::is_window_minimized();
    let is_closed = !win.is_visible();

    if is_minimized || is_closed {
        // 完全停留在后台，什么都不做，静默更新翻译
        return;
    }

    // 在前台开启中被遮挡：立即跳起来置顶到最前面
    win.set_visible(true);
    win.set_focus();

    unsafe {
        let hwnd = crate::hook::find_app_window();
        if !hwnd.is_null() {
            ShowWindow(hwnd, SW_SHOW);

            // 突破前台锁定跳起来
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
                AttachThreadInput(cur_thread, fg_thread, 0);
            } else {
                SetForegroundWindow(hwnd);
                BringWindowToTop(hwnd);
            }

            // 保持强置顶 (HWND_TOPMOST)
            SetWindowPos(
                hwnd,
                -1isize as HWND, // HWND_TOPMOST
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            );
        }
    }
}
