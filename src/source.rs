use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use crate::translator::{TranslationManager, TranslationResult};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TranslationSource {
    pub id: String,
    pub order: u32,
    pub engine: String,
    pub url: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl TranslationSource {
    pub fn new(order: u32, engine: impl Into<String>, url: impl Into<String>) -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        Self {
            id: format!("src_{}_{}", order, ts),
            order,
            engine: engine.into(),
            url: url.into(),
            enabled: true,
        }
    }
}

pub fn default_sources() -> Vec<TranslationSource> {
    vec![
        TranslationSource {
            id: "src_default_google".to_string(),
            order: 1,
            engine: "google".to_string(),
            url: "https://translate.googleapis.com/".to_string(),
            enabled: true,
        },
        TranslationSource {
            id: "src_default_bing".to_string(),
            order: 2,
            engine: "bing".to_string(),
            url: "https://www.bing.com/".to_string(),
            enabled: true,
        },
    ]
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupTranslationResult {
    pub order: u32,
    pub engine: String,
    pub used_url: String,
    pub latency_ms: u64,
    pub is_loading: bool,
    pub result: Result<TranslationResult, String>,
}

impl Default for GroupTranslationResult {
    fn default() -> Self {
        Self {
            order: 1,
            engine: "google".to_string(),
            used_url: String::new(),
            latency_ms: 0,
            is_loading: false,
            result: Ok(TranslationResult::default()),
        }
    }
}

/// 对同一个序号组（拥有相同 order）的所有启用源进行并发竞速请求，
/// 最先返回成功响应（Ok）的节点胜出并直接作为该序号的结果返回。
/// 若全部失败，则返回最后一个错误信息。
pub async fn race_group_sources(
    order: u32,
    sources: Vec<TranslationSource>,
    manager: Arc<TranslationManager>,
    text: String,
    from_lang: String,
    to_lang: String,
    is_mutual: bool,
) -> GroupTranslationResult {
    let count = sources.len();
    if count == 0 {
        return GroupTranslationResult {
            order,
            engine: "unknown".to_string(),
            used_url: String::new(),
            latency_ms: 0,
            is_loading: false,
            result: Err("当前序号未配置或未启用任何有效翻译源".to_string()),
        };
    }

    if count == 1 {
        let src = &sources[0];
        let start = Instant::now();
        let res = manager
            .translate(&src.engine, &text, &from_lang, &to_lang, is_mutual, &src.url)
            .await;
        let latency_ms = start.elapsed().as_millis() as u64;
        return GroupTranslationResult {
            order,
            engine: src.engine.clone(),
            used_url: src.url.clone(),
            latency_ms,
            is_loading: false,
            result: res,
        };
    }

    // 多个源并发竞速
    let (tx, mut rx) = mpsc::channel::<(String, String, u64, Result<TranslationResult, String>)>(count);

    for src in sources {
        let tx = tx.clone();
        let mgr = manager.clone();
        let t = text.clone();
        let f = from_lang.clone();
        let to = to_lang.clone();
        tokio::spawn(async move {
            let start = Instant::now();
            let res = mgr.translate(&src.engine, &t, &f, &to, is_mutual, &src.url).await;
            let latency_ms = start.elapsed().as_millis() as u64;
            let _ = tx.send((src.engine, src.url, latency_ms, res)).await;
        });
    }

    let mut last_engine = String::new();
    let mut last_url = String::new();
    let mut last_latency = 0;
    let mut last_err = "所有节点请求失败".to_string();

    for _ in 0..count {
        if let Some((eng, url, latency, res)) = rx.recv().await {
            match res {
                Ok(ok_res) => {
                    // 最快回复成功结果的节点胜出！
                    return GroupTranslationResult {
                        order,
                        engine: eng,
                        used_url: url,
                        latency_ms: latency,
                        is_loading: false,
                        result: Ok(ok_res),
                    };
                }
                Err(err) => {
                    last_engine = eng;
                    last_url = url;
                    last_latency = latency;
                    last_err = err;
                }
            }
        }
    }

    GroupTranslationResult {
        order,
        engine: last_engine,
        used_url: last_url,
        latency_ms: last_latency,
        is_loading: false,
        result: Err(last_err),
    }
}

/// 流式并发执行所有序号组：
/// 1. 按序号（order）分组
/// 2. 同时并发启动所有序号组的竞速请求
/// 3. 任何一个序号组完成（最快节点胜出），立即通过 channel 异步发射出去
/// 4. 使得 UI 界面能实现“同时发起、哪个序号快就先渲染哪个”的毫秒级流式体验
pub fn execute_all_sources_streaming(
    sources: &[TranslationSource],
    manager: Arc<TranslationManager>,
    text: &str,
    from_lang: &str,
    to_lang: &str,
    is_mutual: bool,
) -> mpsc::UnboundedReceiver<GroupTranslationResult> {
    let (tx, rx) = mpsc::unbounded_channel();
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return rx;
    }

    let mut groups: BTreeMap<u32, Vec<TranslationSource>> = BTreeMap::new();
    for src in sources.iter().filter(|s| s.enabled) {
        groups.entry(src.order).or_default().push(src.clone());
    }

    if groups.is_empty() {
        let mgr = manager.clone();
        let t = trimmed.to_string();
        let f = from_lang.to_string();
        let to = to_lang.to_string();
        tokio::spawn(async move {
            let start = Instant::now();
            let res = mgr.translate("google", &t, &f, &to, is_mutual, "").await;
            let latency_ms = start.elapsed().as_millis() as u64;
            let _ = tx.send(GroupTranslationResult {
                order: 1,
                engine: "google".to_string(),
                used_url: "默认谷歌".to_string(),
                latency_ms,
                is_loading: false,
                result: res,
            });
        });
        return rx;
    }

    for (order, group_sources) in groups {
        let tx = tx.clone();
        let mgr = manager.clone();
        let t = trimmed.to_string();
        let f = from_lang.to_string();
        let to = to_lang.to_string();
        tokio::spawn(async move {
            let res = race_group_sources(order, group_sources, mgr, t, f, to, is_mutual).await;
            let _ = tx.send(res);
        });
    }

    rx
}

/// 执行所有启用的翻译源（收集并返回最终列表）：
pub async fn execute_all_sources(
    sources: &[TranslationSource],
    manager: Arc<TranslationManager>,
    text: &str,
    from_lang: &str,
    to_lang: &str,
    is_mutual: bool,
) -> Vec<GroupTranslationResult> {
    let mut rx = execute_all_sources_streaming(sources, manager, text, from_lang, to_lang, is_mutual);
    let mut results = Vec::new();
    while let Some(res) = rx.recv().await {
        results.push(res);
    }
    results.sort_by_key(|r| r.order);
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_sources() {
        let sources = default_sources();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].order, 1);
        assert_eq!(sources[0].engine, "google");
        assert_eq!(sources[1].order, 2);
        assert_eq!(sources[1].engine, "bing");
    }

    #[test]
    fn test_source_serialization() {
        let src = TranslationSource::new(1, "google", "https://translate.googleapis.com/");
        let json = serde_json::to_string(&src).unwrap();
        assert!(json.contains("Order"));
        assert!(json.contains("Engine"));
        assert!(json.contains("Url"));
        let deserialized: TranslationSource = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.order, 1);
        assert_eq!(deserialized.engine, "google");
        assert!(deserialized.enabled);
    }
}

