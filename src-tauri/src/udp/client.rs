//! UDP 异步客户端模块
//!
//! 该模块基于 `tokio::net::UdpSocket` 实现异步 UDP 客户端，包括：
//! - 连接服务端（绑定本地端口并关联服务端地址）
//! - 发送和接收结构化消息（JSON）或原始字节
//! - 支持 Ping/Pong 心跳检测与往返延迟（RTT）测量
//! - 支持超时控制
//! - 支持独立的接收循环（后台监听服务端推送的消息）
//!
//! # UDP 客户端工作原理
//!
//! UDP 是无连接协议，客户端无需建立连接即可发送数据。
//! 本模块通过 `connect()` 方法将套接字关联到服务端地址，
//! 之后可直接使用 `send()` 发送数据，简化使用方式。
//!
//! ```text
//! 客户端                              服务端
//!   │                                   │
//!   │──── UdpMessage::data("hello") ───►│
//!   │                                   │
//!   │◄───── UdpMessage::data("ok") ─────│
//!   │                                   │
//!   │──── UdpMessage::ping() ──────────►│
//!   │                                   │
//!   │◄──── UdpMessage::pong() ──────────│
//!   │                                   │
//! ```
//!
//! # 使用方式
//!
//! ```ignore
//! use crate::udp::client::UdpClient;
//! use crate::udp::config::UdpClientConfig;
//! use crate::udp::message::UdpMessage;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), String> {
//!     let config = UdpClientConfig::new("127.0.0.1:8080");
//!     let mut client = UdpClient::new(config)?;
//!
//!     // 连接服务端
//!     client.connect().await?;
//!
//!     // 发送消息
//!     client.send(&UdpMessage::data("Hello, Server!")).await?;
//!
//!     // 接收回复
//!     let (msg, addr) = client.recv().await?;
//!     println!("收到回复: {}", msg.payload);
//!
//!     // Ping 测试
//!     let rtt = client.ping().await?;
//!     println!("往返延迟: {}ms", rtt);
//!
//!     Ok(())
//! }
//! ```

use crate::udp::config::UdpClientConfig;
use crate::udp::message::UdpMessage;
// 导入日志宏
use crate::{log_info, log_error, log_debug, log_warn};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, watch, Mutex};

/// 接收到的消息（包含消息内容和发送方地址）
pub type ReceivedMessage = (UdpMessage, SocketAddr);

/// UDP 异步客户端
///
/// 提供面向消息的 UDP 通信接口，支持结构化消息和原始字节。
///
/// # 线程安全
///
/// 内部使用 `Arc` 共享状态，可以安全地克隆并在多个异步任务间使用。
#[derive(Clone)]
pub struct UdpClient {
    /// 客户端配置
    config: Arc<UdpClientConfig>,
    /// UDP 套接字（连接后存在）
    socket: Arc<Mutex<Option<Arc<UdpSocket>>>>,
    /// 后台接收任务的消息通道发送端
    /// （当启动接收循环后，收到的消息会通过此通道发送给消费者）
    recv_tx: Arc<Mutex<Option<mpsc::UnboundedSender<ReceivedMessage>>>>,
    /// 后台接收任务的停止信号
    shutdown_tx: Arc<Mutex<Option<watch::Sender<bool>>>>,
}

impl UdpClient {
    /// 创建新的 UDP 客户端实例
    ///
    /// # 参数
    /// * `config` - 客户端配置
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 UdpClient 实例
    pub fn new(config: UdpClientConfig) -> Result<Self, String> {
        // 验证配置有效性
        config.validate()?;

        log_info!("创建 UDP 客户端实例，目标服务端: {}", config.server_addr);

        Ok(Self {
            config: Arc::new(config),
            socket: Arc::new(Mutex::new(None)),
            recv_tx: Arc::new(Mutex::new(None)),
            shutdown_tx: Arc::new(Mutex::new(None)),
        })
    }

    /// 连接服务端
    ///
    /// 绑定本地端口并关联服务端地址。UDP 是无连接协议，
    /// 这里的"连接"只是设置默认目标地址，不会发送任何数据。
    ///
    /// # 返回值
    /// 返回 Result，成功时返回 ()
    pub async fn connect(&self) -> Result<(), String> {
        log_info!("正在连接 UDP 服务端: {}", self.config.server_addr);

        // 确定本地绑定地址
        let bind_addr = self
            .config
            .bind_addr
            .as_deref()
            .unwrap_or("0.0.0.0:0");

        // 创建 UDP 套接字并绑定到本地地址
        let socket = UdpSocket::bind(bind_addr)
            .await
            .map_err(|e| format!("绑定本地地址 {} 失败: {}", bind_addr, e))?;

        log_debug!("本地绑定地址: {}", socket.local_addr().map_err(|e| e.to_string())?);

        // 关联服务端地址（UDP connect 仅设置默认目标，不发送数据）
        let server_addr = self.config.parse_server_addr()?;
        socket
            .connect(server_addr)
            .await
            .map_err(|e| format!("关联服务端地址失败: {}", e))?;

        log_info!("已连接到服务端: {}", server_addr);

        // 保存套接字
        *self.socket.lock().await = Some(Arc::new(socket));

        Ok(())
    }

    /// 断开连接
    ///
    /// 释放套接字并停止后台接收任务。
    pub async fn disconnect(&self) -> Result<(), String> {
        log_info!("正在断开 UDP 连接...");

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

        // 释放套接字
        *self.socket.lock().await = None;

        log_info!("UDP 连接已断开");
        Ok(())
    }

    /// 发送结构化消息
    ///
    /// 将消息编码为 JSON 字节流并发送。
    /// 需要先调用 `connect()` 建立连接。
    ///
    /// # 参数
    /// * `msg` - 要发送的消息
    pub async fn send(&self, msg: &UdpMessage) -> Result<(), String> {
        let socket = self.get_socket().await?;
        let data = msg.encode()?;

        socket
            .send(&data)
            .await
            .map_err(|e| format!("发送数据失败: {}", e))?;

        log_debug!("已发送 {} 字节", data.len());
        Ok(())
    }

    /// 发送原始字节
    ///
    /// 绕过消息编码，直接发送二进制数据。
    /// 适用于自定义二进制协议。
    ///
    /// # 参数
    /// * `data` - 原始字节数据
    pub async fn send_raw(&self, data: &[u8]) -> Result<(), String> {
        let socket = self.get_socket().await?;

        socket
            .send(data)
            .await
            .map_err(|e| format!("发送原始数据失败: {}", e))?;

        log_debug!("已发送 {} 字节原始数据", data.len());
        Ok(())
    }

    /// 接收一条消息（阻塞等待）
    ///
    /// 从套接字读取一条数据报并解析为结构化消息。
    /// 如果数据无法解析为 JSON，将返回错误。
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 `(UdpMessage, 发送方地址)`
    ///
    /// # 超时
    /// 如果配置了 `timeout_ms > 0`，超时后返回错误。
    pub async fn recv(&self) -> Result<ReceivedMessage, String> {
        let socket = self.get_socket().await?;
        let mut buf = vec![0u8; self.config.buffer_size];

        // 使用 tokio::time::timeout 包装接收操作
        let recv_future = socket.recv_from(&mut buf);

        let len = if self.config.timeout_ms > 0 {
            tokio::time::timeout(
                Duration::from_millis(self.config.timeout_ms),
                recv_future,
            )
            .await
            .map_err(|_| format!("接收超时（{}ms）", self.config.timeout_ms))?
            .map_err(|e| format!("接收数据失败: {}", e))?
            .0
        } else {
            recv_future.await.map_err(|e| format!("接收数据失败: {}", e))?.0
        };

        let addr = socket
            .peer_addr()
            .map_err(|e| format!("获取对端地址失败: {}", e))?;

        let msg = UdpMessage::decode(&buf[..len])?;
        log_debug!("收到 {} 字节来自 {}", len, addr);

        Ok((msg, addr))
    }

    /// 接收原始字节（阻塞等待）
    ///
    /// 从套接字读取一条数据报，不进行解析。
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 `(字节数据, 发送方地址)`
    pub async fn recv_raw(&self) -> Result<(Vec<u8>, SocketAddr), String> {
        let socket = self.get_socket().await?;
        let mut buf = vec![0u8; self.config.buffer_size];

        let recv_future = socket.recv_from(&mut buf);

        let (len, addr) = if self.config.timeout_ms > 0 {
            tokio::time::timeout(
                Duration::from_millis(self.config.timeout_ms),
                recv_future,
            )
            .await
            .map_err(|_| format!("接收超时（{}ms）", self.config.timeout_ms))?
            .map_err(|e| format!("接收数据失败: {}", e))?
        } else {
            recv_future.await.map_err(|e| format!("接收数据失败: {}", e))?
        };

        log_debug!("收到 {} 字节原始数据来自 {}", len, addr);
        Ok((buf[..len].to_vec(), addr))
    }

    /// 发送 Ping 并等待 Pong，测量往返延迟
    ///
    /// 发送一条 Ping 消息，阻塞等待服务端回复 Pong，
    /// 返回往返延迟（RTT，毫秒）。
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 RTT（毫秒）
    pub async fn ping(&self) -> Result<u64, String> {
        let socket = self.get_socket().await?;

        // 发送 Ping
        let ping = UdpMessage::ping();
        let send_time = std::time::Instant::now();
        self.send(&ping).await?;

        // 等待 Pong 回复
        let mut buf = vec![0u8; self.config.buffer_size];

        let recv_future = socket.recv_from(&mut buf);

        let (len, _) = if self.config.timeout_ms > 0 {
            tokio::time::timeout(
                Duration::from_millis(self.config.timeout_ms),
                recv_future,
            )
            .await
            .map_err(|_| format!("Ping 超时（{}ms）", self.config.timeout_ms))?
            .map_err(|e| format!("接收 Pong 失败: {}", e))?
        } else {
            recv_future.await.map_err(|e| format!("接收 Pong 失败: {}", e))?
        };

        let rtt = send_time.elapsed().as_millis() as u64;

        // 验证是否为 Pong 消息
        match UdpMessage::decode(&buf[..len]) {
            Ok(msg) if msg.is_pong() => {
                log_debug!("Ping 成功，RTT: {}ms", rtt);
                Ok(rtt)
            }
            Ok(msg) => {
                Err(format!("期望收到 Pong，实际收到: {:?}", msg.msg_type))
            }
            Err(e) => {
                Err(format!("解析 Pong 失败: {}", e))
            }
        }
    }

    /// 启动后台接收循环
    ///
    /// 在后台任务中持续接收消息，并通过通道发送给消费者。
    /// 适用于服务端主动推送消息的场景。
    ///
    /// # 返回值
    /// 返回消息接收通道的接收端，消费者从中读取消息。
    ///
    /// # 示例
    /// ```ignore
    /// let rx = client.start_recv_loop().await?;
    /// while let Some((msg, addr)) = rx.recv().await {
    ///     println!("收到: {}", msg.payload);
    /// }
    /// ```
    pub async fn start_recv_loop(&self) -> Result<mpsc::UnboundedReceiver<ReceivedMessage>, String> {
        let socket = self.get_socket().await?;

        // 创建消息通道
        let (tx, rx) = mpsc::unbounded_channel::<ReceivedMessage>();
        *self.recv_tx.lock().await = Some(tx.clone());

        // 创建停止信号
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let buffer_size = self.config.buffer_size;

        // 启动后台接收任务
        tokio::spawn(async move {
            let mut buf = vec![0u8; buffer_size];
            let mut shutdown_rx = shutdown_rx;

            log_info!("UDP 客户端接收循环已启动");

            loop {
                tokio::select! {
                    // 检查停止信号
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            log_info!("客户端接收循环收到停止信号");
                            break;
                        }
                    }
                    // 接收数据
                    result = socket.recv_from(&mut buf) => {
                        match result {
                            Ok((len, addr)) => {
                                if len == 0 {
                                    continue;
                                }

                                // 尝试解析为结构化消息
                                match UdpMessage::decode(&buf[..len]) {
                                    Ok(msg) => {
                                        log_debug!("收到 {} 字节来自 {}", len, addr);
                                        if tx.send((msg, addr)).is_err() {
                                            log_warn!("消息通道已关闭，停止接收循环");
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        log_warn!("解析消息失败: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                log_error!("接收数据失败: {}", e);
                                // 短暂等待后重试
                                tokio::time::sleep(Duration::from_millis(100)).await;
                            }
                        }
                    }
                }
            }

            log_info!("UDP 客户端接收循环已退出");
        });

        Ok(rx)
    }

    /// 请求-响应模式
    ///
    /// 发送一条消息并等待回复，适用于简单的请求-响应交互。
    ///
    /// # 参数
    /// * `msg` - 要发送的请求消息
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 `(回复消息, 回复方地址)`
    pub async fn request(&self, msg: &UdpMessage) -> Result<ReceivedMessage, String> {
        self.send(msg).await?;
        self.recv().await
    }

    /// 获取内部套接字
    ///
    /// 辅助方法，确保套接字已初始化。
    async fn get_socket(&self) -> Result<Arc<UdpSocket>, String> {
        self.socket
            .lock()
            .await
            .clone()
            .ok_or_else(|| "客户端未连接，请先调用 connect()".to_string())
    }

    /// 获取本地绑定地址
    pub async fn local_addr(&self) -> Result<SocketAddr, String> {
        let socket = self.get_socket().await?;
        socket
            .local_addr()
            .map_err(|e| format!("获取本地地址失败: {}", e))
    }

    /// 获取服务端地址
    pub async fn peer_addr(&self) -> Result<SocketAddr, String> {
        let socket = self.get_socket().await?;
        socket
            .peer_addr()
            .map_err(|e| format!("获取对端地址失败: {}", e))
    }

    /// 是否已连接
    pub async fn is_connected(&self) -> bool {
        self.socket.lock().await.is_some()
    }

    /// 获取配置的引用
    pub fn config(&self) -> &UdpClientConfig {
        &self.config
    }
}
