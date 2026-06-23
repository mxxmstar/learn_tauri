//! IP 地址池管理模块
//! 
//! 该模块负责管理 DHCP 服务器的 IP 地址池，
//! 包括地址的分配、回收、查询等功能。

use crate::dhcp::config::{DhcpConfig, DhcpPoolRange};
// 导入日志宏（由于使用了 #[macro_export]，需要在每个文件中导入）
use crate::{log_info, log_debug, log_warn};
use std::collections::HashMap;
use std::net::Ipv4Addr;

/// IP 地址池
/// 
/// 管理一个或多个 IP 地址范围，提供地址分配和回收功能
pub struct IpPool {
    /// 配置引用
    config: DhcpConfig,
    /// 所有可用的 IP 地址集合
    available_ips: Vec<u32>,
    /// 已分配的 IP 地址映射（IP -> MAC）
    allocated_ips: HashMap<u32, String>,
    /// 已分配的 IP 数量
    allocated_count: u32,
}

impl IpPool {
    /// 创建新的 IP 地址池
    /// 
    /// # 参数
    /// * `pools` - 地址池范围列表
    /// 
    /// # 返回值
    /// 返回 Result，成功时包含 IpPool 实例
    pub fn new(pools: &[DhcpPoolRange]) -> Result<Self, String> {
        log_info!("正在初始化 IP 地址池...");
        
        let mut available_ips = Vec::new();
        
        // 遍历所有地址池范围
        for (i, pool) in pools.iter().enumerate() {
            let start_ip = Self::ip_to_u32(&pool.start_ip);
            let end_ip = Self::ip_to_u32(&pool.end_ip);
            
            // 将范围内的所有 IP 地址添加到可用列表
            for ip in start_ip..=end_ip {
                available_ips.push(ip);
            }
            
            log_info!(
                "地址池 {}: {}-{} (共 {} 个地址)",
                i + 1,
                pool.start_ip,
                pool.end_ip,
                end_ip - start_ip + 1
            );
        }
        
        let total_count = available_ips.len() as u32;
        log_info!("IP 地址池初始化完成，共 {} 个可用地址", total_count);
        
        Ok(Self {
            config: DhcpConfig {
                subnet: String::new(),
                gateway: String::new(),
                dns_servers: vec![],
                pools: vec![],
                lease_time_default: 0,
                lease_time_max: 0,
                listen_port: 0,
                interface: None,
            },
            available_ips,
            allocated_ips: HashMap::new(),
            allocated_count: 0,
        })
    }
    
    /// 分配一个 IP 地址
    /// 
    /// # 参数
    /// * `mac` - 客户端的 MAC 地址
    /// 
    /// # 返回值
    /// 返回 Option，成功时包含分配的 IP 地址字符串
    pub fn allocate_ip(&mut self, mac: &str) -> Option<String> {
        // 检查该 MAC 是否已经分配了 IP
        for (ip, allocated_mac) in &self.allocated_ips {
            if allocated_mac == mac {
                log_debug!("MAC {} 已分配 IP: {}", mac, Self::u32_to_ip(*ip));
                return Some(Self::u32_to_ip(*ip));
            }
        }
        
        // 如果没有可用的 IP 地址
        if self.available_ips.is_empty() {
            log_warn!("IP 地址池已耗尽，无法分配新地址");
            return None;
        }
        
        // 分配第一个可用的 IP 地址
        let ip = self.available_ips.remove(0);
        self.allocated_ips.insert(ip, mac.to_string());
        self.allocated_count += 1;
        
        let ip_str = Self::u32_to_ip(ip);
        log_info!("为 MAC {} 分配 IP 地址: {}", mac, ip_str);
        
        Some(ip_str)
    }
    
    /// 释放一个 IP 地址
    /// 
    /// # 参数
    /// * `ip` - 要释放的 IP 地址
    /// 
    /// # 返回值
    /// 返回 bool，表示是否成功释放
    pub fn release_ip(&mut self, ip: &str) -> bool {
        let ip_u32 = Self::ip_to_u32(ip);
        
        // 从已分配映射中移除
        if self.allocated_ips.remove(&ip_u32).is_some() {
            // 将 IP 地址添加回可用列表
            self.available_ips.push(ip_u32);
            self.available_ips.sort(); // 保持有序
            self.allocated_count -= 1;
            
            log_info!("释放 IP 地址: {}", ip);
            true
        } else {
            log_warn!("尝试释放未分配的 IP 地址: {}", ip);
            false
        }
    }
    
    /// 根据 MAC 地址释放 IP 地址
    /// 
    /// # 参数
    /// * `mac` - 要释放的 MAC 地址
    /// 
    /// # 返回值
    /// 返回 Option，包含释放的 IP 地址
    pub fn release_ip_by_mac(&mut self, mac: &str) -> Option<String> {
        let mut ip_to_release = None;
        
        // 查找该 MAC 地址对应的 IP
        for (ip, allocated_mac) in &self.allocated_ips {
            if allocated_mac == mac {
                ip_to_release = Some(*ip);
                break;
            }
        }
        
        // 释放找到的 IP
        if let Some(ip) = ip_to_release {
            let ip_str = Self::u32_to_ip(ip);
            self.release_ip(&ip_str);
            Some(ip_str)
        } else {
            None
        }
    }
    
    /// 检查 IP 地址是否已分配
    /// 
    /// # 参数
    /// * `ip` - 要检查的 IP 地址
    /// 
    /// # 返回值
    /// 返回 bool，表示是否已分配
    pub fn is_allocated(&self, ip: &str) -> bool {
        let ip_u32 = Self::ip_to_u32(ip);
        self.allocated_ips.contains_key(&ip_u32)
    }
    
    /// 获取已分配的 IP 数量
    pub fn allocated_count(&self) -> u32 {
        self.allocated_count
    }
    
    /// 获取可用的 IP 数量
    pub fn available_count(&self) -> u32 {
        self.available_ips.len() as u32
    }
    
    /// 获取总地址数
    pub fn total_count(&self) -> u32 {
        self.allocated_count + self.available_ips.len() as u32
    }
    
    /// 将 IP 地址字符串转换为 u32
    /// 
    /// # 参数
    /// * `ip` - IP 地址字符串
    /// 
    /// # 返回值
    /// 返回 u32 格式的 IP 地址
    fn ip_to_u32(ip: &str) -> u32 {
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
    fn u32_to_ip(ip: u32) -> String {
        let ip_addr = Ipv4Addr::from(ip);
        ip_addr.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dhcp::config::DhcpPoolRange;
    
    #[test]
    fn test_ip_allocation() {
        let pools = vec![
            DhcpPoolRange {
                start_ip: "192.168.1.100".to_string(),
                end_ip: "192.168.1.102".to_string(),
            }
        ];
        
        let mut pool = IpPool::new(&pools).unwrap();
        
        // 测试分配
        let ip1 = pool.allocate_ip("00:11:22:33:44:55");
        assert!(ip1.is_some());
        assert_eq!(pool.allocated_count(), 1);
        
        // 测试同一 MAC 再次分配
        let ip2 = pool.allocate_ip("00:11:22:33:44:55");
        assert_eq!(ip1, ip2);
        
        // 测试不同 MAC 分配
        let ip3 = pool.allocate_ip("AA:BB:CC:DD:EE:FF");
        assert!(ip3.is_some());
        assert_ne!(ip1, ip3);
        assert_eq!(pool.allocated_count(), 2);
    }
    
    #[test]
    fn test_ip_release() {
        let pools = vec![
            DhcpPoolRange {
                start_ip: "192.168.1.100".to_string(),
                end_ip: "192.168.1.101".to_string(),
            }
        ];
        
        let mut pool = IpPool::new(&pools).unwrap();
        pool.allocate_ip("00:11:22:33:44:55");
        
        assert_eq!(pool.allocated_count(), 1);
        assert_eq!(pool.available_count(), 1);
        
        // 释放 IP
        pool.release_ip("192.168.1.100");
        assert_eq!(pool.allocated_count(), 0);
        assert_eq!(pool.available_count(), 2);
    }
}
