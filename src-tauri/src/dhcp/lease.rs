//! DHCP 租约管理模块
//! 
//! 该模块负责管理 DHCP 租约，包括租约的创建、续期、过期和查询等功能。

// 导入日志宏（由于使用了 #[macro_export]，需要在每个文件中导入）
use crate::{log_info, log_debug, log_warn};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// 租约状态
#[derive(Debug, Clone, PartialEq)]
pub enum LeaseState {
    /// 可用状态
    Available,
    /// 已分配
    Allocated,
    /// 已过期
    Expired,
}

/// 租约信息
/// 
/// 记录一个 IP 地址的租约详细信息
#[derive(Debug, Clone)]
pub struct LeaseInfo {
    /// IP 地址
    pub ip_address: String,
    /// 客户端 MAC 地址
    pub mac_address: String,
    /// 客户端主机名（可选）
    pub hostname: Option<String>,
    /// 租约开始时间（Unix 时间戳）
    pub start_time: u64,
    /// 租约结束时间（Unix 时间戳）
    pub end_time: u64,
    /// 租约状态
    pub state: LeaseState,
}

impl LeaseInfo {
    /// 检查租约是否已过期
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.end_time
    }
    
    /// 检查租约是否即将过期（剩余时间小于总时间的 50%）
    pub fn is_renewal_needed(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let total_time = self.end_time - self.start_time;
        let remaining_time = self.end_time - now;
        remaining_time < total_time / 2
    }
}

/// 租约管理器
/// 
/// 管理所有 DHCP 租约，提供租约的创建、查询、续期和清理功能
pub struct LeaseManager {
    /// 租约映射（IP -> LeaseInfo）
    leases: HashMap<String, LeaseInfo>,
    /// 默认租约时间（秒）
    default_lease_time: u32,
    /// 最大租约时间（秒）
    max_lease_time: u32,
}

impl LeaseManager {
    /// 创建新的租约管理器
    /// 
    /// # 参数
    /// * `default_lease_time` - 默认租约时间（秒）
    /// * `max_lease_time` - 最大租约时间（秒）
    pub fn new(default_lease_time: u32, max_lease_time: u32) -> Self {
        log_info!("初始化租约管理器");
        log_info!("默认租约时间: {} 秒", default_lease_time);
        log_info!("最大租约时间: {} 秒", max_lease_time);
        
        Self {
            leases: HashMap::new(),
            default_lease_time,
            max_lease_time,
        }
    }
    
    /// 创建新的租约
    /// 
    /// # 参数
    /// * `ip` - IP 地址
    /// * `mac` - MAC 地址
    /// * `hostname` - 主机名（可选）
    /// * `requested_lease_time` - 请求的租约时间（可选）
    /// 
    /// # 返回值
    /// 返回创建的租约信息
    pub fn create_lease(
        &mut self,
        ip: &str,
        mac: &str,
        hostname: Option<String>,
        requested_lease_time: Option<u32>,
    ) -> LeaseInfo {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // 确定租约时间
        let lease_time = requested_lease_time
            .unwrap_or(self.default_lease_time)
            .min(self.max_lease_time);
        
        let lease = LeaseInfo {
            ip_address: ip.to_string(),
            mac_address: mac.to_string(),
            hostname,
            start_time: now,
            end_time: now + lease_time as u64,
            state: LeaseState::Allocated,
        };
        
        log_info!(
            "创建租约: IP={}, MAC={}, 租约时间={}秒",
            ip,
            mac,
            lease_time
        );
        
        self.leases.insert(ip.to_string(), lease.clone());
        lease
    }
    
    /// 续期租约
    /// 
    /// # 参数
    /// * `ip` - IP 地址
    /// * `mac` - MAC 地址
    /// * `requested_lease_time` - 请求的租约时间（可选）
    /// 
    /// # 返回值
    /// 返回 Result，成功时包含更新后的租约信息
    pub fn renew_lease(
        &mut self,
        ip: &str,
        mac: &str,
        requested_lease_time: Option<u32>,
    ) -> Result<LeaseInfo, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // 查找现有租约
        if let Some(lease) = self.leases.get_mut(ip) {
            // 验证 MAC 地址
            if lease.mac_address != mac {
                return Err(format!(
                    "MAC 地址不匹配: 期望 {}, 实际 {}",
                    lease.mac_address, mac
                ));
            }
            
            // 确定新的租约时间
            let lease_time = requested_lease_time
                .unwrap_or(self.default_lease_time)
                .min(self.max_lease_time);
            
            // 更新租约
            lease.start_time = now;
            lease.end_time = now + lease_time as u64;
            lease.state = LeaseState::Allocated;
            
            log_info!("续期租约: IP={}, MAC={}, 新租约时间={}秒", ip, mac, lease_time);
            
            Ok(lease.clone())
        } else {
            Err(format!("未找到 IP {} 的租约", ip))
        }
    }
    
    /// 释放租约
    /// 
    /// # 参数
    /// * `ip` - IP 地址
    /// 
    /// # 返回值
    /// 返回 bool，表示是否成功释放
    pub fn release_lease(&mut self, ip: &str) -> bool {
        if let Some(lease) = self.leases.get_mut(ip) {
            lease.state = LeaseState::Available;
            log_info!("释放租约: IP={}", ip);
            true
        } else {
            log_warn!("尝试释放不存在的租约: IP={}", ip);
            false
        }
    }
    
    /// 根据 MAC 地址查找租约
    /// 
    /// # 参数
    /// * `mac` - MAC 地址
    /// 
    /// # 返回值
    /// 返回 Option，包含找到的租约信息
    pub fn find_lease_by_mac(&self, mac: &str) -> Option<&LeaseInfo> {
        self.leases.values().find(|lease| lease.mac_address == mac && lease.state == LeaseState::Allocated)
    }
    
    /// 根据 IP 地址查找租约
    /// 
    /// # 参数
    /// * `ip` - IP 地址
    /// 
    /// # 返回值
    /// 返回 Option，包含找到的租约信息
    pub fn find_lease_by_ip(&self, ip: &str) -> Option<&LeaseInfo> {
        self.leases.get(ip)
    }
    
    /// 清理过期的租约
    /// 
    /// # 返回值
    /// 返回清理的租约数量
    pub fn cleanup_expired_leases(&mut self) -> u32 {
        let mut cleaned = 0;
        let mut expired_ips = Vec::new();
        
        // 查找所有过期的租约
        for (ip, lease) in &self.leases {
            if lease.state == LeaseState::Allocated && lease.is_expired() {
                expired_ips.push(ip.clone());
            }
        }
        
        // 清理过期的租约
        for ip in expired_ips {
            if let Some(lease) = self.leases.get_mut(&ip) {
                lease.state = LeaseState::Expired;
                cleaned += 1;
                log_info!("清理过期租约: IP={}", ip);
            }
        }
        
        if cleaned > 0 {
            log_info!("共清理 {} 个过期租约", cleaned);
        }
        
        cleaned
    }
    
    /// 获取所有租约列表
    /// 
    /// # 返回值
    /// 返回所有租约信息的向量
    pub fn get_all_leases(&self) -> Vec<LeaseInfo> {
        self.leases.values().cloned().collect()
    }
    
    /// 获取活跃的租约列表
    /// 
    /// # 返回值
    /// 返回所有状态为已分配的租约信息向量
    pub fn get_active_leases(&self) -> Vec<LeaseInfo> {
        self.leases
            .values()
            .filter(|lease| lease.state == LeaseState::Allocated && !lease.is_expired())
            .cloned()
            .collect()
    }
    
    /// 获取租约统计信息
    /// 
    /// # 返回值
    /// 返回 (总租约数, 活跃租约数, 过期租约数)
    pub fn get_stats(&self) -> (u32, u32, u32) {
        let total = self.leases.len() as u32;
        let active = self.get_active_leases().len() as u32;
        let expired = total - active;
        (total, active, expired)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lease_creation() {
        let mut manager = LeaseManager::new(86400, 604800);
        
        let lease = manager.create_lease(
            "192.168.1.100",
            "00:11:22:33:44:55",
            Some("test-host".to_string()),
            None,
        );
        
        assert_eq!(lease.ip_address, "192.168.1.100");
        assert_eq!(lease.mac_address, "00:11:22:33:44:55");
        assert_eq!(lease.hostname, Some("test-host".to_string()));
        assert_eq!(lease.state, LeaseState::Allocated);
    }
    
    #[test]
    fn test_lease_renewal() {
        let mut manager = LeaseManager::new(86400, 604800);
        
        // 创建租约
        manager.create_lease(
            "192.168.1.100",
            "00:11:22:33:44:55",
            None,
            None,
        );
        
        // 续期租约
        let result = manager.renew_lease(
            "192.168.1.100",
            "00:11:22:33:44:55",
            Some(172800),
        );
        
        assert!(result.is_ok());
        let renewed = result.unwrap();
        assert!(renewed.end_time > renewed.start_time);
    }
    
    #[test]
    fn test_lease_cleanup() {
        let mut manager = LeaseManager::new(1, 1); // 设置很短的租约时间
        
        // 创建租约
        manager.create_lease(
            "192.168.1.100",
            "00:11:22:33:44:55",
            None,
            Some(1),
        );
        
        // 等待租约过期
        std::thread::sleep(std::time::Duration::from_secs(2));
        
        // 清理过期租约
        let cleaned = manager.cleanup_expired_leases();
        assert_eq!(cleaned, 1);
    }
}
