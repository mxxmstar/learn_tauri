//! DHCP 服务器模块
//! 
//! 该模块实现了完整的 DHCP 服务器功能，包括：
//! - 配置管理（网段、地址范围等）
//! - IP 地址池管理
//! - 租约管理
//! - DHCP 协议消息处理
//! - 服务器主逻辑

pub mod config;
pub mod pool;
pub mod lease;
pub mod server;
pub mod protocol;

// 导入日志宏（由于使用了 #[macro_export]，需要在每个文件中导入）
use crate::{log_info, log_error, log_debug};

use config::{DhcpConfig, DhcpPoolRange};
use pool::IpPool;
use lease::LeaseManager;
use server::DhcpServer;
use std::sync::Arc;
use tokio::sync::Mutex;

/// DHCP 服务管理器
/// 
/// 封装了 DHCP 服务器的所有功能，提供统一的接口
pub struct DhcpService {
    /// 服务器配置
    config: DhcpConfig,
    /// IP 地址池
    pool: Arc<Mutex<IpPool>>,
    /// 租约管理器
    lease_manager: Arc<Mutex<LeaseManager>>,
    /// DHCP 服务器实例
    server: Option<DhcpServer>,
}

impl DhcpService {
    /// 创建新的 DHCP 服务实例
    /// 
    /// # 参数
    /// * `config` - DHCP 服务器配置
    /// 
    /// # 返回值
    /// 返回 Result，成功时包含 DhcpService 实例
    pub fn new(config: DhcpConfig) -> Result<Self, String> {
        log_info!("正在初始化 DHCP 服务...");
        
        // 验证配置
        config.validate()?;
        
        // 创建 IP 地址池
        let pool = IpPool::new(&config.pools)?;
        
        // 创建租约管理器
        let lease_manager = LeaseManager::new(config.lease_time_default, config.lease_time_max);
        
        log_info!("DHCP 服务初始化完成");
        log_info!("网段: {}", config.subnet);
        log_info!("网关: {}", config.gateway);
        log_info!("DNS 服务器: {:?}", config.dns_servers);
        
        Ok(Self {
            config,
            pool: Arc::new(Mutex::new(pool)),
            lease_manager: Arc::new(Mutex::new(lease_manager)),
            server: None,
        })
    }
    
    /// 启动 DHCP 服务器
    /// 
    /// # 返回值
    /// 返回 Result，成功时返回 ()
    pub async fn start(&mut self) -> Result<(), String> {
        log_info!("正在启动 DHCP 服务器...");
        
        let server = DhcpServer::new(
            self.config.clone(),
            self.pool.clone(),
            self.lease_manager.clone(),
        )?;
        
        server.start().await?;
        self.server = Some(server);
        
        log_info!("DHCP 服务器启动成功，监听端口: {}", self.config.listen_port);
        Ok(())
    }
    
    /// 停止 DHCP 服务器
    pub async fn stop(&mut self) -> Result<(), String> {
        log_info!("正在停止 DHCP 服务器...");
        
        if let Some(server) = &self.server {
            server.stop().await?;
        }
        
        self.server = None;
        log_info!("DHCP 服务器已停止");
        Ok(())
    }
    
    /// 获取当前配置
    pub fn get_config(&self) -> &DhcpConfig {
        &self.config
    }
    
    /// 获取地址池统计信息
    pub async fn get_pool_stats(&self) -> (u32, u32) {
        let pool = self.pool.lock().await;
        (pool.allocated_count(), pool.available_count())
    }
    
    /// 获取租约列表
    pub async fn get_leases(&self) -> Vec<lease::LeaseInfo> {
        let manager = self.lease_manager.lock().await;
        manager.get_all_leases()
    }
}

/// 初始化 DHCP 服务的便捷函数
/// 
/// # 参数
/// * `subnet` - 网段（如 "192.168.1.0/24"）
/// * `gateway` - 网关地址
/// * `dns_servers` - DNS 服务器列表
/// * `pool_ranges` - 地址池范围列表
/// 
/// # 返回值
/// 返回 Result，成功时包含 DhcpService 实例
pub fn init_dhcp_service(
    subnet: &str,
    gateway: &str,
    dns_servers: Vec<String>,
    pool_ranges: Vec<(String, String)>,
) -> Result<DhcpService, String> {
    let pools = pool_ranges
        .into_iter()
        .map(|(start, end)| DhcpPoolRange {
            start_ip: start,
            end_ip: end,
        })
        .collect();
    
    let config = DhcpConfig {
        subnet: subnet.to_string(),
        gateway: gateway.to_string(),
        dns_servers,
        pools,
        lease_time_default: 86400,  // 默认 24 小时
        lease_time_max: 604800,     // 最大 7 天
        listen_port: 67,
        interface: None,
    };
    
    DhcpService::new(config)
}
