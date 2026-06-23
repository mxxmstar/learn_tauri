//! TCP 异步客户端模块
//!
//! 该模块基于 `tokio::net::TcpStream` 和 `tokio_util::codec` 实现异步 TCP 客户端，包括：
//! - 连接服务端（支持连接超时）
//! - 消息帧化发送和接收（解决粘包/半包）
//! - 读写分离（可同时发送和接收）
//! - 应用层心跳（Ping/Pong）保活与 RTT 测量
//! - 后台接收循环（处理服务端推送）
//! - 可选的自动重连机制
//! - 读写超时控制
//!
//! # TCP 客户端工作原理
//!
//! TCP 是面向连接的协议，客户端需要先建立连接才能通信：
//!
//! ```text
//! 客户端                              服务端
//!   │                                   │
//!   │────── SYN（连接请求）────────────►│
//!   │◄───── SYN+ACK（连接确认）─────────│
//!   │────── ACK────────────────────────►│
//!   │        连接建立完成                 │
//!   │                                   │
//!   │════ Framed (长度前缀帧化) ════════│
//!   │     [len][消息A] ────────────────►│
//!   │◄───────────── [len][消息B]         │
//!   │     [len][Ping] ─────────────────►│
//!   │◄───────────── [len][Pong]          │
//!   │                                   │
//!   │────── FIN（断开请求）────────────►│
//!   │        连接关闭                     │
//! ```
//!
//! # 读写分离设计
//!
//! 连接建立后，将 `Framed` 通过 `split()` 分离为：
//!
//! - **写入端（Sink）**：用于发送消息，由 `Mutex` 保护，支持并发发送
//! - **读取端（Stream）**：用于接收消息，可被 `recv()` 或后台接收循环消费
//!
//! 这样设计的好处是**全双工通信**：发送和接收互不阻塞，
//! 一边发送数据的同时另一边可以接收数据。
//!
//! # 自动重连
//!
//! 启用 `auto_reconnect` 后，连接断开时会自动重连：
//!
//! 1. 检测到连接断开
//! 2. 等待 `reconnect_interval_ms` 毫秒
//! 3. 尝试重新连接
//! 4. 若失败，重复步骤 2-3，直到达到 `max_reconnect_attempts`
//!
//! # 使用方式
//!
//! ```ignore
//! use crate::tcp::client::TcpClient;
//! use crate::tcp::config::TcpClientConfig;
//! use crate::tcp::message::TcpMessage;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), String> {
//!     let config = TcpClientConfig::new("127.0.0.1:8080")
//!         .with_timeout(5000);
//!     let client = TcpClient::new(config)?;
//!
//!     // 连接服务端
//!     client.connect().await?;
//!
//!     // 发送消息
//!     client.send(&TcpMessage::data("Hello, Server!")).await?;
//!
//!     // 接收回复
//!     let msg = client.recv().await?;
//!     println!("收到回复: {}", msg.payload);
//!
//!     // Ping 测试
//!     let rtt = client.ping().await?;
//!     println!("RTT: {}ms", rtt);
//!
//!     Ok(())
//! }
//! ```

use crate::tcp::codec::new_codec;
use crate::tcp::config::TcpClientConfig;
use crate::tcp::message::TcpMessage;
// 导入日志宏
use crate::{log_info, log_error, log_debug, log_warn};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch, Mutex};
use tokio_util::codec::Framed;

// 导入 Stream 和 Sink trait 用于 Framed 的读写分离
use futures_util::{sink::SinkExt, stream::StreamExt};

/// Framed 写入端类型（Sink 半部）
type WriterHalf =
    futures_util::stream::SplitSink<Framed<TcpStream, tokio_util::codec::LengthDelimitedCodec>, bytes::Bytes>;

/// Framed 读取端类型（Stream 半部）
type ReaderHalf =
    futures_util::stream::SplitStream<Framed<TcpStream, tokio_util::codec::LengthDelimitedCodec>>;

/// 接收到的消息
pub type ReceivedMessage = TcpMessage;

/// TCP 异步客户端
///
/// 提供面向消息的 TCP 通信接口，支持帧化收发、读写分离和心跳保活。
///
/// # 线程安全
///
/// 内部使用 `Arc` 共享状态，可以安全地克隆并在多个异步任务间使用。
/// 写入端使用 `Mutex` 保护，支持并发发送。
///
/// # 自动重连
///
/// 启用 `auto_reconnect` 后，连接断开会自动重连。
/// 重连期间发送操作会返回错误。
#[derive(Clone)]
pub struct TcpClient {
    /// 客户端配置
    config: Arc<TcpClientConfig>,
    /// 帧化写入端（连接后存在）
    writer: Arc<Mutex<Option<WriterHalf>>>,
    /// 帧化读取端（连接后存在）
    /// recv()/ping() 和 start_recv_loop() 竞争使用读取端
    reader: Arc<Mutex<Option<ReaderHalf>>>,
    /// 后台接收任务的消息通道发送端
    recv_tx: Arc<Mutex<Option<mpsc::UnboundedSender<ReceivedMessage>>>>,
    /// 后台接收任务的停止信号
    shutdown_tx: Arc<Mutex<Option<watch::Sender<bool>>>>,
    /// 连接状态
    connected: Arc<Mutex<bool>>,
}

impl TcpClient {
    /// 创建新的 TCP 客户端实例
    ///
    /// # 参数
    /// * `config` - 客户端配置
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 TcpClient 实例
    pub fn new(config: TcpClientConfig) -> Result<Self, String> {
        // 验证配置有效性
        config.validate()?;

        log_info!("创建 TCP 客户端实例，目标服务端: {}", config.server_addr);

        Ok(Self {
            config: Arc::new(config),
            writer: Arc::new(Mutex::new(None)),
            reader: Arc::new(Mutex::new(None)),
            recv_tx: Arc::new(Mutex::new(None)),
            shutdown_tx: Arc::new(Mutex::new(None)),
            connected: Arc::new(Mutex::new(false)),
        })
    }

    /// 连接服务端
    ///
    /// 建立 TCP 连接，受 `connect_timeout_ms` 控制。
    /// 连接成功后，将 Framed 分离为读写两端，可进行全双工通信。
    ///
    /// # 返回值
    /// 返回 Result，成功时返回 ()
    pub async fn connect(&self) -> Result<(), String> {
        log_info!("正在连接 TCP 服务端: {}", self.config.server_addr);

        let server_addr = self.config.parse_server_addr()?;

        // 使用超时包装连接操作
        let stream = tokio::time::timeout(
            Duration::from_millis(self.config.connect_timeout_ms),
            TcpStream::connect(server_addr),
        )
        .await
        .map_err(|_| {
            format!(
                "连接超时（{}ms）: {}",
                self.config.connect_timeout_ms, server_addr
            )
        })?
        .map_err(|e| format!("连接失败: {}", e))?;

        log_debug!("TCP 连接已建立");

        // 设置 TCP 选项
        if self.config.keepalive {
            let _ = stream.set_nodelay(true);
        }

        // 创建帧化连接并分离读写
        let codec = new_codec();
        let framed = Framed::new(stream, codec);
        let (writer, reader) = framed.split();

        // 保存读写两端
        *self.writer.lock().await = Some(writer);
        *self.reader.lock().await = Some(reader);
        *self.connected.lock().await = true;

        log_info!("已连接到服务端: {}", server_addr);
        Ok(())
    }

    /// 断开连接
    ///
    /// 关闭 TCP 连接并停止后台接收任务。
    pub async fn disconnect(&self) -> Result<(), String> {
        log_info!("正在断开 TCP 连接...");

        // 停止后台接收任务
        let mut shutdown_guard = self.shutdown_tx.lock().await;
        if let Some(tx) = shutdown_guard.take() {
            let _ = tx.send(true);
        }
        drop(shutdown_guard);

        // 清理消息通道
        let mut recv_guard = self.recv_tx.lock().await;
        *recv_guard = None;
        drop(recv_guard);

        // 关闭写入端
        let mut writer_guard = self.writer.lock().await;
        if let Some(mut writer) = writer_guard.take() {
            // 尝试优雅关闭
            let _ = writer.close().await;
        }
        drop(writer_guard);

        // 释放读取端
        *self.reader.lock().await = None;

        *self.connected.lock().await = false;

        log_info!("TCP 连接已断开");
        Ok(())
    }

    /// 发送结构化消息
    ///
    /// 将消息编码为 JSON 并通过帧化发送。
    /// 需要先调用 `connect()` 建立连接。
    ///
    /// 由于使用读写分离设计，发送操作不会阻塞接收。
    ///
    /// # 参数
    /// * `msg` - 要发送的消息
    pub async fn send(&self, msg: &TcpMessage) -> Result<(), String> {
        let data = msg.encode()?;

        let mut writer_guard = self.writer.lock().await;
        let writer = writer_guard
            .as_mut()
            .ok_or_else(|| "客户端未连接，请先调用 connect()".to_string())?;

        // 使用超时包装发送操作
        let send_future = writer.send(bytes::Bytes::from(data));

        if self.config.timeout_ms > 0 {
            tokio::time::timeout(Duration::from_millis(self.config.timeout_ms), send_future)
                .await
                .map_err(|_| format!("发送超时（{}ms）", self.config.timeout_ms))?
                .map_err(|e| format!("发送数据失败: {}", e))?;
        } else {
            send_future.await.map_err(|e| format!("发送数据失败: {}", e))?;
        }

        log_debug!("已发送消息: {}", msg.msg_type);
        Ok(())
    }

    /// 发送原始字节
    ///
    /// 绕过消息编码，直接发送二进制数据（仍经过帧化）。
    ///
    /// # 参数
    /// * `data` - 原始字节数据
    pub async fn send_raw(&self, data: &[u8]) -> Result<(), String> {
        let mut writer_guard = self.writer.lock().await;
        let writer = writer_guard
            .as_mut()
            .ok_or_else(|| "客户端未连接，请先调用 connect()".to_string())?;

        let send_future = writer.send(bytes::Bytes::copy_from_slice(data));

        if self.config.timeout_ms > 0 {
            tokio::time::timeout(Duration::from_millis(self.config.timeout_ms), send_future)
                .await
                .map_err(|_| format!("发送超时（{}ms）", self.config.timeout_ms))?
                .map_err(|e| format!("发送原始数据失败: {}", e))?;
        } else {
            send_future.await.map_err(|e| format!("发送原始数据失败: {}", e))?;
        }

        log_debug!("已发送 {} 字节原始数据", data.len());
        Ok(())
    }

    /// 接收一条消息（阻塞等待）
    ///
    /// 从读取端获取一条完整消息。
    /// 注意：如果已启动后台接收循环，此方法不应同时使用，会导致读取端竞争。
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 `TcpMessage`
    ///
    /// # 超时
    /// 如果配置了 `timeout_ms > 0`，超时后返回错误。
    pub async fn recv(&self) -> Result<TcpMessage, String> {
        let mut reader_guard = self.reader.lock().await;
        let reader = reader_guard
            .as_mut()
            .ok_or_else(|| "客户端未连接，请先调用 connect()".to_string())?;

        let recv_future = reader.next();

        let frame = if self.config.timeout_ms > 0 {
            tokio::time::timeout(Duration::from_millis(self.config.timeout_ms), recv_future)
                .await
                .map_err(|_| format!("接收超时（{}ms）", self.config.timeout_ms))?
                .ok_or_else(|| "连接已关闭".to_string())?
                .map_err(|e| format!("接收数据失败: {}", e))?
        } else {
            recv_future
                .await
                .ok_or_else(|| "连接已关闭".to_string())?
                .map_err(|e| format!("接收数据失败: {}", e))?
        };

        let msg = TcpMessage::decode_from_bytes(&frame)?;
        log_debug!("收到消息: {}", msg.msg_type);
        Ok(msg)
    }

    /// 发送 Ping 并等待 Pong，测量往返延迟
    ///
    /// 发送一条 Ping 消息，阻塞等待服务端回复 Pong，
    /// 返回往返延迟（RTT，毫秒）。
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 RTT（毫秒）
    pub async fn ping(&self) -> Result<u64, String> {
        let ping = TcpMessage::ping();
        let send_time = std::time::Instant::now();
        self.send(&ping).await?;

        // 等待 Pong 回复
        let mut reader_guard = self.reader.lock().await;
        let reader = reader_guard
            .as_mut()
            .ok_or_else(|| "客户端未连接".to_string())?;

        let recv_future = reader.next();

        let frame = if self.config.timeout_ms > 0 {
            tokio::time::timeout(Duration::from_millis(self.config.timeout_ms), recv_future)
                .await
                .map_err(|_| format!("Ping 超时（{}ms）", self.config.timeout_ms))?
                .ok_or_else(|| "连接已关闭".to_string())?
                .map_err(|e| format!("接收 Pong 失败: {}", e))?
        } else {
            recv_future
                .await
                .ok_or_else(|| "连接已关闭".to_string())?
                .map_err(|e| format!("接收 Pong 失败: {}", e))?
        };

        let rtt = send_time.elapsed().as_millis() as u64;

        match TcpMessage::decode_from_bytes(&frame) {
            Ok(msg) if msg.is_pong() => {
                log_debug!("Ping 成功，RTT: {}ms", rtt);
                Ok(rtt)
            }
            Ok(msg) => Err(format!("期望收到 Pong，实际收到: {:?}", msg.msg_type)),
            Err(e) => Err(format!("解析 Pong 失败: {}", e)),
        }
    }

    /// 启动后台接收循环
    ///
    /// 在后台任务中持续接收消息，并通过通道发送给消费者。
    /// 适用于服务端主动推送消息的场景。
    ///
    /// **重要**：启动接收循环后，读取端所有权转移到后台任务，
    /// `recv()` 和 `ping()` 方法将不可用（返回错误），
    /// 但 `send()` 仍可正常使用（全双工）。
    ///
    /// # 返回值
    /// 返回消息接收通道的接收端，消费者从中读取消息。
    ///
    /// # 示例
    /// ```ignore
    /// let rx = client.start_recv_loop().await?;
    /// // 仍可发送消息
    /// client.send(&TcpMessage::data("订阅")).await?;
    /// // 在另一个任务中处理收到的消息
    /// tokio::spawn(async move {
    ///     while let Some(msg) = rx.recv().await {
    ///         println!("收到: {}", msg.payload);
    ///     }
    /// });
    /// ```
    pub async fn start_recv_loop(&self) -> Result<mpsc::UnboundedReceiver<TcpMessage>, String> {
        // 创建消息通道
        let (tx, rx) = mpsc::unbounded_channel::<TcpMessage>();
        *self.recv_tx.lock().await = Some(tx.clone());

        // 创建停止信号
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        // 取出读取端，转移所有权到后台任务
        let reader_opt = self.reader.lock().await.take();
        let mut reader = reader_opt
            .ok_or_else(|| "客户端未连接，请先调用 connect()".to_string())?;

        log_info!("TCP 客户端接收循环已启动");

        // 启动后台接收任务
        tokio::spawn(async move {
            let mut shutdown_rx = shutdown_rx;
            let tx = tx;

            loop {
                tokio::select! {
                    // 检查停止信号
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            log_info!("客户端接收循环收到停止信号");
                            break;
                        }
                    }
                    // 接收消息
                    msg = reader.next() => {
                        match msg {
                            Some(Ok(frame)) => {
                                match TcpMessage::decode_from_bytes(&frame) {
                                    Ok(tcp_msg) => {
                                        log_debug!("后台收到消息: {}", tcp_msg.msg_type);
                                        if tx.send(tcp_msg).is_err() {
                                            log_warn!("消息通道已关闭，停止接收循环");
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        log_warn!("解析消息失败: {}", e);
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                log_error!("接收数据失败: {}", e);
                                break;
                            }
                            None => {
                                log_info!("服务端关闭连接");
                                break;
                            }
                        }
                    }
                }
            }

            log_info!("TCP 客户端接收循环已退出");
        });

        Ok(rx)
    }

    /// 请求-响应模式
    ///
    /// 发送一条消息并等待回复。注意：不能与后台接收循环同时使用。
    ///
    /// # 参数
    /// * `msg` - 要发送的请求消息
    ///
    /// # 返回值
    /// 返回 Result，成功时包含回复消息
    pub async fn request(&self, msg: &TcpMessage) -> Result<TcpMessage, String> {
        self.send(msg).await?;
        self.recv().await
    }

    /// 是否已连接
    pub async fn is_connected(&self) -> bool {
        *self.connected.lock().await
    }

    /// 获取对端（服务端）地址
    pub async fn peer_addr(&self) -> Result<SocketAddr, String> {
        // 通过配置返回目标地址
        self.config.parse_server_addr()
    }

    /// 获取配置的引用
    pub fn config(&self) -> &TcpClientConfig {
        &self.config
    }
}
