#![allow(unsafe_code)]

use muda::{Menu, MenuItem, PredefinedMenuItem};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct TrayManager {
    _tray_icon: TrayIcon,
    pub show_item_id: muda::MenuId,
    pub toggle_item_id: muda::MenuId,
    pub exit_item_id: muda::MenuId,
    pub toggle_item: MenuItem,
    is_hook_enabled: Arc<AtomicBool>,
}

static TRAY_UPDATE_TX: OnceLock<mpsc::UnboundedSender<bool>> = OnceLock::new();
static TRAY_UPDATE_RX: OnceLock<Mutex<mpsc::UnboundedReceiver<bool>>> = OnceLock::new();

static MENU_EVENT_RX: OnceLock<Mutex<mpsc::UnboundedReceiver<muda::MenuId>>> = OnceLock::new();
static TRAY_EVENT_RX: OnceLock<Mutex<mpsc::UnboundedReceiver<tray_icon::TrayIconEvent>>> = OnceLock::new();

pub fn init_tray_channel() {
    let (tx, rx) = mpsc::unbounded_channel::<bool>();
    let _ = TRAY_UPDATE_TX.set(tx);
    let _ = TRAY_UPDATE_RX.set(Mutex::new(rx));

    let (menu_tx, menu_rx) = mpsc::unbounded_channel::<muda::MenuId>();
    muda::MenuEvent::set_event_handler(Some(move |event: muda::MenuEvent| {
        let _ = menu_tx.send(event.id);
    }));
    let _ = MENU_EVENT_RX.set(Mutex::new(menu_rx));

    let (tray_tx, tray_rx) = mpsc::unbounded_channel::<tray_icon::TrayIconEvent>();
    tray_icon::TrayIconEvent::set_event_handler(Some(move |event: tray_icon::TrayIconEvent| {
        let _ = tray_tx.send(event);
    }));
    let _ = TRAY_EVENT_RX.set(Mutex::new(tray_rx));
}

#[allow(dead_code)]
pub fn notify_tray_translation_state(enabled: bool) {
    if let Some(tx) = TRAY_UPDATE_TX.get() {
        let _ = tx.send(enabled);
    }
}

#[allow(dead_code)]
pub fn poll_tray_update() -> Option<bool> {
    if let Some(rx_lock) = TRAY_UPDATE_RX.get() {
        if let Ok(mut rx) = rx_lock.try_lock() {
            return rx.try_recv().ok();
        }
    }
    None
}

pub fn poll_menu_event() -> Option<muda::MenuId> {
    if let Some(rx_lock) = MENU_EVENT_RX.get() {
        if let Ok(mut rx) = rx_lock.try_lock() {
            return rx.try_recv().ok();
        }
    }
    None
}

pub fn poll_tray_event() -> Option<tray_icon::TrayIconEvent> {
    if let Some(rx_lock) = TRAY_EVENT_RX.get() {
        if let Ok(mut rx) = rx_lock.try_lock() {
            return rx.try_recv().ok();
        }
    }
    None
}

/// Win32 底层确保唤醒并显示窗口
pub fn restore_and_show_window() {
    unsafe {
        let hwnd = crate::hook::find_app_window();
        if !hwnd.is_null() {
            if windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(hwnd) != 0 {
                windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                    hwnd,
                    windows_sys::Win32::UI::WindowsAndMessaging::SW_RESTORE,
                );
            } else {
                windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                    hwnd,
                    windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOW,
                );
            }
            windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
            windows_sys::Win32::UI::WindowsAndMessaging::BringWindowToTop(hwnd);
        }
    }
}

impl TrayManager {
    pub fn new(is_hook_enabled: Arc<AtomicBool>) -> Result<Self, Box<dyn std::error::Error>> {
        let icon = load_app_icon()?;
        let menu = Menu::new();

        let show_item = MenuItem::new("还原窗口 / 显示", true, None);
        let is_enabled = is_hook_enabled.load(Ordering::SeqCst);
        let toggle_text = if is_enabled {
            "关闭划词翻译"
        } else {
            "开启划词翻译"
        };
        let toggle_item = MenuItem::new(toggle_text, true, None);
        let separator = PredefinedMenuItem::separator();
        let exit_item = MenuItem::new("退出程序", true, None);

        let show_id = show_item.id().clone();
        let toggle_id = toggle_item.id().clone();
        let exit_id = exit_item.id().clone();

        menu.append(&show_item)?;
        menu.append(&toggle_item)?;
        menu.append(&separator)?;
        menu.append(&exit_item)?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Translation WordSwipe")
            .with_icon(icon)
            .build()?;

        Ok(Self {
            _tray_icon: tray_icon,
            show_item_id: show_id,
            toggle_item_id: toggle_id,
            exit_item_id: exit_id,
            toggle_item,
            is_hook_enabled,
        })
    }

    pub fn update_toggle_text(&self, enabled: bool) {
        self.is_hook_enabled.store(enabled, Ordering::SeqCst);
        let text = if enabled { "关闭划词翻译" } else { "开启划词翻译" };
        self.toggle_item.set_text(text);
    }
}

fn load_app_icon() -> Result<Icon, Box<dyn std::error::Error>> {
    let ico_bytes = include_bytes!("../Translation WordSwipe.ico");
    if let Ok(img) = image::load_from_memory(ico_bytes) {
        let rgba = img.into_rgba8();
        let (width, height) = rgba.dimensions();
        return Ok(Icon::from_rgba(rgba.into_raw(), width, height)?);
    }

    // Fallback: 16x16 blue icon
    let width = 16;
    let height = 16;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        rgba.extend_from_slice(&[25, 103, 210, 255]);
    }
    Ok(Icon::from_rgba(rgba, width, height)?)
}

#[cfg(test)]
mod tests {
    use image::GenericImageView;

    #[test]
    fn test_load_app_icon() {
        let ico_bytes = include_bytes!("../Translation WordSwipe.ico");
        let img = image::load_from_memory(ico_bytes).expect("Failed to load icon");
        println!("App icon dimensions: {:?}", img.dimensions());
    }
}
