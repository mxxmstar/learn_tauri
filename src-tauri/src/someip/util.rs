//! SomeIP 工具函数
//!
//! 对应 C++ `someip_client.h:9-120` 中的内联函数。
//!
//! 提供 IP 地址和 MAC 地址的字符串与二进制表示之间的转换。
//! 使用 `std::net::Ipv4Addr` 替代 C++ 的 `QHostAddress`。

use std::net::Ipv4Addr;
use crate::someip::error::{SomeIPError, SomeIPResult};

/// 将 IPv4 地址字符串转换为大端序 `u32`。
///
/// 对应 C++ `IpStringToUint32`（someip_client.h:12-20）。
///
/// C++ 中 `QHostAddress::toIPv4Address()` 返回的是大端序（网络序）的 `quint32`，
/// 即 `0xC0A842A6` 对应 `"192.168.66.166"`。
///
/// Rust 的 `Ipv4Addr::from_str` 返回本机序，需手动转为大端序。
///
/// # 参数
///
/// * `ip_string` - IPv4 地址字符串，例如 `"192.168.66.166"`
///
/// # 返回
///
/// 大端序的 `u32`，例如 `0xC0A842A6`。
///
/// # 错误
///
/// 当字符串不是有效的 IPv4 地址时返回 `InvalidIpAddress`。
///
/// # 示例
///
/// ```
/// # use crate::someip::util::ip_string_to_u32_be;
/// let value = ip_string_to_u32_be("192.168.66.166").unwrap();
/// assert_eq!(value, 0xC0A842A6);
/// ```
pub fn ip_string_to_u32_be(ip_string: &str) -> SomeIPResult<u32> {
    let addr: Ipv4Addr = ip_string.parse()
        .map_err(|_| SomeIPError::InvalidIpAddress(ip_string.to_string()))?;
    // Ipv4Addr::octets() 返回本机序字节，需转为大端序 u32
    let octets = addr.octets();
    Ok(u32::from_be_bytes(octets))
}

/// 将大端序 `u32` 转换为 IPv4 地址字符串。
///
/// 对应 C++ `IpUint32ToString`（someip_client.h:25-29）。
///
/// # 参数
///
/// * `ip_value` - 大端序的 `u32`，例如 `0xC0A842A6`
///
/// # 返回
///
/// IPv4 地址字符串，例如 `"192.168.66.166"`。
///
/// # 示例
///
/// ```
/// # use crate::someip::util::u32_be_to_ip_string;
/// let ip = u32_be_to_ip_string(0xC0A842A6);
/// assert_eq!(ip, "192.168.66.166");
/// ```
pub fn u32_be_to_ip_string(ip_value: u32) -> String {
    let octets = ip_value.to_be_bytes();
    Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]).to_string()
}

/// 将 IPv4 地址字符串转换为本机序 `u32`。
///
/// 与 `ip_string_to_u32_be` 不同，此函数返回本机序。
/// 用于需要本机序的场景（如与本机序设备通信）。
pub fn ip_string_to_u32_ne(ip_string: &str) -> SomeIPResult<u32> {
    let addr: Ipv4Addr = ip_string.parse()
        .map_err(|_| SomeIPError::InvalidIpAddress(ip_string.to_string()))?;
    let octets = addr.octets();
    Ok(u32::from_ne_bytes(octets))
}

/// 将 MAC 地址字符串转换为 6 字节数组。
///
/// 对应 C++ `MacStringToByteArray`（someip_client.h:38-66）。
///
/// 支持多种格式：
/// - `"aa:27:7a:9c:bd:d2"`（冒号分隔）
/// - `"aa-27-7a-9c-bd-d2"`（短横线分隔）
/// - `"aa27.7a9c.bdd2"`（点分隔，Cisco 格式）
/// - `"aa277a9cbdd2"`（无分隔符）
///
/// # 参数
///
/// * `mac_string` - MAC 地址字符串
///
/// # 返回
///
/// 6 字节数组（本机序，无需转换）。
///
/// # 错误
///
/// 当字符串格式无效时返回 `InvalidMacAddress`。
pub fn mac_string_to_bytes(mac_string: &str) -> SomeIPResult<[u8; 6]> {
    let clean_mac = mac_string.trim().to_lowercase()
        .replace(':', "")
        .replace('-', "")
        .replace('.', "");

    if clean_mac.len() != 12 {
        return Err(SomeIPError::InvalidMacAddress(mac_string.to_string()));
    }

    let mut mac = [0u8; 6];
    for i in 0..6 {
        let byte_str = &clean_mac[i * 2..i * 2 + 2];
        mac[i] = u8::from_str_radix(byte_str, 16)
            .map_err(|_| SomeIPError::InvalidMacAddress(mac_string.to_string()))?;
    }

    Ok(mac)
}

/// 将 6 字节数组转换为 MAC 地址字符串。
///
/// 对应 C++ `MacByteArrayToString`（someip_client.h:73-90）。
///
/// # 参数
///
/// * `mac_bytes` - 6 字节数组
/// * `separator` - 分隔符，默认为 `:`
/// * `uppercase` - 是否使用大写，默认为 `false`
///
/// # 返回
///
/// MAC 地址字符串，例如 `"aa:27:7a:9c:bd:d2"`。
pub fn mac_bytes_to_string(mac_bytes: &[u8; 6], separator: Option<char>, uppercase: bool) -> String {
    let sep = separator.unwrap_or(':');
	let octets: Vec<String> = mac_bytes.iter()
        .map(|b| {
            let hex = format!("{:02x}", b);
            if uppercase { hex.to_uppercase() } else { hex }
        })
        .collect();
    octets.join(&sep.to_string())
}

/// 将 MAC 地址字符串转换为 `u64`（高 16 位补 0）。
///
/// 对应 C++ `MacStringToUint64`（someip_client.h:95-105）。
///
/// # 参数
///
/// * `mac_string` - MAC 地址字符串
///
/// # 返回
///
/// `u64` 值，低 48 位为 MAC 地址。
pub fn mac_string_to_u64(mac_string: &str) -> SomeIPResult<u64> {
    let mac_bytes = mac_string_to_bytes(mac_string)?;
    let mut result: u64 = 0;
    for i in 0..6 {
        result = (result << 8) | mac_bytes[i] as u64;
    }
    Ok(result)
}

/// 将 `u64` 转换为 MAC 地址字符串（仅使用低 48 位）。
///
/// 对应 C++ `MacUint64ToString`（someip_client.h:110-120）。
///
/// # 参数
///
/// * `value` - `u64` 值
///
/// # 返回
///
/// MAC 地址字符串。
pub fn u64_to_mac_string(value: u64) -> String {
    let mut mac_bytes = [0u8; 6];
    let mut v = value;
    for i in (0..6).rev() {
        mac_bytes[i] = (v & 0xFF) as u8;
        v >>= 8;
    }
    mac_bytes_to_string(&mac_bytes, Some(':'), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_string_to_u32_be() {
        let value = ip_string_to_u32_be("192.168.66.166").unwrap();
        assert_eq!(value, 0xC0A842A6);
    }

    #[test]
    fn test_u32_be_to_ip_string() {
        let ip = u32_be_to_ip_string(0xC0A842A6);
        assert_eq!(ip, "192.168.66.166");
    }

    #[test]
    fn test_mac_string_to_bytes() {
        let mac = mac_string_to_bytes("aa:27:7a:9c:bd:d2").unwrap();
        assert_eq!(mac, [0xaa, 0x27, 0x7a, 0x9c, 0xbd, 0xd2]);
    }

    #[test]
    fn test_mac_bytes_to_string() {
        let mac = [0xaa, 0x27, 0x7a, 0x9c, 0xbd, 0xd2];
        let s = mac_bytes_to_string(&mac, Some(':'), false);
        assert_eq!(s, "aa:27:7a:9c:bd:d2");
    }
}
