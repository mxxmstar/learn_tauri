//! SetNetworkPayload 实现
//!
//! 对应 C++ `SetNetworkPayload`（someip_client.h:270-323）。
//!
//! # 字节布局（大端序）
//!
//! ```text
//! Offset  Size  Field
//! 0       4     dhcpEnable        DHCP 使能（bool → u32）
//! 4       4     dhcpTimeout       DHCP 超时（u32）
//! 8       4     ipAddress         IP 地址（u32 大端序）
//! 12      4     subnetMask        子网掩码（u32 大端序）
//! 16      4     gateway           网关（u32 大端序）
//! 20      6     macAddress        MAC 地址（6 字节，本机序）
//! 26      4     rtspPort          RTSP 端口（u16 → u32）
//! 30      4     onvifPort         ONVIF 端口（u16 → u32）
//! 34      4     onvifDiscoverEnable ONVIF 发现使能（bool → u32）
//! 38      8     avtpStreamId      AVTP 流 ID（u64 大端序）
//! 46      6     avtpMacAddress    AVTP MAC 地址（6 字节）
//! 52      4     someipPort        SomeIP 端口（u16 → u32）
//! ```
//!
//! 共 56 字节（4+4+4+4+4+6+4+4+4+8+6+4 = 56）。

use crate::someip::error::{SomeIPError, SomeIPResult};
use crate::someip::method::SomeIPMethod;
use crate::someip::payload::{Payload, PayloadCodec};
use crate::someip::util::{ip_string_to_u32_be, u32_be_to_ip_string, mac_string_to_bytes, mac_bytes_to_string};

/// SetNetworkPayload（网络配置）。
///
/// 对应 C++ `SetNetworkPayload`（someip_client.h:270-323）。
///
/// # C++ 兼容性说明
///
/// C++ 中 IP 地址通过 `IpStringToUint32()` 转为大端序 `quint32`，
/// 然后再次 `qToBigEndian` 写入。这意味着：
/// 1. `IpStringToUint32()` 返回大端序（QHostAddress 行为）
/// 2. `qToBigEndian` 在大端序系统上无操作，在小端序系统上会反转
///
/// 为保持兼容，Rust 实现中：
/// - `ip_address` 存储为大端序 `u32`（与 C++ 内存布局一致）
/// - `to_bytes()` 直接写入大端序字节
/// - `from_bytes()` 直接读取大端序字节
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetNetworkPayload {
    /// DHCP 使能（默认 `false`）
    pub dhcp_enable: bool,
    /// DHCP 超时（默认 `16`，单位：秒）
    pub dhcp_timeout: u32,
    /// IP 地址（大端序 `u32`，默认 `192.168.66.166` = `0xC0A842A6`）
    pub ip_address: u32,
    /// 子网掩码（大端序 `u32`，默认 `255.255.255.0` = `0xFFFFFF00`）
    pub subnet_mask: u32,
    /// 网关（大端序 `u32`，默认 `192.168.66.1` = `0xC0A84201`）
    pub gateway: u32,
    /// MAC 地址（6 字节，本机序）
    pub mac_address: [u8; 6],
    /// RTSP 端口（默认 `554`）
    pub rtsp_port: u16,
    /// ONVIF 端口（默认 `80`）
    pub onvif_port: u16,
    /// ONVIF 发现使能（默认 `true`）
    pub onvif_discover_enable: bool,
    /// AVTP 流 ID（默认 `0x102`）
    pub avtp_stream_id: u64,
    /// AVTP MAC 地址（6 字节，本机序，默认 `00:11:22:33:44:55`）
    pub avtp_mac_address: [u8; 6],
    /// SomeIP 端口（默认 `17215`）
    pub someip_port: u16,
}

impl SetNetworkPayload {
    /// 创建新的 SetNetworkPayload。
    pub fn new(
        dhcp_enable: bool,
        dhcp_timeout: u32,
        ip_address: u32,
        subnet_mask: u32,
        gateway: u32,
        mac_address: [u8; 6],
        rtsp_port: u16,
        onvif_port: u16,
        onvif_discover_enable: bool,
        avtp_stream_id: u64,
        avtp_mac_address: [u8; 6],
        someip_port: u16,
    ) -> Self {
        SetNetworkPayload {
            dhcp_enable,
            dhcp_timeout,
            ip_address,
            subnet_mask,
            gateway,
            mac_address,
            rtsp_port,
            onvif_port,
            onvif_discover_enable,
            avtp_stream_id,
            avtp_mac_address,
            someip_port,
        }
    }

    /// 返回默认 SetNetworkPayload（与 C++ 默认值一致）。
    pub fn default_payload() -> Self {
        SetNetworkPayload {
            dhcp_enable: false,
            dhcp_timeout: 16,
            ip_address: 0xC0A842A6, // 192.168.66.166 大端序
            subnet_mask: 0xFFFFFF00, // 255.255.255.0 大端序
            gateway: 0xC0A84201,     // 192.168.66.1 大端序
            mac_address: [0xaa, 0x27, 0x7a, 0x9c, 0xbd, 0xd2],
            rtsp_port: 554,
            onvif_port: 80,
            onvif_discover_enable: true,
            avtp_stream_id: 0x102,
            avtp_mac_address: [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
            someip_port: 17215,
        }
    }

    /// 从字符串创建 SetNetworkPayload。
    pub fn from_strings(
        dhcp_enable: bool,
        dhcp_timeout: u32,
        ip_address: &str,
        subnet_mask: &str,
        gateway: &str,
        mac_address: &str,
        rtsp_port: u16,
        onvif_port: u16,
        onvif_discover_enable: bool,
        avtp_stream_id: u64,
        avtp_mac_address: &str,
        someip_port: u16,
    ) -> SomeIPResult<Self> {
        Ok(SetNetworkPayload {
            dhcp_enable,
            dhcp_timeout,
            ip_address: ip_string_to_u32_be(ip_address)?,
            subnet_mask: ip_string_to_u32_be(subnet_mask)?,
            gateway: ip_string_to_u32_be(gateway)?,
            mac_address: mac_string_to_bytes(mac_address)?,
            rtsp_port,
            onvif_port,
            onvif_discover_enable,
            avtp_stream_id,
            avtp_mac_address: mac_string_to_bytes(avtp_mac_address)?,
            someip_port,
        })
    }

    /// 序列化为字节数组（大端序）。
    ///
    /// 对应 C++ `SetNetworkPayload::ToByteArray()`（someip_client.h:285-322）。
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(56);

        // dhcpEnable: bool → u32 → 大端序
        let val: u32 = if self.dhcp_enable { 1 } else { 0 };
        bytes.extend_from_slice(&val.to_be_bytes());

        // dhcpTimeout: u32 → 大端序
        bytes.extend_from_slice(&self.dhcp_timeout.to_be_bytes());

        // ipAddress: u32 大端序（直接写入）
        bytes.extend_from_slice(&self.ip_address.to_be_bytes());

        // subnetMask: u32 大端序（直接写入）
        bytes.extend_from_slice(&self.subnet_mask.to_be_bytes());

        // gateway: u32 大端序（直接写入）
        bytes.extend_from_slice(&self.gateway.to_be_bytes());

        // macAddress: 6 字节（本机序，直接写入）
        bytes.extend_from_slice(&self.mac_address);

        // rtspPort: u16 → u32 → 大端序
        bytes.extend_from_slice(&(self.rtsp_port as u32).to_be_bytes());

        // onvifPort: u16 → u32 → 大端序
        bytes.extend_from_slice(&(self.onvif_port as u32).to_be_bytes());

        // onvifDiscoverEnable: bool → u32 → 大端序
        let val: u32 = if self.onvif_discover_enable { 1 } else { 0 };
        bytes.extend_from_slice(&val.to_be_bytes());

        // avtpStreamId: u64 → 大端序
        bytes.extend_from_slice(&self.avtp_stream_id.to_be_bytes());

        // avtpMacAddress: 6 字节（直接写入）
        bytes.extend_from_slice(&self.avtp_mac_address);

        // someipPort: u16 → u32 → 大端序
        bytes.extend_from_slice(&(self.someip_port as u32).to_be_bytes());

        bytes
    }

    /// 从字节数组反序列化。
    ///
    /// # 参数
    ///
    /// * `bytes` - 至少 56 字节的字节数组（大端序）
    ///
    /// # 错误
    ///
    /// 当 `bytes.len() < 56` 时返回 `InsufficientBuffer`。
    pub fn from_bytes(bytes: &[u8]) -> SomeIPResult<Self> {
        if bytes.len() < 56 {
            return Err(SomeIPError::insufficient_buffer(56, bytes.len()));
        }

        let dhcp_enable = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) != 0;
        let dhcp_timeout = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let ip_address = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let subnet_mask = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let gateway = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);

        let mut mac_address = [0u8; 6];
        mac_address.copy_from_slice(&bytes[20..26]);

        let rtsp_port = u32::from_be_bytes([bytes[26], bytes[27], bytes[28], bytes[29]]) as u16;
        let onvif_port = u32::from_be_bytes([bytes[30], bytes[31], bytes[32], bytes[33]]) as u16;
        let onvif_discover_enable = u32::from_be_bytes([bytes[34], bytes[35], bytes[36], bytes[37]]) != 0;

        let avtp_stream_id = u64::from_be_bytes([
            bytes[38], bytes[39], bytes[40], bytes[41],
            bytes[42], bytes[43], bytes[44], bytes[45],
        ]);

        let mut avtp_mac_address = [0u8; 6];
        avtp_mac_address.copy_from_slice(&bytes[46..52]);

        let someip_port = u32::from_be_bytes([bytes[52], bytes[53], bytes[54], bytes[55]]) as u16;

        Ok(SetNetworkPayload {
            dhcp_enable,
            dhcp_timeout,
            ip_address,
            subnet_mask,
            gateway,
            mac_address,
            rtsp_port,
            onvif_port,
            onvif_discover_enable,
            avtp_stream_id,
            avtp_mac_address,
            someip_port,
        })
    }

    /// 获取 IP 地址字符串（大端序 → 字符串）。
    pub fn get_ip_address_string(&self) -> String {
        u32_be_to_ip_string(self.ip_address)
    }

    /// 获取 MAC 地址字符串。
    pub fn get_mac_address_string(&self) -> String {
        mac_bytes_to_string(&self.mac_address, Some(':'), false)
    }
}

impl Payload for SetNetworkPayload {
    fn method_id(&self) -> SomeIPMethod {
        SomeIPMethod::SetNetwork
    }

    fn encode(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl PayloadCodec for SetNetworkPayload {
    fn decode(data: &[u8]) -> SomeIPResult<Self> {
        Self::from_bytes(data)
    }
}

impl Default for SetNetworkPayload {
    fn default() -> Self {
        Self::default_payload()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_payload_to_bytes_roundtrip() {
        let payload = SetNetworkPayload::default_payload();
        let bytes = payload.to_bytes();
        assert_eq!(bytes.len(), 56);

        let parsed = SetNetworkPayload::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.ip_address, payload.ip_address);
        assert_eq!(parsed.rtsp_port, payload.rtsp_port);
    }
}
