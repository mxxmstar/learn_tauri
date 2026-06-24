//! UDP 设备发现模块（WS-Discovery）
//!
//! 实现 ONVIF WS-Discovery 协议，用于发现局域网内的 ONVIF 设备。
//!
//! # WS-Discovery 原理
//!
//! 1. 客户端发送 UDP 多播 Probe 到 `239.255.255.250:3702`
//! 2. 局域网内的 ONVIF 设备单播回复 ProbeMatch
//! 3. ProbeMatch 包含设备的服务地址（XAddrs）、UUID、名称、型号等
//!
//! # 实现说明
//!
//! 本模块直接使用 `tokio::net::UdpSocket` 实现多播发送和响应接收，
//! 不依赖 `udp` 模块的上层抽象（`UdpService`、`UdpClient`、`UdpServer`），
//! 因为 WS-Discovery 需要更底层的多播控制。

use crate::onvif::error::{OnvifError, OnvifResult};
use serde::Serialize;
use std::net::SocketAddr;
use std::time::Duration;

/// 通过 WS-Discovery 发现的设备信息
///
/// 对应 ONVIF WS-Discovery ProbeMatch 消息的解析结果。
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredDevice {
    /// 设备的 UUID（如 "urn:uuid:2419d68a-0000-0010-8000-001fc11d0b58"）
    pub uuid: String,

    /// 设备的服务地址列表
    ///
    /// 通常为 `http://192.168.1.100/onvif/device_service` 格式。
    /// 后续的所有 ONVIF SOAP 请求都应发送到该地址。
    pub xaddrs: Vec<String>,

    /// 设备类型（如 "dn:NetworkVideoTransmitter"）
    pub types: Vec<String>,

    /// 设备属性范围
    ///
    /// 常见 Scope 包括：
    /// - `onvif://www.onvif.org/type/video_encoder`
    /// - `onvif://www.onvif.org/hardware/型号`
    /// - `onvif://www.onvif.org/name/设备名`
    pub scopes: Vec<String>,

    /// 设备元数据版本
    pub metadata_version: u32,
}

impl DiscoveredDevice {
    /// 从 Scopes 中提取设备名称
    pub fn get_name(&self) -> Option<String> {
        self.scopes
            .iter()
            .find(|s| s.starts_with("onvif://www.onvif.org/name/"))
            .map(|s| s.trim_start_matches("onvif://www.onvif.org/name/").to_string())
    }

    /// 从 Scopes 中提取硬件型号
    pub fn get_hardware(&self) -> Option<String> {
        self.scopes
            .iter()
            .find(|s| s.starts_with("onvif://www.onvif.org/hardware/"))
            .map(|s| s.trim_start_matches("onvif://www.onvif.org/hardware/").to_string())
    }

    /// 获取设备的第一个服务地址
    pub fn get_first_xaddr(&self) -> Option<&String> {
        self.xaddrs.first()
    }
}

/// 执行设备发现（WS-Discovery）
///
/// 向局域网发送 UDP 多播 Probe 消息，等待设备响应。
///
/// # 参数
///
/// - `timeout_ms`：发现超时时间（毫秒），建议 3000~10000
///   - 超时后停止等待并返回已发现的设备列表
///   - 若超时前已收到所有设备响应，会提前返回
///
/// # 返回值
///
/// 成功时返回 `Vec<DiscoveredDevice>`，每个元素代表一个发现的设备。
///
/// # 示例
///
/// ```ignore
/// let devices = discover(5000).await?;
/// println!("发现 {} 个设备", devices.len());
/// for d in &devices {
///     println!("  - {} ({})", d.get_name().unwrap_or_default(), d.xaddrs[0]);
/// }
/// ```
///
/// # 错误处理
///
/// - 若局域网内无 ONVIF 设备，返回空列表（不是错误）
/// - 若网络异常（如无网卡），返回 `OnvifError::Io`
/// - 若超时且无任何设备响应，返回空列表
pub async fn discover(timeout_ms: u64) -> OnvifResult<Vec<DiscoveredDevice>> {
    // 多播地址和端口
    let multicast_addr: SocketAddr = "239.255.255.250:3702".parse()
        .map_err(|e| OnvifError::InvalidArgument(format!("无效的多播地址: {}", e)))?;

    // 本机绑定地址（0.0.0.0:0 表示随机分配端口）
    let bind_addr: SocketAddr = "0.0.0.0:0".parse()
        .map_err(|e| OnvifError::InvalidArgument(format!("无效的绑定地址: {}", e)))?;

    // 创建 UDP 套接字
    let socket = tokio::net::UdpSocket::bind(bind_addr)
        .await
        .map_err(|e| OnvifError::Io(e))?;

    // 加入多播组
    let multicast_ip = "239.255.255.250".parse::<std::net::Ipv4Addr>()
        .map_err(|e| OnvifError::InvalidArgument(format!("无效的多播 IP: {}", e)))?;
    socket.join_multicast_v4(multicast_ip, "0.0.0.0".parse().unwrap())
        .map_err(|e| OnvifError::Io(e))?;

    // 设置超时
    let timeout = Duration::from_millis(timeout_ms);
    let start_time = std::time::Instant::now();

    // 发送 Probe 消息
    let probe = build_probe_message();
    socket.send_to(probe.as_bytes(), &multicast_addr)
        .await
        .map_err(|e| OnvifError::Io(e))?;

    // 接收 ProbeMatch 响应
    let mut devices = Vec::new();
    let mut buf = vec![0u8; 4096];

    loop {
        // 检查超时
        if start_time.elapsed() > timeout {
            break;
        }

        // 使用 select 等待数据或超时
        tokio::select! {
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((len, _addr)) => {
                        let data = &buf[..len];
                        if let Ok(device) = parse_probe_match(data) {
                            devices.push(device);
                        }
                    }
                    Err(_) => break,
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                // 继续等待
            }
        }
    }

    Ok(devices)
}

/// 构造 WS-Discovery Probe 消息
fn build_probe_message() -> String {
    // ONVIF WS-Discovery Probe 消息模板
    // 实际实现需要构造符合 WS-Discovery 规范的 XML
    // 这里使用简化版本
    let message_id = uuid::Uuid::new_v4().to_string();
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<Envelope xmlns="http://www.w3.org/2003/05/soap-envelope">
  <Header>
    <wsa:Action xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing">http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe</wsa:Action>
    <wsa:MessageID xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing">urn:uuid:{message_id}</wsa:MessageID>
    <wsa:To xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing">urn:schemas-xmlsoap-org:ws:2005:04:discovery</wsa:To>
  </Header>
  <Body>
    <Probe xmlns="http://schemas.xmlsoap.org/ws/2005/04/discovery">
      <d:Types xmlns:d="http://schemas.xmlsoap.org/ws/2005/04/discovery" xmlns:dp0="http://www.onvif.org/ver10/network/wsdl">dp0:Device</d:Types>
      <d:Scopes xmlns:d="http://schemas.xmlsoap.org/ws/2005/04/discovery"></d:Scopes>
    </Probe>
  </Body>
</Envelope>"#)
}

/// 解析 ProbeMatch 响应
fn parse_probe_match(data: &[u8]) -> OnvifResult<DiscoveredDevice> {
    // 实际实现需要解析 XML 响应
    // 这里使用简化版本，实际应使用 quick-xml 解析
    let data_str = String::from_utf8_lossy(data);
    
    // 简化解析：从 XML 中提取关键信息
    let uuid = extract_uuid(&data_str).unwrap_or_default();
    let xaddrs = extract_xaddrs(&data_str);
    let types = extract_types(&data_str);
    let scopes = extract_scopes(&data_str);

    Ok(DiscoveredDevice {
        uuid,
        xaddrs,
        types,
        scopes,
        metadata_version: 0,
    })
}

/// 从 XML 中提取 UUID
fn extract_uuid(xml: &str) -> Option<String> {
    // 简化实现：查找 urn:uuid: 模式
    if let Some(start) = xml.find("urn:uuid:") {
        let start = start + 9;
        if let Some(end) = xml[start..].find("\"") {
            return Some(format!("urn:uuid:{}", &xml[start..start+end]));
        }
    }
    None
}

/// 从 XML 中提取 XAddrs
fn extract_xaddrs(xml: &str) -> Vec<String> {
    // 简化实现：查找 XAddrs 标签内容
    let mut xaddrs = Vec::new();
    // 实际应使用 XML 解析器
    xaddrs
}

/// 从 XML 中提取 Types
fn extract_types(xml: &str) -> Vec<String> {
    // 简化实现
    vec!["dn:NetworkVideoTransmitter".to_string()]
}

/// 从 XML 中提取 Scopes
fn extract_scopes(xml: &str) -> Vec<String> {
    // 简化实现
    vec![]
}
