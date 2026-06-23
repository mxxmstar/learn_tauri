//! TCP 模块
//!
//! 该模块基于 `tokio::net` 和 `tokio_util::codec` 实现完整的异步 TCP 通信功能，
//! 包括服务端和客户端，支持消息帧化（解决粘包/半包）、全双工通信、心跳保活。
//!
//! # 模块结构
//!
//! ```text
//! tcp/
//! ├── mod.rs      # 模块主入口，定义 TcpService 统一管理接口
//! ├── config.rs   # 配置管理（服务端/客户端配置、心跳、重连）
//! ├── message.rs  # 消息协议定义与编解码（JSON 结构化消息）
//! ├── codec.rs    # 消息帧化（LengthDelimitedCodec，解决粘包/半包）
//! ├── server.rs   # TCP 异步服务端（多连接并发）
//! ├── client.rs   # TCP 异步客户端（读写分离、自动重连）
//! └── README.md   # 使用说明文档
//! ```
//!
//! # 核心特性
//!
//! - **消息帧化**：使用 `tokio_util::codec::LengthDelimitedCodec`，4 字节长度前缀，解决 TCP 粘包/半包
//! - **异步 IO**：基于 tokio 异步运行时，高并发低开销
//! - **多连接并发**：服务端为每个连接创建独立任务，互不阻塞
//! - **全双工通信**：客户端读写分离，可同时发送和接收
//! - **心跳保活**：内置 Ping/Pong 机制，支持 RTT 测量
//! - **自动重连**：客户端支持可配置的自动重连机制
//! - **超时控制**：支持连接超时、读写超时
//! - **优雅停机**：通过 watch 通道实现服务端/客户端的优雅停止
//! - **配置验证**：所有配置在创建时自动验证
//!
//! # 快速开始
//!
//! ## 服务端示例
//!
//! ```ignore
//! use crate::tcp::{TcpService, TcpServerConfig};
//! use crate::tcp::message::TcpMessage;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), String> {
//!     let config = TcpServerConfig::new("0.0.0.0:8080");
//!     let mut service = TcpService::new_server(config)?;
//!
//!     // 设置消息回调
//!     service.on_message(|msg, addr, conn_id| {
//!         println!("连接 {} ({}) 消息: {}", conn_id, addr, msg.payload);
//!         Box::pin(async {})
//!     }).await;
//!
//!     service.start_server().await?;
//!
//!     tokio::signal::ctrl_c().await.unwrap();
//!     service.stop_server().await
//! }
//! ```
//!
//! ## 客户端示例
//!
//! ```ignore
//! use crate::tcp::{TcpService, TcpClientConfig};
//! use crate::tcp::message::TcpMessage;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), String> {
//!     let config = TcpClientConfig::new("127.0.0.1:8080")
//!         .with_timeout(5000);
//!     let service = TcpService::new_client(config)?;
//!
//!     service.connect().await?;
//!     service.send(&TcpMessage::data("Hello, Server!")).await?;
//!
//!     let msg = service.recv().await?;
//!     println!("收到回复: {}", msg.payload);
//!
//!     let rtt = service.ping().await?;
//!     println!("RTT: {}ms", rtt);
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod message;
pub mod codec;
pub mod server;
pub mod client;

// 重新导出常用类型，方便外部使用
pub use config::{TcpClientConfig, TcpServerConfig};
pub use message::{MessageType, TcpMessage};
pub use server::{TcpServer, ConnectionId, ConnectionInfo, ConnectionEvent};
pub use client::TcpClient;

// 导入日志宏
use crate::log_info;
use std::net::SocketAddr;
use tokio::sync::mpsc;

/// TCP 服务统一管理器
///
/// 封装了 TCP 服务端和客户端的功能，提供统一的接口。
/// 一个 `TcpService` 实例可以是服务端或客户端，但不能同时是两者。
///
/// # 设计说明
///
/// 该结构体将服务端和客户端的创建与使用统一到一个接口中，
/// 方便在 Tauri 应用中通过命令调用使用。
///
/// # 使用模式
///
/// ## 作为服务端使用
///
/// ```ignore
/// let mut service = TcpService::new_server(config)?;
/// service.on_message(|msg, addr, conn_id| { ... }).await;
/// service.start_server().await?;
/// ```
///
/// ## 作为客户端使用
///
/// ```ignore
/// let service = TcpService::new_client(config)?;
/// service.connect().await?;
/// service.send(&TcpMessage::data("hello")).await?;
/// ```
pub struct TcpService {
    /// 服务端实例（如果是服务端模式）
    server: Option<TcpServer>,
    /// 客户端实例（如果是客户端模式）
    client: Option<TcpClient>,
}

impl TcpService {
    /// 创建 TCP 服务端
    ///
    /// # 参数
    /// * `config` - 服务端配置
    pub fn new_server(config: TcpServerConfig) -> Result<Self, String> {
        log_info!("创建 TCP 服务（服务端模式）");
        let server = TcpServer::new(config)?;
        Ok(Self {
            server: Some(server),
            client: None,
        })
    }

    /// 创建 TCP 客户端
    ///
    /// # 参数
    /// * `config` - 客户端配置
    pub fn new_client(config: TcpClientConfig) -> Result<Self, String> {
        log_info!("创建 TCP 服务（客户端模式）");
        let client = TcpClient::new(config)?;
        Ok(Self {
            server: None,
            client: Some(client),
        })
    }

    // ============================================================
    // 服务端相关方法
    // ============================================================

    /// 设置服务端的消息回调（仅服务端模式有效）
    ///
    /// # 参数
    /// * `callback` - 消息处理回调，接收 `(TcpMessage, SocketAddr, ConnectionId)`
    pub async fn on_message<F, Fut>(&mut self, callback: F) -> Result<(), String>
    where
        F: Fn(TcpMessage, SocketAddr, ConnectionId) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let server = self
            .server
            .as_mut()
            .ok_or_else(|| "当前不是服务端模式".to_string())?;
        server.on_message(callback).await;
        Ok(())
    }

    /// 设置服务端的连接事件回调（仅服务端模式有效）
    ///
    /// # 参数
    /// * `callback` - 连接事件回调
    pub async fn on_connection_event<F, Fut>(&mut self, callback: F) -> Result<(), String>
    where
        F: Fn(ConnectionEvent) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let server = self
            .server
            .as_mut()
            .ok_or_else(|| "当前不是服务端模式".to_string())?;
        server.on_connection_event(callback).await;
        Ok(())
    }

    /// 启动服务端（仅服务端模式有效）
    pub async fn start_server(&mut self) -> Result<(), String> {
        let server = self
            .server
            .as_mut()
            .ok_or_else(|| "当前不是服务端模式".to_string())?;
        server.start().await
    }

    /// 停止服务端（仅服务端模式有效）
    pub async fn stop_server(&mut self) -> Result<(), String> {
        let server = self
            .server
            .as_mut()
            .ok_or_else(|| "当前不是服务端模式".to_string())?;
        server.stop().await
    }

    /// 服务端广播消息给所有连接（仅服务端模式有效）
    pub async fn broadcast(&self, msg: &TcpMessage) -> Result<(), String> {
        let server = self
            .server
            .as_ref()
            .ok_or_else(|| "当前不是服务端模式".to_string())?;
        server.broadcast(msg).await
    }

    /// 获取服务端活跃连接列表（仅服务端模式有效）
    pub async fn get_connections(&self) -> Result<Vec<ConnectionInfo>, String> {
        let server = self
            .server
            .as_ref()
            .ok_or_else(|| "当前不是服务端模式".to_string())?;
        Ok(server.get_connections().await)
    }

    /// 获取服务端活跃连接数（仅服务端模式有效）
    pub async fn connection_count(&self) -> Result<usize, String> {
        let server = self
            .server
            .as_ref()
            .ok_or_else(|| "当前不是服务端模式".to_string())?;
        Ok(server.connection_count().await)
    }

    // ============================================================
    // 客户端相关方法
    // ============================================================

    /// 客户端连接服务端（仅客户端模式有效）
    pub async fn connect(&self) -> Result<(), String> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| "当前不是客户端模式".to_string())?;
        client.connect().await
    }

    /// 客户端断开连接（仅客户端模式有效）
    pub async fn disconnect(&self) -> Result<(), String> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| "当前不是客户端模式".to_string())?;
        client.disconnect().await
    }

    /// 客户端发送消息（仅客户端模式有效）
    pub async fn send(&self, msg: &TcpMessage) -> Result<(), String> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| "当前不是客户端模式".to_string())?;
        client.send(msg).await
    }

    /// 客户端接收消息（仅客户端模式有效）
    pub async fn recv(&self) -> Result<TcpMessage, String> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| "当前不是客户端模式".to_string())?;
        client.recv().await
    }

    /// 客户端 Ping 测试（仅客户端模式有效）
    pub async fn ping(&self) -> Result<u64, String> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| "当前不是客户端模式".to_string())?;
        client.ping().await
    }

    /// 客户端启动后台接收循环（仅客户端模式有效）
    pub async fn start_recv_loop(&self) -> Result<mpsc::UnboundedReceiver<TcpMessage>, String> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| "当前不是客户端模式".to_string())?;
        client.start_recv_loop().await
    }

    /// 客户端请求-响应（仅客户端模式有效）
    pub async fn request(&self, msg: &TcpMessage) -> Result<TcpMessage, String> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| "当前不是客户端模式".to_string())?;
        client.request(msg).await
    }

    /// 判断是否为服务端模式
    pub fn is_server(&self) -> bool {
        self.server.is_some()
    }

    /// 判断是否为客户端模式
    pub fn is_client(&self) -> bool {
        self.client.is_some()
    }
}
