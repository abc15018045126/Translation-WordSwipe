#![allow(unsafe_code)]

use std::sync::RwLock;
use std::time::Duration;
use reqwest::{Client, Proxy};

#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, KEY_READ, REG_DWORD, REG_SZ,
};

static PROXY_MODE: RwLock<String> = RwLock::new(String::new());
static PROXY_CUSTOM_URL: RwLock<String> = RwLock::new(String::new());
static CACHED_CLIENT: RwLock<Option<(String, String, u64, Client)>> = RwLock::new(None);

/// 全局同步当前代理配置
/// mode: "system" (系统代理) | "custom" (设置代码/设置代理) | "none" (不代理/直连)
/// custom_url: 当 mode 为 custom 时的自定义代理地址，如 "http://127.0.0.1:10808"
pub fn set_proxy_config(mode: &str, custom_url: &str) {
    let mode_str = mode.trim();
    let url_str = custom_url.trim();

    if let Ok(mut m) = PROXY_MODE.write() {
        *m = mode_str.to_string();
    }
    if let Ok(mut u) = PROXY_CUSTOM_URL.write() {
        *u = url_str.to_string();
    }
    // 清空客户端缓存以确保下次获取时重新生效
    if let Ok(mut c) = CACHED_CLIENT.write() {
        *c = None;
    }
}

pub fn get_proxy_config() -> (String, String) {
    let mode = PROXY_MODE.read().map(|m| m.clone()).unwrap_or_default();
    let url = PROXY_CUSTOM_URL.read().map(|u| u.clone()).unwrap_or_default();
    (
        if mode.is_empty() { "system".to_string() } else { mode },
        url,
    )
}

/// 自动检测操作系统代理配置（优先环境变量，Windows 下自动读取注册表 Internet Settings）
pub fn detect_system_proxy() -> Option<String> {
    // 1. 检查通用环境变量
    for key in &["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy", "HTTP_PROXY", "http_proxy"] {
        if let Ok(val) = std::env::var(key) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    // 2. Windows 下自动读取注册表系统代理（Clash / v2rayN / 系统设置 代理开关）
    #[cfg(target_os = "windows")]
    unsafe {
        let subkey: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings\0"
            .encode_utf16()
            .collect();
        let mut hkey = std::mem::zeroed();
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return None;
        }

        // 检查 ProxyEnable 开关 (DWORD: 1 为启用, 0 为关闭)
        let name_enable: Vec<u16> = "ProxyEnable\0".encode_utf16().collect();
        let mut proxy_enable: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        let mut rtype = 0u32;
        let res_enable = RegQueryValueExW(
            hkey,
            name_enable.as_ptr(),
            std::ptr::null_mut(),
            &mut rtype,
            &mut proxy_enable as *mut _ as *mut u8,
            &mut size,
        );

        if res_enable != 0 || rtype != REG_DWORD || proxy_enable == 0 {
            RegCloseKey(hkey);
            return None;
        }

        // 读取 ProxyServer 代理地址 (SZ 字符串，例如 127.0.0.1:10808)
        let name_server: Vec<u16> = "ProxyServer\0".encode_utf16().collect();
        let mut buffer = [0u16; 512];
        let mut buf_size = (buffer.len() * 2) as u32;
        let res_server = RegQueryValueExW(
            hkey,
            name_server.as_ptr(),
            std::ptr::null_mut(),
            &mut rtype,
            buffer.as_mut_ptr() as *mut u8,
            &mut buf_size,
        );
        RegCloseKey(hkey);

        if res_server == 0 && rtype == REG_SZ && buf_size > 2 {
            let len = (buf_size / 2) as usize;
            let slice = &buffer[..len];
            let clean_len = slice.iter().position(|&c| c == 0).unwrap_or(len);
            let raw_str = String::from_utf16_lossy(&slice[..clean_len]);
            let s = raw_str.trim();
            if !s.is_empty() {
                // 处理可能的多协议写法（如 http=127.0.0.1:10808;https=127.0.0.1:10808）
                let target = if let Some(pos) = s.find("http=") {
                    let sub = &s[pos + 5..];
                    sub.split(';').next().unwrap_or(sub).trim()
                } else if let Some(pos) = s.find("https=") {
                    let sub = &s[pos + 6..];
                    sub.split(';').next().unwrap_or(sub).trim()
                } else {
                    s.split(';').next().unwrap_or(s).trim()
                };

                let formatted = if target.starts_with("http://")
                    || target.starts_with("https://")
                    || target.starts_with("socks5://")
                {
                    target.to_string()
                } else {
                    format!("http://{}", target)
                };
                return Some(formatted);
            }
        }
    }

    None
}

/// 统一创建配置好代理模式（系统代理 / 设置代码 / 不代理）、超时和 User-Agent 的 HTTP 客户端
pub fn build_http_client(timeout_secs: u64) -> Client {
    let (mode, custom_url) = get_proxy_config();

    let mut builder = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36");

    match mode.as_str() {
        "none" | "direct" | "不代理" => {
            // 选项3：不代理 (直连)，显式禁用代理和环境变量
            builder = builder.no_proxy();
        }
        "custom" | "设置代码" | "设置代理" => {
            // 选项2：设置代码 / 自定义代理
            let target = custom_url.trim();
            if !target.is_empty() {
                let formatted = if target.starts_with("http://")
                    || target.starts_with("https://")
                    || target.starts_with("socks5://")
                {
                    target.to_string()
                } else {
                    format!("http://{}", target)
                };
                if let Ok(proxy) = Proxy::all(&formatted) {
                    builder = builder.proxy(proxy);
                }
            } else {
                builder = builder.no_proxy();
            }
        }
        _ => {
            // 选项1：系统代理 (自动检测)
            if let Some(proxy_url) = detect_system_proxy() {
                if let Ok(proxy) = Proxy::all(&proxy_url) {
                    builder = builder.proxy(proxy);
                }
            }
        }
    }

    builder.build().unwrap_or_default()
}

/// 获取带缓存的 HTTP 客户端（当代理模式或地址变更时自动重建）
pub fn get_http_client(timeout_secs: u64) -> Client {
    let (mode, custom_url) = get_proxy_config();

    if let Ok(guard) = CACHED_CLIENT.read() {
        if let Some((m, u, t, c)) = guard.as_ref() {
            if m == &mode && u == &custom_url && *t == timeout_secs {
                return c.clone();
            }
        }
    }

    let client = build_http_client(timeout_secs);
    if let Ok(mut guard) = CACHED_CLIENT.write() {
        *guard = Some((mode, custom_url, timeout_secs, client.clone()));
    }
    client
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_proxy() {
        let proxy = detect_system_proxy();
        println!("Detected proxy: {:?}", proxy);
    }

    #[test]
    fn test_proxy_options() {
        // 1. 系统代理
        set_proxy_config("system", "");
        let _ = get_http_client(5);

        // 2. 设置代码 (自定义代理)
        set_proxy_config("custom", "http://127.0.0.1:10808");
        let _ = get_http_client(5);

        // 3. 不代理
        set_proxy_config("none", "");
        let _ = get_http_client(5);
    }
}
