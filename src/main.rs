#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unsafe_code)]

mod config;
pub mod autostart;
mod daemon;
mod hook;
mod ipc;
pub mod ocr;
mod pinned_mode;
pub mod source;
mod translator;
mod tray;
mod ui;
mod unpinned_mode;

use dioxus::desktop::{Config, WindowBuilder, WindowCloseBehaviour};
use dioxus::prelude::*;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, OnceLock};
use tokio::sync::{mpsc, Mutex};
use crate::config::AppSettings;
use crate::translator::TranslationManager;
use crate::ui::App;

pub static TRANSLATION_MANAGER: OnceLock<Arc<TranslationManager>> = OnceLock::new();
pub static SELECTION_RX: OnceLock<Mutex<mpsc::UnboundedReceiver<(String, i32, i32)>>> = OnceLock::new();
pub static IPC_CMD_RX: OnceLock<Mutex<mpsc::UnboundedReceiver<crate::ipc::IpcMessage>>> = OnceLock::new();
pub static HOOK_ENABLED: OnceLock<Arc<AtomicBool>> = OnceLock::new();

#[cfg(target_os = "windows")]
#[allow(clippy::manual_c_str_literals)]
fn enable_dpi_awareness() {
    unsafe {
        use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress, LoadLibraryA};

        // 1. 尝试 Windows 10 (1607+) / Windows 11 推荐的 Per-Monitor V2 模式（高分屏自适应、右键菜单原生清晰度渲染）
        let user32 = GetModuleHandleA(b"user32.dll\0".as_ptr());
        if !user32.is_null() {
            let set_context_fn = GetProcAddress(user32, b"SetProcessDpiAwarenessContext\0".as_ptr());
            if let Some(set_context) = set_context_fn {
                type SetContextFn = unsafe extern "system" fn(isize) -> windows_sys::Win32::Foundation::BOOL;
                let set_context: SetContextFn = std::mem::transmute(set_context);
                // -4: DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2
                // -3: DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE
                if set_context(-4) != 0 || set_context(-3) != 0 {
                    return;
                }
            }
        }

        // 2. 尝试 Windows 8.1+ 的 SetProcessDpiAwareness (Shcore.dll)
        let shcore = LoadLibraryA(b"shcore.dll\0".as_ptr());
        if !shcore.is_null() {
            let set_awareness_fn = GetProcAddress(shcore, b"SetProcessDpiAwareness\0".as_ptr());
            if let Some(set_awareness) = set_awareness_fn {
                type SetAwarenessFn = unsafe extern "system" fn(i32) -> i32;
                let set_awareness: SetAwarenessFn = std::mem::transmute(set_awareness);
                // 2: PROCESS_PER_MONITOR_DPI_AWARE
                if set_awareness(2) == 0 {
                    return;
                }
            }
        }

        // 3. 最终回退 Windows Vista+ 的 SetProcessDPIAware (user32.dll)
        if !user32.is_null() {
            let set_dpi_aware_fn = GetProcAddress(user32, b"SetProcessDPIAware\0".as_ptr());
            if let Some(set_dpi_aware) = set_dpi_aware_fn {
                type SetDpiAwareFn = unsafe extern "system" fn() -> windows_sys::Win32::Foundation::BOOL;
                let set_dpi_aware: SetDpiAwareFn = std::mem::transmute(set_dpi_aware);
                set_dpi_aware();
            }
        }
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    enable_dpi_awareness();

    std::panic::set_hook(Box::new(|info| {
        let msg = format!(
            "PANIC: {}\nLocation: {:?}\nBacktrace:\n{:?}",
            info,
            info.location(),
            std::backtrace::Backtrace::capture()
        );
        eprintln!("{}", msg);
        let _ = std::fs::write("panic.log", msg);
    }));

    let is_ui = std::env::args().any(|arg| arg == "--ui");

    if !is_ui {
        // 如果后台托盘 Daemon 已在运行，则请求唤醒已有窗口并立即退出当前进程
        if crate::ipc::try_send_command(&crate::ipc::IpcMessage::ShowWindow) {
            return;
        }

        // 启动纯 Rust 独立托盘小后台（零 WebView2 资源消耗）
        crate::daemon::daemon_main();
        return;
    }

    // UI 进程启动（仅在此进程中载入 WebView2，窗口关闭时本进程完全杀死 WebView2）
    ui_main();
}

fn ui_main() {
    let settings = AppSettings::load();

    let (sel_tx, sel_rx) = mpsc::unbounded_channel::<(String, i32, i32)>();
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<crate::ipc::IpcMessage>();

    let is_hook_enabled = Arc::new(AtomicBool::new(settings.is_translation_enabled));
    let translation_manager = Arc::new(TranslationManager::new());

    let _ = TRANSLATION_MANAGER.set(translation_manager);
    let _ = SELECTION_RX.set(Mutex::new(sel_rx));
    let _ = IPC_CMD_RX.set(Mutex::new(cmd_rx));
    let _ = HOOK_ENABLED.set(is_hook_enabled);

    // 解析初始命令行选词参数（例如不固定模式下划词拉起 UI）
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "--select" {
            if let Some(text) = args.next() {
                let _ = sel_tx.send((text, 0, 0));
            }
        }
    }

    // 在后台独立线程中连接 Daemon 的 IPC Named Pipe
    let sel_tx_clone = sel_tx.clone();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(r) => r,
            Err(_) => return,
        };

        rt.block_on(async move {
            // 尝试重试连接，以防 Daemon 管道稍微晚一点就绪
            let mut client_opt = None;
            for _ in 0..15 {
                if let Ok(pair) = crate::ipc::connect_ui_client().await {
                    client_opt = Some(pair);
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            }

            if let Some((_out_tx, mut in_rx)) = client_opt {
                while let Some(msg) = in_rx.recv().await {
                    match msg {
                        crate::ipc::IpcMessage::Selection { text, x, y } => {
                            let _ = sel_tx_clone.send((text, x, y));
                        }
                        cmd @ (crate::ipc::IpcMessage::ShowWindow | crate::ipc::IpcMessage::ToggleTranslation { .. }) => {
                            let _ = cmd_tx.send(cmd);
                        }
                        _ => {}
                    }
                }
            }
        });
    });

    let window_icon = load_window_icon();
    let mut window_builder = WindowBuilder::new()
        .with_title("Translation WordSwipe")
        .with_inner_size(dioxus::desktop::LogicalSize::new(840.0, 600.0))
        .with_resizable(true)
        .with_always_on_top(settings.is_pinned);

    if let Some(icon) = window_icon {
        window_builder = window_builder.with_window_icon(Some(icon));
    }

    let desktop_config = Config::new()
        .with_window(window_builder)
        // 关键改动：显式开启右键上下文菜单（允许在 release 模式下右键复制、选择等操作）
        .with_disable_context_menu(false)
        // 关键改动：关闭窗口时彻底退出 UI 进程，完全杀死 WebView2，释放全部 Chromium 内存
        .with_close_behaviour(WindowCloseBehaviour::WindowCloses);

    LaunchBuilder::desktop()
        .with_cfg(desktop_config)
        .launch(App);

    // 窗口关闭退出后，彻底退出 UI 进程，内核自动回收所有 WebView2 子进程
    std::process::exit(0);
}

fn load_window_icon() -> Option<dioxus::desktop::tao::window::Icon> {
    let ico_bytes = include_bytes!("../Translation WordSwipe.ico");
    if let Ok(img) = image::load_from_memory(ico_bytes) {
        let rgba = img.into_rgba8();
        let (width, height) = rgba.dimensions();
        return dioxus::desktop::tao::window::Icon::from_rgba(rgba.into_raw(), width, height).ok();
    }
    None
}
