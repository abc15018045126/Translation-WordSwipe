pub const APP_CSS: &str = r#"
:root {
  --primary: #1a73e8;
  --primary-hover: #1557b0;
  --primary-light: #e8f0fe;
  --bg-main: #f8fafc;
  --bg-card: #ffffff;
  --bg-sidebar: #f1f5f9;
  --text-main: #1e293b;
  --text-muted: #64748b;
  --border: #e2e8f0;
  --border-focus: #93c5fd;
  --shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -2px rgba(0, 0, 0, 0.1);
  --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -4px rgba(0, 0, 0, 0.1);
  --radius-sm: 6px;
  --radius-md: 10px;
  --radius-lg: 14px;
}

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
  user-select: none;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", sans-serif;
}

body {
  background: var(--bg-main);
  color: var(--text-main);
  overflow: hidden;
  height: 100vh;
  width: 100vw;
}

/* App Container */
.app-container {
  display: flex;
  height: 100vh;
  width: 100vw;
  background: var(--bg-main);
  position: relative;
  overflow: hidden;
}

.app-container.resizing {
  user-select: none !important;
  cursor: col-resize !important;
}

/* Sidebar */
.sidebar {
  background: var(--bg-sidebar);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  padding: 16px 12px;
  gap: 8px;
  height: 100vh;
  flex-shrink: 0;
  overflow: hidden;
  white-space: nowrap;
  transition: width 0.18s cubic-bezier(0.4, 0, 0.2, 1), padding 0.18s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.18s ease;
  position: relative;
}

.sidebar.resizing {
  transition: none !important;
}

.sidebar.collapsed {
  width: 0px !important;
  min-width: 0px !important;
  padding-left: 0px !important;
  padding-right: 0px !important;
  border-right: none !important;
  opacity: 0 !important;
  pointer-events: none !important;
}

.brand {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 6px 16px 6px;
  font-size: 15px;
  font-weight: 700;
  color: var(--primary);
  border-bottom: 1px solid var(--border);
  margin-bottom: 8px;
  flex-shrink: 0;
}

.brand-title {
  display: flex;
  align-items: center;
  gap: 8px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.brand-icon {
  font-size: 20px;
  flex-shrink: 0;
}

.btn-sidebar-collapse {
  background: transparent;
  border: 1px solid transparent;
  cursor: pointer;
  font-size: 13px;
  color: var(--text-muted);
  width: 28px;
  height: 28px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.btn-sidebar-collapse:hover {
  background: var(--primary-light);
  color: var(--primary);
  border-color: var(--border-focus);
}

.btn-expand-sidebar {
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--primary);
  border-radius: var(--radius-sm);
  padding: 6px 12px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  transition: all 0.15s ease;
  box-shadow: var(--shadow-sm);
}

.btn-expand-sidebar:hover {
  background: var(--primary-light);
  border-color: var(--border-focus);
  color: var(--primary-hover);
}

.btn-icon {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  width: 34px;
  height: 34px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  font-size: 16px;
  color: var(--text-main);
  box-shadow: var(--shadow-sm);
  transition: all 0.15s ease;
  user-select: none;
}

.btn-icon:hover {
  background: var(--primary-light);
  border-color: var(--border-focus);
  color: var(--primary);
  transform: translateY(-1px);
  box-shadow: var(--shadow-md);
}

.btn-icon.active {
  background: #e0f2fe;
  border-color: #0284c7;
  color: #0369a1;
  box-shadow: 0 0 0 2px rgba(2, 132, 199, 0.28);
  transform: translateY(0);
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  border-radius: var(--radius-md);
  font-size: 14px;
  font-weight: 500;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.15s ease;
  border: none;
  background: transparent;
  width: 100%;
  text-align: left;
}

.nav-item:hover {
  background: rgba(0, 0, 0, 0.04);
  color: var(--text-main);
}

.nav-item.active {
  background: var(--primary-light);
  color: var(--primary);
  font-weight: 600;
}

.sidebar-footer {
  margin-top: auto;
  padding: 12px 8px;
  font-size: 12px;
  color: var(--text-muted);
  border-top: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex-shrink: 0;
}

/* Sidebar Resizer (Draggable Splitter Handle) */
.sidebar-resizer {
  width: 10px;
  margin-left: -5px;
  margin-right: -5px;
  z-index: 25;
  cursor: col-resize;
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  flex-shrink: 0;
  user-select: none;
  transition: background 0.15s ease;
}

.sidebar-resizer:hover,
.sidebar-resizer.active {
  background: rgba(26, 115, 232, 0.16);
}

.sidebar-resizer::after {
  content: "";
  width: 2px;
  height: 100%;
  background: var(--border);
  transition: background 0.15s ease;
}

.sidebar-resizer:hover::after,
.sidebar-resizer.active::after {
  background: var(--primary);
}

/* Resizer Toggle Button */
.resizer-toggle-btn {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  width: 18px;
  height: 40px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 9px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  font-size: 13px;
  font-weight: 700;
  color: var(--text-muted);
  box-shadow: var(--shadow-md);
  transition: all 0.15s ease;
  z-index: 30;
  padding: 0;
  user-select: none;
}

.resizer-toggle-btn:hover {
  background: var(--primary-light);
  color: var(--primary);
  border-color: var(--primary);
  transform: translateY(-50%) scale(1.1);
}

.sidebar-resizer.collapsed-resizer {
  margin-left: 0;
  width: 12px;
  cursor: pointer;
}

/* Main Content Area */
.content-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  background: var(--bg-main);
  min-width: 0;
}

.page-header {
  padding: 20px 28px 12px 28px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.page-title {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-main);
}

.page-body {
  padding: 12px 28px 28px 28px;
  display: flex;
  flex-direction: column;
  gap: 20px;
  flex: 1;
}

/* Controls Bar */
.controls-bar {
  display: flex;
  align-items: center;
  gap: 12px;
  background: var(--bg-card);
  padding: 10px 16px;
  border-radius: var(--radius-md);
  border: 1px solid var(--border);
  box-shadow: var(--shadow-sm);
}

select.dropdown {
  padding: 6px 12px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  background: #fff;
  font-size: 13px;
  color: var(--text-main);
  outline: none;
  cursor: pointer;
}

select.dropdown:focus {
  border-color: var(--primary);
}

button.btn-icon {
  background: #fff;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  font-size: 14px;
  color: var(--text-main);
  transition: all 0.15s ease;
}

button.btn-icon:hover {
  background: var(--bg-sidebar);
  border-color: #cbd5e1;
}

button.btn-icon.active {
  background: var(--primary-light);
  color: var(--primary);
  border-color: var(--primary);
}

button.btn-primary {
  background: var(--primary);
  color: white;
  border: none;
  border-radius: var(--radius-sm);
  padding: 7px 18px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s ease;
  margin-left: auto;
}

button.btn-primary:hover {
  background: var(--primary-hover);
}

/* Translation Panels */
.translation-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  flex: 1;
  min-height: 280px;
}

.text-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: var(--shadow-sm);
}

.text-card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 14px;
  border-bottom: 1px solid var(--border);
  background: #fafafa;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-muted);
}

.text-card-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 14px;
}

textarea.text-input {
  width: 100%;
  height: 100%;
  border: none;
  outline: none;
  resize: none;
  font-size: 14px;
  line-height: 1.6;
  color: var(--text-main);
  background: transparent;
  user-select: text;
}

.result-display {
  flex: 1;
  font-size: 14px;
  line-height: 1.6;
  color: var(--text-main);
  user-select: text;
  white-space: pre-wrap;
  word-break: break-word;
  overflow-y: auto;
}

.phonetic {
  margin-top: 8px;
  font-size: 12px;
  color: var(--primary);
  font-style: italic;
  user-select: text;
}

.text-card-body,
.text-card-body * {
  user-select: text;
}

/* Dictionary Details */
.dictionary-section {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 14px;
  font-size: 13px;
  box-shadow: var(--shadow-sm);
}

.dict-pos {
  font-weight: 700;
  color: var(--primary);
  margin-top: 6px;
  margin-bottom: 4px;
}

.dict-item {
  display: flex;
  gap: 8px;
  margin-bottom: 4px;
  color: var(--text-main);
}

.dict-synonyms {
  color: var(--text-muted);
  font-size: 12px;
}

/* Settings Form */
.settings-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
  background: var(--bg-card);
  padding: 20px;
  border-radius: var(--radius-md);
  border: 1px solid var(--border);
  box-shadow: var(--shadow-sm);
}

.setting-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-bottom: 14px;
  border-bottom: 1px solid #f1f5f9;
}

.setting-row:last-child {
  border-bottom: none;
  padding-bottom: 0;
}

.setting-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.setting-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-main);
}

.setting-desc {
  font-size: 12px;
  color: var(--text-muted);
}

.input-text {
  padding: 7px 12px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  font-size: 13px;
  outline: none;
  width: 260px;
  user-select: text;
}

.input-text:focus {
  border-color: var(--primary);
}

/* 模式切换分段按钮组 (谷歌网页设置等) */
.mode-btn-group {
  display: inline-flex;
  border-radius: var(--radius-sm);
  background: #f1f5f9;
  padding: 3px;
  gap: 4px;
  border: 1px solid var(--border);
}

.mode-btn {
  padding: 6px 14px;
  font-size: 12px;
  font-weight: 500;
  border-radius: 4px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.15s ease;
  white-space: nowrap;
}

.mode-btn:hover {
  color: var(--text-main);
  background: rgba(255, 255, 255, 0.6);
}

.mode-btn.active {
  background: var(--primary);
  color: #ffffff;
  font-weight: 600;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
}

/* Toggle Switch */
.switch {
  position: relative;
  display: inline-block;
  width: 44px;
  height: 24px;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.slider {
  position: absolute;
  cursor: pointer;
  top: 0; left: 0; right: 0; bottom: 0;
  background-color: #cbd5e1;
  transition: .3s;
  border-radius: 24px;
}

.slider:before {
  position: absolute;
  content: "";
  height: 18px;
  width: 18px;
  left: 3px;
  bottom: 3px;
  background-color: white;
  transition: .3s;
  border-radius: 50%;
}

input:checked + .slider {
  background-color: var(--primary);
}

input:checked + .slider:before {
  transform: translateX(20px);
}

/* Floating Popup Mode Styling */
.popup-mode {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  background: #ffffff;
  border: 1px solid var(--border);
  box-shadow: var(--shadow-lg);
}

.popup-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #f8fafc;
  border-bottom: 1px solid var(--border);
  height: 48px;
}

.popup-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 10px 12px;
  gap: 8px;
  overflow-y: auto;
}

.popup-source-box {
  min-height: 50px;
  max-height: 110px;
  padding: 8px;
  background: #f8fafc;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-size: 13px;
  color: var(--text-muted);
  overflow-y: auto;
  user-select: text;
  line-height: 1.4;
}

.popup-target-box {
  flex: 1;
  min-height: 80px;
  padding: 10px;
  background: #ffffff;
  border: 1px solid #bfdbfe;
  border-radius: var(--radius-sm);
  font-size: 14px;
  color: var(--text-main);
  overflow-y: auto;
  user-select: text;
  line-height: 1.5;
  font-weight: 500;
}

/* Loading animation */
.loading-dots {
  display: inline-flex;
  gap: 4px;
  align-items: center;
}

.loading-dots span {
  width: 6px;
  height: 6px;
  background: var(--primary);
  border-radius: 50%;
  animation: pulse 1s infinite alternate;
}

.loading-dots span:nth-child(2) { animation-delay: 0.2s; }
.loading-dots span:nth-child(3) { animation-delay: 0.4s; }

@keyframes pulse {
  0% { transform: scale(0.6); opacity: 0.4; }
  100% { transform: scale(1.1); opacity: 1; }
}

/* Translation Vertical Layout */
.translation-vertical-layout {
  display: flex;
  flex-direction: column;
  gap: 16px;
  flex: 1;
  min-height: 0;
}

.input-card-compact {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-sm);
  flex-shrink: 0;
  overflow: hidden;
}

.input-card-compact textarea.text-input {
  min-height: 80px;
  max-height: 160px;
  padding: 12px 14px;
}

.results-stack-vertical {
  display: flex;
  flex-direction: column;
  gap: 14px;
  flex: 1;
  padding-bottom: 20px;
}

.result-card-item {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-sm);
  overflow: hidden;
  transition: all 0.15s ease;
}

.result-card-item:hover {
  border-color: #cbd5e1;
  box-shadow: var(--shadow-md);
}

.badge-order {
  background: #eff6ff;
  color: var(--primary);
  font-weight: 700;
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
  border: 1px solid #bfdbfe;
  display: inline-flex;
  align-items: center;
}

.badge-engine {
  background: #f8fafc;
  color: #334155;
  font-weight: 600;
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
  border: 1px solid #e2e8f0;
  display: inline-flex;
  align-items: center;
}

.badge-speed {
  font-size: 11px;
  color: #10b981;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  gap: 3px;
}

.badge-speed.slow {
  color: #f59e0b;
}

.badge-url {
  font-size: 11px;
  color: var(--text-muted);
  max-width: 280px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.badge-fail {
  font-size: 11px;
  color: #dc2626;
  background: #fee2e2;
  border: 1px solid #fecaca;
  padding: 1px 6px;
  border-radius: 4px;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  cursor: help;
}

.result-card-item.loading-card {
  border: 1px dashed #cbd5e1;
  background: #fbfcfe;
}

.badge-loading {
  font-size: 11px;
  color: var(--primary);
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.skeleton-line {
  height: 14px;
  background: linear-gradient(90deg, #f1f5f9 25%, #e2e8f0 50%, #f1f5f9 75%);
  background-size: 200% 100%;
  animation: shimmer 1.5s infinite;
  border-radius: 4px;
  margin-top: 6px;
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}

/* Sources List in Settings */
.sources-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-top: 8px;
}

.source-item-row {
  display: flex;
  align-items: center;
  gap: 10px;
  background: #ffffff;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 10px 14px;
  transition: all 0.15s ease;
}

.source-item-row:hover {
  border-color: #cbd5e1;
  box-shadow: var(--shadow-sm);
}

.source-order-input {
  width: 65px;
  padding: 6px 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-size: 13px;
  font-weight: 700;
  text-align: center;
  color: var(--primary);
  outline: none;
  background: #f8fafc;
}

.source-order-input:focus {
  border-color: var(--primary);
  background: #fff;
}

.source-engine-select {
  width: 120px;
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-size: 13px;
  outline: none;
  background: #fff;
  cursor: pointer;
  color: var(--text-main);
}

.source-engine-select:focus {
  border-color: var(--primary);
}

.source-url-input {
  flex: 1;
  min-width: 180px;
  padding: 6px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-size: 13px;
  outline: none;
  color: var(--text-main);
}

.source-url-input:focus {
  border-color: var(--primary);
}

.btn-danger-icon {
  background: transparent;
  border: 1px solid transparent;
  color: #ef4444;
  cursor: pointer;
  padding: 5px 8px;
  border-radius: var(--radius-sm);
  font-size: 13px;
  transition: all 0.15s ease;
}

.btn-danger-icon:hover {
  background: #fef2f2;
  border-color: #fecaca;
}

.btn-add-source {
  background: #ffffff;
  border: 1px dashed var(--primary);
  color: var(--primary);
  border-radius: var(--radius-sm);
  padding: 9px 16px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  transition: all 0.15s ease;
  margin-top: 6px;
}

.btn-add-source:hover {
  background: var(--primary-light);
}
"#;
