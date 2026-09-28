#![allow(unsafe_code)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PeekMessageW, PM_REMOVE, TranslateMessage, WM_QUIT,
};

use crate::config::AppSettings;
use crate::hook::SelectionHook;
use crate::ipc::{start_daemon_server, IpcMessage};
use crate::tray::{restore_and_show_window, TrayManager};

fn is_running(child_opt: &mut Option<std::process::Child>) -> bool {
    if let Some(child) = child_opt.as_mut() {
        match child.try_wait() {
            Ok(None) => true,
            _ => {
                *child_opt = None;
                crate::hook::set_ui_pid(0);
                false
            }
        }
    } else {
        false
    }
}

fn launch_ui_process(initial_text: Option<String>, child_opt: &mut Option<std::process::Child>) {
    if is_running(child_opt) {
        restore_and_show_window();
        return;
    }

    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--ui");
    if let Some(text) = initial_text {
        cmd.arg("--select");
        cmd.arg(text);
    }

    match cmd.spawn() {
        Ok(child) => {
            crate::hook::set_ui_pid(child.id());
            *child_opt = Some(child);
        }
        Err(e) => {
            eprintln!("Failed to spawn UI process: {:?}", e);
        }
    }
}

fn show_or_launch(to_ui_tx: &tokio::sync::mpsc::UnboundedSender<IpcMessage>, child_opt: &mut Option<std::process::Child>) {
    if is_running(child_opt) {
        restore_and_show_window();
        let _ = to_ui_tx.send(IpcMessage::ShowWindow);
    } else {
        launch_ui_process(None, child_opt);
    }
}

fn kill_ui_process(child_opt: &mut Option<std::process::Child>) {
    if let Some(mut child) = child_opt.take() {
        let _ = child.kill();
        let _ = child.wait();
        crate::hook::set_ui_pid(0);
    }
}

pub fn daemon_main() {
    let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to build Tokio runtime: {:?}", e);
            return;
        }
    };

    rt.block_on(async {
        let settings = AppSettings::load();
        let is_hook_enabled = Arc::new(AtomicBool::new(settings.is_translation_enabled));
        SelectionHook::set_enabled(settings.is_translation_enabled);
        crate::ocr::set_ocr_trigger_enabled(settings.ocr_trigger_enabled);

        crate::tray::init_tray_channel();

        let tray_manager = match TrayManager::new(is_hook_enabled.clone()) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Failed to create tray manager: {:?}", e);
                return;
            }
        };

        let (to_ui_tx, mut from_ui_rx) = start_daemon_server();

        let (sel_tx, mut sel_rx) = tokio::sync::mpsc::unbounded_channel::<(String, i32, i32)>();
        SelectionHook::start(sel_tx);

        let mut ui_child: Option<std::process::Child> = None;
        let mut current_settings = settings;

        let has_silent = std::env::args().any(|arg| arg == "--silent" || arg == "--minimized");
        if !has_silent {
            launch_ui_process(None, &mut ui_child);
        }

        let mut timer = tokio::time::interval(std::time::Duration::from_millis(30));

        loop {
            tokio::select! {
                _ = timer.tick() => {
                    unsafe {
                        let mut msg: MSG = std::mem::zeroed();
                        while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) > 0 {
                            if msg.message == WM_QUIT {
                                kill_ui_process(&mut ui_child);
                                return;
                            }
                            TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }
                    }

                    while let Some(tray_event) = crate::tray::poll_tray_event() {
                        if let tray_icon::TrayIconEvent::Click { button, .. } = tray_event {
                            if button == tray_icon::MouseButton::Left {
                                show_or_launch(&to_ui_tx, &mut ui_child);
                            }
                        }
                    }

                    while let Some(menu_id) = crate::tray::poll_menu_event() {
                        if menu_id == tray_manager.show_item_id {
                            show_or_launch(&to_ui_tx, &mut ui_child);
                        } else if menu_id == tray_manager.toggle_item_id {
                            let next = !is_hook_enabled.load(Ordering::SeqCst);
                            SelectionHook::set_enabled(next);
                            is_hook_enabled.store(next, Ordering::SeqCst);
                            tray_manager.update_toggle_text(next);
                            current_settings.is_translation_enabled = next;
                            current_settings.save();
                            let _ = to_ui_tx.send(IpcMessage::ToggleTranslation { enabled: next });
                        } else if menu_id == tray_manager.exit_item_id {
                            kill_ui_process(&mut ui_child);
                            std::process::exit(0);
                        }
                    }
                }

                Some((text, cx, cy)) = sel_rx.recv() => {
                    let current_settings = AppSettings::load();
                    if !current_settings.is_translation_enabled {
                        continue;
                    }

                    if is_running(&mut ui_child) {
                        let _ = to_ui_tx.send(IpcMessage::Selection { text, x: cx, y: cy });
                        if !current_settings.is_pinned {
                            // 不固定模式：保证窗口取消最小化并弹出到最前台激活
                            restore_and_show_window();
                        }
                        // 固定模式：UI 正在运行时，严禁在 Daemon 中调用 restore_and_show_window()！
                        // 窗口状态完全由 UI 进程内的 pinned_mode::on_selection 裁决：
                        // 若窗口最小化在任务栏，100% 保持在任务栏静默，绝不弹窗打扰、不恢复最小化；
                        // 仅当窗口原本在前台可见时，才在前台跳起置顶展示。
                    } else {
                        // 窗口关闭（退到托盘）：无论是固定模式还是不固定模式，
                        // 只要窗口已关闭，划词均立即拉起 UI 进程，带词弹出展示！
                        launch_ui_process(Some(text), &mut ui_child);
                    }
                }

                Some(msg) = from_ui_rx.recv() => {
                    match msg {
                        IpcMessage::ShowWindow => {
                            show_or_launch(&to_ui_tx, &mut ui_child);
                        }
                        IpcMessage::ToggleTranslation { enabled } => {
                            SelectionHook::set_enabled(enabled);
                            is_hook_enabled.store(enabled, Ordering::SeqCst);
                            tray_manager.update_toggle_text(enabled);
                            current_settings.is_translation_enabled = enabled;
                            current_settings.save();
                        }
                        IpcMessage::SetOcrTrigger { enabled } => {
                            crate::ocr::set_ocr_trigger_enabled(enabled);
                            current_settings.ocr_trigger_enabled = enabled;
                            current_settings.save();
                        }
                        _ => {}
                    }
                }
            }
        }
    });
}
