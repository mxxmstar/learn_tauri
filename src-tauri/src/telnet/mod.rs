//! Telnet 模块
//!
//! 该模块基于 `tokio::net::TcpStream` 实现简单的 TCP 文本命令通信功能，
//! 用于连接 Linux 嵌入式设备（路由器、摄像头等），支持：
//! - Telnet 连接管理（建立/断开与设备的 TCP 连接）
//! - 登录认证（自动处理用户名/密码登录流程）
//! - 命令发送与输出接收（发送 shell 命令并读取设备返回）
//! - 文件下载（通过执行 `cat` 命令读取设备文件内容）
//!
//! # 模块结构
//!
//! ```text
//! telnet/
//! ├── mod.rs          # 模块入口，重新导出常用类型
//! ├── client.rs       # 核心客户端实现（TelnetClient）
//! ├── error.rs        # 错误类型定义（TelnetError）
//! ├── config.rs       # 配置管理（TelnetConfig）
//! ├── types.rs        # 数据类型定义（登录结果、命令结果等）
//! └── README.md       # 使用说明文档
//! ```
//!
//! # 核心特性
//!
//! - **简单文本模式**：不实现完整的 telnet 协议选项协商，仅使用原始 TCP 发送文本命令
//! - **自动提示符检测**：通过匹配常见 shell 提示符（#, $, >）判断命令执行状态
//! - **登录流程自动化**：自动检测 login:、Password: 提示符，发送凭据
//! - **超时控制**：所有操作支持可配置的超时时间
//! - **ANSI 清理**：可选清理输出中的 ANSI 转义序列
//! - **异步 IO**：基于 tokio 异步运行时，无阻塞
//!
//! # 快速开始
//!
//! ```ignore
//! use crate::telnet::{TelnetClient, TelnetConfig};
//! use crate::telnet::types::LoginResult;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 创建配置
//!     let config = TelnetConfig::new("192.168.1.1:23")
//!         .with_connect_timeout(10000)
//!         .with_login_timeout(15000)
//!         .with_command_timeout(30000);
//!
//!     // 创建客户端
//!     let client = TelnetClient::new(config)?;
//!
//!     // 连接设备
//!     client.connect().await?;
//!
//!     // 登录
//!     let login_result = client.login("admin", "password").await?;
//!     if login_result.success {
//!         println!("登录成功，提示符: {}", login_result.prompt);
//!     }
//!
//!     // 执行命令
//!     let cmd_result = client.execute_command("ls -la").await?;
//!     println!("命令输出:\n{}", cmd_result.output);
//!
//!     // 下载文件
//!     let download_result = client.download_file("/etc/config", "config.txt").await?;
//!     if download_result.success {
//!         println!("文件下载成功，大小: {} 字节", download_result.file_size);
//!     }
//!
//!     // 断开连接
//!     client.disconnect().await?;
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod error;
pub mod config;
pub mod types;

// 重新导出常用类型，方便外部使用
pub use client::TelnetClient;
pub use error::{TelnetError, TelnetResult};
pub use config::TelnetConfig;
pub use types::{
    CommandResult, ConnectionStatus, DeviceInfo, DownloadProgress, FileDownloadResult,
    LoginResult, MountResult, TelnetOpResult,
};
