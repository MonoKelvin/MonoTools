//! Python 进程管理 - 负责 Python 子进程的生命周期和 JSON-RPC 通信

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, Mutex};

use super::config::PyBridgeConfig;
use super::jsonrpc::{JsonRpcRequest, JsonRpcResponse};
use super::types::{PyBridgeError, PyBridgeResult};

type PendingMap = HashMap<u64, oneshot::Sender<PyBridgeResult<serde_json::Value>>>;

/// Python 进程管理器
pub struct PythonProcess {
    config: PyBridgeConfig,
    child: Option<Child>,
    request_tx: mpsc::Sender<JsonRpcRequest>,
    next_id: Arc<AtomicU64>,
    pending: Arc<Mutex<PendingMap>>,
    stop_tx: Option<mpsc::Sender<()>>,
}

impl PythonProcess {
    /// 启动 Python 进程
    pub async fn start(config: &PyBridgeConfig) -> PyBridgeResult<Self> {
        log::debug!(
            "[pybridge] 启动 Python 进程: {} {}",
            config.python_path,
            config.script_path
        );

        let mut child = Command::new(&config.python_path)
            .arg(&config.script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| PyBridgeError::StartFailed(e.to_string()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| PyBridgeError::StartFailed("无法获取 stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| PyBridgeError::StartFailed("无法获取 stdout".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| PyBridgeError::StartFailed("无法获取 stderr".to_string()))?;

        let (request_tx, mut request_rx) = mpsc::channel::<JsonRpcRequest>(64);
        let pending: Arc<Mutex<PendingMap>> = Arc::new(Mutex::new(HashMap::new()));
        let next_id = Arc::new(AtomicU64::new(1));
        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);

        // 写请求的任务
        let pending_clone = Arc::clone(&pending);
        let next_id_clone = Arc::clone(&next_id);
        let mut stdin_writer = tokio::io::BufWriter::new(stdin);
        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            loop {
                tokio::select! {
                    Some(req) = request_rx.recv() => {
                        match req.to_line() {
                            Ok(line) => {
                                if let Err(e) = stdin_writer.write_all(line.as_bytes()).await {
                                    log::error!("[pybridge] 写入 stdin 失败: {}", e);
                                    break;
                                }
                                if let Err(e) = stdin_writer.flush().await {
                                    log::error!("[pybridge] flush stdin 失败: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                log::error!("[pybridge] 序列化请求失败: {}", e);
                            }
                        }
                    }
                    _ = stop_rx.recv() => {
                        log::debug!("[pybridge] 请求写入任务收到停止信号");
                        break;
                    }
                }
            }
            let _ = pending_clone;
            let _ = next_id_clone;
        });

        // 读响应的任务
        let pending_clone = Arc::clone(&pending);
        let mut stdout_reader = BufReader::new(stdout).lines();
        tokio::spawn(async move {
            loop {
                match stdout_reader.next_line().await {
                    Ok(Some(line)) => {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        match JsonRpcResponse::from_line(line) {
                            Ok(response) => {
                                let id = response.id();
                                let mut pending = pending_clone.lock().await;
                                if let Some(tx) = pending.remove(&id) {
                                    let result = match response {
                                        JsonRpcResponse::Success(s) => Ok(s.result),
                                        JsonRpcResponse::Error(e) => {
                                            Err(PyBridgeError::RpcError {
                                                code: e.error.code,
                                                message: e.error.message,
                                            })
                                        }
                                    };
                                    let _ = tx.send(result);
                                }
                            }
                            Err(e) => {
                                log::warn!("[pybridge] 解析响应失败: {} (line: {})", e, line);
                            }
                        }
                    }
                    Ok(None) => {
                        log::info!("[pybridge] Python stdout 已关闭");
                        break;
                    }
                    Err(e) => {
                        log::error!("[pybridge] 读取 stdout 失败: {}", e);
                        break;
                    }
                }
            }
        });

        // 读 stderr 的任务
        tokio::spawn(async move {
            let mut stderr_reader = BufReader::new(stderr).lines();
            loop {
                match stderr_reader.next_line().await {
                    Ok(Some(line)) => {
                        log::info!("[py:stderr] {}", line);
                    }
                    Ok(None) => {
                        log::info!("[pybridge] Python stderr 已关闭");
                        break;
                    }
                    Err(e) => {
                        log::error!("[pybridge] 读取 stderr 失败: {}", e);
                        break;
                    }
                }
            }
        });

        // 等待健康检查确认进程就绪
        let process = Self {
            config: config.clone(),
            child: Some(child),
            request_tx,
            next_id,
            pending,
            stop_tx: Some(stop_tx),
        };

        // 发送健康检查
        let timeout = tokio::time::Duration::from_millis(config.startup_timeout_ms);
        match tokio::time::timeout(timeout, process.health_check()).await {
            Ok(Ok(_)) => {
                log::info!("[pybridge] Python 进程就绪");
            }
            Ok(Err(e)) => {
                log::warn!("[pybridge] 健康检查失败: {}", e);
            }
            Err(_) => {
                log::warn!("[pybridge] 启动超时 ({}ms)", config.startup_timeout_ms);
            }
        }

        Ok(process)
    }

    /// 发送 JSON-RPC 请求并等待响应
    pub async fn call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> PyBridgeResult<serde_json::Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest::new(id, method, params);

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id, tx);
        }

        self.request_tx
            .send(request)
            .await
            .map_err(|e| PyBridgeError::Other(format!("发送请求失败: {}", e)))?;

        // 使用配置的超时时间
        let timeout = tokio::time::Duration::from_millis(self.config.request_timeout_ms);
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(PyBridgeError::Other("响应通道已关闭".to_string())),
            Err(_) => {
                let mut pending = self.pending.lock().await;
                pending.remove(&id);
                Err(PyBridgeError::Timeout)
            }
        }
    }

    /// 健康检查
    async fn health_check(&self) -> PyBridgeResult<()> {
        let result = self
            .call("health_check", serde_json::json!({}))
            .await?;
        if result.get("status").and_then(|v| v.as_str()) == Some("ok") {
            Ok(())
        } else {
            Err(PyBridgeError::Other("健康检查返回异常".to_string()))
        }
    }

    /// 停止进程
    pub async fn stop(&mut self) {
        log::info!("[pybridge] 正在停止 Python 进程...");

        // 通知写入任务停止
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(()).await;
        }

        // 清理 pending 请求
        let mut pending = self.pending.lock().await;
        for (_, tx) in pending.drain() {
            let _ = tx.send(Err(PyBridgeError::Other("进程已停止".to_string())));
        }
        drop(pending);

        // 终止子进程
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }

        log::info!("[pybridge] Python 进程已停止");
    }
}

impl Drop for PythonProcess {
    fn drop(&mut self) {
        if self.child.is_some() {
            log::warn!("[pybridge] PythonProcess dropped without explicit stop");
        }
    }
}
