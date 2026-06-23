//! SomeIP 错误类型定义
//!
//! 使用 `thiserror` 派生宏定义丰富的错误变体，
//! 覆盖编解码、长度不足、未知方法等场景。

use thiserror::Error;

/// SomeIP 协议错误类型。
///
/// 对应 C++ 中通过返回值或异常表达的错误处理逻辑。
/// 使用 `thiserror` 自动实现 `std::error::Error` trait。
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SomeIPError {
    /// 缓冲区长度不足，无法完成反序列化。
    ///
    /// 对应 C++ 中 `if (payload.size() >= ...)` 的检查。
    #[error("缓冲区长度不足：需要 {expected} 字节，实际 {actual} 字节")]
    InsufficientBuffer {
        /// 期望的最小字节数
        expected: usize,
        /// 实际可用的字节数
        actual: usize,
    },

    /// 未知的方法 ID。
    ///
    /// 对应 C++ `enum class SomeIPMethod` 中没有匹配的变体。
    #[error("未知的方法 ID: 0x{0:04X}")]
    UnknownMethod(u16),

    /// 编解码错误。
    ///
    /// 用于 payload 序列化/反序列化失败的场景（如 JSON 解析失败）。
    #[error("编解码错误: {0}")]
    CodecError(String),

    /// 无效的 IP 地址字符串。
    #[error("无效的 IPv4 地址: {0}")]
    InvalidIpAddress(String),

    /// 无效的 MAC 地址字符串。
    #[error("无效的 MAC 地址: {0}")]
    InvalidMacAddress(String),

    /// 配置文件读取失败。
    #[error("配置文件错误: {0}")]
    ConfigError(String),

    /// 消息类型或返回码不匹配。
    #[error("消息验证失败: {0}")]
    ValidationError(String),
}

impl SomeIPError {
    /// 创建缓冲区长度不足错误。
    pub fn insufficient_buffer(expected: usize, actual: usize) -> Self {
        SomeIPError::InsufficientBuffer { expected, actual }
    }

    /// 创建编解码错误。
    pub fn codec_error(msg: impl Into<String>) -> Self {
        SomeIPError::CodecError(msg.into())
    }
}

/// SomeIP 操作结果的类型别名。
pub type SomeIPResult<T> = Result<T, SomeIPError>;
