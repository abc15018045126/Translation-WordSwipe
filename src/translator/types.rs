use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TranslationResult {
    pub original_text: String,
    pub main_meaning: String,
    pub s_pronunciation: Option<String>,
    pub t_pronunciation: Option<String>,
    pub source_language: Option<String>,
    pub target_language: Option<String>,
    pub detailed_meanings: Vec<DetailedMeaning>,
    pub definitions: Vec<Definition>,
    pub examples: Vec<Example>,
    #[serde(default)]
    pub method_used: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetailedMeaning {
    pub pos: Option<String>,
    pub meaning: Option<String>,
    pub synonyms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Definition {
    pub pos: Option<String>,
    pub meaning: Option<String>,
    pub synonyms: Vec<String>,
    pub example: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Example {
    pub source: Option<String>,
    pub target: Option<String>,
}

pub fn map_language_name_to_code(name: &str) -> &'static str {
    match name {
        "自动" | "auto" => "auto",
        "中文" | "zh" | "zh-CN" => "zh-CN",
        "英文" | "en" => "en",
        "日文" | "日语" | "ja" => "ja",
        "韩文" | "韩语" | "ko" => "ko",
        "法语" | "fr" => "fr",
        "俄语" | "ru" => "ru",
        "西班牙语" | "es" => "es",
        "德语" | "de" => "de",
        "阿拉伯语" | "ar" => "ar",
        _ => "auto",
    }
}

#[allow(dead_code)]
pub fn map_language_code_to_name(code: &str) -> &'static str {
    match code {
        "auto" => "自动",
        "zh" | "zh-CN" | "zh-Hans" => "中文",
        "en" => "英文",
        "ja" => "日文",
        "ko" => "韩文",
        "fr" => "法语",
        "ru" => "俄语",
        "es" => "西班牙语",
        "de" => "德语",
        "ar" => "阿拉伯语",
        _ => "未知",
    }
}
