use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};
use tokio::sync::mpsc;

pub const PIPE_NAME: &str = r"\\.\pipe\translation_wordswipe_ipc";

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "data")]
pub enum IpcMessage {
    Selection { text: String, x: i32, y: i32 },
    ShowWindow,
    ToggleTranslation { enabled: bool },
    SetOcrTrigger { enabled: bool },
    UiReady,
}

/// 尝试向已存在的后台 Daemon 发送命令（如双击 exe 时请求唤醒已有窗口）
pub fn try_send_command(msg: &IpcMessage) -> bool {
    let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(r) => r,
        Err(_) => return false,
    };

    rt.block_on(async {
        let client = match ClientOptions::new().open(PIPE_NAME) {
            Ok(c) => c,
            Err(_) => return false,
        };

        let mut lines_client = client;
        let serialized = match serde_json::to_string(msg) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let data = format!("{}\n", serialized);
        let write_res = tokio::time::timeout(
            Duration::from_millis(1500),
            lines_client.write_all(data.as_bytes())
        ).await;

        match write_res {
            Ok(Ok(_)) => {
                let _ = lines_client.flush().await;
                true
            }
            _ => false,
        }
    })
}

/// UI 客户端连接到 Daemon 的 Named Pipe
pub async fn connect_ui_client() -> Result<(mpsc::UnboundedSender<IpcMessage>, mpsc::UnboundedReceiver<IpcMessage>), Box<dyn std::error::Error + Send + Sync>> {
    let client = ClientOptions::new().open(PIPE_NAME)?;
    let (read_half, mut write_half) = tokio::io::split(client);

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<IpcMessage>();
    let (in_tx, in_rx) = mpsc::unbounded_channel::<IpcMessage>();

    // 写入任务：将 out_rx 收到的消息发送给 Named Pipe
    tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if let Ok(line) = serde_json::to_string(&msg) {
                let data = format!("{}\n", line);
                if write_half.write_all(data.as_bytes()).await.is_err() {
                    break;
                }
                let _ = write_half.flush().await;
            }
        }
    });

    // 读取任务：从 Named Pipe 读取 Daemon 发来的消息
    tokio::spawn(async move {
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF, Daemon 或连接关闭
                Ok(_) => {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        if let Ok(msg) = serde_json::from_str::<IpcMessage>(trimmed) {
                            if in_tx.send(msg).is_err() {
                                break;
                            }
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    // 发送 UI 就绪确认
    let _ = out_tx.send(IpcMessage::UiReady);
    let _ = UI_IPC_TX.set(out_tx.clone());

    Ok((out_tx, in_rx))
}

pub static UI_IPC_TX: std::sync::OnceLock<mpsc::UnboundedSender<IpcMessage>> = std::sync::OnceLock::new();

pub fn send_to_daemon(msg: IpcMessage) {
    if let Some(tx) = UI_IPC_TX.get() {
        let _ = tx.send(msg);
    }
}

/// 启动后台 Daemon Named Pipe 服务
pub fn start_daemon_server() -> (mpsc::UnboundedSender<IpcMessage>, mpsc::UnboundedReceiver<IpcMessage>) {
    let (to_ui_tx, to_ui_rx) = mpsc::unbounded_channel::<IpcMessage>();
    let (from_ui_tx, from_ui_rx) = mpsc::unbounded_channel::<IpcMessage>();

    tokio::spawn(async move {
        run_server_loop(to_ui_rx, from_ui_tx).await;
    });

    (to_ui_tx, from_ui_rx)
}

async fn run_server_loop(
    mut send_to_ui_rx: mpsc::UnboundedReceiver<IpcMessage>,
    recv_from_ui_tx: mpsc::UnboundedSender<IpcMessage>,
) {
    let mut is_first = true;
    // 维护当前活跃的 UI 连接写通道
    let active_client_tx: std::sync::Arc<tokio::sync::Mutex<Option<mpsc::UnboundedSender<IpcMessage>>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(None));

    // 广播任务：把 Daemon 想发给 UI 的消息投递给当前已连接的 UI
    let active_client_clone = active_client_tx.clone();
    tokio::spawn(async move {
        while let Some(msg) = send_to_ui_rx.recv().await {
            let mut guard = active_client_clone.lock().await;
            if let Some(tx) = guard.as_ref() {
                if tx.send(msg).is_err() {
                    *guard = None;
                }
            }
        }
    });

    loop {
        let server_res = if is_first {
            is_first = false;
            ServerOptions::new().first_pipe_instance(true).create(PIPE_NAME)
        } else {
            ServerOptions::new().create(PIPE_NAME)
        };

        let server = match server_res {
            Ok(s) => s,
            Err(e) => {
                eprintln!("NamedPipeServer create failed: {:?}", e);
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
        };

        if server.connect().await.is_err() {
            continue;
        }

        let (read_half, mut write_half) = tokio::io::split(server);
        let (client_write_tx, mut client_write_rx) = mpsc::unbounded_channel::<IpcMessage>();

        // 注册当前活跃 UI 客户端
        {
            let mut guard = active_client_tx.lock().await;
            *guard = Some(client_write_tx);
        }

        let active_ref = active_client_tx.clone();
        // 客户端写入任务
        tokio::spawn(async move {
            while let Some(msg) = client_write_rx.recv().await {
                if let Ok(line) = serde_json::to_string(&msg) {
                    let data = format!("{}\n", line);
                    if write_half.write_all(data.as_bytes()).await.is_err() {
                        break;
                    }
                    let _ = write_half.flush().await;
                }
            }
        });

        // 客户端读取任务
        let to_daemon_tx = recv_from_ui_tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(read_half);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // UI 进程退出或断开连接
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            if let Ok(msg) = serde_json::from_str::<IpcMessage>(trimmed) {
                                let _ = to_daemon_tx.send(msg);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            // 断开后清空活跃引用
            let mut guard = active_ref.lock().await;
            *guard = None;
        });
    }
}
