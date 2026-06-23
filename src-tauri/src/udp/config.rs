//! UDP 配置管理模块
//!
//! 该模块负责管理 UDP 服务端和客户端的配置信息，
//! 包括监听地址、缓冲区大小、超时时间、广播模式等。

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// 默认接收缓冲区大小（字节）
pub const DEFAULT_BUFFER_SIZE: usize = 4096;

/// 默认读写超时时间（毫秒）
pub const DEFAULT_TIMEOUT_MS: u64 = 5000;

/// UDP 服务端配置
///
/// 包含 UDP 服务端运行所需的所有配置参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpServerConfig {
    /// 监听地址（如 "0.0.0.0:8080"）
    pub listen_addr: String,
    /// 接收缓冲区大小（字节）
    pub buffer_size: usize,
    /// 是否启用广播模式
    pub broadcast: bool,
    /// 是否启用多播（可选）
    pub multicast: Option<MulticastConfig>,
}

/// 多播配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MulticastConfig {
    /// 多播组地址（如 "239.0.0.1"）
    pub group_addr: String,
    /// 多播接口（可选，None 表示使用默认接口）
    pub interface: Option<String>,
    /// 多播 TTL（生存时间）
    pub ttl: u32,
}

impl UdpServerConfig {
    /// 创建新的服务端配置
    ///
    /// # 参数
    /// * `listen_addr` - 监听地址（如 "0.0.0.0:8080"）
    pub fn new(listen_addr: impl Into<String>) -> Self {
        Self {
            listen_addr: listen_addr.into(),
            buffer_size: DEFAULT_BUFFER_SIZE,
            broadcast: false,
            multicast: None,
        }
    }

    /// 设置缓冲区大小
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// 启用广播模式
    pub fn with_broadcast(mut self, enabled: bool) -> Self {
        self.broadcast = enabled;
        self
    }

    /// 设置多播配置
    pub fn with_multicast(mut self, config: MulticastConfig) -> Self {
        self.multicast = Some(config);
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

        // 验证多播配置
        if let Some(ref mc) = self.multicast {
            if mc.group_addr.parse::<std::net::Ipv4Addr>().is_err() {
                return Err(format!("无效的多播组地址: {}", mc.group_addr));
            }
            // 检查是否为合法的多播地址（224.0.0.0 ~ 239.255.255.255）
            let addr: std::net::Ipv4Addr = mc.group_addr.parse().unwrap();
            if !addr.is_multicast() {
                return Err(format!("地址 {} 不是有效的多播地址", mc.group_addr));
            }
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

impl Default for UdpServerConfig {
    fn default() -> Self {
        Self::new("0.0.0.0:8080")
    }
}

/// UDP 客户端配置
///
/// 包含 UDP 客户端运行所需的所有配置参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpClientConfig {
    /// 目标服务端地址（如 "127.0.0.1:8080"）
    pub server_addr: String,
    /// 本地绑定地址（可选，None 表示由系统自动分配）
    pub bind_addr: Option<String>,
    /// 接收缓冲区大小（字节）
    pub buffer_size: usize,
    /// 读写超时时间（毫秒，0 表示不超时）
    pub timeout_ms: u64,
}

impl UdpClientConfig {
    /// 创建新的客户端配置
    ///
    /// # 参数
    /// * `server_addr` - 目标服务端地址（如 "127.0.0.1:8080"）
    pub fn new(server_addr: impl Into<String>) -> Self {
        Self {
            server_addr: server_addr.into(),
            bind_addr: None,
            buffer_size: DEFAULT_BUFFER_SIZE,
            timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }

    /// 设置本地绑定地址
    pub fn with_bind_addr(mut self, addr: impl Into<String>) -> Self {
        self.bind_addr = Some(addr.into());
        self
    }

    /// 设置缓冲区大小
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// 设置超时时间（毫秒，0 表示不超时）
    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
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

        // 验证绑定地址格式（如果存在）
        if let Some(ref bind) = self.bind_addr {
            if bind.parse::<SocketAddr>().is_err() {
                return Err(format!("无效的绑定地址: {}", bind));
            }
        }

        // 验证缓冲区大小
        if self.buffer_size == 0 {
            return Err("缓冲区大小必须大于 0".to_string());
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

impl Default for UdpClientConfig {
    fn default() -> Self {
        Self::new("127.0.0.1:8080")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_validation() {
        let config = UdpServerConfig::new("0.0.0.0:8080")
            .with_buffer_size(8192)
            .with_broadcast(true);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_server_addr() {
        let config = UdpServerConfig::new("invalid_addr");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_client_config_validation() {
        let config = UdpClientConfig::new("127.0.0.1:8080")
            .with_bind_addr("0.0.0.0:0")
            .with_timeout(3000);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_multicast_validation() {
        // 合法多播地址
        let config = UdpServerConfig::new("0.0.0.0:8080").with_multicast(MulticastConfig {
            group_addr: "239.0.0.1".to_string(),
            interface: None,
            ttl: 4,
        });
        assert!(config.validate().is_ok());

        // 非法多播地址（非多播范围）
        let config = UdpServerConfig::new("0.0.0.0:8080").with_multicast(MulticastConfig {
            group_addr: "192.168.1.1".to_string(),
            interface: None,
            ttl: 4,
        });
        assert!(config.validate().is_err());
    }
}
