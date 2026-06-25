//! 串口通信模块
//!
//! 提供跨平台的串口通信能力，支持：
//! - 基本的串口操作（打开/关闭/配置/读写）
//! - 数据帧解析（支持自定义协议）
//! - 协议扩展接口（通过 `ProtocolParser` trait）
//!
//! # 模块结构
//!
//! ```text
//! serial/
//! ├── mod.rs          # 模块入口，重新导出常用类型
//! ├── client.rs       # 核心客户端实现（SerialClient）
//! ├── error.rs        # 错误类型定义（SerialError）
//! ├── config.rs       # 配置管理（SerialConfig）
//! ├── types.rs        # 数据类型定义
//! └── protocol.rs    # 协议解析 trait 和内置解析器
//! ```
//!
//! # 快速开始
//!
//! ```rust
//! use crate::serial::{SerialClient, SerialConfig, ProtocolParser};
//! use crate::serial::protocol::DelimiterParser;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 1. 创建配置
//!     let config = SerialConfig::new("COM1")
//!         .baud_rate(115200);
//!
//!     // 2. 创建客户端
//!     let client = SerialClient::new(config)?;
//!
//!     // 3. 打开串口
//!     client.open().await?;
//!
//!     // 4. 写入数据
//!     client.write(b"hello").await?;
//!
//!     // 5. 读取数据
//!     let data = client.read(100).await?;
//!     println!("收到数据: {:?}", data);
//!
//!     // 6. 关闭串口
//!     client.close().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # 自定义协议
//!
//! 实现 `ProtocolParser` trait 来支持自定义协议：
//!
//! ```rust
//! use crate::serial::protocol::{ProtocolParser, ParseResult};
//!
//! struct MyProtocol;
//!
//! impl ProtocolParser for MyProtocol {
//!     fn parse_frame(&self, buffer: &[u8]) -> ParseResult {
//!         // 自定义解析逻辑
//!         // ...
//!         ParseResult::Incomplete
//!     }
//!
//!     fn encode_frame(&self, data: &[u8]) -> Vec<u8> {
//!         // 自定义编码逻辑
//!         // ...
//!         data.to_vec()
//!     }
//! }
//! ```

// 声明子模块
pub mod types;      // 数据类型定义
pub mod error;      // 错误类型定义
pub mod config;     // 配置管理
pub mod protocol;   // 协议解析
pub mod client;     // 核心客户端

// 重新导出常用类型，方便用户使用
// 设计风格与 telnet 模块保持一致

// 客户端
pub use client::SerialClient;

// 配置
pub use config::SerialConfig;

// 错误类型
pub use error::SerialError;

// 数据类型（包含 SerialOpResult）
pub use types::{
    ConnectionStatus, DataBits, StopBits, Parity, FlowControl,
    SerialOpResult,
};

// 协议解析
pub use protocol::{ProtocolParser, ParseResult};
pub use protocol::DelimiterParser;
pub use protocol::LengthPrefixParser;

/// 模块版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 列出所有可用的串口
///
/// 便捷函数，封装 `SerialClient::list_ports()`。
///
/// # 返回值
/// 返回 Result，成功时包含可用串口名称列表
///
/// # 示例
///
/// ```rust
/// use crate::serial::list_available_ports;
///
/// let ports = list_available_ports().unwrap();
/// for port in ports {
///     println!("可用串口: {}", port);
/// }
/// ```
pub fn list_available_ports() -> SerialOpResult<Vec<String>> {
    SerialClient::list_ports()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_list_ports() {
        // 测试列出串口（不会实际打开）
        let result = list_available_ports();
        assert!(result.is_ok());
    }
}
