//! DHCP 配置管理模块
//! 
//! 该模块负责管理和验证 DHCP 服务器的配置信息，
//! 包括网段、网关、DNS 服务器、地址池范围等。

use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// DHCP 地址池范围
/// 
/// 定义一个 IP 地址分配范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhcpPoolRange {
    /// 起始 IP 地址
    pub start_ip: String,
    /// 结束 IP 地址
    pub end_ip: String,
}

/// DHCP 服务器配置
/// 
/// 包含所有 DHCP 服务器运行所需的配置参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhcpConfig {
    /// 网段（如 "192.168.1.0/24"）
    pub subnet: String,
    /// 网关地址
    pub gateway: String,
    /// DNS 服务器列表
    pub dns_servers: Vec<String>,
    /// DHCP 地址池范围列表
    pub pools: Vec<DhcpPoolRange>,
    /// 默认租约时间（秒）
    pub lease_time_default: u32,
    /// 最大租约时间（秒）
    pub lease_time_max: u32,
    /// 监听端口（默认 67）
    pub listen_port: u16,
    /// 网络接口（可选）
    pub interface: Option<String>,
}

impl DhcpConfig {
    /// 验证配置的有效性
    /// 
    /// # 返回值
    /// 返回 Result，成功时返回 ()，失败时返回错误信息
    pub fn validate(&self) -> Result<(), String> {
        // 验证网段格式
        if !self.is_valid_subnet(&self.subnet) {
            return Err(format!("无效的网段格式: {}", self.subnet));
        }
        
        // 验证网关地址
        if self.gateway.parse::<Ipv4Addr>().is_err() {
            return Err(format!("无效的网关地址: {}", self.gateway));
        }
        
        // 验证 DNS 服务器
        for dns in &self.dns_servers {
            if dns.parse::<Ipv4Addr>().is_err() {
                return Err(format!("无效的 DNS 服务器地址: {}", dns));
            }
        }
        
        // 验证地址池范围
        for (i, pool) in self.pools.iter().enumerate() {
            if pool.start_ip.parse::<Ipv4Addr>().is_err() {
                return Err(format!("地址池 {} 的起始 IP 无效: {}", i, pool.start_ip));
            }
            if pool.end_ip.parse::<Ipv4Addr>().is_err() {
                return Err(format!("地址池 {} 的结束 IP 无效: {}", i, pool.end_ip));
            }
            
            // 验证起始 IP 是否小于等于结束 IP
            let start: u32 = self.ip_to_u32(&pool.start_ip);
            let end: u32 = self.ip_to_u32(&pool.end_ip);
            if start > end {
                return Err(format!(
                    "地址池 {} 的起始 IP ({}) 大于结束 IP ({})",
                    i, pool.start_ip, pool.end_ip
                ));
            }
        }
        
        // 验证租约时间
        if self.lease_time_default == 0 {
            return Err("默认租约时间必须大于 0".to_string());
        }
        if self.lease_time_max < self.lease_time_default {
            return Err("最大租约时间必须大于等于默认租约时间".to_string());
        }
        
        Ok(())
    }
    
    /// 检查网段格式是否有效（简单验证）
    fn is_valid_subnet(&self, subnet: &str) -> bool {
        // 简单验证格式：xxx.xxx.xxx.xxx/xx
        let parts: Vec<&str> = subnet.split('/').collect();
        if parts.len() != 2 {
            return false;
        }
        
        // 验证 IP 部分
        if parts[0].parse::<Ipv4Addr>().is_err() {
            return false;
        }
        
        // 验证掩码部分
        if let Ok(mask) = parts[1].parse::<u8>() {
            if mask <= 32 {
                return true;
            }
        }
        
        false
    }
    
    /// 将 IP 地址字符串转换为 u32
    /// 
    /// # 参数
    /// * `ip` - IP 地址字符串
    /// 
    /// # 返回值
    /// 返回 u32 格式的 IP 地址
    pub fn ip_to_u32(&self, ip: &str) -> u32 {
        let ip_addr: Ipv4Addr = ip.parse().unwrap_or(Ipv4Addr::new(0, 0, 0, 0));
        u32::from(ip_addr)
    }
    
    /// 将 u32 格式的 IP 地址转换为字符串
    /// 
    /// # 参数
    /// * `ip` - u32 格式的 IP 地址
    /// 
    /// # 返回值
    /// 返回 IP 地址字符串
    pub fn u32_to_ip(&self, ip: u32) -> String {
        let ip_addr = Ipv4Addr::from(ip);
        ip_addr.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_validation() {
        let config = DhcpConfig {
            subnet: "192.168.1.0/24".to_string(),
            gateway: "192.168.1.1".to_string(),
            dns_servers: vec!["8.8.8.8".to_string(), "114.114.114.114".to_string()],
            pools: vec![
                DhcpPoolRange {
                    start_ip: "192.168.1.100".to_string(),
                    end_ip: "192.168.1.200".to_string(),
                }
            ],
            lease_time_default: 86400,
            lease_time_max: 604800,
            listen_port: 67,
            interface: None,
        };
        
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_invalid_subnet() {
        let mut config = DhcpConfig {
            subnet: "invalid".to_string(),
            gateway: "192.168.1.1".to_string(),
            dns_servers: vec![],
            pools: vec![],
            lease_time_default: 86400,
            lease_time_max: 604800,
            listen_port: 67,
            interface: None,
        };
        
        assert!(config.validate().is_err());
    }
}
