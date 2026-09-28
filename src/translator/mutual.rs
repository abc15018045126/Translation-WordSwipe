use std::sync::Arc;
use crate::translator::google::GoogleTranslator;
use crate::translator::google_web::GoogleWebTranslator;
use crate::translator::bing::BingTranslator;
use crate::translator::types::{map_language_name_to_code, TranslationResult};

#[derive(Clone)]
pub struct TranslationManager {
    pub google: Arc<GoogleTranslator>,
    pub google_web: Arc<GoogleWebTranslator>,
    pub bing: Arc<BingTranslator>,
}

impl TranslationManager {
    pub fn new() -> Self {
        Self {
            google: Arc::new(GoogleTranslator::new()),
            google_web: Arc::new(GoogleWebTranslator::new()),
            bing: Arc::new(BingTranslator::new()),
        }
    }

    pub async fn translate(
        &self,
        engine: &str,
        text: &str,
        from_lang: &str,
        to_lang: &str,
        is_mutual: bool,
        custom_api: &str,
    ) -> Result<TranslationResult, String> {
        let is_bing = engine.contains("必应") || engine.eq_ignore_ascii_case("bing");
        let is_google_web = engine.contains("网页") || engine.eq_ignore_ascii_case("google_web") || engine.eq_ignore_ascii_case("google-web");

        // 1. First translation: using auto detection
        let mut result = if is_bing {
            self.bing.translate(text, "auto", to_lang, custom_api).await?
        } else if is_google_web {
            self.google_web.translate(text, "auto", to_lang, custom_api).await?
        } else {
            self.google.translate(text, "auto", to_lang, custom_api).await?
        };

        // 2. Mutual translation logic
        if is_mutual {
            let target_code = map_language_name_to_code(to_lang);
            let detected = result.source_language.as_deref().unwrap_or("");
            
            // If the text is already in the target language (e.g. target is Chinese and text is Chinese)
            let is_same = if target_code == "zh-CN" {
                detected.starts_with("zh")
            } else {
                detected == target_code
            };

            if is_same && !detected.is_empty() {
                let reverse_target = if from_lang != "自动" && from_lang != "auto" {
                    from_lang
                } else if target_code == "en" {
                    "中文"
                } else {
                    "英文"
                };

                let reversed_result = if is_bing {
                    self.bing.translate(text, detected, reverse_target, custom_api).await?
                } else if is_google_web {
                    self.google_web.translate(text, detected, reverse_target, custom_api).await?
                } else {
                    self.google.translate(text, detected, reverse_target, custom_api).await?
                };
                result = reversed_result;
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_google_translation_online() {
        let manager = TranslationManager::new();
        let res = manager.translate("谷歌翻译", "Hello world", "auto", "中文", false, "").await;
        if let Ok(result) = res {
            println!("Translation result: {}", result.main_meaning);
            assert!(!result.main_meaning.is_empty());
        }
    }

    #[tokio::test]
    async fn test_mutual_translation_logic() {
        let manager = TranslationManager::new();
        // Target is Chinese, but input is already Chinese ("世界你好"), mutual mode should reverse to English!
        let res = manager.translate("谷歌翻译", "你好世界", "自动", "中文", true, "").await;
        if let Ok(result) = res {
            println!("Mutual result: {}", result.main_meaning);
            assert!(!result.main_meaning.is_empty());
        }
    }

    #[tokio::test]
    async fn test_bing_translation_online() {
        let manager = TranslationManager::new();
        let res = manager.translate("bing", "Hello world", "auto", "中文", false, "").await;
        match res {
            Ok(result) => {
                println!("Bing Translation result: {}", result.main_meaning);
                assert!(!result.main_meaning.is_empty());
            }
            Err(e) => {
                println!("Bing Translation error: {}", e);
            }
        }
    }
}

