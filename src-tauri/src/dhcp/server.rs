//! DHCP 服务器主逻辑模块
//! 
//! 该模块实现了 DHCP 服务器的核心逻辑，包括：
//! - 监听 DHCP 客户端请求
//! - 处理各种 DHCP 消息
//! - 管理 IP 地址分配
//! - 发送 DHCP 响应

use crate::dhcp::config::DhcpConfig;
use crate::dhcp::pool::IpPool;
use crate::dhcp::lease::LeaseManager;
use crate::dhcp::protocol::{DhcpMessage, DhcpMessageType, DhcpOptionCode};
// 导入日志宏（由于使用了 #[macro_export]，需要在每个文件中导入）
use crate::{log_info, log_debug, log_error, log_warn};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::UdpSocket;
use std::net::SocketAddr;
use std::net::Ipv4Addr;

/// 将 IP 地址字符串转换为 4 字节向量
fn ip_to_bytes(ip: &str) -> Vec<u8> {
    if let Ok(addr) = ip.parse::<Ipv4Addr>() {
        addr.octets().to_vec()
    } else {
        vec![0, 0, 0, 0]
    }
}

/// 将多个 IP 地址转换为字节向量
fn ips_to_bytes(ips: &[String]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for ip in ips {
        bytes.extend_from_slice(&ip_to_bytes(ip));
    }
    bytes
}

/// 将 u32 租约时间转换为 4 字节向量
fn lease_time_to_bytes(seconds: u32) -> Vec<u8> {
    seconds.to_be_bytes().to_vec()
}

/// 获取服务器 IP 地址（从配置中提取）
fn get_server_ip(config: &DhcpConfig) -> Ipv4Addr {
    // 从网关地址或子网中提取服务器 IP
    if let Ok(addr) = config.gateway.parse::<Ipv4Addr>() {
        addr
    } else {
        // 默认使用 192.168.1.1
        Ipv4Addr::new(192, 168, 1, 1)
    }
}

/// DHCP 服务器
/// 
/// 封装了 DHCP 服务器的所有核心功能
#[allow(dead_code)]
pub struct DhcpServer {
    /// 服务器配置
    config: DhcpConfig,
    /// IP 地址池（共享引用）
    pool: Arc<Mutex<IpPool>>,
    /// 租约管理器（共享引用）
    lease_manager: Arc<Mutex<LeaseManager>>,
    /// UDP 套接字
    socket: Option<Arc<UdpSocket>>,
    /// 是否正在运行
    running: bool,
}

impl DhcpServer {
    /// 创建新的 DHCP 服务器实例
    /// 
    /// # 参数
    /// * `config` - DHCP 配置
    /// * `pool` - IP 地址池
    /// * `lease_manager` - 租约管理器
    /// 
    /// # 返回值
    /// 返回 Result，成功时包含 DhcpServer 实例
    pub fn new(
        config: DhcpConfig,
        pool: Arc<Mutex<IpPool>>,
        lease_manager: Arc<Mutex<LeaseManager>>,
    ) -> Result<Self, String> {
        log_info!("创建 DHCP 服务器实例");
        
        Ok(Self {
            config,
            pool,
            lease_manager,
            socket: None,
            running: false,
        })
    }
    
    /// 启动 DHCP 服务器
    /// 
    /// # 返回值
    /// 返回 Result，成功时返回 ()
    pub async fn start(&self) -> Result<(), String> {
        log_info!("正在启动 DHCP 服务器...");
        
        // 创建 UDP 套接字绑定到 0.0.0.0:67（DHCP 服务器端口）
        let addr = SocketAddr::new(
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
            self.config.listen_port,
        );
        
        let socket = UdpSocket::bind(addr)
            .await
            .map_err(|e| format!("无法绑定到地址 {}: {}", addr, e))?;
        
        log_info!("DHCP 服务器已绑定到 {}", addr);
        
        // 设置为广播模式
        socket
            .set_broadcast(true)
            .map_err(|e| format!("无法设置广播模式: {}", e))?;
        
        let socket = Arc::new(socket);
        
        // 启动消息处理循环
        let socket_clone = socket.clone();
        let pool_clone = self.pool.clone();
        let lease_manager_clone = self.lease_manager.clone();
        let config_clone = self.config.clone();
        
        tokio::spawn(async move {
            Self::message_handler(socket_clone, pool_clone, lease_manager_clone, config_clone).await;
        });
        
        log_info!("DHCP 服务器启动成功");
        Ok(())
    }
    
    /// 停止 DHCP 服务器
    pub async fn stop(&self) -> Result<(), String> {
        log_info!("正在停止 DHCP 服务器...");
        
        // 在实际实现中，需要设置一个标志来停止消息处理循环
        // 这里为了简化，只是记录日志
        
        log_info!("DHCP 服务器已停止");
        Ok(())
    }
    
    /// 消息处理循环
    /// 
    /// 持续监听 DHCP 客户端请求并处理
    async fn message_handler(
        socket: Arc<UdpSocket>,
        pool: Arc<Mutex<IpPool>>,
        lease_manager: Arc<Mutex<LeaseManager>>,
        config: DhcpConfig,
    ) {
        let mut buf = [0u8; 1500];
        
        log_info!("开始监听 DHCP 消息...");
        
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    log_debug!("收到来自 {} 的 {} 字节数据", addr, len);
                    
                    // 解析 DHCP 消息
                    if let Some(msg) = DhcpMessage::decode(&buf[..len]) {
                        // 处理 DHCP 消息
                        if let Err(e) = Self::handle_message(
                            &socket,
                            msg,
                            &pool,
                            &lease_manager,
                            &config,
                        ).await {
                            log_error!("处理 DHCP 消息失败: {}", e);
                        }
                    } else {
                        log_warn!("无法解析 DHCP 消息");
                    }
                }
                Err(e) => {
                    log_error!("接收 DHCP 消息失败: {}", e);
                }
            }
        }
    }
    
    /// 处理 DHCP 消息
    /// 
    /// 根据消息类型调用相应的处理函数
    async fn handle_message(
        socket: &Arc<UdpSocket>,
        msg: DhcpMessage,
        pool: &Arc<Mutex<IpPool>>,
        lease_manager: &Arc<Mutex<LeaseManager>>,
        config: &DhcpConfig,
    ) -> Result<(), String> {
        let msg_type = msg.get_message_type();
        
        match msg_type {
            Some(DhcpMessageType::Discover) => {
                Self::handle_discover(socket, msg, pool, lease_manager, config).await
            }
            Some(DhcpMessageType::Request) => {
                Self::handle_request(socket, msg, pool, lease_manager, config).await
            }
            Some(DhcpMessageType::Release) => {
                Self::handle_release(socket, msg, pool, lease_manager).await
            }
            Some(DhcpMessageType::Decline) => {
                Self::handle_decline(socket, msg, pool, lease_manager).await
            }
            Some(DhcpMessageType::Inform) => {
                Self::handle_inform(socket, msg, config).await
            }
            _ => {
                log_warn!("未支持的 DHCP 消息类型: {:?}", msg_type);
                Ok(())
            }
        }
    }
    
    /// 处理 DHCP DISCOVER 消息
    /// 
    /// 当客户端发送 DISCOVER 消息时，服务器应该发送 OFFER 消息
    async fn handle_discover(
        socket: &Arc<UdpSocket>,
        msg: DhcpMessage,
        pool: &Arc<Mutex<IpPool>>,
        lease_manager: &Arc<Mutex<LeaseManager>>,
        config: &DhcpConfig,
    ) -> Result<(), String> {
        let mac = msg.get_mac_address();
        log_info!("收到 DHCP DISCOVER 消息，MAC: {}", mac);
        
        // 分配 IP 地址
        let mut pool_guard = pool.lock().await;
        let ip = pool_guard.allocate_ip(&mac);
        drop(pool_guard);
        
        if let Some(ip) = ip {
            log_info!("为客户端 {} 分配 IP: {}", mac, ip);
            
            // 提取客户端主机名（如果有）
            let hostname = msg.get_option(DhcpOptionCode::HostName as u8)
                .map(|opt| String::from_utf8_lossy(&opt.data).to_string());
            
            // 提取客户端请求的租约时间（如果有）
            let requested_lease_time = msg.get_option(DhcpOptionCode::LeaseTime as u8)
                .and_then(|opt| {
                    if opt.data.len() >= 4 {
                        Some(u32::from_be_bytes([opt.data[0], opt.data[1], opt.data[2], opt.data[3]]))
                    } else {
                        None
                    }
                });
            
            // 创建租约
            let mut lease_guard = lease_manager.lock().await;
            let _lease = lease_guard.create_lease(
                &ip,
                &mac,
                hostname,
                requested_lease_time,
            );
            drop(lease_guard);
            
            // 获取服务器 IP
            let server_ip = get_server_ip(config);
            let server_ip_bytes = ip_to_bytes(&server_ip.to_string());
            
            // 构造 OFFER 消息
            let mut offer = DhcpMessage::new();
            offer.op = 2; // BOOTREPLY
            offer.htype = msg.htype; // 复制客户端的硬件类型
            offer.hlen = msg.hlen;   // 复制客户端的硬件地址长度
            offer.xid = msg.xid;     // 事务 ID 必须匹配
            offer.flags = msg.flags; // 复制广播标志
            offer.yiaddr = ip.parse().unwrap_or(Ipv4Addr::new(0, 0, 0, 0));
            offer.siaddr = server_ip; // 设置服务器 IP
            offer.set_mac_address(&mac);
            offer.set_message_type(DhcpMessageType::Offer);
            
            // 添加 DHCP 选项（使用正确的 4 字节格式）
            offer.add_option(DhcpOptionCode::SubnetMask as u8, ip_to_bytes(&config.subnet_mask())); // 子网掩码
            offer.add_option(DhcpOptionCode::Router as u8, ip_to_bytes(&config.gateway)); // 网关
            offer.add_option(DhcpOptionCode::DnsServer as u8, ips_to_bytes(&config.dns_servers)); // DNS
            offer.add_option(DhcpOptionCode::ServerIdentifier as u8, server_ip_bytes.clone()); // 服务器标识符（必需）
            offer.add_option(DhcpOptionCode::LeaseTime as u8, lease_time_to_bytes(config.lease_time_default)); // 租约时间（必需）
            
            // 添加 Renewal Time (T1) 和 Rebinding Time (T2)
            let t1 = config.lease_time_default / 2;
            let t2 = config.lease_time_default * 7 / 8;
            offer.add_option(58, lease_time_to_bytes(t1)); // Renewal Time
            offer.add_option(59, lease_time_to_bytes(t2)); // Rebinding Time
            
            // 发送 OFFER 消息
            let response = offer.encode();
            
            // 根据广播标志决定发送目标
            let target_addr = if msg.flags & 0x8000 != 0 {
                // 广播标志设置，发送到广播地址
                "255.255.255.255:68"
            } else {
                // 单播到客户端（虽然此时客户端还没有 IP，但通常还是广播）
                "255.255.255.255:68"
            };
            
            socket
                .send_to(&response, target_addr)
                .await
                .map_err(|e| format!("发送 OFFER 消息失败: {}", e))?;
            
            log_info!("发送 DHCP OFFER 消息，IP: {}, 目标: {}", ip, target_addr);
        } else {
            log_error!("无法为客户端 {} 分配 IP 地址", mac);
        }
        
        Ok(())
    }
    
    /// 处理 DHCP REQUEST 消息
    /// 
    /// 当客户端发送 REQUEST 消息时，服务器应该发送 ACK 或 NAK 消息
    /// 处理三种场景：
    /// 1. SELECTING: 客户端选择 OFFER，包含 Server ID (Option 54)
    /// 2. INIT-REBOOT: 客户端重启，验证之前的 IP
    /// 3. RENEWING/REBINDING: 租约续期，单播到服务器
    async fn handle_request(
        socket: &Arc<UdpSocket>,
        msg: DhcpMessage,
        pool: &Arc<Mutex<IpPool>>,
        lease_manager: &Arc<Mutex<LeaseManager>>,
        config: &DhcpConfig,
    ) -> Result<(), String> {
        let mac = msg.get_mac_address();
        log_info!("收到 DHCP REQUEST 消息，MAC: {}", mac);
        
        // 获取服务器 IP
        let server_ip = get_server_ip(config);
        let server_ip_bytes = ip_to_bytes(&server_ip.to_string());
        
        // 检查是否有 Server Identifier 选项
        let has_server_id = msg.get_option(DhcpOptionCode::ServerIdentifier as u8).is_some();
        let server_id_matches = if let Some(server_id_opt) = msg.get_option(DhcpOptionCode::ServerIdentifier as u8) {
            if server_id_opt.data.len() >= 4 {
                let server_id = Ipv4Addr::new(
                    server_id_opt.data[0],
                    server_id_opt.data[1],
                    server_id_opt.data[2],
                    server_id_opt.data[3],
                );
                server_id == server_ip
            } else {
                false
            }
        } else {
            false
        };
        
        // 获取请求的 IP 地址（Option 50）
        let requested_ip = if let Some(option) = msg.get_option(DhcpOptionCode::RequestedIpAddress as u8) {
            if option.data.len() >= 4 {
                Some(Ipv4Addr::new(
                    option.data[0],
                    option.data[1],
                    option.data[2],
                    option.data[3],
                ))
            } else {
                None
            }
        } else {
            // 如果是 RENEWING 状态，ciaddr 包含客户端的 IP
            if msg.ciaddr != Ipv4Addr::new(0, 0, 0, 0) {
                Some(msg.ciaddr)
            } else {
                None
            }
        };
        
        // 判断请求场景
        let scenario = if has_server_id {
            if server_id_matches {
                "SELECTING"
            } else {
                "WRONG_SERVER" // 不是给我们的请求
            }
        } else if msg.ciaddr != Ipv4Addr::new(0, 0, 0, 0) {
            "RENEWING" // 单播续期
        } else if requested_ip.is_some() {
            "INIT-REBOOT"
        } else {
            "UNKNOWN"
        };
        
        log_info!("REQUEST 场景: {}", scenario);
        
        match scenario {
            "WRONG_SERVER" => {
                log_debug!("REQUEST 不是给我们的，忽略");
                return Ok(());
            }
            "SELECTING" | "INIT-REBOOT" | "RENEWING" => {
                if let Some(ip) = requested_ip {
                    let ip_str = ip.to_string();
                    log_info!("客户端 {} 请求 IP: {}", mac, ip_str);
                    
                    // 检查 IP 是否可用或已分配给该 MAC
                    let lease_guard = lease_manager.lock().await;
                    let existing_lease = lease_guard.find_lease_by_mac(&mac);
                    
                    let pool_guard = pool.lock().await;
                    
                    // 检查是否允许分配该 IP
                    let allowed = if let Some(lease) = existing_lease {
                        // 已有租约，检查 IP 是否匹配
                        lease.ip_address == ip_str
                    } else {
                        // 没有租约，检查 IP 是否可用
                        !pool_guard.is_allocated(&ip_str)
                    };
                    
                    drop(pool_guard);
                    drop(lease_guard);
                    
                    if allowed {
                        // 提取客户端主机名（如果有）
                        let hostname = msg.get_option(DhcpOptionCode::HostName as u8)
                            .map(|opt| String::from_utf8_lossy(&opt.data).to_string());
                        
                        // 提取客户端请求的租约时间（如果有）
                        let requested_lease_time = msg.get_option(DhcpOptionCode::LeaseTime as u8)
                            .and_then(|opt| {
                                if opt.data.len() >= 4 {
                                    Some(u32::from_be_bytes([opt.data[0], opt.data[1], opt.data[2], opt.data[3]]))
                                } else {
                                    None
                                }
                            });
                        
                        // 创建或续期租约
                        let mut lease_guard = lease_manager.lock().await;
                        if lease_guard.find_lease_by_mac(&mac).is_some() {
                            // 续期租约
                            lease_guard.renew_lease(&ip_str, &mac, requested_lease_time).ok();
                        } else {
                            // 创建新租约
                            lease_guard.create_lease(&ip_str, &mac, hostname, requested_lease_time);
                        }
                        drop(lease_guard);
                        
                        // 发送 ACK 消息
                        let mut ack = DhcpMessage::new();
                        ack.op = 2; // BOOTREPLY
                        ack.htype = msg.htype;
                        ack.hlen = msg.hlen;
                        ack.xid = msg.xid;
                        ack.flags = msg.flags;
                        ack.ciaddr = if scenario == "RENEWING" { msg.ciaddr } else { Ipv4Addr::new(0, 0, 0, 0) };
                        ack.yiaddr = ip;
                        ack.siaddr = server_ip;
                        ack.set_mac_address(&mac);
                        ack.set_message_type(DhcpMessageType::Ack);
                        
                        // 添加 DHCP 选项（使用正确的 4 字节格式）
                        ack.add_option(DhcpOptionCode::SubnetMask as u8, ip_to_bytes(&config.subnet_mask()));
                        ack.add_option(DhcpOptionCode::Router as u8, ip_to_bytes(&config.gateway));
                        ack.add_option(DhcpOptionCode::DnsServer as u8, ips_to_bytes(&config.dns_servers));
                        ack.add_option(DhcpOptionCode::ServerIdentifier as u8, server_ip_bytes.clone());
                        ack.add_option(DhcpOptionCode::LeaseTime as u8, lease_time_to_bytes(config.lease_time_default));
                        
                        // 添加 Renewal Time (T1) 和 Rebinding Time (T2)
                        let t1 = config.lease_time_default / 2;
                        let t2 = config.lease_time_default * 7 / 8;
                        ack.add_option(58, lease_time_to_bytes(t1));
                        ack.add_option(59, lease_time_to_bytes(t2));
                        
                        // 发送 ACK 消息
                        let response = ack.encode();
                        
                        // 根据场景决定发送目标
                        let target_addr = if scenario == "RENEWING" {
                            // RENEWING: 单播到客户端
                            format!("{}:68", ip)
                        } else if msg.flags & 0x8000 != 0 {
                            // 广播标志设置
                            "255.255.255.255:68".to_string()
                        } else {
                            // 单播（但通常客户端还没有配置 IP，所以还是广播）
                            "255.255.255.255:68".to_string()
                        };
                        
                        socket
                            .send_to(&response, &target_addr)
                            .await
                            .map_err(|e| format!("发送 ACK 消息失败: {}", e))?;
                        
                        log_info!("发送 DHCP ACK 消息，IP: {}, 目标: {}", ip_str, target_addr);
                    } else {
                        // 发送 NAK 消息
                        let mut nak = DhcpMessage::new();
                        nak.op = 2; // BOOTREPLY
                        nak.xid = msg.xid;
                        nak.set_mac_address(&mac);
                        nak.set_message_type(DhcpMessageType::Nak);
                        
                        // NAK 需要包含 Server Identifier
                        nak.add_option(DhcpOptionCode::ServerIdentifier as u8, server_ip_bytes);
                        
                        let response = nak.encode();
                        let target_addr = if msg.flags & 0x8000 != 0 || scenario == "SELECTING" {
                            "255.255.255.255:68"
                        } else {
                            &ip_str
                        };
                        
                        socket
                            .send_to(&response, target_addr)
                            .await
                            .map_err(|e| format!("发送 NAK 消息失败: {}", e))?;
                        
                        log_warn!("发送 DHCP NAK 消息，IP: {} 不可用", ip_str);
                    }
                } else {
                    log_warn!("DHCP REQUEST 消息中没有有效的 IP 地址");
                }
            }
            _ => {
                log_warn!("未知的 DHCP REQUEST 场景");
            }
        }
        
        Ok(())
    }
    
    /// 处理 DHCP RELEASE 消息
    async fn handle_release(
        _socket: &Arc<UdpSocket>,
        msg: DhcpMessage,
        pool: &Arc<Mutex<IpPool>>,
        lease_manager: &Arc<Mutex<LeaseManager>>,
    ) -> Result<(), String> {
        let mac = msg.get_mac_address();
        log_info!("收到 DHCP RELEASE 消息，MAC: {}", mac);
        
        // 查找该 MAC 的租约
        let lease_guard = lease_manager.lock().await;
        if let Some(lease) = lease_guard.find_lease_by_mac(&mac) {
            let ip = lease.ip_address.clone();
            drop(lease_guard);
            
            // 释放 IP 地址
            let mut pool_guard = pool.lock().await;
            pool_guard.release_ip_by_mac(&mac);
            drop(pool_guard);
            
            // 释放租约
            let mut lease_guard = lease_manager.lock().await;
            lease_guard.release_lease(&ip);
            
            log_info!("释放 IP: {}, MAC: {}", ip, mac);
        } else {
            log_warn!("未找到 MAC {} 的租约", mac);
        }
        
        Ok(())
    }
    
    /// 处理 DHCP DECLINE 消息
    async fn handle_decline(
        _socket: &Arc<UdpSocket>,
        msg: DhcpMessage,
        _pool: &Arc<Mutex<IpPool>>,
        _lease_manager: &Arc<Mutex<LeaseManager>>,
    ) -> Result<(), String> {
        let mac = msg.get_mac_address();
        log_info!("收到 DHCP DECLINE 消息，MAC: {}", mac);
        
        // 处理客户端拒绝 IP 的情况
        // 通常需要将 IP 地址标记为冲突，暂时不可用
        // TODO: 实现 IP 冲突检测和处理逻辑
        
        Ok(())
    }
    
    /// 处理 DHCP INFORM 消息
    async fn handle_inform(
        socket: &Arc<UdpSocket>,
        msg: DhcpMessage,
        config: &DhcpConfig,
    ) -> Result<(), String> {
        let mac = msg.get_mac_address();
        log_info!("收到 DHCP INFORM 消息，MAC: {}", mac);
        
        // 获取服务器 IP
        let server_ip = get_server_ip(config);
        let server_ip_bytes = ip_to_bytes(&server_ip.to_string());
        
        // INFORM 消息用于客户端已经有 IP 地址，只需要获取其他配置信息
        // 发送 ACK 消息，但不分配 IP
        let mut ack = DhcpMessage::new();
        ack.op = 2; // BOOTREPLY
        ack.htype = msg.htype;
        ack.hlen = msg.hlen;
        ack.xid = msg.xid;
        ack.flags = msg.flags;
        ack.ciaddr = msg.ciaddr; // 客户端的 IP 地址
        ack.siaddr = server_ip;
        ack.set_mac_address(&mac);
        ack.set_message_type(DhcpMessageType::Ack);
        
        // 添加 DHCP 选项（使用正确的 4 字节格式）
        ack.add_option(DhcpOptionCode::SubnetMask as u8, ip_to_bytes(&config.subnet_mask()));
        ack.add_option(DhcpOptionCode::Router as u8, ip_to_bytes(&config.gateway));
        ack.add_option(DhcpOptionCode::DnsServer as u8, ips_to_bytes(&config.dns_servers));
        ack.add_option(DhcpOptionCode::ServerIdentifier as u8, server_ip_bytes);
        
        let response = ack.encode();
        
        // 根据广播标志决定发送目标
        let target_addr = if msg.flags & 0x8000 != 0 {
            "255.255.255.255:68".to_string()
        } else {
            // 单播到客户端的 ciaddr
            format!("{}:68", msg.ciaddr)
        };
        
        socket
            .send_to(&response, &target_addr)
            .await
            .map_err(|e| format!("发送 ACK 消息失败: {}", e))?;
        
        log_info!("发送 DHCP ACK (INFORM)，目标: {}", target_addr);
        
        Ok(())
    }
}

/// 为 DhcpConfig 添加辅助方法
impl DhcpConfig {
    /// 获取子网掩码
    pub fn subnet_mask(&self) -> String {
        // 从子网掩码格式 "192.168.1.0/24" 中提取前缀长度
        let parts: Vec<&str> = self.subnet.split('/').collect();
        if parts.len() == 2 {
            if let Ok(prefix_len) = parts[1].parse::<u8>() {
                let mask = match prefix_len {
                    24 => "255.255.255.0",
                    16 => "255.255.0.0",
                    8 => "255.0.0.0",
                    _ => {
                        // 计算子网掩码
                        let mut mask: u32 = 0;
                        for i in 0..prefix_len {
                            mask |= 1 << (31 - i);
                        }
                        return format!(
                            "{}.{}.{}.{}",
                            (mask >> 24) & 0xFF,
                            (mask >> 16) & 0xFF,
                            (mask >> 8) & 0xFF,
                            mask & 0xFF
                        );
                    }
                };
                return mask.to_string();
            }
        }
        "255.255.255.0".to_string() // 默认子网掩码
    }
}
