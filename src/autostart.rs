#![allow(unsafe_code)]

#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ,
};

const RUN_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0";
const APP_VALUE_NAME: &str = "Translation WordSwipe\0";

/// 检查当前是否已启用 Windows 开机自启动
pub fn is_autostart_enabled() -> bool {
    #[cfg(target_os = "windows")]
    unsafe {
        let subkey: Vec<u16> = RUN_SUBKEY.encode_utf16().collect();
        let value_name: Vec<u16> = APP_VALUE_NAME.encode_utf16().collect();

        let mut hkey = std::mem::zeroed();
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return false;
        }

        let mut rtype = 0u32;
        let mut buffer = [0u16; 512];
        let mut buf_size = (buffer.len() * 2) as u32;

        let res = RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            std::ptr::null_mut(),
            &mut rtype,
            buffer.as_mut_ptr() as *mut u8,
            &mut buf_size,
        );

        RegCloseKey(hkey);
        res == 0 && rtype == REG_SZ && buf_size > 2
    }

    #[cfg(not(target_os = "windows"))]
    false
}

/// 设置 Windows 开机自启动状态（写入或删除注册表 HKEY_CURRENT_USER\...\Run 项）
pub fn set_autostart_enabled(enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    unsafe {
        let subkey: Vec<u16> = RUN_SUBKEY.encode_utf16().collect();
        let value_name: Vec<u16> = APP_VALUE_NAME.encode_utf16().collect();

        let mut hkey = std::mem::zeroed();
        let open_res = RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_WRITE, &mut hkey);
        if open_res != 0 {
            return Err(format!("打开注册表 Run 项失败，错误码: {}", open_res));
        }

        if enabled {
            let exe_path = match std::env::current_exe() {
                Ok(p) => p,
                Err(e) => {
                    RegCloseKey(hkey);
                    return Err(format!("获取当前程序执行路径失败: {}", e));
                }
            };

            // 使用带引号的路径并附带 --silent 参数，确保开机在托盘后台静默运行
            let cmd_str = format!("\"{}\" --silent\0", exe_path.to_string_lossy());
            let val_utf16: Vec<u16> = cmd_str.encode_utf16().collect();
            let data_bytes_len = (val_utf16.len() * 2) as u32;

            let set_res = RegSetValueExW(
                hkey,
                value_name.as_ptr(),
                0,
                REG_SZ,
                val_utf16.as_ptr() as *const u8,
                data_bytes_len,
            );

            RegCloseKey(hkey);

            if set_res != 0 {
                return Err(format!("写入注册表自启动项失败，错误码: {}", set_res));
            }
            Ok(())
        } else {
            // 删除已存在的自启配置
            let del_res = RegDeleteValueW(hkey, value_name.as_ptr());
            RegCloseKey(hkey);

            // 0 为成功，2 为 ERROR_FILE_NOT_FOUND (已不存在)，均视为成功
            if del_res == 0 || del_res == 2 {
                Ok(())
            } else {
                Err(format!("删除注册表自启动项失败，错误码: {}", del_res))
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = enabled;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autostart_query() {
        // 查询当前机器上的自启配置，确保不抛 panic
        let is_enabled = is_autostart_enabled();
        println!("Current autostart state: {}", is_enabled);
    }
}
