use std::sync::Mutex;
use std::time::Duration;
use serde_json::Value;
use crate::translator::types::{Definition, DetailedMeaning, TranslationResult};
use crate::translator::cache::SimpleCache;

struct BingTokens {
    ig: String,
    iid: String,
    key: String,
    token: String,
    host: String,
}

pub struct BingTranslator {
    cache: SimpleCache,
    tokens: Mutex<Option<BingTokens>>,
}

impl BingTranslator {
    pub fn new() -> Self {
        Self {
            cache: SimpleCache::new(300, Duration::from_secs(600)),
            tokens: Mutex::new(None),
        }
    }

    async fn get_or_update_tokens(&self) -> Result<BingTokens, String> {
        {
            if let Ok(guard) = self.tokens.lock() {
                if let Some(ref t) = *guard {
                    if !t.token.is_empty() && !t.key.is_empty() {
                        return Ok(BingTokens {
                            ig: t.ig.clone(),
                            iid: t.iid.clone(),
                            key: t.key.clone(),
                            token: t.token.clone(),
                            host: t.host.clone(),
                        });
                    }
                }
            }
        }

        let client = crate::translator::client::get_http_client(8);
        let resp = client.get("https://www.bing.com/translator")
            .send()
            .await
            .map_err(|e| format!("无法连接 Bing 翻译主页: {}", e))?;

        let final_url = resp.url().as_str().to_string();
        let host = if let Some(m) = regex::Regex::new(r"https://.*?\.bing\.com/").ok().and_then(|r| r.find(&final_url)) {
            m.as_str().to_string()
        } else {
            "https://www.bing.com/".to_string()
        };

        let content = resp.text().await.map_err(|e| format!("获取 Bing 页面内容失败: {}", e))?;

        let ig = regex::Regex::new(r#"IG:"([A-Za-z0-9]+)""#)
            .ok()
            .and_then(|r| r.captures(&content))
            .map(|c| c[1].to_string())
            .unwrap_or_default();

        let (key, token) = if let Some(c) = regex::Regex::new(r#"params_AbusePreventionHelper\s*=\s*\[([0-9]+),\s*"([^"]+)",[^\]]*\]"#)
            .ok()
            .and_then(|r| r.captures(&content)) {
            (c[1].to_string(), c[2].to_string())
        } else {
            (String::new(), String::new())
        };

        let iid = regex::Regex::new(r#"data-iid="([^"]+)""#)
            .ok()
            .and_then(|r| r.captures(&content))
            .map(|c| c[1].to_string())
            .unwrap_or_else(|| "translator.5028".to_string());

        let tokens = BingTokens { ig, iid, key, token, host };
        if let Ok(mut guard) = self.tokens.lock() {
            *guard = Some(BingTokens {
                ig: tokens.ig.clone(),
                iid: tokens.iid.clone(),
                key: tokens.key.clone(),
                token: tokens.token.clone(),
                host: tokens.host.clone(),
            });
        }

        Ok(tokens)
    }

    pub async fn translate(&self, text: &str, from: &str, to: &str, custom_api: &str) -> Result<TranslationResult, String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(TranslationResult::default());
        }

        let cache_key = format!("B|{}|{}|{}|{}", from, to, custom_api, trimmed);
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached);
        }

        let tokens = self.get_or_update_tokens().await?;
        let from_code = map_bing_lang(from);
        let to_code = map_bing_lang(to);

        let body_params = format!(
            "&fromLang={}&to={}&text={}&token={}&key={}",
            from_code,
            to_code,
            urlencoding::encode(trimmed),
            urlencoding::encode(&tokens.token),
            urlencoding::encode(&tokens.key)
        );

        let custom_url = custom_api.trim();
        let url = if !custom_url.is_empty() {
            if custom_url.contains("ttranslatev3") {
                if custom_url.contains('?') {
                    format!("{}&isVertical=1&IG={}&IID={}.1", custom_url, tokens.ig, tokens.iid)
                } else {
                    format!("{}?isVertical=1&IG={}&IID={}.1", custom_url, tokens.ig, tokens.iid)
                }
            } else {
                format!(
                    "{}{}ttranslatev3?isVertical=1&IG={}&IID={}.1",
                    custom_url,
                    if custom_url.ends_with('/') { "" } else { "/" },
                    tokens.ig,
                    tokens.iid
                )
            }
        } else {
            format!("{}ttranslatev3?isVertical=1&IG={}&IID={}.1", tokens.host, tokens.ig, tokens.iid)
        };

        let client = crate::translator::client::get_http_client(8);
        let resp = client.post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body_params)
            .send()
            .await
            .map_err(|e| format!("Bing 翻译请求失败: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Bing 返回状态码: {}", resp.status()));
        }

        let json_str = resp.text().await.map_err(|e| format!("读取 Bing 响应失败: {}", e))?;
        let json: Value = serde_json::from_str(&json_str).map_err(|e| format!("解析 Bing JSON 失败: {}", e))?;

        let mut result = TranslationResult {
            original_text: trimmed.to_string(),
            ..Default::default()
        };

        if let Some(arr) = json.as_array() {
            if let Some(first) = arr.first() {
                if let Some(det) = first.get("detectedLanguage").and_then(|v| v.get("language")).and_then(|v| v.as_str()) {
                    result.source_language = Some(if det.contains("zh-Han") { "zh-CN".to_string() } else { det.to_string() });
                }
                if let Some(translations) = first.get("translations").and_then(|v| v.as_array()) {
                    if let Some(first_trans) = translations.first() {
                        result.main_meaning = first_trans.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        result.t_pronunciation = first_trans.get("transliteration").and_then(|v| v.get("text")).and_then(|v| v.as_str()).map(|s| s.to_string());
                    }
                }
            }
        }

        // Try lookup for word definitions
        let lookup_params = format!(
            "&from={}&to={}&text={}&token={}&key={}",
            from_code,
            to_code,
            urlencoding::encode(trimmed),
            urlencoding::encode(&tokens.token),
            urlencoding::encode(&tokens.key)
        );
        let lookup_url = format!("{}tlookupv3?isVertical=1&IG={}&IID={}.1", tokens.host, tokens.ig, tokens.iid);
        if let Ok(lookup_resp) = client.post(&lookup_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(lookup_params)
            .send()
            .await {
            if lookup_resp.status().is_success() {
                if let Ok(lookup_text) = lookup_resp.text().await {
                    if let Ok(lookup_json) = serde_json::from_str::<Value>(&lookup_text) {
                        parse_bing_lookup(&lookup_json, &mut result);
                    }
                }
            }
        }

        self.cache.set(cache_key, result.clone());
        Ok(result)
    }
}

fn map_bing_lang(name: &str) -> &'static str {
    match name {
        "自动" | "auto" => "auto-detect",
        "中文" | "zh" | "zh-CN" => "zh-Hans",
        "英文" | "en" => "en",
        "日文" | "日语" | "ja" => "ja",
        "韩文" | "韩语" | "ko" => "ko",
        "法语" | "fr" => "fr",
        "俄语" | "ru" => "ru",
        "西班牙语" | "es" => "es",
        "德语" | "de" => "de",
        "阿拉伯语" | "ar" => "ar",
        _ => "auto-detect",
    }
}

fn parse_bing_lookup(val: &Value, result: &mut TranslationResult) {
    if let Some(arr) = val.as_array() {
        if let Some(first) = arr.first() {
            if let Some(translations) = first.get("translations").and_then(|v| v.as_array()) {
                for trans in translations {
                    let pos = trans.get("posTag").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let meaning = trans.get("displayTarget").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let mut synonyms = Vec::new();
                    if let Some(back_trans) = trans.get("backTranslations").and_then(|v| v.as_array()) {
                        for bt in back_trans {
                            if let Some(txt) = bt.get("displayText").and_then(|v| v.as_str()) {
                                synonyms.push(txt.to_string());
                            }
                        }
                    }
                    result.detailed_meanings.push(DetailedMeaning {
                        pos: pos.clone(),
                        meaning: meaning.clone(),
                        synonyms,
                    });
                    if let Some(examples) = trans.get("examples").and_then(|v| v.as_array()) {
                        for ex in examples {
                            let src = ex.get("sourceExample").and_then(|v| v.as_str()).map(|s| s.to_string());
                            result.definitions.push(Definition {
                                pos: pos.clone(),
                                meaning: meaning.clone(),
                                synonyms: vec![],
                                example: src,
                            });
                        }
                    }
                }
            }
        }
    }
}
