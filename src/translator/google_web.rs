use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::RwLock;
use std::time::Duration;
use serde_json::Value;
use crate::translator::types::{Definition, DetailedMeaning, Example, TranslationResult};
use crate::translator::cache::SimpleCache;

static GOOGLE_WEB_MODE: RwLock<String> = RwLock::new(String::new());

/// 设置谷歌网页翻译模式："native" (内置原生) | "curl" (系统 cURL) | "race" (并发竞速)
pub fn set_google_web_mode(mode: &str) {
    if let Ok(mut m) = GOOGLE_WEB_MODE.write() {
        *m = mode.trim().to_string();
    }
}

pub fn get_google_web_mode() -> String {
    let m = GOOGLE_WEB_MODE.read().map(|m| m.clone()).unwrap_or_default();
    if m.is_empty() {
        "race".to_string()
    } else {
        m
    }
}

/// 谷歌网页版翻译器 (Google Web Translator)
/// 专门针对 Google 网页端 (https://translate.google.com/) 的交互与数据交互设计，
/// 支持内置原生请求 (reqwest)、系统 cURL 工具调用，以及双路并发竞速（谁快用谁）。
pub struct GoogleWebTranslator {
    cache: SimpleCache,
    tkk_0: AtomicI64,
    tkk_1: AtomicI64,
}

impl GoogleWebTranslator {
    pub fn new() -> Self {
        Self {
            cache: SimpleCache::new(300, Duration::from_secs(600)),
            tkk_0: AtomicI64::new(434217),
            tkk_1: AtomicI64::new(1534559001),
        }
    }

    /// 生成可供 WebView2 (通过 document::eval) 在前端浏览器内核中直接执行的 JS 脚本。
    /// 在 WebView 运行时环境内发起 fetch 请求并自动将结果通过 dioxus.send 传回 Rust。
    #[allow(dead_code)]
    pub fn generate_webview_eval_js(text: &str, from: &str, to: &str) -> String {
        let escaped_text = text
            .replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('\n', "\\n")
            .replace('\r', "");
        let from_code = crate::translator::types::map_language_name_to_code(from);
        let to_code = crate::translator::types::map_language_name_to_code(to);

        format!(
            r#"
            (async () => {{
                try {{
                    const query = encodeURIComponent('{escaped_text}');
                    const url = 'https://translate.googleapis.com/translate_a/single?client=gtx&dj=1&dt=t&dt=at&dt=bd&dt=ex&dt=md&dt=rw&dt=ss&dt=rm&sl={from_code}&tl={to_code}&q=' + query;
                    const res = await fetch(url, {{
                        headers: {{ 'Accept': 'application/json' }}
                    }});
                    if (!res.ok) throw new Error('HTTP ' + res.status);
                    const json = await res.json();
                    dioxus.send(JSON.stringify({{ success: true, data: json }}));
                }} catch (e) {{
                    dioxus.send(JSON.stringify({{ success: false, error: String(e) }}));
                }}
            }})();
            "#
        )
    }

    /// 计算 Google TK 校验参数
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

    /// 动态刷新 Google 网页版 TKK（使用短超时，避免阻塞正常请求）
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

    /// 方式 1：内置原生请求 (reqwest)
    async fn translate_native(&self, url: &str, trimmed: &str) -> Result<TranslationResult, String> {
        let client = crate::translator::client::get_http_client(8);
        let response = match client.get(url)
            .header("Referer", "https://translate.google.com/")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => return Err(format!("内置网络请求失败: {}", e)),
        };

        if response.status() == 429 {
            return Err("谷歌网页翻译访问过于频繁 (429 Too Many Requests)，请稍后重试或在设置中配置代理".to_string());
        }

        if !response.status().is_success() {
            return Err(format!("谷歌网页翻译接口返回状态: {}", response.status()));
        }

        let body = response.text().await.map_err(|e| format!("读取网页翻译响应失败: {}", e))?;
        let json_val = serde_json::from_str::<Value>(&body)
            .map_err(|e| format!("解析网页翻译数据失败: {}", e))?;

        let mut result = parse_google_web_result(&json_val, trimmed);
        result.method_used = Some("内置原生".to_string());
        Ok(result)
    }

    /// 方式 2：系统 cURL 请求 (curl.exe)
    async fn translate_curl(&self, url: &str, trimmed: &str) -> Result<TranslationResult, String> {
        let mut cmd = tokio::process::Command::new("curl.exe");

        #[cfg(target_os = "windows")]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        cmd.arg("--ssl-no-revoke")
            .arg("-s")
            .arg("-m")
            .arg("8")
            .arg("-A")
            .arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
            .arg("-e")
            .arg("https://translate.google.com/")
            .arg("-H")
            .arg("Accept-Language: zh-CN,zh;q=0.9,en;q=0.8")
            .arg("-w")
            .arg("\nHTTP_STATUS:%{http_code}");

        // 同步全局代理设置
        let (proxy_mode, custom_url) = crate::translator::client::get_proxy_config();
        match proxy_mode.as_str() {
            "none" | "direct" | "不代理" => {
                cmd.arg("--noproxy").arg("*");
            }
            "custom" | "设置代码" | "设置代理" => {
                let target = custom_url.trim();
                if !target.is_empty() {
                    cmd.arg("-x").arg(target);
                } else {
                    cmd.arg("--noproxy").arg("*");
                }
            }
            _ => {
                if let Some(proxy_url) = crate::translator::client::detect_system_proxy() {
                    cmd.arg("-x").arg(proxy_url);
                }
            }
        }

        cmd.arg(url);

        let output = cmd.output().await.map_err(|e| format!("启动 cURL 失败: {}", e))?;
        let full_str = String::from_utf8_lossy(&output.stdout);

        let (body, status_code) = if let Some(idx) = full_str.rfind("\nHTTP_STATUS:") {
            let body = &full_str[..idx];
            let status_str = full_str[idx + 13..].trim();
            let code = status_str.parse::<u16>().unwrap_or(0);
            (body, code)
        } else {
            (full_str.as_ref(), if output.status.success() { 200 } else { 0 })
        };

        if status_code == 429 {
            return Err("谷歌网页翻译(cURL)访问过于频繁 (429 Too Many Requests)，请稍后重试或在设置中配置代理".to_string());
        }

        if status_code == 403 {
            return Err("谷歌网页翻译(cURL)访问被拒绝 (403 Forbidden)，请配置可用代理重试".to_string());
        }

        if status_code != 200 {
            return Err(format!("谷歌网页翻译(cURL)返回状态: {}", status_code));
        }

        let json_val = serde_json::from_str::<Value>(body)
            .map_err(|e| format!("解析 cURL 网页翻译响应失败: {}", e))?;

        let mut result = parse_google_web_result(&json_val, trimmed);
        result.method_used = Some("系统 cURL".to_string());
        Ok(result)
    }

    /// 方式 3：并发竞速（内置原生与系统 cURL 一起发起，谁先回应成功就用谁）
    async fn translate_race(&self, url: &str, trimmed: &str) -> Result<TranslationResult, String> {
        let f_native = self.translate_native(url, trimmed);
        let f_curl = self.translate_curl(url, trimmed);
        tokio::pin!(f_native);
        tokio::pin!(f_curl);

        let mut native_done = false;
        let mut curl_done = false;
        let mut last_err = String::new();

        while !native_done || !curl_done {
            tokio::select! {
                res = &mut f_native, if !native_done => {
                    native_done = true;
                    match res {
                        Ok(val) => return Ok(val),
                        Err(e) => last_err = format!("内置原生: {}", e),
                    }
                }
                res = &mut f_curl, if !curl_done => {
                    curl_done = true;
                    match res {
                        Ok(val) => return Ok(val),
                        Err(e) => last_err = format!("系统 cURL: {}", e),
                    }
                }
            }
        }

        Err(format!("双路竞速均未成功: {}", last_err))
    }

    /// 执行翻译请求（根据设置分发：内置原生 / 系统 cURL / 并发竞速）
    pub async fn translate(&self, text: &str, from: &str, to: &str, custom_api: &str) -> Result<TranslationResult, String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(TranslationResult::default());
        }

        let mode = get_google_web_mode();
        let cache_key = format!("GWEB|{}|{}|{}|{}|{}", mode, from, to, custom_api, trimmed);
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached);
        }

        let from_code = crate::translator::types::map_language_name_to_code(from);
        let to_code = crate::translator::types::map_language_name_to_code(to);
        let tk = self.generate_tk(trimmed);
        let encoded_text = urlencoding::encode(trimmed);

        // 智能处理 URL：
        // 1. 如果用户误填或遗留了 bing 等其他引擎的 URL，自动忽略并使用谷歌地址
        // 2. 自动清洗 URL 网页参数（如 ?sl=...&op=translate），避免 404
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
                let clean_base = if let Some(idx) = raw_api.find('?') {
                    &raw_api[..idx]
                } else {
                    raw_api
                };
                let base = clean_base.trim_end_matches('/');

                if base.contains("google") {
                    format!(
                        "{}/translate_a/single?ie=UTF-8&client=webapp&otf=1&ssel=0&tsel=0&kc=5&dt=t&dt=at&dt=bd&dt=ex&dt=md&dt=rw&dt=ss&dt=rm&sl={}&tl={}&tk={}&q={}",
                        base, from_code, to_code, tk, encoded_text
                    )
                } else {
                    format!(
                        "https://translate.google.com/translate_a/single?ie=UTF-8&client=webapp&otf=1&ssel=0&tsel=0&kc=5&dt=t&dt=at&dt=bd&dt=ex&dt=md&dt=rw&dt=ss&dt=rm&sl={}&tl={}&tk={}&q={}",
                        from_code, to_code, tk, encoded_text
                    )
                }
            }
        } else {
            format!(
                "https://translate.google.com/translate_a/single?ie=UTF-8&client=webapp&otf=1&ssel=0&tsel=0&kc=5&dt=t&dt=at&dt=bd&dt=ex&dt=md&dt=rw&dt=ss&dt=rm&sl={}&tl={}&tk={}&q={}",
                from_code, to_code, tk, encoded_text
            )
        };

        let result = match mode.as_str() {
            "native" | "内置原生" => self.translate_native(&url, trimmed).await?,
            "curl" | "cURL" | "系统 cURL" => self.translate_curl(&url, trimmed).await?,
            _ => self.translate_race(&url, trimmed).await?,
        };

        self.cache.set(cache_key, result.clone());
        Ok(result)
    }
}

fn magic(mut a: i64, b: &str) -> i64 {
    let bytes = b.as_bytes();
    let mut c = 0;
    while c < bytes.len().saturating_sub(2) {
        let char_d = bytes[c + 2];
        let d = if char_d >= b'a' {
            (char_d - b'a' + 10) as i64
        } else {
            (char_d - b'0') as i64
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

fn parse_google_web_result(val: &Value, original: &str) -> TranslationResult {
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
        if let Some(sentences) = arr.first().and_then(|v| v.as_array()) {
            let mut main_text = String::new();
            for s in sentences {
                if let Some(s_arr) = s.as_array() {
                    if let Some(trans) = s_arr.first().and_then(|v| v.as_str()) {
                        main_text.push_str(trans);
                    }
                    // 提取目标语发音/音标 (translit)
                    if s_arr.len() >= 3 {
                        if let Some(translit) = s_arr.get(2).or_else(|| s_arr.get(3)).and_then(|v| v.as_str()) {
                            if !translit.is_empty() && res.t_pronunciation.is_none() {
                                res.t_pronunciation = Some(translit.to_string());
                            }
                        }
                    }
                }
            }
            res.main_meaning = main_text;
        }

        // 解析词典词性与同义词
        if let Some(dict_arr) = arr.get(1).and_then(|v| v.as_array()) {
            for d in dict_arr {
                if let Some(d_item) = d.as_array() {
                    let pos = d_item.first().and_then(|v| v.as_str()).map(|s| s.to_string());
                    let terms = d_item.get(1)
                        .and_then(|v| v.as_array())
                        .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect::<Vec<_>>())
                        .unwrap_or_default();
                    
                    for term in terms {
                        res.detailed_meanings.push(DetailedMeaning {
                            pos: pos.clone(),
                            meaning: Some(term),
                            synonyms: vec![],
                        });
                    }
                }
            }
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
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_generate_tk() {
        let translator = GoogleWebTranslator::new();
        let tk = translator.generate_tk("hello");
        assert!(!tk.is_empty());
        assert!(tk.contains('.'));
    }

    #[test]
    fn test_parse_google_web_array_result() {
        let mock_json = serde_json::json!([
            [
                ["你好", "hello", null, null, 10],
                [null, null, "nǐ hǎo"]
            ],
            [
                ["感叹词", ["喂", "哈罗"]]
            ],
            "en"
        ]);
        let parsed = parse_google_web_result(&mock_json, "hello");
        assert_eq!(parsed.main_meaning, "你好");
        assert_eq!(parsed.source_language, Some("en".to_string()));
        assert_eq!(parsed.t_pronunciation, Some("nǐ hǎo".to_string()));
        assert_eq!(parsed.detailed_meanings.len(), 2);
    }

    #[test]
    fn test_google_web_mode_switch() {
        set_google_web_mode("native");
        assert_eq!(get_google_web_mode(), "native");

        set_google_web_mode("curl");
        assert_eq!(get_google_web_mode(), "curl");

        set_google_web_mode("race");
        assert_eq!(get_google_web_mode(), "race");
    }
}
