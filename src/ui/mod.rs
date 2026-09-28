pub mod settings;
pub mod style;

use settings::SettingsPage;

use dioxus::desktop::use_window;
use dioxus::prelude::*;
use std::sync::Arc;
use crate::config::AppSettings;
use crate::translator::TranslationManager;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DashboardTab {
    Home,
    Settings,
}

#[component]
pub fn App() -> Element {
    let initial_settings = AppSettings::load();
    let mut settings = use_signal(|| initial_settings.clone());
    let mut dashboard_tab = use_signal(|| DashboardTab::Home);
    let mut is_pinned = use_signal(|| initial_settings.is_pinned);

    use_hook(|| {
        let mut eval = document::eval(r#"
            console.log("WebView2 is running JS successfully!");
            dioxus.send("WebView2 JS executed OK");
        "#);
        spawn(async move {
            if let Ok(msg) = eval.recv::<String>().await {
                println!("[WebView2 -> Rust] {}", msg);
            }
        });
    });

    // Sidebar resizable width & collapse state
    let initial_sidebar_width = initial_settings.sidebar_width.clamp(160.0, 450.0);
    let initial_sidebar_collapsed = initial_settings.sidebar_collapsed;

    let mut sidebar_width = use_signal(|| initial_sidebar_width);
    let mut is_sidebar_collapsed = use_signal(|| initial_sidebar_collapsed);
    let mut is_resizing_sidebar = use_signal(|| false);
    let mut last_expanded_width = use_signal(|| initial_sidebar_width);

    // Translation states for Main Home Page
    let mut manual_input = use_signal(String::new);
    let mut manual_from = use_signal(|| initial_settings.from_lang.clone());
    let mut manual_to = use_signal(|| initial_settings.to_lang.clone());
    let mut group_results = use_signal(Vec::<crate::source::GroupTranslationResult>::new);
    let mut manual_loading = use_signal(|| false);
    let mut error_msg = use_signal(|| Option::<String>::None);

    // Selection events listener: ONLY triggers when a REAL selection occurs from the Win32 hook
    use_future(move || async move {
        if let Some(rx_lock) = crate::SELECTION_RX.get() {
            let mut rx = rx_lock.lock().await;
            while let Some((text, _cx, _cy)) = rx.recv().await {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // 1. 划词时自动切回主页并填入划词文本
                dashboard_tab.set(DashboardTab::Home);
                manual_input.set(trimmed.to_string());

                // 2. 窗口模式响应（固定模式与不固定模式完全由两个独立文件分别实现）
                let win = use_window();
                let is_pinned_effective = *is_pinned.peek() || settings.peek().is_pinned || AppSettings::load().is_pinned;
                if is_pinned_effective {
                    crate::pinned_mode::on_selection(&win);
                } else {
                    crate::unpinned_mode::on_selection(&win);
                }

                // 3. 触发即时翻译（按序号分组竞速并发执行）
                let mgr = match crate::TRANSLATION_MANAGER.get() {
                    Some(m) => m.clone(),
                    None => Arc::new(TranslationManager::new()),
                };

                let cfg = settings.peek().clone();
                let sources = cfg.sources.clone();
                let from = manual_from.peek().clone();
                let to = manual_to.peek().clone();
                let mutual = cfg.is_mutual;
                let target_text = trimmed.to_string();

                manual_loading.set(true);
                error_msg.set(None);

                // 立即为所有启用的序号创建骨架占位卡片，所有序号卡片【同时全部显示】
                let mut unique_orders: Vec<u32> = sources.iter().filter(|s| s.enabled).map(|s| s.order).collect();
                unique_orders.sort_unstable();
                unique_orders.dedup();
                if unique_orders.is_empty() {
                    unique_orders.push(1);
                }

                let initial_cards: Vec<crate::source::GroupTranslationResult> = unique_orders
                    .into_iter()
                    .map(|order| crate::source::GroupTranslationResult {
                        order,
                        engine: String::new(),
                        used_url: String::new(),
                        latency_ms: 0,
                        is_loading: true,
                        result: Ok(crate::translator::TranslationResult::default()),
                    })
                    .collect();
                group_results.set(initial_cards);

                // 哪个序号快就先显示哪个（流式增量刷新）
                spawn(async move {
                    let mut stream = crate::source::execute_all_sources_streaming(&sources, mgr, &target_text, &from, &to, mutual);
                    while let Some(res) = stream.recv().await {
                        let mut curr = group_results.peek().clone();
                        if let Some(pos) = curr.iter().position(|r| r.order == res.order) {
                            curr[pos] = res;
                        } else {
                            curr.push(res);
                        }
                        curr.sort_by_key(|r| r.order);
                        group_results.set(curr);
                    }
                    manual_loading.set(false);
                });
            }
        }
    });

    // 监听来自后台纯 Rust 托盘 Daemon 的 IPC 控制指令（如唤醒显示窗口、同步划词开关）
    use_future(move || async move {
        if let Some(cmd_rx_lock) = crate::IPC_CMD_RX.get() {
            let mut cmd_rx = cmd_rx_lock.lock().await;
            while let Some(msg) = cmd_rx.recv().await {
                match msg {
                    crate::ipc::IpcMessage::ShowWindow => {
                        let win = use_window();
                        crate::tray::restore_and_show_window();
                        win.set_visible(true);
                        win.set_focus();
                    }
                    crate::ipc::IpcMessage::ToggleTranslation { enabled }
                        if settings.peek().is_translation_enabled != enabled => {
                        settings.write().is_translation_enabled = enabled;
                    }
                    _ => {}
                }
            }
        }
    });

    rsx! {
        style { "{style::APP_CSS}" }

        div {
            class: if is_resizing_sidebar() { "app-container resizing" } else { "app-container" },
            onmousemove: move |evt| {
                if is_resizing_sidebar() {
                    let x = evt.client_coordinates().x;
                    if x < 90.0 {
                        if !is_sidebar_collapsed() {
                            is_sidebar_collapsed.set(true);
                        }
                    } else {
                        if is_sidebar_collapsed() {
                            is_sidebar_collapsed.set(false);
                        }
                        let clamped = x.clamp(160.0, 450.0);
                        sidebar_width.set(clamped);
                        last_expanded_width.set(clamped);
                    }
                }
            },
            onmouseup: move |_| {
                if is_resizing_sidebar() {
                    is_resizing_sidebar.set(false);
                    if settings().remember_layout {
                        settings.write().sidebar_width = sidebar_width();
                        settings.write().sidebar_collapsed = is_sidebar_collapsed();
                        settings().save();
                    }
                }
            },

            // Sidebar Navigation
            div {
                class: {
                    let mut cls = "sidebar".to_string();
                    if is_sidebar_collapsed() {
                        cls.push_str(" collapsed");
                    }
                    if is_resizing_sidebar() {
                        cls.push_str(" resizing");
                    }
                    cls
                },
                style: {
                    if is_sidebar_collapsed() {
                        "width: 0px; min-width: 0px;".to_string()
                    } else {
                        format!("width: {}px; min-width: {}px;", sidebar_width(), sidebar_width())
                    }
                },

                // Brand header with collapse toggle
                div { class: "brand",
                    div { class: "brand-title",
                        TranslateLogoIcon {}
                        span { "Translation WordSwipe" }
                    }
                    button {
                        class: "btn-sidebar-collapse",
                        title: "收起侧边栏",
                        onclick: move |_| {
                            last_expanded_width.set(sidebar_width());
                            is_sidebar_collapsed.set(true);
                            if settings().remember_layout {
                                settings.write().sidebar_collapsed = true;
                                settings().save();
                            }
                        },
                        "◀"
                    }
                }

                button {
                    class: if dashboard_tab() == DashboardTab::Home { "nav-item active" } else { "nav-item" },
                    onclick: move |_| dashboard_tab.set(DashboardTab::Home),
                    span { "🏠" }
                    span { "翻译主页" }
                }

                button {
                    class: if dashboard_tab() == DashboardTab::Settings { "nav-item active" } else { "nav-item" },
                    onclick: move |_| dashboard_tab.set(DashboardTab::Settings),
                    span { "⚙" }
                    span { "全局设置" }
                }

                div { class: "sidebar-footer",
                    div { "Dioxus 0.7.10 生产稳定版" }
                    div {
                        if settings().is_translation_enabled {
                            "划词状态: 已开启"
                        } else {
                            "划词状态: 已暂停"
                        }
                    }
                }
            }

            // Draggable Splitter Handle & Quick Toggle Pill
            div {
                class: {
                    let mut cls = "sidebar-resizer".to_string();
                    if is_resizing_sidebar() {
                        cls.push_str(" active");
                    }
                    if is_sidebar_collapsed() {
                        cls.push_str(" collapsed-resizer");
                    }
                    cls
                },
                title: if is_sidebar_collapsed() { "点击展开侧边栏" } else { "拖拽调整侧栏宽度，双击快速折叠" },
                onmousedown: move |evt| {
                    evt.stop_propagation();
                    is_resizing_sidebar.set(true);
                },
                ondoubleclick: move |evt| {
                    evt.stop_propagation();
                    let collapsed = is_sidebar_collapsed();
                    if collapsed {
                        let target = last_expanded_width().max(180.0);
                        sidebar_width.set(target);
                        is_sidebar_collapsed.set(false);
                    } else {
                        last_expanded_width.set(sidebar_width());
                        is_sidebar_collapsed.set(true);
                    }
                    if settings().remember_layout {
                        settings.write().sidebar_width = sidebar_width();
                        settings.write().sidebar_collapsed = is_sidebar_collapsed();
                        settings().save();
                    }
                },
                button {
                    class: "resizer-toggle-btn",
                    title: if is_sidebar_collapsed() { "展开侧边栏" } else { "收起侧边栏" },
                    onclick: move |evt| {
                        evt.stop_propagation();
                        let collapsed = is_sidebar_collapsed();
                        if collapsed {
                            let target = last_expanded_width().max(180.0);
                            sidebar_width.set(target);
                            is_sidebar_collapsed.set(false);
                        } else {
                            last_expanded_width.set(sidebar_width());
                            is_sidebar_collapsed.set(true);
                        }
                        if settings().remember_layout {
                            settings.write().sidebar_width = sidebar_width();
                            settings.write().sidebar_collapsed = is_sidebar_collapsed();
                            settings().save();
                        }
                    },
                    if is_sidebar_collapsed() { "›" } else { "‹" }
                }
            }

            // Main Content Area
            div { class: "content-area",
                if dashboard_tab() == DashboardTab::Home {
                    // --- HOME / TRANSLATE TAB ---
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
                            h2 { class: "page-title", "即时翻译与划词响应" }
                        }
                        div { style: "display: flex; gap: 8px; align-items: center;",
                            // Pin toggle
                            button {
                                class: if is_pinned() { "btn-icon active" } else { "btn-icon" },
                                title: if is_pinned() { "窗口已固定置顶（点击取消）" } else { "固定窗口置顶（点击固定）" },
                                onclick: move |_| {
                                    let next = !is_pinned();
                                    is_pinned.set(next);
                                    crate::pinned_mode::set_pinned(&use_window(), next);
                                    settings.write().is_pinned = next;
                                    settings().save();
                                },
                                "📌"
                            }
                        }
                    }

                    div { class: "page-body",
                        // Toolbar controls
                        div { class: "controls-bar",
                            select {
                                class: "dropdown",
                                value: "{manual_from()}",
                                onchange: move |evt| manual_from.set(evt.value()),
                                option { value: "auto", "自动检测" }
                                option { value: "zh", "中文" }
                                option { value: "en", "英文" }
                                option { value: "ja", "日文" }
                                option { value: "ko", "韩文" }
                                option { value: "fr", "法语" }
                                option { value: "ru", "俄语" }
                                option { value: "es", "西班牙语" }
                                option { value: "de", "德语" }
                            }

                            button {
                                class: "btn-icon",
                                title: "互换源语言与目标语言",
                                onclick: move |_| {
                                    let f = manual_from();
                                    let t = manual_to();
                                    if f != "auto" && f != "自动" {
                                        manual_from.set(t);
                                        manual_to.set(f);
                                    } else {
                                        manual_from.set(t);
                                        manual_to.set("zh".to_string());
                                    }
                                },
                                "⇄"
                            }

                            select {
                                class: "dropdown",
                                value: "{manual_to()}",
                                onchange: move |evt| manual_to.set(evt.value()),
                                option { value: "zh", "中文" }
                                option { value: "en", "英文" }
                                option { value: "ja", "日文" }
                                option { value: "ko", "韩文" }
                                option { value: "fr", "法语" }
                                option { value: "ru", "俄语" }
                                option { value: "es", "西班牙语" }
                                option { value: "de", "德语" }
                                option { value: "ar", "阿拉伯语" }
                            }

                            button {
                                class: if settings().is_mutual { "btn-icon active" } else { "btn-icon" },
                                title: "智能互译模式 (若选中文本与目标语言一致，自动反转)",
                                onclick: move |_| {
                                    let curr = settings().is_mutual;
                                    settings.write().is_mutual = !curr;
                                    settings().save();
                                },
                                "↔"
                            }

                            div {
                                style: "display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text-muted); background: #f1f5f9; padding: 5px 12px; border-radius: var(--radius-sm); border: 1px solid var(--border);",
                                span { "⚡" }
                                span { "启用 {settings().sources.iter().filter(|s| s.enabled).count()} 个源" }
                            }

                            button {
                                class: "btn-primary",
                                onclick: move |_| {
                                    let text = manual_input();
                                    if text.trim().is_empty() { return; }
                                    let mgr = match crate::TRANSLATION_MANAGER.get() {
                                        Some(m) => m.clone(),
                                        None => Arc::new(TranslationManager::new()),
                                    };
                                    let sources = settings().sources.clone();
                                    let from = manual_from();
                                    let to = manual_to();
                                    let mutual = settings().is_mutual;
                                    manual_loading.set(true);
                                    error_msg.set(None);

                                    // 立即为所有启用的序号创建骨架占位卡片，所有序号卡片【同时全部显示】
                                    let mut unique_orders: Vec<u32> = sources.iter().filter(|s| s.enabled).map(|s| s.order).collect();
                                    unique_orders.sort_unstable();
                                    unique_orders.dedup();
                                    if unique_orders.is_empty() {
                                        unique_orders.push(1);
                                    }

                                    let initial_cards: Vec<crate::source::GroupTranslationResult> = unique_orders
                                        .into_iter()
                                        .map(|order| crate::source::GroupTranslationResult {
                                            order,
                                            engine: String::new(),
                                            used_url: String::new(),
                                            latency_ms: 0,
                                            is_loading: true,
                                            result: Ok(crate::translator::TranslationResult::default()),
                                        })
                                        .collect();
                                    group_results.set(initial_cards);

                                    // 哪个序号快就先显示哪个（流式增量刷新）
                                    let target_text = text.clone();
                                    spawn(async move {
                                        let mut stream = crate::source::execute_all_sources_streaming(&sources, mgr, &target_text, &from, &to, mutual);
                                        while let Some(res) = stream.recv().await {
                                            let mut curr = group_results.peek().clone();
                                            if let Some(pos) = curr.iter().position(|r| r.order == res.order) {
                                                curr[pos] = res;
                                            } else {
                                                curr.push(res);
                                            }
                                            curr.sort_by_key(|r| r.order);
                                            group_results.set(curr);
                                        }
                                        manual_loading.set(false);
                                    });
                                },
                                if manual_loading() { "并发翻译中..." } else { "立即翻译" }
                            }
                        }

                        // Translation Vertical Layout (原文在上，翻译结果自上而下竖向排列)
                        div { class: "translation-vertical-layout",
                            // 1. 原文输入卡片（顶部）
                            div { class: "input-card-compact",
                                div { class: "text-card-header",
                                    span { "原文（支持划词输入与手动键入）" }
                                    if !manual_input().is_empty() {
                                        button {
                                            style: "border: none; background: transparent; color: #64748b; cursor: pointer; font-size: 12px;",
                                            onclick: move |_| {
                                                manual_input.set(String::new());
                                                group_results.set(Vec::new());
                                            },
                                            "清空"
                                        }
                                    }
                                }
                                div { class: "text-card-body", style: "padding: 8px 14px;",
                                    textarea {
                                        class: "text-input",
                                        placeholder: "在此键入或在任何软件中用鼠标长按划词，内容将自动填入并执行多源并发翻译...",
                                        value: "{manual_input()}",
                                        oninput: move |evt| manual_input.set(evt.value()),
                                    }
                                }
                            }

                            // 2. 翻译结果列表（自上而下竖向排列，序号相同的源竞速取最快，序号不同的都显示）
                            div { class: "results-stack-vertical",
                                if manual_loading() && group_results().is_empty() {
                                    div { class: "result-card-item", style: "padding: 24px; text-align: center;",
                                        div { class: "loading-dots",
                                            "正在并发请求与测速中 " span {} span {} span {}
                                        }
                                    }
                                } else if group_results().is_empty() {
                                    div { class: "result-card-item", style: "padding: 28px; text-align: center; color: var(--text-muted); font-size: 13px;",
                                        "💡 划词或输入文本后点击「立即翻译」，各个序号对应的翻译结果将在此处自上而下竖向展示。"
                                    }
                                } else {
                                    for group in group_results() {
                                        {
                                            let order = group.order;
                                            let engine_name = if group.engine.contains("bing") {
                                                "必应 (Bing)".to_string()
                                            } else if group.engine.contains("web") || group.engine.contains("网页") {
                                                let method = group.result.as_ref().ok()
                                                    .and_then(|r| r.method_used.clone())
                                                    .unwrap_or_else(|| {
                                                        match crate::translator::google_web::get_google_web_mode().as_str() {
                                                            "curl" => "系统 cURL".to_string(),
                                                            "native" => "内置原生".to_string(),
                                                            _ => "并发竞速".to_string(),
                                                        }
                                                    });
                                                format!("谷歌网页 ({})", method)
                                            } else {
                                                "谷歌 (Google)".to_string()
                                            };
                                            let used_url = group.used_url.clone();
                                            let latency = group.latency_ms;
                                            let res = group.result.clone();
                                            let copy_text = res.as_ref().ok().map(|ok| ok.main_meaning.clone()).unwrap_or_default();
                                            let to_copy = copy_text.clone();
                                            let is_err = res.is_err();

                                            rsx! {
                                                if group.is_loading {
                                                    div {
                                                        key: "{order}",
                                                        class: "result-card-item loading-card",
                                                        div { class: "text-card-header",
                                                            div { style: "display: flex; align-items: center; gap: 8px;",
                                                                span { class: "badge-order", "序号 {order}" }
                                                                span { class: "badge-engine", "{engine_name}" }
                                                                span { class: "badge-loading",
                                                                    "⚡ 正在并发测速请求中..."
                                                                }
                                                            }
                                                        }
                                                        div { class: "text-card-body", style: "padding: 14px 18px;",
                                                            div { class: "skeleton-line", style: "width: 70%;" }
                                                            div { class: "skeleton-line", style: "width: 40%; margin-top: 8px;" }
                                                        }
                                                    }
                                                } else {
                                                    div {
                                                        key: "{order}",
                                                        class: if is_err { "result-card-item result-card-failed" } else { "result-card-item" },
                                                        div { class: "text-card-header",
                                                            div { style: "display: flex; align-items: center; gap: 8px; flex-wrap: wrap;",
                                                                span { class: "badge-order", "序号 {order}" }
                                                                span { class: "badge-engine", "{engine_name}" }
                                                                if latency > 0 {
                                                                    span {
                                                                        class: if is_err { "badge-speed slow" } else if latency < 600 { "badge-speed" } else { "badge-speed slow" },
                                                                        if is_err {
                                                                            "⚡ {latency}ms"
                                                                        } else {
                                                                            "⚡ {latency}ms (最快回复)"
                                                                        }
                                                                    }
                                                                }
                                                                if !used_url.is_empty() {
                                                                    span {
                                                                        class: "badge-url",
                                                                        title: if let Err(e) = &res { format!("{} (失败详情: {})", used_url, e) } else { used_url.clone() },
                                                                        "({used_url})"
                                                                    }
                                                                }
                                                                if let Err(err) = &res {
                                                                    span {
                                                                        class: "badge-fail",
                                                                        title: "{err}",
                                                                        "失败"
                                                                    }
                                                                }
                                                            }
                                                            if !copy_text.is_empty() {
                                                                button {
                                                                    style: "border: none; background: transparent; color: var(--primary); cursor: pointer; font-size: 12px; font-weight: 600;",
                                                                    onclick: move |_| {
                                                                        let _ = arboard::Clipboard::new().map(|mut cb| cb.set_text(to_copy.clone()));
                                                                    },
                                                                    "📋 复制译文"
                                                                }
                                                            }
                                                        }

                                                        if let Ok(trans) = &res {
                                                            div { class: "text-card-body",
                                                                div { class: "result-display",
                                                                    "{trans.main_meaning}"
                                                                }
                                                                if let Some(tp) = &trans.t_pronunciation {
                                                                    div { class: "phonetic", "/ {tp} /" }
                                                                }
                                                                if !trans.detailed_meanings.is_empty() {
                                                                    div { class: "dictionary-section", style: "margin-top: 10px;",
                                                                        for dm in trans.detailed_meanings.iter().take(3) {
                                                                            div { class: "dict-item",
                                                                                if let Some(pos) = &dm.pos {
                                                                                    span { class: "dict-pos", "[{pos}]" }
                                                                                }
                                                                                if let Some(m) = &dm.meaning {
                                                                                    span { "{m}" }
                                                                                }
                                                                                if !dm.synonyms.is_empty() {
                                                                                    span { class: "dict-synonyms", "({dm.synonyms.join(\", \")})" }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // --- SETTINGS TAB (彻底封装在单独文件 src/ui/settings.rs) ---
                    SettingsPage {
                        settings,
                        manual_from,
                        manual_to,
                        is_pinned,
                        is_sidebar_collapsed,
                        sidebar_width,
                        last_expanded_width,
                    }
                }
            }
        }
    }
}

#[component]
fn TranslateLogoIcon() -> Element {
    rsx! {
        svg {
            view_box: "0 0 998.1 998.3",
            style: "width: 22px; height: 22px; flex-shrink: 0; display: inline-block; vertical-align: middle; border-radius: 4px;",
            path { fill: "#DBDBDB", d: "M931.7 998.3c36.5 0 66.4-29.4 66.4-65.4V265.8c0-36-29.9-65.4-66.4-65.4H283.6l260.1 797.9h388z" }
            path { fill: "#DCDCDC", d: "M931.7 230.4c9.7 0 18.9 3.8 25.8 10.6 6.8 6.7 10.6 15.5 10.6 24.8v667.1c0 9.3-3.7 18.1-10.6 24.8-6.9 6.8-16.1 10.6-25.8 10.6H565.5L324.9 230.4h606.8m0-30H283.6l260.1 797.9h388c36.5 0 66.4-29.4 66.4-65.4V265.8c0-36-29.9-65.4-66.4-65.4z" }
            polygon { fill: "#4352B8", points: "482.3,809.8 543.7,998.3 714.4,809.8" }
            path { fill: "#607988", d: "M936.1 476.1V437H747.6v-63.2h-61.2V437H566.1v39.1h239.4c-12.8 45.1-41.1 87.7-68.7 120.8-48.9-57.9-49.1-76.7-49.1-76.7h-50.8s2.1 28.2 70.7 108.6c-22.3 22.8-39.2 36.3-39.2 36.3l15.6 48.8s23.6-20.3 53.1-51.6c29.6 32.1 67.8 70.7 117.2 116.7l32.1-32.1c-52.9-48-91.7-86.1-120.2-116.7 38.2-45.2 77-102.1 85.2-154.2H936v.1z" }
            path { fill: "#4285F4", d: "M66.4 0C29.9 0 0 29.9 0 66.5v677c0 36.5 29.9 66.4 66.4 66.4h648.1L454.4 0h-388z" }
            path { fill: "#EEEEEE", d: "M371.4 430.6c-2.5 30.3-28.4 75.2-91.1 75.2-54.3 0-98.3-44.9-98.3-100.2s44-100.2 98.3-100.2c30.9 0 51.5 13.4 63.3 24.3l41.2-39.6c-27.1-25-62.4-40.6-104.5-40.6-86.1 0-156 69.9-156 156s69.9 156 156 156c90.2 0 149.8-63.3 149.8-152.6 0-12.8-1.6-22.2-3.7-31.8h-146v53.4l91 .1z" }
        }
    }
}
