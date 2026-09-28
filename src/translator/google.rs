use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;
use serde_json::Value;
use crate::translator::types::{Definition, DetailedMeaning, Example, TranslationResult};
use crate::translator::cache::SimpleCache;

pub struct GoogleTranslator {
    cache: SimpleCache,
    tkk_0: AtomicI64,
    tkk_1: AtomicI64,
}

impl GoogleTranslator {
    pub fn new() -> Self {
        Self {
            cache: SimpleCache::new(300, Duration::from_secs(600)),
            tkk_0: AtomicI64::new(434217),
            tkk_1: AtomicI64::new(1534559001),
        }
    }

    pub fn generate_tk(&self, text: &str) -> String {
        let b = self.tkk_0.load(Ordering::Relaxed);
        let c = self.tkk_1.load(Ordering::Relaxed);

        let mut current_a = b;
        for &byte in text.as_bytes() {
            current_a += byte as i64;
            current_a = magic(current_a, "+-a^+6");
        }
        current_a = magic(current_a, "+-3^+b+-f");
        current_a ^= c;
        if current_a < 0 {
            current_a = (current_a & 2147483647) + 2147483648;
        }
        current_a %= 1000000;
        format!("{}.{}", current_a, current_a ^ b)
    }

    pub async fn update_tkk(&self) {
        let short_client = crate::translator::client::build_http_client(2);
        if let Ok(resp) = short_client.get("https://translate.google.com/").send().await {
            if let Ok(body) = resp.text().await {
                if let Some(caps) = regex::Regex::new(r#"TKK[=:]['"](\d+?)\.(\d+?)['"]"#).ok().and_then(|r| r.captures(&body)) {
                    if let (Ok(p0), Ok(p1)) = (caps[1].parse::<i64>(), caps[2].parse::<i64>()) {
                        self.tkk_0.store(p0, Ordering::Relaxed);
                        self.tkk_1.store(p1, Ordering::Relaxed);
                    }
                }
            }
        }
    }

    pub async fn translate(&self, text: &str, from: &str, to: &str, custom_api: &str) -> Result<TranslationResult, String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(TranslationResult::default());
        }

        let cache_key = format!("G|{}|{}|{}|{}", from, to, custom_api, trimmed);
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached);
        }

        let from_code = crate::translator::types::map_language_name_to_code(from);
        let to_code = crate::translator::types::map_language_name_to_code(to);
        let tk = self.generate_tk(trimmed);
        let encoded_text = urlencoding::encode(trimmed);

        let raw_api = custom_api.trim();
        let is_foreign_engine = raw_api.contains("bing.com");
        let url = if !raw_api.is_empty() && !is_foreign_engine {
            if raw_api.contains("{text}") {
                raw_api.replace("{text}", &encoded_text)
                   .replace("{from}", from_code)
                   .replace("{fromLang}", from_code)
                   .replace("{to}", to_code)
                   .replace("{toLang}", to_code)
            } else {
                format!(
                    "{}{}translate_a/single?client=gtx&dj=1&dt=t&dt=at&dt=bd&dt=ex&dt=md&dt=rw&dt=ss&dt=rm&sl={}&tl={}&tk={}&q={}",
                    raw_api,
                    if raw_api.ends_with('/') { "" } else { "/" },
                    from_code, to_code, tk, encoded_text
                )
            }
        } else {
            format!(
                "https://translate.googleapis.com/translate_a/single?client=gtx&dj=1&dt=t&dt=at&dt=bd&dt=ex&dt=md&dt=rw&dt=ss&dt=rm&sl={}&tl={}&tk={}&q={}",
                from_code, to_code, tk, encoded_text
            )
        };

        let client = crate::translator::client::get_http_client(8);
        let response = match client.get(&url)
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => return Err(format!("谷歌翻译网络请求失败: {}", e)),
        };

        if response.status() == 429 {
            return Err("谷歌翻译请求过于频繁 (429 Too Many Requests)，请稍后重试或在设置中配置代理".to_string());
        }

        if !response.status().is_success() {
            return Err(format!("谷歌翻译接口返回状态: {}", response.status()));
        }

        let body_str = response.text().await.map_err(|e| format!("读取响应失败: {}", e))?;
        let json_val = serde_json::from_str::<Value>(&body_str).map_err(|e| format!("解析 JSON 响应失败: {}", e))?;

        let result = parse_google_result(&json_val, trimmed);
        self.cache.set(cache_key, result.clone());
        Ok(result)
    }
}

fn magic(mut a: i64, b: &str) -> i64 {
    let bytes = b.as_bytes();
    let mut c = 0;
    while c < bytes.len() - 2 {
        let char_d = bytes[c + 2] as char;
        let d = if char_d >= 'a' {
            (char_d as i64) - 87
        } else {
            (char_d as i64) - ('0' as i64)
        };
        let d_val = if bytes[c + 1] == b'+' {
            ((a as u64) >> d) as i64
        } else {
            a << d
        };
        a = if bytes[c] == b'+' {
            (a + d_val) & 4294967295
        } else {
            a ^ d_val
        };
        c += 3;
    }
    a
}

fn parse_google_result(val: &Value, original: &str) -> TranslationResult {
    let mut res = TranslationResult {
        original_text: original.to_string(),
        ..Default::default()
    };

    if let Some(obj) = val.as_object() {
        if let Some(src) = obj.get("src").and_then(|v| v.as_str()) {
            res.source_language = Some(if src == "zh" { "zh-CN".to_string() } else { src.to_string() });
        }

        if let Some(sentences) = obj.get("sentences").and_then(|v| v.as_array()) {
            let mut main_text = String::new();
            for s in sentences {
                if let Some(trans) = s.get("trans").and_then(|v| v.as_str()) {
                    main_text.push_str(trans);
                }
                if let Some(translit) = s.get("translit").and_then(|v| v.as_str()) {
                    res.t_pronunciation = Some(translit.to_string());
                }
                if let Some(src_translit) = s.get("src_translit").and_then(|v| v.as_str()) {
                    res.s_pronunciation = Some(src_translit.to_string());
                }
            }
            res.main_meaning = main_text;
        }

        if let Some(dict) = obj.get("dict").and_then(|v| v.as_array()) {
            for d in dict {
                let pos = d.get("pos").and_then(|v| v.as_str()).map(|s| s.to_string());
                if let Some(entries) = d.get("entry").and_then(|v| v.as_array()) {
                    for entry in entries {
                        let word = entry.get("word").and_then(|v| v.as_str()).map(|s| s.to_string());
                        let synonyms = entry.get("reverse_translation")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                            .unwrap_or_default();
                        res.detailed_meanings.push(DetailedMeaning {
                            pos: pos.clone(),
                            meaning: word,
                            synonyms,
                        });
                    }
                }
            }
        }

        if let Some(definitions) = obj.get("definitions").and_then(|v| v.as_array()) {
            for def in definitions {
                let pos = def.get("pos").and_then(|v| v.as_str()).map(|s| s.to_string());
                if let Some(entries) = def.get("entry").and_then(|v| v.as_array()) {
                    for entry in entries {
                        let gloss = entry.get("gloss").and_then(|v| v.as_str()).map(|s| s.to_string());
                        let example = entry.get("example").and_then(|v| v.as_str()).map(|s| s.to_string());
                        res.definitions.push(Definition {
                            pos: pos.clone(),
                            meaning: gloss,
                            synonyms: vec![],
                            example,
                        });
                    }
                }
            }
        }

        if let Some(examples) = obj.get("examples").and_then(|v| v.get("example")).and_then(|v| v.as_array()) {
            for ex in examples {
                if let Some(t) = ex.get("text").and_then(|v| v.as_str()) {
                    res.examples.push(Example {
                        source: Some(t.to_string()),
                        target: None,
                    });
                }
            }
        }
    } else if let Some(arr) = val.as_array() {
        // Fallback array structure [ [ [trans, orig, translit, src_translit], ... ], dict, src ]
        if let Some(sentences) = arr.first().and_then(|v| v.as_array()) {
            let mut main_text = String::new();
            for s in sentences {
                if let Some(s_arr) = s.as_array() {
                    if let Some(trans) = s_arr.first().and_then(|v| v.as_str()) {
                        main_text.push_str(trans);
                    }
                    if s_arr.len() >= 4 {
                        if let Some(tp) = s_arr.get(2).and_then(|v| v.as_str()) {
                            res.t_pronunciation = Some(tp.to_string());
                        }
                        if let Some(sp) = s_arr.get(3).and_then(|v| v.as_str()) {
                            res.s_pronunciation = Some(sp.to_string());
                        }
                    }
                }
            }
            res.main_meaning = main_text;
        }

        if arr.len() > 2 {
            if let Some(src) = arr.get(2).and_then(|v| v.as_str()) {
                res.source_language = Some(if src == "zh" { "zh-CN".to_string() } else { src.to_string() });
            }
        }
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tk_generation() {
        let translator = GoogleTranslator::new();
        let tk = translator.generate_tk("Hello");
        assert!(!tk.is_empty());
        assert!(tk.contains('.'));

        let tk_chinese = translator.generate_tk("你好世界");
        assert!(!tk_chinese.is_empty());
        assert!(tk_chinese.contains('.'));
    }
}

