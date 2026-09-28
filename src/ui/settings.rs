use dioxus::desktop::use_window;
use dioxus::prelude::*;
use std::sync::atomic::Ordering;
use crate::config::AppSettings;

/// 统一功能函数：一处修改，全局同步（Win32 划词钩子 + 系统托盘菜单文字 + 响应式配置 + 磁盘持久化）
pub fn set_translation_enabled(enabled: bool, settings: &mut Signal<AppSettings>) {
    // 1. 同步底层 Win32 全局鼠标划词钩子
    crate::hook::SelectionHook::set_enabled(enabled);
    if let Some(hook_flag) = crate::HOOK_ENABLED.get() {
        hook_flag.store(enabled, Ordering::SeqCst);
    }

    // 2. 通过 IPC 同步给后台纯 Rust 托盘 Daemon（同步 Win32 钩子与托盘右键菜单文字）
    crate::ipc::send_to_daemon(crate::ipc::IpcMessage::ToggleTranslation { enabled });

    // 3. 同步 Dioxus 响应式状态（自动同步设置界面的 Switch 开关与主界面状态）
    if settings.peek().is_translation_enabled != enabled {
        settings.write().is_translation_enabled = enabled;
    }

    // 4. 持久化保存到配置文件 settings.json
    settings.peek().save();
}

/// 统一设置逻辑：重置为默认配置并全系统同步
pub fn reset_to_defaults(
    settings: &mut Signal<AppSettings>,
    manual_from: &mut Signal<String>,
    manual_to: &mut Signal<String>,
    is_pinned: &mut Signal<bool>,
) {
    let def = AppSettings::default();

    // 1. 同步划词开关与托盘
    set_translation_enabled(def.is_translation_enabled, settings);

    // 2. 同步固定置顶模式
    crate::pinned_mode::set_pinned(&use_window(), def.is_pinned);
    is_pinned.set(def.is_pinned);

    // 3. 同步语言偏好
    manual_from.set(def.from_lang.clone());
    manual_to.set(def.to_lang.clone());

    // 4. 同步 OCR 触发模式
    crate::ocr::set_ocr_trigger_enabled(def.ocr_trigger_enabled);
    crate::ipc::send_to_daemon(crate::ipc::IpcMessage::SetOcrTrigger { enabled: def.ocr_trigger_enabled });

    // 5. 同步开机自启动设置
    let _ = crate::autostart::set_autostart_enabled(def.auto_start);

    // 6. 保存并覆盖当前配置
    settings.set(def.clone());
    def.save();
}

#[component]
pub fn SettingsPage(
    mut settings: Signal<AppSettings>,
    mut manual_from: Signal<String>,
    mut manual_to: Signal<String>,
    mut is_pinned: Signal<bool>,
    mut is_sidebar_collapsed: Signal<bool>,
    mut sidebar_width: Signal<f64>,
    last_expanded_width: Signal<f64>,
) -> Element {
    let mut is_saved = use_signal(|| false);

    rsx! {
        div { class: "page-header",
            div { style: "display: flex; align-items: center; gap: 12px;",
                if is_sidebar_collapsed() {
                    button {
                        class: "btn-expand-sidebar",
                        title: "展开侧边栏",
                        onclick: move |_| {
                            let target = last_expanded_width().max(180.0);
                            sidebar_width.set(target);
                            is_sidebar_collapsed.set(false);
                            if settings().remember_layout {
                                settings.write().sidebar_width = target;
                                settings.write().sidebar_collapsed = false;
                                settings().save();
                            }
                        },
                        span { "☰" }
                        span { "展开侧栏" }
                    }
                }
                h2 { class: "page-title", "全局设置与管理" }
            }
            div { style: "display: flex; gap: 10px; align-items: center;",
                button {
                    class: "btn-primary",
                    style: if is_saved() { "background: #10b981; border-color: #10b981; display: inline-flex; align-items: center; gap: 6px;" } else { "display: inline-flex; align-items: center; gap: 6px;" },
                    onclick: move |_| {
                        settings().save();
                        is_saved.set(true);
                        spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                            is_saved.set(false);
                        });
                    },
                    if is_saved() { "✓ 设置已保存" } else { "💾 保存设置" }
                }
            }
        }

        div { class: "page-body",
            div { class: "settings-list",
                // 1. Translation Engine
                div { class: "setting-row",
                    div { class: "setting-info",
                        span { class: "setting-title", "默认翻译引擎" }
                        span { class: "setting-desc", "设置默认使用的在线翻译引擎" }
                    }
                    select {
                        class: "dropdown",
                        value: "{settings().engine}",
                        onchange: move |evt| {
                            settings.write().engine = evt.value();
                            settings().save();
                        },
                        option { value: "google", "谷歌翻译 (Google Translate)" }
                        option { value: "google_web", "谷歌网页翻译 (Google Web)" }
                        option { value: "bing", "必应翻译 (Bing Translator)" }
                    }
                }

                // 2. Source language preference
                div { class: "setting-row",
                    div { class: "setting-info",
                        span { class: "setting-title", "源语言偏好" }
                        span { class: "setting-desc", "划词默认优先使用的源语种" }
                    }
                    select {
                        class: "dropdown",
                        value: "{settings().from_lang}",
                        onchange: move |evt| {
                            settings.write().from_lang = evt.value();
                            manual_from.set(evt.value());
                            settings().save();
                        },
                        option { value: "auto", "自动检测 (Auto)" }
                        option { value: "en", "英文 (English)" }
                        option { value: "zh", "中文 (Chinese)" }
                        option { value: "ja", "日文 (Japanese)" }
                        option { value: "ko", "韩文 (Korean)" }
                    }
                }

                // 3. Target language
                div { class: "setting-row",
                    div { class: "setting-info",
                        span { class: "setting-title", "目标翻译语言" }
                        span { class: "setting-desc", "翻译呈现的目标语言" }
                    }
                    select {
                        class: "dropdown",
                        value: "{settings().to_lang}",
                        onchange: move |evt| {
                            settings.write().to_lang = evt.value();
                            manual_to.set(evt.value());
                            settings().save();
                        },
                        option { value: "zh", "中文 (Chinese)" }
                        option { value: "en", "英文 (English)" }
                        option { value: "ja", "日文 (Japanese)" }
                        option { value: "ko", "韩文 (Korean)" }
                        option { value: "fr", "法语 (French)" }
                        option { value: "ru", "俄语 (Russian)" }
                        option { value: "es", "西班牙语 (Spanish)" }
                        option { value: "de", "德语 (German)" }
                        option { value: "ar", "阿拉伯语 (Arabic)" }
                    }
                }

                // 4. Mutual Translation toggle
                div { class: "setting-row",
                    div { class: "setting-info",
                        span { class: "setting-title", "智能互译模式 (↔)" }
                        span { class: "setting-desc", "若选中文本与目标语言一致，自动智能反转为英文/中文" }
                    }
                    label { class: "switch",
                        input {
                            r#type: "checkbox",
                            checked: settings().is_mutual,
                            onchange: move |evt| {
                                settings.write().is_mutual = evt.checked();
                                settings().save();
                            }
                        }
                        span { class: "slider" }
                    }
                }

                // 5. Hover translation toggle (使用统一功能函数，同步托盘菜单与划词钩子)
                div { class: "setting-row",
                    div { class: "setting-info",
                        span { class: "setting-title", "启用全局划词翻译" }
                        span { class: "setting-desc", "鼠标拖拽划词后自动唤起主窗口并显示翻译结果" }
                    }
                    label { class: "switch",
                        input {
                            r#type: "checkbox",
                            checked: settings().is_translation_enabled,
                            onchange: move |evt| {
                                set_translation_enabled(evt.checked(), &mut settings);
                            }
                        }
                        span { class: "slider" }
                    }
                }

                // 6. Auto save layout
                div { class: "setting-row",
                    div { class: "setting-info",
                        span { class: "setting-title", "自动记忆窗口位置与大小" }
                        span { class: "setting-desc", "退出或最小化时保存窗口位置与尺寸" }
                    }
                    label { class: "switch",
                        input {
                            r#type: "checkbox",
                            checked: settings().remember_layout,
                            onchange: move |evt| {
                                settings.write().remember_layout = evt.checked();
                                settings().save();
                            }
                        }
                        span { class: "slider" }
                    }
                }

                // 7. Window Pin option (通过单独文件 pinned_mode 处理)
                div { class: "setting-row",
                    div { class: "setting-info",
                        span { class: "setting-title", "固定置顶窗口" }
                        span { class: "setting-desc", "在前台时始终置顶显示（点击其他软件不沉底）；最小化到任务栏后划词停留在后台" }
                    }
                    label { class: "switch",
                        input {
                            r#type: "checkbox",
                            checked: is_pinned(),
                            onchange: move |evt| {
                                let next = evt.checked();
                                is_pinned.set(next);
                                crate::pinned_mode::set_pinned(&use_window(), next);
                                settings.write().is_pinned = next;
                                settings().save();
                            }
                        }
                        span { class: "slider" }
                    }
                }

                // 8. 开机自启动
                div { class: "setting-row",
                    div { class: "setting-info",
                        span { class: "setting-title", "开机自启动" }
                        span { class: "setting-desc", "电脑开机登录后自动在后台托盘运行（静默启动）" }
                    }
                    label { class: "switch",
                        input {
                            r#type: "checkbox",
                            checked: settings().auto_start,
                            onchange: move |evt| {
                                let enabled = evt.checked();
                                settings.write().auto_start = enabled;
                                let _ = crate::autostart::set_autostart_enabled(enabled);
                                settings().save();
                            }
                        }
                        span { class: "slider" }
                    }
                }

                // 9. Custom API
                div { class: "setting-row",
                    div { class: "setting-info",
                        span { class: "setting-title", "全局默认自定义 API 代理地址" }
                        span { class: "setting-desc", "如 Cloudflare Worker 反代地址，留空则使用官方接口" }
                    }
                    input {
                        class: "input-text",
                        placeholder: "https://worker.dev/",
                        value: "{settings().custom_api_url}",
                        oninput: move |evt| {
                            settings.write().custom_api_url = evt.value();
                            settings().save();
                        }
                    }
                }

                // 9. 网络代理模式设置 (全局系统代理 / 设置代码 / 不代理)
                div { class: "setting-row",
                    div { class: "setting-info",
                        span { class: "setting-title", "全局网络代理" }
                        span { class: "setting-desc", "选择网络连接代理方式（系统代理、设置代码、不代理）" }
                    }
                    select {
                        class: "dropdown",
                        value: "{settings().proxy_mode}",
                        onchange: move |evt| {
                            let mode = evt.value();
                            settings.write().proxy_mode = mode.clone();
                            crate::translator::client::set_proxy_config(&mode, &settings.peek().proxy_url);
                            settings().save();
                        },
                        option { value: "system", "系统代理 (自动检测)" }
                        option { value: "custom", "设置代码 (自定义代理)" }
                        option { value: "none", "不代理 (直连网络)" }
                    }
                }

                // 当选择 "设置代码" (自定义代理) 时，显示代理地址输入框
                if settings().proxy_mode == "custom" {
                    div { class: "setting-row",
                        div { class: "setting-info",
                            span { class: "setting-title", "代理设置代码 / 地址" }
                            span { class: "setting-desc", "输入 HTTP/SOCKS5 代理地址，例如 http://127.0.0.1:10808" }
                        }
                        input {
                            class: "input-text",
                            placeholder: "http://127.0.0.1:10808",
                            value: "{settings().proxy_url}",
                            oninput: move |evt| {
                                let val = evt.value();
                                settings.write().proxy_url = val.clone();
                                crate::translator::client::set_proxy_config(&settings.peek().proxy_mode, &val);
                                settings().save();
                            }
                        }
                    }
                }

                // 10. 谷歌网页设置 (内置原生 / 系统 cURL / 并发竞速)
                div { class: "setting-row",
                    div { class: "setting-info",
                        span { class: "setting-title", "谷歌网页设置" }
                        span { class: "setting-desc", "选择谷歌网页端翻译的底层通道：内置原生(reqwest)、系统 cURL，或两者一起并发竞速" }
                    }
                    div { class: "mode-btn-group",
                        button {
                            r#type: "button",
                            class: if settings().google_web_mode == "native" { "mode-btn active" } else { "mode-btn" },
                            onclick: move |_| {
                                settings.write().google_web_mode = "native".to_string();
                                crate::translator::google_web::set_google_web_mode("native");
                                settings().save();
                            },
                            "内置原生"
                        }
                        button {
                            r#type: "button",
                            class: if settings().google_web_mode == "curl" { "mode-btn active" } else { "mode-btn" },
                            onclick: move |_| {
                                settings.write().google_web_mode = "curl".to_string();
                                crate::translator::google_web::set_google_web_mode("curl");
                                settings().save();
                            },
                            "系统 cURL"
                        }
                        button {
                            r#type: "button",
                            class: if settings().google_web_mode == "race" || settings().google_web_mode.is_empty() { "mode-btn active" } else { "mode-btn" },
                            onclick: move |_| {
                                settings.write().google_web_mode = "race".to_string();
                                crate::translator::google_web::set_google_web_mode("race");
                                settings().save();
                            },
                            "一起 (谁先回应就用谁)"
                        }
                    }
                }

                // 11. 屏幕划词与 OCR 触发设置 (新功能：OCR也触发 / 普通(OCR不触发))
                div { class: "setting-row",
                    div { class: "setting-info",
                        span { class: "setting-title", "屏幕划词与 OCR 触发设置" }
                        span { class: "setting-desc", "设置是否响应 PowerToys 文本提取器、截图识图等屏幕 OCR 工具的划选识别" }
                    }
                    div { class: "mode-btn-group",
                        button {
                            r#type: "button",
                            class: if settings().ocr_trigger_enabled { "mode-btn active" } else { "mode-btn" },
                            onclick: move |_| {
                                settings.write().ocr_trigger_enabled = true;
                                crate::ocr::set_ocr_trigger_enabled(true);
                                crate::ipc::send_to_daemon(crate::ipc::IpcMessage::SetOcrTrigger { enabled: true });
                                settings().save();
                            },
                            "OCR 也触发 (默认)"
                        }
                        button {
                            r#type: "button",
                            class: if !settings().ocr_trigger_enabled { "mode-btn active" } else { "mode-btn" },
                            onclick: move |_| {
                                settings.write().ocr_trigger_enabled = false;
                                crate::ocr::set_ocr_trigger_enabled(false);
                                crate::ipc::send_to_daemon(crate::ipc::IpcMessage::SetOcrTrigger { enabled: false });
                                settings().save();
                            },
                            "普通 (OCR 不触发)"
                        }
                    }
                }

                // 12. Reset defaults (调用统一重置逻辑)
                div { class: "setting-row",
                    div { class: "setting-info",
                        span { class: "setting-title", "还原默认设置" }
                        span { class: "setting-desc", "清除个性化配置并重置为初始状态" }
                    }
                    button {
                        style: "background: #fee2e2; color: #dc2626; border: 1px solid #fecaca; border-radius: 6px; padding: 6px 16px; font-size: 13px; cursor: pointer;",
                        onclick: move |_| {
                            reset_to_defaults(&mut settings, &mut manual_from, &mut manual_to, &mut is_pinned);
                        },
                        "重置为默认值"
                    }
                }

                // 10. Translation Sources Management (新增加翻译源列表配置)
                div {
                    style: "display: flex; flex-direction: column; gap: 10px; margin-top: 14px; padding-top: 16px; border-top: 1px solid var(--border);",
                    div {
                        style: "display: flex; justify-content: space-between; align-items: center;",
                        div {
                            span { style: "font-size: 15px; font-weight: 700; color: var(--text-main); display: block;", "自定义翻译源配置 (序号竞速与多源展示)" }
                            span { style: "font-size: 12px; color: var(--text-muted);", "💡 规则说明：序号一样的源将并发测速并采用最快回复的节点；序号不同的源将在翻译主页中自上而下竖向并列展示。" }
                        }
                        button {
                            class: "btn-primary",
                            style: "padding: 6px 14px; font-size: 12px;",
                            onclick: move |_| {
                                let max_order = settings().sources.iter().map(|s| s.order).max().unwrap_or(0);
                                let new_source = crate::source::TranslationSource::new(
                                    max_order + 1,
                                    "google",
                                    "https://translate.googleapis.com/"
                                );
                                settings.write().sources.push(new_source);
                                settings().save();
                            },
                            "➕ 添加新翻译源"
                        }
                    }

                    div { class: "sources-list",
                        // 列头
                        div {
                            style: "display: flex; gap: 10px; padding: 4px 14px; font-size: 12px; font-weight: 600; color: var(--text-muted);",
                            span { style: "width: 65px; text-align: center;", "序号" }
                            span { style: "width: 120px;", "翻译引擎" }
                            span { style: "flex: 1;", "链接地址 / API 代理 (URL)" }
                            span { style: "width: 48px; text-align: center;", "启用" }
                            span { style: "width: 32px; text-align: center;", "操作" }
                        }

                        // 翻译源列表项
                        for (idx, src) in settings().sources.clone().into_iter().enumerate() {
                            {
                                let id = src.id.clone();
                                let curr_order = src.order;
                                let curr_engine = src.engine.clone();
                                let curr_url = src.url.clone();
                                let is_enabled = src.enabled;

                                rsx! {
                                    div {
                                        key: "{id}",
                                        class: "source-item-row",
                                        // 1. 序号输入
                                        input {
                                            class: "source-order-input",
                                            r#type: "number",
                                            min: "1",
                                            max: "99",
                                            value: "{curr_order}",
                                            title: "相同序号并发竞速，不同序号在主页竖向排列展示",
                                            onchange: move |evt| {
                                                if let Ok(val) = evt.value().parse::<u32>() {
                                                    if let Some(item) = settings.write().sources.get_mut(idx) {
                                                        item.order = val;
                                                    }
                                                    settings().save();
                                                }
                                            }
                                        }

                                        // 2. 翻译引擎选择
                                        select {
                                            class: "source-engine-select",
                                            value: "{curr_engine}",
                                            onchange: move |evt| {
                                                let new_engine = evt.value();
                                                if let Some(item) = settings.write().sources.get_mut(idx) {
                                                    let old_is_default = match item.engine.as_str() {
                                                        "google" => item.url.is_empty() || item.url == "https://translate.googleapis.com/",
                                                        "google_web" => item.url.is_empty() || item.url == "https://translate.google.com/" || item.url.contains("translate.google.com"),
                                                        "bing" => item.url.is_empty() || item.url == "https://www.bing.com/" || item.url == "https://cn.bing.com/",
                                                        _ => false,
                                                    };
                                                    item.engine = new_engine.clone();
                                                    if old_is_default {
                                                        item.url = match new_engine.as_str() {
                                                            "google" => "https://translate.googleapis.com/".to_string(),
                                                            "google_web" => "https://translate.google.com/".to_string(),
                                                            "bing" => "https://www.bing.com/".to_string(),
                                                            _ => item.url.clone(),
                                                        };
                                                    }
                                                }
                                                settings().save();
                                            },
                                            option { value: "google", "谷歌 (Google)" }
                                            option { value: "google_web", "谷歌网页 (Google Web)" }
                                            option { value: "bing", "必应 (Bing)" }
                                        }

                                        // 3. 链接地址输入
                                        input {
                                            class: "source-url-input",
                                            placeholder: "例如 https://translate.googleapis.com/ 或 Workers 代理链接",
                                            value: "{curr_url}",
                                            oninput: move |evt| {
                                                if let Some(item) = settings.write().sources.get_mut(idx) {
                                                    item.url = evt.value();
                                                }
                                                settings().save();
                                            }
                                        }

                                        // 启用/禁用开关
                                        div {
                                            style: "width: 48px; display: flex; justify-content: center;",
                                            label { class: "switch", style: "transform: scale(0.8);",
                                                input {
                                                    r#type: "checkbox",
                                                    checked: is_enabled,
                                                    onchange: move |evt| {
                                                        if let Some(item) = settings.write().sources.get_mut(idx) {
                                                            item.enabled = evt.checked();
                                                        }
                                                        settings().save();
                                                    }
                                                }
                                                span { class: "slider" }
                                            }
                                        }

                                        // 删除按钮
                                        button {
                                            class: "btn-danger-icon",
                                            title: "删除此翻译源",
                                            onclick: move |_| {
                                                settings.write().sources.retain(|s| s.id != id);
                                                settings().save();
                                            },
                                            "🗑️"
                                        }
                                    }
                                }
                            }
                        }

                        if settings().sources.is_empty() {
                            div {
                                style: "text-align: center; padding: 24px; color: var(--text-muted); font-size: 13px;",
                                "暂无翻译源，点击上方「➕ 添加新翻译源」添加"
                            }
                        }
                    }
                }
            }
        }
    }
}
