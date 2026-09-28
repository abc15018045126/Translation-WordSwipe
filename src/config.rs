use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::source::{default_sources, TranslationSource};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AppSettings {
    #[serde(default = "default_x")]
    pub popup_x: i32,
    #[serde(default = "default_y")]
    pub popup_y: i32,
    #[serde(default = "default_width")]
    pub popup_width: u32,
    #[serde(default = "default_height")]
    pub popup_height: u32,
    #[serde(default = "default_engine")]
    pub engine: String,
    #[serde(default = "default_from_lang")]
    pub from_lang: String,
    #[serde(default = "default_to_lang")]
    pub to_lang: String,
    #[serde(default)]
    pub is_mutual: bool,
    #[serde(default)]
    pub custom_api_url: String,
    #[serde(default = "default_true")]
    pub is_translation_enabled: bool,
    #[serde(default = "default_true")]
    pub remember_layout: bool,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: f64,
    #[serde(default)]
    pub sidebar_collapsed: bool,
    #[serde(default = "default_proxy_mode")]
    pub proxy_mode: String,
    #[serde(default)]
    pub proxy_url: String,
    #[serde(default = "default_google_web_mode")]
    pub google_web_mode: String,
    #[serde(default = "default_true")]
    pub ocr_trigger_enabled: bool,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default = "default_sources")]
    pub sources: Vec<TranslationSource>,
}

fn default_proxy_mode() -> String { "system".to_string() }
fn default_google_web_mode() -> String { "race".to_string() }

fn default_sidebar_width() -> f64 { 200.0 }
fn default_x() -> i32 { 100 }
fn default_y() -> i32 { 100 }
fn default_width() -> u32 { 520 }
fn default_height() -> u32 { 340 }
fn default_engine() -> String { "google".to_string() }
fn default_from_lang() -> String { "auto".to_string() }
fn default_to_lang() -> String { "zh".to_string() }
fn default_true() -> bool { true }

pub fn normalize_engine(e: &str) -> String {
    if e.contains("必应") || e.eq_ignore_ascii_case("bing") {
        "bing".to_string()
    } else {
        "google".to_string()
    }
}

pub fn normalize_lang(l: &str, is_target: bool) -> String {
    match l {
        "auto" | "自动" => if is_target { "zh".to_string() } else { "auto".to_string() },
        "zh" | "zh-CN" | "中文" => "zh".to_string(),
        "en" | "英文" => "en".to_string(),
        "ja" | "日文" | "日语" => "ja".to_string(),
        "ko" | "韩文" | "韩语" => "ko".to_string(),
        "fr" | "法语" => "fr".to_string(),
        "ru" | "俄语" => "ru".to_string(),
        "es" | "西班牙语" => "es".to_string(),
        "de" | "德语" => "de".to_string(),
        "ar" | "阿拉伯语" => "ar".to_string(),
        other => if is_target { "zh".to_string() } else { other.to_string() },
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            popup_x: default_x(),
            popup_y: default_y(),
            popup_width: default_width(),
            popup_height: default_height(),
            engine: default_engine(),
            from_lang: default_from_lang(),
            to_lang: default_to_lang(),
            is_mutual: false,
            custom_api_url: String::new(),
            is_translation_enabled: true,
            remember_layout: true,
            is_pinned: false,
            sidebar_width: default_sidebar_width(),
            sidebar_collapsed: false,
            proxy_mode: default_proxy_mode(),
            proxy_url: String::new(),
            google_web_mode: default_google_web_mode(),
            ocr_trigger_enabled: true,
            auto_start: false,
            sources: default_sources(),
        }
    }
}

impl AppSettings {
    pub fn config_path() -> PathBuf {
        // 1. 优先使用 exe 所在目录下的 settings.json（确保便携与自启时路径正确）
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let exe_file = dir.join("settings.json");
                if exe_file.exists() {
                    return exe_file;
                }
            }
        }

        // 2. 其次检查当前工作目录下是否存在 settings.json（兼容开发调试环境）
        let cwd_file = PathBuf::from("settings.json");
        if cwd_file.exists() {
            return cwd_file;
        }

        // 3. 若均不存在，默认创建目标统一设为 exe 所在目录
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                return dir.join("settings.json");
            }
        }

        cwd_file
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(mut settings) = serde_json::from_str::<AppSettings>(&content) {
                    settings.engine = normalize_engine(&settings.engine);
                    settings.from_lang = normalize_lang(&settings.from_lang, false);
                    settings.to_lang = normalize_lang(&settings.to_lang, true);
                    crate::translator::client::set_proxy_config(&settings.proxy_mode, &settings.proxy_url);
                    crate::translator::google_web::set_google_web_mode(&settings.google_web_mode);
                    crate::ocr::set_ocr_trigger_enabled(settings.ocr_trigger_enabled);
                    settings.auto_start = crate::autostart::is_autostart_enabled();
                    return settings;
                }
            }
        }

        // 当 settings.json 不存在时，自动在目标目录（exe 所在目录）创建并写入默认配置文件
        let default_settings = Self::default();
        default_settings.save();
        default_settings
    }

    pub fn save(&self) {
        crate::translator::client::set_proxy_config(&self.proxy_mode, &self.proxy_url);
        crate::translator::google_web::set_google_web_mode(&self.google_web_mode);
        crate::ocr::set_ocr_trigger_enabled(self.ocr_trigger_enabled);
        let _ = crate::autostart::set_autostart_enabled(self.auto_start);
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_serialization() {
        let settings = AppSettings::default();
        let json = serde_json::to_string_pretty(&settings).unwrap();
        assert!(json.contains("PopupX"));
        assert!(json.contains("google"));

        let deserialized: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.popup_width, 520);
        assert_eq!(deserialized.engine, "google");
        assert_eq!(deserialized.from_lang, "auto");
        assert_eq!(deserialized.to_lang, "zh");
        assert_eq!(deserialized.google_web_mode, "race");
        assert!(deserialized.ocr_trigger_enabled);
        assert!(deserialized.is_translation_enabled);
    }

    #[test]
    fn test_settings_chinese_normalization() {
        let old_json = r#"{
            "Engine": "谷歌翻译",
            "FromLang": "英文",
            "ToLang": "中文",
            "IsMutual": true
        }"#;
        let mut settings: AppSettings = serde_json::from_str(old_json).unwrap();
        settings.engine = normalize_engine(&settings.engine);
        settings.from_lang = normalize_lang(&settings.from_lang, false);
        settings.to_lang = normalize_lang(&settings.to_lang, true);

        assert_eq!(settings.engine, "google");
        assert_eq!(settings.from_lang, "en");
        assert_eq!(settings.to_lang, "zh");
        assert!(settings.is_mutual);
    }
}
