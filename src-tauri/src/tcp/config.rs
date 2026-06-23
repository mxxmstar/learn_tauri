//! TCP 配置管理模块
//!
//! 该模块负责管理 TCP 服务端和客户端的配置信息，
//! 包括监听地址、连接超时、心跳间隔、最大连接数等。
//!
//! # 配置验证
//!
//! 所有配置在创建时会自动调用 `validate()` 验证有效性，
//! 避免运行时因配置错误导致异常。

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// 默认接收缓冲区大小（字节）
pub const DEFAULT_BUFFER_SIZE: usize = 8192;

/// 默认连接超时时间（毫秒）
pub const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 5000;

/// 默认读写超时时间（毫秒，0 表示不超时）
pub const DEFAULT_TIMEOUT_MS: u64 = 10000;

/// 默认心跳间隔（毫秒）
pub const DEFAULT_HEARTBEAT_INTERVAL_MS: u64 = 30000;

/// 默认最大连接数（仅服务端）
pub const DEFAULT_MAX_CONNECTIONS: usize = 1024;

/// TCP 服务端配置
///
/// 包含 TCP 服务端运行所需的所有配置参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpServerConfig {
    /// 监听地址（如 "0.0.0.0:8080"）
    pub listen_addr: String,
    /// 接收缓冲区大小（字节）
    pub buffer_size: usize,
    /// 是否启用 TCP Keep-Alive（系统级保活）
    pub keepalive: bool,
    /// 是否启用应用层心跳（Ping/Pong）
    pub heartbeat: bool,
    /// 心跳间隔（毫秒）
    pub heartbeat_interval_ms: u64,
    /// 最大并发连接数
    pub max_connections: usize,
    /// 单连接读写超时（毫秒，0 表示不超时）
    pub connection_timeout_ms: u64,
}

impl TcpServerConfig {
    /// 创建新的服务端配置
    ///
    /// # 参数
    /// * `listen_addr` - 监听地址（如 "0.0.0.0:8080"）
    pub fn new(listen_addr: impl Into<String>) -> Self {
        Self {
            listen_addr: listen_addr.into(),
            buffer_size: DEFAULT_BUFFER_SIZE,
            keepalive: true,
            heartbeat: true,
            heartbeat_interval_ms: DEFAULT_HEARTBEAT_INTERVAL_MS,
            max_connections: DEFAULT_MAX_CONNECTIONS,
            connection_timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }

    /// 设置缓冲区大小
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// 启用/禁用 TCP Keep-Alive（系统级保活）
    pub fn with_keepalive(mut self, enabled: bool) -> Self {
        self.keepalive = enabled;
        self
    }

    /// 启用/禁用应用层心跳
    pub fn with_heartbeat(mut self, enabled: bool) -> Self {
        self.heartbeat = enabled;
        self
    }

    /// 设置心跳间隔（毫秒）
    pub fn with_heartbeat_interval(mut self, ms: u64) -> Self {
        self.heartbeat_interval_ms = ms;
        self
    }

    /// 设置最大并发连接数
    pub fn with_max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }

    /// 设置单连接读写超时（毫秒，0 表示不超时）
    pub fn with_connection_timeout(mut self, ms: u64) -> Self {
        self.connection_timeout_ms = ms;
        self
    }

    /// 验证配置有效性
    ///
    /// # 返回值
    /// 返回 Result，成功时返回 ()，失败时返回错误信息
    pub fn validate(&self) -> Result<(), String> {
        // 验证监听地址格式
        if self.listen_addr.parse::<SocketAddr>().is_err() {
            return Err(format!("无效的监听地址: {}", self.listen_addr));
        }

        // 验证缓冲区大小
        if self.buffer_size == 0 {
            return Err("缓冲区大小必须大于 0".to_string());
        }

        // 验证最大连接数
        if self.max_connections == 0 {
            return Err("最大连接数必须大于 0".to_string());
        }

        // 验证心跳间隔
        if self.heartbeat && self.heartbeat_interval_ms == 0 {
            return Err("心跳间隔必须大于 0".to_string());
        }

        Ok(())
    }

    /// 解析监听地址为 SocketAddr
    pub fn parse_listen_addr(&self) -> Result<SocketAddr, String> {
        self.listen_addr
            .parse::<SocketAddr>()
            .map_err(|e| format!("解析监听地址失败: {}", e))
    }
}

impl Default for TcpServerConfig {
    fn default() -> Self {
        Self::new("0.0.0.0:8080")
    }
}

/// TCP 客户端配置
///
/// 包含 TCP 客户端运行所需的所有配置参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpClientConfig {
    /// 目标服务端地址（如 "127.0.0.1:8080"）
    pub server_addr: String,
    /// 接收缓冲区大小（字节）
    pub buffer_size: usize,
    /// 连接超时时间（毫秒）
    pub connect_timeout_ms: u64,
    /// 读写超时时间（毫秒，0 表示不超时）
    pub timeout_ms: u64,
    /// 是否启用 TCP Keep-Alive
    pub keepalive: bool,
    /// 是否启用应用层心跳
    pub heartbeat: bool,
    /// 心跳间隔（毫秒）
    pub heartbeat_interval_ms: u64,
    /// 是否自动重连
    pub auto_reconnect: bool,
    /// 自动重连间隔（毫秒）
    pub reconnect_interval_ms: u64,
    /// 最大重连次数（0 表示无限重试）
    pub max_reconnect_attempts: u32,
}

impl TcpClientConfig {
    /// 创建新的客户端配置
    ///
    /// # 参数
    /// * `server_addr` - 目标服务端地址（如 "127.0.0.1:8080"）
    pub fn new(server_addr: impl Into<String>) -> Self {
        Self {
            server_addr: server_addr.into(),
            buffer_size: DEFAULT_BUFFER_SIZE,
            connect_timeout_ms: DEFAULT_CONNECT_TIMEOUT_MS,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            keepalive: true,
            heartbeat: true,
            heartbeat_interval_ms: DEFAULT_HEARTBEAT_INTERVAL_MS,
            auto_reconnect: false,
            reconnect_interval_ms: 3000,
            max_reconnect_attempts: 0,
        }
    }

    /// 设置缓冲区大小
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// 设置连接超时时间（毫秒）
    pub fn with_connect_timeout(mut self, ms: u64) -> Self {
        self.connect_timeout_ms = ms;
        self
    }

    /// 设置读写超时时间（毫秒，0 表示不超时）
    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// 启用/禁用 TCP Keep-Alive
    pub fn with_keepalive(mut self, enabled: bool) -> Self {
        self.keepalive = enabled;
        self
    }

    /// 启用/禁用应用层心跳
    pub fn with_heartbeat(mut self, enabled: bool) -> Self {
        self.heartbeat = enabled;
        self
    }

    /// 设置心跳间隔（毫秒）
    pub fn with_heartbeat_interval(mut self, ms: u64) -> Self {
        self.heartbeat_interval_ms = ms;
        self
    }

    /// 启用自动重连
    pub fn with_auto_reconnect(mut self, enabled: bool) -> Self {
        self.auto_reconnect = enabled;
        self
    }

    /// 设置重连间隔（毫秒）
    pub fn with_reconnect_interval(mut self, ms: u64) -> Self {
        self.reconnect_interval_ms = ms;
        self
    }

    /// 设置最大重连次数（0 表示无限重试）
    pub fn with_max_reconnect_attempts(mut self, attempts: u32) -> Self {
        self.max_reconnect_attempts = attempts;
        self
    }

    /// 验证配置有效性
    ///
    /// # 返回值
    /// 返回 Result，成功时返回 ()，失败时返回错误信息
    pub fn validate(&self) -> Result<(), String> {
        // 验证服务端地址格式
        if self.server_addr.parse::<SocketAddr>().is_err() {
            return Err(format!("无效的服务端地址: {}", self.server_addr));
        }

        // 验证缓冲区大小
        if self.buffer_size == 0 {
            return Err("缓冲区大小必须大于 0".to_string());
        }

        // 验证连接超时
        if self.connect_timeout_ms == 0 {
            return Err("连接超时时间必须大于 0".to_string());
        }

        // 验证心跳间隔
        if self.heartbeat && self.heartbeat_interval_ms == 0 {
            return Err("心跳间隔必须大于 0".to_string());
        }

        // 验证重连间隔
        if self.auto_reconnect && self.reconnect_interval_ms == 0 {
            return Err("重连间隔必须大于 0".to_string());
        }

        Ok(())
    }

    /// 解析服务端地址为 SocketAddr
    pub fn parse_server_addr(&self) -> Result<SocketAddr, String> {
        self.server_addr
            .parse::<SocketAddr>()
            .map_err(|e| format!("解析服务端地址失败: {}", e))
    }
}

impl Default for TcpClientConfig {
    fn default() -> Self {
        Self::new("127.0.0.1:8080")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_validation() {
        let config = TcpServerConfig::new("0.0.0.0:8080")
            .with_buffer_size(16384)
            .with_max_connections(2048);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_server_addr() {
        let config = TcpServerConfig::new("invalid_addr");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_client_config_validation() {
        let config = TcpClientConfig::new("127.0.0.1:8080")
            .with_connect_timeout(3000)
            .with_timeout(5000)
            .with_auto_reconnect(true)
            .with_reconnect_interval(2000);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_zero_max_connections() {
        let config = TcpServerConfig::new("0.0.0.0:8080").with_max_connections(0);
        assert!(config.validate().is_err());
    }
}
