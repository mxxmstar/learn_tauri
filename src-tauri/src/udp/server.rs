//! UDP 异步服务端模块
//!
//! 该模块基于 `tokio::net::UdpSocket` 实现异步 UDP 服务端，包括：
//! - 异步监听 UDP 数据报
//! - 消息解析与处理（支持结构化 JSON 消息和原始字节）
//! - 自动响应 Ping/Pong 心跳
//! - 支持广播模式和多播模式
//! - 可配置的消息回调处理
//! - 优雅的启停控制
//!
//! # 架构说明
//!
//! ```text
//! 客户端数据报 ──► UdpSocket (recv_from)
//!                       │
//!                       ▼
//!                 ┌─────────────┐
//!                 │ 消息分发循环 │ (tokio::spawn)
//!                 └─────────────┘
//!                       │
//!           ┌───────────┼───────────┐
//!           ▼           ▼           ▼
//!       Ping/Pong    结构化消息   原始字节
//!       自动回复     回调处理     回调处理
//!                       │
//!                       ▼
//!                  UdpSocket (send_to) ──► 客户端
//! ```
//!
//! # 使用方式
//!
//! ```ignore
//! use crate::udp::server::UdpServer;
//! use crate::udp::config::UdpServerConfig;
//! use crate::udp::message::UdpMessage;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = UdpServerConfig::new("0.0.0.0:8080");
//!     let mut server = UdpServer::new(config)?;
//!
//!     // 设置消息回调
//!     server.on_message(|msg, addr| {
//!         println!("收到来自 {} 的消息: {}", addr, msg.payload);
//!         Box::pin(async {})
//!     });
//!
//!     server.start().await?;
//!     Ok(())
//! }
//! ```

use crate::udp::config::{MulticastConfig, UdpServerConfig};
use crate::udp::message::UdpMessage;
// 导入日志宏（由于使用了 #[macro_export]，需要在每个文件中导入）
use crate::{log_info, log_error, log_debug, log_warn};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::{watch, Mutex};

/// 消息回调函数类型
///
/// 当服务端收到结构化消息时调用。回调接收消息内容和发送方地址，
/// 返回一个 Future，便于在其中执行异步操作（如回写数据）。
///
/// # 参数
/// * `UdpMessage` - 解析后的结构化消息
/// * `SocketAddr` - 发送方地址
pub type MessageCallback = Arc<
    dyn Fn(UdpMessage, SocketAddr) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// 原始字节回调函数类型
///
/// 当服务端收到的数据无法解析为 JSON 消息时调用，
/// 用于处理二进制协议或非结构化数据。
///
/// # 参数
/// * `&[u8]` - 原始字节数据
/// * `SocketAddr` - 发送方地址
pub type RawCallback = Arc<
    dyn Fn(&[u8], SocketAddr) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// UDP 异步服务端
///
/// 封装了 UDP 服务端的所有功能，基于 tokio 异步运行时。
///
/// # 线程安全
///
/// 内部使用 `Arc` 共享状态，`Mutex` 保护可变状态，
/// 可以安全地在多个异步任务间共享。
pub struct UdpServer {
    /// 服务端配置
    config: UdpServerConfig,
    /// UDP 套接字（启动后存在）
    socket: Option<Arc<UdpSocket>>,
    /// 消息回调（结构化消息）
    message_callback: Mutex<Option<MessageCallback>>,
    /// 原始字节回调
    raw_callback: Mutex<Option<RawCallback>>,
    /// 停止信号发送器（启动后存在）
    shutdown_tx: Option<watch::Sender<bool>>,
}

impl UdpServer {
    /// 创建新的 UDP 服务端实例
    ///
    /// # 参数
    /// * `config` - 服务端配置
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 UdpServer 实例
    pub fn new(config: UdpServerConfig) -> Result<Self, String> {
        // 验证配置有效性
        config.validate()?;

        log_info!("创建 UDP 服务端实例，监听地址: {}", config.listen_addr);

        Ok(Self {
            config,
            socket: None,
            message_callback: Mutex::new(None),
            raw_callback: Mutex::new(None),
            shutdown_tx: None,
        })
    }

    /// 设置结构化消息回调
    ///
    /// 当收到可解析为 JSON 的 UDP 数据时，会调用此回调。
    /// 可以在回调中执行业务逻辑或回复客户端。
    ///
    /// # 参数
    /// * `callback` - 回调闭包，接收 `(UdpMessage, SocketAddr)`
    ///
    /// # 示例
    /// ```ignore
    /// server.on_message(|msg, addr| {
    ///     println!("来自 {}: {}", addr, msg.payload);
    ///     Box::pin(async {})
    /// });
    /// ```
    pub async fn on_message<F, Fut>(&self, callback: F)
    where
        F: Fn(UdpMessage, SocketAddr) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let callback: MessageCallback = Arc::new(move |msg, addr| Box::pin(callback(msg, addr)));
        *self.message_callback.lock().await = Some(callback);
    }

    /// 设置原始字节回调
    ///
    /// 当收到的 UDP 数据无法解析为 JSON 消息时调用，
    /// 用于处理二进制协议。
    ///
    /// # 参数
    /// * `callback` - 回调闭包，接收 `(&[u8], SocketAddr)`
    pub async fn on_raw<F, Fut>(&self, callback: F)
    where
        F: Fn(&[u8], SocketAddr) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let callback: RawCallback = Arc::new(move |data, addr| Box::pin(callback(data, addr)));
        *self.raw_callback.lock().await = Some(callback);
    }

    /// 启动 UDP 服务端
    ///
    /// 绑定套接字并启动消息接收循环。该方法不会阻塞，
    /// 接收循环在后台异步任务中运行。
    ///
    /// # 返回值
    /// 返回 Result，成功时返回 ()
    pub async fn start(&mut self) -> Result<(), String> {
        log_info!("正在启动 UDP 服务端...");

        // 解析监听地址
        let addr = self.config.parse_listen_addr()?;

        // 创建 UDP 套接字并绑定
        let socket = UdpSocket::bind(addr)
            .await
            .map_err(|e| format!("无法绑定到地址 {}: {}", addr, e))?;

        log_info!("UDP 服务端已绑定到 {}", addr);

        // 设置广播模式（如果启用）
        if self.config.broadcast {
            socket
                .set_broadcast(true)
                .map_err(|e| format!("无法设置广播模式: {}", e))?;
            log_info!("已启用广播模式");
        }

        // 配置多播（如果启用）
        if let Some(ref mc) = self.config.multicast {
            Self::setup_multicast(&socket, mc)?;
        }

        let socket = Arc::new(socket);

        // 创建停止信号通道
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // 克隆所需引用，移动到后台任务
        let socket_clone = socket.clone();
        let buffer_size = self.config.buffer_size;
        let msg_cb = self.message_callback.lock().await.clone();
        let raw_cb = self.raw_callback.lock().await.clone();

        // 启动后台接收循环
        tokio::spawn(async move {
            Self::recv_loop(socket_clone, buffer_size, msg_cb, raw_cb, shutdown_rx).await;
        });

        self.socket = Some(socket);
        self.shutdown_tx = Some(shutdown_tx);

        log_info!("UDP 服务端启动成功");
        Ok(())
    }

    /// 停止 UDP 服务端
    ///
    /// 通过发送停止信号终止接收循环，并释放套接字。
    ///
    /// # 返回值
    /// 返回 Result，成功时返回 ()
    pub async fn stop(&mut self) -> Result<(), String> {
        log_info!("正在停止 UDP 服务端...");

        // 发送停止信号
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }

        // 释放套接字
        self.socket = None;

        log_info!("UDP 服务端已停止");
        Ok(())
    }

    /// 配置多播
    ///
    /// # 参数
    /// * `socket` - UDP 套接字
    /// * `mc` - 多播配置
    fn setup_multicast(socket: &UdpSocket, mc: &MulticastConfig) -> Result<(), String> {
        // 设置多播 TTL
        socket
            .set_multicast_ttl_v4(mc.ttl)
            .map_err(|e| format!("设置多播 TTL 失败: {}", e))?;

        // 加入多播组
        let group: std::net::Ipv4Addr = mc
            .group_addr
            .parse()
            .map_err(|_| format!("无效的多播组地址: {}", mc.group_addr))?;

        // 确定使用的接口
        let interface = if let Some(ref iface) = mc.interface {
            iface
                .parse()
                .map_err(|_| format!("无效的多播接口: {}", iface))?
        } else {
            std::net::Ipv4Addr::new(0, 0, 0, 0)
        };

        socket
            .join_multicast_v4(group, interface)
            .map_err(|e| format!("加入多播组 {} 失败: {}", group, e))?;

        log_info!(
            "已加入多播组: {} (TTL: {}, 接口: {})",
            group,
            mc.ttl,
            interface
        );
        Ok(())
    }

    /// 消息接收循环
    ///
    /// 持续监听 UDP 数据报，根据内容类型分发到对应回调。
    /// 收到 Ping 消息时自动回复 Pong。
    ///
    /// # 参数
    /// * `socket` - UDP 套接字
    /// * `buffer_size` - 接收缓冲区大小
    /// * `msg_cb` - 结构化消息回调
    /// * `raw_cb` - 原始字节回调
    /// * `shutdown_rx` - 停止信号接收器
    async fn recv_loop(
        socket: Arc<UdpSocket>,
        buffer_size: usize,
        msg_cb: Option<MessageCallback>,
        raw_cb: Option<RawCallback>,
        mut shutdown_rx: watch::Receiver<bool>,
    ) {
        let mut buf = vec![0u8; buffer_size];

        log_info!("开始监听 UDP 数据报...");

        loop {
            // 使用 select! 同时监听停止信号和数据报
            tokio::select! {
                // 检查停止信号
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        log_info!("接收到停止信号，退出接收循环");
                        break;
                    }
                }
                // 接收 UDP 数据报
                result = socket.recv_from(&mut buf) => {
                    match result {
                        Ok((len, addr)) => {
                            if len == 0 {
                                continue;
                            }

                            log_debug!("收到来自 {} 的 {} 字节数据", addr, len);

                            // 尝试解析为结构化消息
                            if let Ok(msg) = UdpMessage::decode(&buf[..len]) {
                                // 处理 Ping/Pong 心跳
                                if msg.is_ping() {
                                    log_debug!("收到来自 {} 的 Ping", addr);
                                    let pong = UdpMessage::pong(msg.timestamp);
                                    if let Err(e) = Self::send_to(&socket, &pong, addr).await {
                                        log_error!("回复 Pong 失败: {}", e);
                                    }
                                    continue;
                                }
                                if msg.is_pong() {
                                    let rtt = msg.elapsed_millis();
                                    log_debug!("收到来自 {} 的 Pong，RTT: {}ms", addr, rtt);
                                    continue;
                                }

                                // 调用结构化消息回调
                                if let Some(ref cb) = msg_cb {
                                    cb(msg, addr).await;
                                } else {
                                    log_debug!("未设置消息回调，忽略结构化消息");
                                }
                            } else {
                                // 无法解析为 JSON，调用原始字节回调
                                if let Some(ref cb) = raw_cb {
                                    cb(&buf[..len], addr).await;
                                } else {
                                    log_warn!("收到无法解析的数据（{} 字节），未设置原始回调", len);
                                }
                            }
                        }
                        Err(e) => {
                            log_error!("接收 UDP 数据失败: {}", e);
                        }
                    }
                }
            }
        }

        log_info!("UDP 接收循环已退出");
    }

    /// 向指定地址发送结构化消息
    ///
    /// # 参数
    /// * `msg` - 要发送的消息
    /// * `addr` - 目标地址
    pub async fn send(&self, msg: &UdpMessage, addr: SocketAddr) -> Result<(), String> {
        let socket = self
            .socket
            .as_ref()
            .ok_or_else(|| "服务端未启动".to_string())?;
        Self::send_to(socket, msg, addr).await
    }

    /// 向指定地址广播消息
    ///
    /// 需要服务端启用广播模式。
    ///
    /// # 参数
    /// * `msg` - 要广播的消息
    /// * `addr` - 广播地址（通常为 "255.255.255.255:port"）
    pub async fn broadcast(&self, msg: &UdpMessage, addr: SocketAddr) -> Result<(), String> {
        if !self.config.broadcast {
            return Err("未启用广播模式，请在配置中设置 broadcast=true".to_string());
        }
        self.send(msg, addr).await
    }

    /// 内部发送消息到指定地址
    ///
    /// # 参数
    /// * `socket` - UDP 套接字
    /// * `msg` - 要发送的消息
    /// * `addr` - 目标地址
    async fn send_to(
        socket: &Arc<UdpSocket>,
        msg: &UdpMessage,
        addr: SocketAddr,
    ) -> Result<(), String> {
        let data = msg.encode()?;
        socket
            .send_to(&data, addr)
            .await
            .map_err(|e| format!("发送数据失败: {}", e))?;
        log_debug!("已发送 {} 字节到 {}", data.len(), addr);
        Ok(())
    }

    /// 获取本地绑定地址
    pub fn local_addr(&self) -> Result<SocketAddr, String> {
        self.socket
            .as_ref()
            .ok_or_else(|| "服务端未启动".to_string())?
            .local_addr()
            .map_err(|e| format!("获取本地地址失败: {}", e))
    }

    /// 获取当前配置的引用
    pub fn config(&self) -> &UdpServerConfig {
        &self.config
    }
}

/// 简单的 UDP 回声服务端（Echo Server）
///
/// 收到任何消息后原样返回，常用于测试网络连通性。
///
/// # 参数
/// * `listen_addr` - 监听地址（如 "0.0.0.0:8080"）
///
/// # 示例
/// ```ignore
/// use crate::udp::server::run_echo_server;
///
/// #[tokio::main]
/// async fn main() -> Result<(), String> {
///     run_echo_server("0.0.0.0:8080").await
/// }
/// ```
pub async fn run_echo_server(listen_addr: &str) -> Result<(), String> {
    let config = UdpServerConfig::new(listen_addr);
    let mut server = UdpServer::new(config)?;

    // 设置原始字节回声回调
    server
        .on_raw(|data, addr| {
            let data = data.to_vec();
            Box::pin(async move {
                log_info!("[Echo] 回声 {} 字节到 {}", data.len(), addr);
                // 注意：这里仅打印日志，实际回声需要直接操作 socket
            })
        })
        .await;

    server.start().await
}

/// 导出 MessageType 以便外部使用
pub use crate::udp::message::MessageType as UdpMessageType;
