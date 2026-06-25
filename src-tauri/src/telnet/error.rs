//! Telnet 错误类型定义
//!
//! 定义 telnet 模块的统一错误类型，实现 `std::error::Error` trait，
//! 并支持序列化以便通过 Tauri 命令返回给前端。

use serde::Serialize;
use std::fmt;

/// Telnet 操作错误类型
#[derive(Debug, Clone, Serialize)]
pub enum TelnetError {
    /// 连接错误
    ConnectionError(String),
    /// 连接超时
    ConnectionTimeout(String),
    /// 未连接
    NotConnected(String),
    /// 认证失败
    AuthenticationError(String),
    /// 登录超时
    LoginTimeout(String),
    /// 命令执行错误
    CommandError(String),
    /// 命令执行超时
    CommandTimeout(String),
    /// 文件下载错误
    FileDownloadError(String),
    /// IO 错误
    IoError(String),
    /// 配置错误
    ConfigError(String),
    /// 未知错误
    Unknown(String),
}

impl std::error::Error for TelnetError {}

impl fmt::Display for TelnetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TelnetError::ConnectionError(e) => write!(f, "连接错误: {}", e),
            TelnetError::ConnectionTimeout(e) => write!(f, "连接超时: {}", e),
            TelnetError::NotConnected(e) => write!(f, "未连接: {}", e),
            TelnetError::AuthenticationError(e) => write!(f, "认证失败: {}", e),
            TelnetError::LoginTimeout(e) => write!(f, "登录超时: {}", e),
            TelnetError::CommandError(e) => write!(f, "命令执行错误: {}", e),
            TelnetError::CommandTimeout(e) => write!(f, "命令执行超时: {}", e),
            TelnetError::FileDownloadError(e) => write!(f, "文件下载错误: {}", e),
            TelnetError::IoError(e) => write!(f, "IO 错误: {}", e),
            TelnetError::ConfigError(e) => write!(f, "配置错误: {}", e),
            TelnetError::Unknown(e) => write!(f, "未知错误: {}", e),
        }
    }
}

/// 将 std::io::Error 转换为 TelnetError
impl From<std::io::Error> for TelnetError {
    fn from(err: std::io::Error) -> Self {
        TelnetError::IoError(err.to_string())
    }
}

/// 将 tokio::time::error::Elapsed 转换为 TelnetError
impl From<tokio::time::error::Elapsed> for TelnetError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        TelnetError::CommandTimeout("操作超时".to_string())
    }
}

/// 将 serde_json::Error 转换为 TelnetError
impl From<serde_json::Error> for TelnetError {
    fn from(err: serde_json::Error) -> Self {
        TelnetError::Unknown(format!("序列化错误: {}", err))
    }
}

/// Telnet 操作结果类型
pub type TelnetResult<T> = Result<T, TelnetError>;
