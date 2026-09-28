#![allow(unsafe_code)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::Foundation::{CloseHandle, HWND};
use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetWindowTextW, GetWindowThreadProcessId};

static OCR_TRIGGER_ENABLED: AtomicBool = AtomicBool::new(true);

/// 设置是否允许 OCR 工具（如 PowerToys 文本提取器、截图识图等）触发划词翻译
pub fn set_ocr_trigger_enabled(enabled: bool) {
    OCR_TRIGGER_ENABLED.store(enabled, Ordering::SeqCst);
}

/// 检查当前是否允许 OCR 工具触发划词翻译（默认开启）
pub fn is_ocr_trigger_enabled() -> bool {
    OCR_TRIGGER_ENABLED.load(Ordering::SeqCst)
}

/// 判断给定的窗口是否属于已知的屏幕 OCR / 截图识图工具
pub fn is_ocr_window(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }

    // 1. 检查窗口类名 (Class Name)
    let mut class_buf = [0u16; 256];
    let class_len = unsafe { GetClassNameW(hwnd, class_buf.as_mut_ptr(), 256) };
    if class_len > 0 {
        let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]).to_lowercase();
        if is_known_ocr_keyword(&class_name) {
            return true;
        }
    }

    // 2. 检查窗口标题 (Window Title)
    let mut title_buf = [0u16; 256];
    let title_len = unsafe { GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 256) };
    if title_len > 0 {
        let title = String::from_utf16_lossy(&title_buf[..title_len as usize]).to_lowercase();
        if is_known_ocr_keyword(&title) {
            return true;
        }
    }

    // 3. 检查窗口宿主进程可执行文件名 (Process Image Name)
    if let Some(proc_name) = get_process_name_by_hwnd(hwnd) {
        if is_known_ocr_process(&proc_name) {
            return true;
        }
    }

    false
}

/// 根据 HWND 获取进程名称（例如 powertoys.textextractor.exe）
pub fn get_process_name_by_hwnd(hwnd: HWND) -> Option<String> {
    if hwnd.is_null() {
        return None;
    }

    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut pid);
    }
    if pid == 0 {
        return None;
    }

    unsafe {
        let h_proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h_proc.is_null() {
            return None;
        }

        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let success = QueryFullProcessImageNameW(h_proc, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(h_proc);

        if success != 0 && size > 0 {
            let full_path = String::from_utf16_lossy(&buf[..size as usize]);
            let file_name = std::path::Path::new(&full_path)
                .file_name()
                .and_then(|f| f.to_str())
                .map(|s| s.to_string())
                .unwrap_or(full_path);
            return Some(file_name);
        }
    }

    None
}

/// 匹配已知的 OCR / 截屏文字提取工具进程名
pub fn is_known_ocr_process(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("powertoys.textextractor")
        || lower.contains("textextractor")
        || lower.contains("snippingtool")
        || lower.contains("screenclippinghost")
        || lower.contains("screensketch")
        || lower.contains("pixpin")
        || lower.contains("snipaste")
        || lower.contains("pearocr")
        || lower.contains("pandaocr")
        || lower.contains("sharex")
        || lower.contains("esearch")
}

/// 匹配窗口类名或标题中的 OCR 特征关键字
pub fn is_known_ocr_keyword(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("textextractor")
        || lower.contains("文本提取器")
        || lower.contains("screenclippinghost")
        || lower.contains("screensketch")
        || lower.contains("snippingtool")
        || lower.contains("截屏工具")
        || lower.contains("截图识图")
        || lower.contains("屏幕识图")
        || lower.contains("pixpin")
        || lower.contains("wechat_recognize_text")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_trigger_switch() {
        set_ocr_trigger_enabled(true);
        assert!(is_ocr_trigger_enabled());
        set_ocr_trigger_enabled(false);
        assert!(!is_ocr_trigger_enabled());
        set_ocr_trigger_enabled(true);
    }

    #[test]
    fn test_known_ocr_process_matching() {
        assert!(is_known_ocr_process("PowerToys.TextExtractor.exe"));
        assert!(is_known_ocr_process("C:\\Program Files\\PowerToys\\PowerToys.TextExtractor.exe"));
        assert!(is_known_ocr_process("ScreenClippingHost.exe"));
        assert!(is_known_ocr_process("SnippingTool.exe"));
        assert!(is_known_ocr_process("PixPin.exe"));
        assert!(!is_known_ocr_process("chrome.exe"));
        assert!(!is_known_ocr_process("notepad.exe"));
    }

    #[test]
    fn test_known_ocr_keyword_matching() {
        assert!(is_known_ocr_keyword("PowerToys.TextExtractor"));
        assert!(is_known_ocr_keyword("Windows 文本提取器"));
        assert!(is_known_ocr_keyword("ScreenClippingHost"));
        assert!(!is_known_ocr_keyword("Google Chrome"));
        assert!(!is_known_ocr_keyword("Notepad"));
    }
}
