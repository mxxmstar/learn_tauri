//! UDP 模块
//!
//! 该模块基于 `tokio::net::UdpSocket` 实现完整的 UDP 通信功能，
//! 包括服务端和客户端，支持结构化消息（JSON）和原始字节传输。
//!
//! # 模块结构
//!
//! ```text
//! udp/
//! ├── mod.rs      # 模块主入口，定义 UdpService 统一管理接口
//! ├── config.rs   # 配置管理（服务端/客户端配置、多播配置）
//! ├── message.rs  # 消息协议定义与编解码（JSON 结构化消息）
//! ├── server.rs   # UDP 异步服务端
//! ├── client.rs   # UDP 异步客户端
//! └── README.md   # 使用说明文档
//! ```
//!
//! # 核心特性
//!
//! - **异步 IO**：基于 tokio 异步运行时，高并发低开销
//! - **结构化消息**：使用 JSON 编码的 `UdpMessage`，支持消息类型区分
//! - **心跳检测**：内置 Ping/Pong 机制，支持 RTT 测量
//! - **广播与多播**：支持 UDP 广播和多播组通信
//! - **回调机制**：服务端支持注册消息回调，灵活处理业务逻辑
//! - **超时控制**：客户端支持可配置的读写超时
//! - **优雅停机**：通过 watch 通道实现服务端/客户端的优雅停止
//!
//! # 快速开始
//!
//! ## 服务端示例
//!
//! ```ignore
//! use crate::udp::{UdpService, UdpServerConfig};
//! use crate::udp::message::UdpMessage;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), String> {
//!     // 创建服务端配置
//!     let config = UdpServerConfig::new("0.0.0.0:8080")
//!         .with_broadcast(true);
//!
//!     // 创建服务
//!     let mut service = UdpService::new_server(config)?;
//!
//!     // 设置消息回调
//!     service.on_message(|msg, addr| {
//!         println!("收到来自 {} 的消息: {}", addr, msg.payload);
//!         Box::pin(async {})
//!     }).await;
//!
//!     // 启动服务
//!     service.start_server().await?;
//!
//!     // 保持运行
//!     tokio::signal::ctrl_c().await.unwrap();
//!     service.stop_server().await
//! }
//! ```
//!
//! ## 客户端示例
//!
//! ```ignore
//! use crate::udp::{UdpService, UdpClientConfig};
//! use crate::udp::message::UdpMessage;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), String> {
//!     // 创建客户端配置
//!     let config = UdpClientConfig::new("127.0.0.1:8080")
//!         .with_timeout(3000);
//!
//!     // 创建服务
//!     let service = UdpService::new_client(config)?;
//!
//!     // 连接服务端
//!     service.connect().await?;
//!
//!     // 发送消息
//!     service.send(&UdpMessage::data("Hello, Server!")).await?;
//!
//!     // 接收回复
//!     let (msg, addr) = service.recv().await?;
//!     println!("收到回复: {}", msg.payload);
//!
//!     // Ping 测试
//!     let rtt = service.ping().await?;
//!     println!("RTT: {}ms", rtt);
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod message;
pub mod server;
pub mod client;

// 重新导出常用类型，方便外部使用
pub use config::{MulticastConfig, UdpClientConfig, UdpServerConfig};
pub use message::{MessageType, UdpMessage};
pub use server::UdpServer;
pub use client::UdpClient;

// 导入日志宏
use crate::log_info;
use std::net::SocketAddr;
use tokio::sync::mpsc;

/// UDP 服务统一管理器
///
/// 封装了 UDP 服务端和客户端的功能，提供统一的接口。
/// 一个 `UdpService` 实例可以是服务端或客户端，但不能同时是两者。
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
/// let mut service = UdpService::new_server(config)?;
/// service.on_message(|msg, addr| { ... }).await;
/// service.start_server().await?;
/// ```
///
/// ## 作为客户端使用
///
/// ```ignore
/// let service = UdpService::new_client(config)?;
/// service.connect().await?;
/// service.send(&UdpMessage::data("hello")).await?;
/// ```
pub struct UdpService {
    /// 服务端实例（如果是服务端模式）
    server: Option<UdpServer>,
    /// 客户端实例（如果是客户端模式）
    client: Option<UdpClient>,
}

impl UdpService {
    /// 创建 UDP 服务端
    ///
    /// # 参数
    /// * `config` - 服务端配置
    pub fn new_server(config: UdpServerConfig) -> Result<Self, String> {
        log_info!("创建 UDP 服务（服务端模式）");
        let server = UdpServer::new(config)?;
        Ok(Self {
            server: Some(server),
            client: None,
        })
    }

    /// 创建 UDP 客户端
    ///
    /// # 参数
    /// * `config` - 客户端配置
    pub fn new_client(config: UdpClientConfig) -> Result<Self, String> {
        log_info!("创建 UDP 服务（客户端模式）");
        let client = UdpClient::new(config)?;
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
    /// * `callback` - 消息处理回调
    pub async fn on_message<F, Fut>(&mut self, callback: F) -> Result<(), String>
    where
        F: Fn(UdpMessage, SocketAddr) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let server = self
            .server
            .as_mut()
            .ok_or_else(|| "当前不是服务端模式".to_string())?;
        server.on_message(callback).await;
        Ok(())
    }

    /// 设置服务端的原始字节回调（仅服务端模式有效）
    ///
    /// # 参数
    /// * `callback` - 原始字节处理回调
    pub async fn on_raw<F, Fut>(&mut self, callback: F) -> Result<(), String>
    where
        F: Fn(&[u8], SocketAddr) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let server = self
            .server
            .as_mut()
            .ok_or_else(|| "当前不是服务端模式".to_string())?;
        server.on_raw(callback).await;
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

    /// 服务端发送消息到指定地址（仅服务端模式有效）
    pub async fn server_send(&self, msg: &UdpMessage, addr: SocketAddr) -> Result<(), String> {
        let server = self
            .server
            .as_ref()
            .ok_or_else(|| "当前不是服务端模式".to_string())?;
        server.send(msg, addr).await
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
    pub async fn send(&self, msg: &UdpMessage) -> Result<(), String> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| "当前不是客户端模式".to_string())?;
        client.send(msg).await
    }

    /// 客户端接收消息（仅客户端模式有效）
    pub async fn recv(&self) -> Result<(UdpMessage, SocketAddr), String> {
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
    pub async fn start_recv_loop(
        &self,
    ) -> Result<mpsc::UnboundedReceiver<(UdpMessage, SocketAddr)>, String> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| "当前不是客户端模式".to_string())?;
        client.start_recv_loop().await
    }

    /// 客户端请求-响应（仅客户端模式有效）
    pub async fn request(&self, msg: &UdpMessage) -> Result<(UdpMessage, SocketAddr), String> {
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
