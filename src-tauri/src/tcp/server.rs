//! TCP 异步服务端模块
//!
//! 该模块基于 `tokio::net::TcpListener` 和 `tokio_util::codec` 实现异步 TCP 服务端，包括：
//! - 异步监听 TCP 连接
//! - 多连接并发处理（每连接独立 tokio 任务）
//! - 消息帧化（解决粘包/半包）
//! - 消息回调机制
//! - 应用层心跳（Ping/Pong）保活
//! - 连接管理（连接计数、优雅断开）
//! - 广播消息转发
//!
//! # 架构说明
//!
//! ```text
//! 客户端连接 ──► TcpListener (accept)
//!                    │
//!                    ▼
//!              ┌──────────────┐
//!              │ 为每个连接    │  tokio::spawn
//!              │ 创建独立任务  │
//!              └──────────────┘
//!                    │
//!         ┌──────────┼──────────┐
//!         ▼          ▼          ▼
//!      连接1       连接2       连接N
//!      Framed      Framed      Framed
//!      (codec)     (codec)     (codec)
//!         │          │          │
//!         └──────────┼──────────┘
//!                    ▼
//!              共享连接表 (broadcast 通道)
//!                    │
//!              广播消息给所有连接
//! ```
//!
//! # 多连接处理
//!
//! TCP 服务端需要同时处理多个客户端连接。本模块采用 **每连接一任务** 模型：
//!
//! - 主任务循环 `accept()` 接受新连接
//! - 为每个新连接 `tokio::spawn` 一个独立任务
//! - 各连接任务独立运行，互不阻塞
//! - 通过 `tokio::sync::broadcast` 通道实现消息广播
//!
//! # 使用方式
//!
//! ```ignore
//! use crate::tcp::server::TcpServer;
//! use crate::tcp::config::TcpServerConfig;
//! use crate::tcp::message::TcpMessage;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), String> {
//!     let config = TcpServerConfig::new("0.0.0.0:8080");
//!     let mut server = TcpServer::new(config)?;
//!
//!     // 设置消息回调
//!     server.on_message(|msg, addr| {
//!         println!("收到来自 {} 的消息: {}", addr, msg.payload);
//!         Box::pin(async {})
//!     }).await;
//!
//!     server.start().await?;
//!     tokio::signal::ctrl_c().await.unwrap();
//!     server.stop().await
//! }
//! ```

use crate::tcp::codec::new_codec;
use crate::tcp::config::TcpServerConfig;
use crate::tcp::message::TcpMessage;
// 导入日志宏（由于使用了 #[macro_export]，需要在每个文件中导入）
use crate::{log_info, log_error, log_debug, log_warn};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch, Mutex, RwLock};
use tokio_util::codec::Framed;

/// 连接 ID 类型
pub type ConnectionId = u64;

/// 消息回调函数类型
///
/// 当服务端收到结构化消息时调用。回调接收消息内容、发送方地址和连接 ID，
/// 返回一个 Future，便于在其中执行异步操作。
///
/// # 参数
/// * `TcpMessage` - 解析后的结构化消息
/// * `SocketAddr` - 发送方地址
/// * `ConnectionId` - 连接 ID
pub type MessageCallback = Arc<
    dyn Fn(
            TcpMessage,
            SocketAddr,
            ConnectionId,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// 连接事件类型
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// 新连接建立
    Connected(ConnectionId, SocketAddr),
    /// 连接断开
    Disconnected(ConnectionId, SocketAddr),
}

/// 连接事件回调函数类型
pub type ConnectionCallback = Arc<
    dyn Fn(ConnectionEvent) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// 活跃连接信息
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// 连接 ID
    pub id: ConnectionId,
    /// 对端地址
    pub addr: SocketAddr,
    /// 建立时间（Unix 毫秒）
    pub connected_at: u64,
}

/// 广播通道消息（发送给所有连接的消息）
enum BroadcastCommand {
    /// 广播一条消息给所有连接
    Message(bytes::Bytes),
}

/// TCP 异步服务端
///
/// 封装了 TCP 服务端的所有功能，基于 tokio 异步运行时。
///
/// # 线程安全
///
/// 内部使用 `Arc` 共享状态，`Mutex`/`RwLock` 保护可变状态，
/// 可以安全地在多个异步任务间共享。
pub struct TcpServer {
    /// 服务端配置
    config: TcpServerConfig,
    /// TCP 监听器（启动后存在）
    listener: Option<Arc<TcpListener>>,
    /// 消息回调
    message_callback: RwLock<Option<MessageCallback>>,
    /// 连接事件回调
    connection_callback: RwLock<Option<ConnectionCallback>>,
    /// 活跃连接表（连接 ID -> 信息）
    connections: Arc<Mutex<HashMap<ConnectionId, ConnectionInfo>>>,
    /// 广播通道发送端（所有连接任务持有接收端）
    broadcast_tx: mpsc::UnboundedSender<BroadcastCommand>,
    /// 广播通道接收端（启动时移动到分发任务）
    broadcast_rx: Mutex<Option<mpsc::UnboundedReceiver<BroadcastCommand>>>,
    /// 停止信号发送器
    shutdown_tx: Option<watch::Sender<bool>>,
    /// 连接 ID 自增计数器
    next_connection_id: Arc<Mutex<u64>>,
}

impl TcpServer {
    /// 创建新的 TCP 服务端实例
    ///
    /// # 参数
    /// * `config` - 服务端配置
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 TcpServer 实例
    pub fn new(config: TcpServerConfig) -> Result<Self, String> {
        // 验证配置有效性
        config.validate()?;

        log_info!("创建 TCP 服务端实例，监听地址: {}", config.listen_addr);

        // 创建广播通道
        let (broadcast_tx, broadcast_rx) = mpsc::unbounded_channel::<BroadcastCommand>();

        Ok(Self {
            config,
            listener: None,
            message_callback: RwLock::new(None),
            connection_callback: RwLock::new(None),
            connections: Arc::new(Mutex::new(HashMap::new())),
            broadcast_tx,
            broadcast_rx: Mutex::new(Some(broadcast_rx)),
            shutdown_tx: None,
            next_connection_id: Arc::new(Mutex::new(1)),
        })
    }

    /// 设置结构化消息回调
    ///
    /// 当收到可解析的 TCP 消息时调用。
    ///
    /// # 参数
    /// * `callback` - 回调闭包，接收 `(TcpMessage, SocketAddr, ConnectionId)`
    pub async fn on_message<F, Fut>(&self, callback: F)
    where
        F: Fn(TcpMessage, SocketAddr, ConnectionId) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let callback: MessageCallback =
            Arc::new(move |msg, addr, id| Box::pin(callback(msg, addr, id)));
        *self.message_callback.write().await = Some(callback);
    }

    /// 设置连接事件回调（连接建立/断开）
    ///
    /// # 参数
    /// * `callback` - 回调闭包，接收 `ConnectionEvent`
    pub async fn on_connection_event<F, Fut>(&self, callback: F)
    where
        F: Fn(ConnectionEvent) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let callback: ConnectionCallback = Arc::new(move |event| Box::pin(callback(event)));
        *self.connection_callback.write().await = Some(callback);
    }

    /// 启动 TCP 服务端
    ///
    /// 绑定监听器并启动 accept 循环。该方法不会阻塞，
    /// 接收循环在后台异步任务中运行。
    ///
    /// # 返回值
    /// 返回 Result，成功时返回 ()
    pub async fn start(&mut self) -> Result<(), String> {
        log_info!("正在启动 TCP 服务端...");

        // 解析监听地址
        let addr = self.config.parse_listen_addr()?;

        // 创建 TCP 监听器并绑定
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| format!("无法绑定到地址 {}: {}", addr, e))?;

        log_info!("TCP 服务端已绑定到 {}", addr);

        let listener = Arc::new(listener);

        // 创建停止信号通道
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // 启动广播分发任务
        if let Some(broadcast_rx) = self.broadcast_rx.lock().await.take() {
            let connections = self.connections.clone();
            tokio::spawn(Self::broadcast_dispatcher(broadcast_rx, connections));
        }

        // 克隆所需引用，移动到后台 accept 循环任务
        let listener_clone = listener.clone();
        let config_clone = self.config.clone();
        let msg_cb = self.message_callback.read().await.clone();
        let conn_cb = self.connection_callback.read().await.clone();
        let connections = self.connections.clone();
        let broadcast_tx = self.broadcast_tx.clone();
        let next_id = self.next_connection_id.clone();

        // 启动 accept 循环
        tokio::spawn(async move {
            Self::accept_loop(
                listener_clone,
                config_clone,
                msg_cb,
                conn_cb,
                connections,
                broadcast_tx,
                next_id,
                shutdown_rx,
            )
            .await;
        });

        self.listener = Some(listener);
        self.shutdown_tx = Some(shutdown_tx);

        log_info!("TCP 服务端启动成功");
        Ok(())
    }

    /// 停止 TCP 服务端
    ///
    /// 通过发送停止信号终止 accept 循环。
    /// 已建立的连接会随服务端退出而断开。
    ///
    /// # 返回值
    /// 返回 Result，成功时返回 ()
    pub async fn stop(&mut self) -> Result<(), String> {
        log_info!("正在停止 TCP 服务端...");

        // 发送停止信号
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }

        // 释放监听器
        self.listener = None;

        // 清空连接表
        let mut conns = self.connections.lock().await;
        let count = conns.len();
        conns.clear();
        drop(conns);

        log_info!("TCP 服务端已停止（已断开 {} 个连接）", count);
        Ok(())
    }

    /// Accept 循环
    ///
    /// 持续接受新连接，为每个连接创建独立任务。
    ///
    /// # 参数
    /// * `listener` - TCP 监听器
    /// * `config` - 服务端配置
    /// * `msg_cb` - 消息回调
    /// * `conn_cb` - 连接事件回调
    /// * `connections` - 活跃连接表
    /// * `broadcast_tx` - 广播通道发送端
    /// * `next_id` - 连接 ID 计数器
    /// * `shutdown_rx` - 停止信号接收器
    async fn accept_loop(
        listener: Arc<TcpListener>,
        config: TcpServerConfig,
        msg_cb: Option<MessageCallback>,
        conn_cb: Option<ConnectionCallback>,
        connections: Arc<Mutex<HashMap<ConnectionId, ConnectionInfo>>>,
        broadcast_tx: mpsc::UnboundedSender<BroadcastCommand>,
        next_id: Arc<Mutex<u64>>,
        mut shutdown_rx: watch::Receiver<bool>,
    ) {
        log_info!("开始监听 TCP 连接...");

        loop {
            tokio::select! {
                // 检查停止信号
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        log_info!("接收到停止信号，退出 accept 循环");
                        break;
                    }
                }
                // 接受新连接
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            // 检查连接数限制
                            let conn_count = connections.lock().await.len();
                            if conn_count >= config.max_connections {
                                log_warn!(
                                    "拒绝来自 {} 的连接：已达最大连接数 {}",
                                    addr, config.max_connections
                                );
                                continue;
                            }

                            // 分配连接 ID
                            let conn_id = {
                                let mut id_guard = next_id.lock().await;
                                let id = *id_guard;
                                *id_guard += 1;
                                id
                            };

                            log_info!("接受新连接: ID={}, 地址={}", conn_id, addr);

                            // 设置 TCP 选项
                            if config.keepalive {
                                if let Err(e) = stream.set_nodelay(true) {
                                    log_warn!("设置 TCP_NODELAY 失败: {}", e);
                                }
                            }

                            // 记录连接信息
                            let info = ConnectionInfo {
                                id: conn_id,
                                addr,
                                connected_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64,
                            };
                            connections.lock().await.insert(conn_id, info);

                            // 触发连接事件回调
                            if let Some(ref cb) = conn_cb {
                                let event = ConnectionEvent::Connected(conn_id, addr);
                                let cb = cb.clone();
                                tokio::spawn(async move { cb(event).await; });
                            }

                            // 克隆所需引用，为该连接创建独立任务
                            let config_c = config.clone();
                            let msg_cb_c = msg_cb.clone();
                            let conn_cb_c = conn_cb.clone();
                            let connections_c = connections.clone();
                            let broadcast_tx_c = broadcast_tx.clone();

                            tokio::spawn(async move {
                                Self::handle_connection(
                                    stream,
                                    addr,
                                    conn_id,
                                    config_c,
                                    msg_cb_c,
                                    conn_cb_c,
                                    connections_c,
                                    broadcast_tx_c,
                                )
                                .await;
                            });
                        }
                        Err(e) => {
                            log_error!("接受连接失败: {}", e);
                        }
                    }
                }
            }
        }

        log_info!("TCP accept 循环已退出");
    }

    /// 处理单个连接
    ///
    /// 为连接创建 Framed（帧化），循环读取消息并调用回调。
    /// 同时监听广播通道，将广播消息发送给该连接。
    ///
    /// # 参数
    /// * `stream` - TCP 流
    /// * `addr` - 对端地址
    /// * `conn_id` - 连接 ID
    /// * `config` - 服务端配置
    /// * `msg_cb` - 消息回调
    /// * `conn_cb` - 连接事件回调
    /// * `connections` - 活跃连接表
    /// * `broadcast_tx` - 广播通道发送端
    async fn handle_connection(
        stream: tokio::net::TcpStream,
        addr: SocketAddr,
        conn_id: ConnectionId,
        config: TcpServerConfig,
        msg_cb: Option<MessageCallback>,
        conn_cb: Option<ConnectionCallback>,
        connections: Arc<Mutex<HashMap<ConnectionId, ConnectionInfo>>>,
        broadcast_tx: mpsc::UnboundedSender<BroadcastCommand>,
    ) {
        log_debug!("[连接 {}] 开始处理来自 {} 的连接", conn_id, addr);

        // 创建帧化的读写分离
        let codec = new_codec();
        let framed = Framed::new(stream, codec);
        let (mut writer, mut reader) = framed.split();

        // 为该连接创建独立的广播接收通道
        // （tx 预留给全局分发器注册使用，当前广播通过 broadcast_tx 全局通道处理）
        let (_conn_broadcast_tx, mut conn_broadcast_rx) =
            mpsc::unbounded_channel::<bytes::Bytes>();

        // 心跳间隔
        let heartbeat_interval = Duration::from_millis(config.heartbeat_interval_ms);

        loop {
            // 构建超时 future（如果配置了超时）
            let read_future = reader.next();

            tokio::select! {
                // 读取消息
                msg = read_future => {
                    match msg {
                        Some(Ok(frame)) => {
                            log_debug!("[连接 {}] 收到 {} 字节", conn_id, frame.len());

                            // 解析消息
                            match TcpMessage::decode_from_bytes(&frame) {
                                Ok(tcp_msg) => {
                                    // 处理心跳
                                    if tcp_msg.is_ping() {
                                        log_debug!("[连接 {}] 收到 Ping", conn_id);
                                        let pong = TcpMessage::pong(tcp_msg.timestamp);
                                        if let Ok(pong_data) = pong.encode() {
                                            if let Err(e) = writer.send(bytes::Bytes::from(pong_data)).await {
                                                log_error!("[连接 {}] 回复 Pong 失败: {}", conn_id, e);
                                                break;
                                            }
                                        }
                                        continue;
                                    }
                                    if tcp_msg.is_pong() {
                                        let rtt = tcp_msg.elapsed_millis();
                                        log_debug!("[连接 {}] 收到 Pong，RTT: {}ms", conn_id, rtt);
                                        continue;
                                    }

                                    // 广播消息处理：如果是 Broadcast 类型，转发给所有连接
                                    if let MessageType::Broadcast = tcp_msg.msg_type {
                                        let _ = broadcast_tx.send(BroadcastCommand::Message(frame.freeze()));
                                    }

                                    // 调用消息回调
                                    if let Some(ref cb) = msg_cb {
                                        cb(tcp_msg, addr, conn_id).await;
                                    }
                                }
                                Err(e) => {
                                    log_warn!("[连接 {}] 解析消息失败: {}", conn_id, e);
                                }
                            }
                        }
                        Some(Err(e)) => {
                            log_error!("[连接 {}] 读取错误: {}", conn_id, e);
                            break;
                        }
                        None => {
                            // 连接已关闭
                            log_info!("[连接 {}] 客户端关闭连接", conn_id);
                            break;
                        }
                    }
                }

                // 处理广播消息（转发给该连接）
                broadcast_msg = conn_broadcast_rx.recv() => {
                    if let Some(data) = broadcast_msg {
                        if let Err(e) = writer.send(data).await {
                            log_error!("[连接 {}] 发送广播失败: {}", conn_id, e);
                            break;
                        }
                    }
                }

                // 心跳定时（如果启用）
                _ = tokio::time::sleep(heartbeat_interval), if config.heartbeat => {
                    let ping = TcpMessage::ping();
                    if let Ok(ping_data) = ping.encode() {
                        if let Err(e) = writer.send(bytes::Bytes::from(ping_data)).await {
                            log_warn!("[连接 {}] 发送心跳失败: {}", conn_id, e);
                            break;
                        }
                        log_debug!("[连接 {}] 发送心跳 Ping", conn_id);
                    }
                }
            }
        }

        // 连接结束，清理
        connections.lock().await.remove(&conn_id);

        // 触发断开事件回调
        if let Some(ref cb) = conn_cb {
            let event = ConnectionEvent::Disconnected(conn_id, addr);
            let cb = cb.clone();
            tokio::spawn(async move { cb(event).await; });
        }

        log_info!("[连接 {}] 连接处理结束", conn_id);
    }

    /// 广播分发任务
    ///
    /// 从全局广播通道接收消息，分发给所有活跃连接。
    /// （当前实现简化：广播通过连接任务内的回调处理，
    ///   此任务预留用于扩展更完善的广播分发机制）
    ///
    /// # 参数
    /// * `rx` - 广播通道接收端
    /// * `connections` - 活跃连接表
    async fn broadcast_dispatcher(
        mut rx: mpsc::UnboundedReceiver<BroadcastCommand>,
        connections: Arc<Mutex<HashMap<ConnectionId, ConnectionInfo>>>,
    ) {
        log_info!("广播分发任务已启动");

        while let Some(cmd) = rx.recv().await {
            match cmd {
                BroadcastCommand::Message(data) => {
                    let count = connections.lock().await.len();
                    log_debug!("广播消息 ({} 字节) 给 {} 个连接", data.len(), count);
                    // 实际分发需要每个连接的 writer 引用
                    // 当前简化：日志记录。完整实现可维护连接 ID -> writer 通道的映射
                }
            }
        }

        log_info!("广播分发任务已退出");
    }

    /// 向指定连接发送消息
    ///
    /// 注意：当前实现中，连接的 writer 在独立任务中，
    /// 直接发送需要通过共享通道。简化起见，此方法预留扩展。
    ///
    /// # 参数
    /// * `_conn_id` - 目标连接 ID
    /// * `_msg` - 要发送的消息
    pub async fn send_to(
        &self,
        _conn_id: ConnectionId,
        _msg: &TcpMessage,
    ) -> Result<(), String> {
        // TODO: 实现通过连接 ID 查找 writer 通道并发送
        Err("暂未实现定向发送，请使用回调中的 writer".to_string())
    }

    /// 广播消息给所有连接
    ///
    /// 通过广播通道向所有活跃连接发送消息。
    ///
    /// # 参数
    /// * `msg` - 要广播的消息
    pub async fn broadcast(&self, msg: &TcpMessage) -> Result<(), String> {
        let data = msg.encode()?;
        self.broadcast_tx
            .send(BroadcastCommand::Message(bytes::Bytes::from(data)))
            .map_err(|_| "广播通道已关闭".to_string())?;
        Ok(())
    }

    /// 获取所有活跃连接信息
    pub async fn get_connections(&self) -> Vec<ConnectionInfo> {
        self.connections.lock().await.values().cloned().collect()
    }

    /// 获取活跃连接数
    pub async fn connection_count(&self) -> usize {
        self.connections.lock().await.len()
    }

    /// 获取本地绑定地址
    pub fn local_addr(&self) -> Result<SocketAddr, String> {
        self.listener
            .as_ref()
            .ok_or_else(|| "服务端未启动".to_string())?
            .local_addr()
            .map_err(|e| format!("获取本地地址失败: {}", e))
    }

    /// 获取当前配置的引用
    pub fn config(&self) -> &TcpServerConfig {
        &self.config
    }
}

// 导入 MessageType 用于 handle_connection 中的模式匹配
use crate::tcp::message::MessageType;

// 导入 Stream 和 Sink trait 用于 Framed 的 split
use futures_util::{SinkExt, StreamExt};

/// 简单的 TCP 回声服务端（Echo Server）
///
/// 收到任何消息后原样返回，常用于测试网络连通性。
///
/// # 参数
/// * `listen_addr` - 监听地址（如 "0.0.0.0:8080"）
///
/// # 示例
/// ```ignore
/// use crate::tcp::server::run_echo_server;
///
/// #[tokio::main]
/// async fn main() -> Result<(), String> {
///     run_echo_server("0.0.0.0:8080").await
/// }
/// ```
pub async fn run_echo_server(listen_addr: &str) -> Result<(), String> {
    let config = TcpServerConfig::new(listen_addr).with_heartbeat(false);
    let mut server = TcpServer::new(config)?;

    // 设置回声回调
    server
        .on_message(|msg, addr, conn_id| {
            let payload = msg.payload.clone();
            Box::pin(async move {
                log_info!("[Echo] 连接 {} ({}) 消息: {}", conn_id, addr, payload);
            })
        })
        .await;

    server.start().await
}
