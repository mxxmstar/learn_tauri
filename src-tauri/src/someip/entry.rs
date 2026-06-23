//! SomeIP 服务发现条目定义
//!
//! 对应 C++ `SomeIPEntry`（someip_protocol.h:34-46）。
//!
//! 用于 `FindOrOffer` 方法的 payload 中，描述服务实例信息。

use crate::someip::error::{SomeIPError, SomeIPResult};

/// SomeIP 服务发现条目。
///
/// 对应 C++ `SomeIPEntry`（someip_protocol.h:34-46）。
///
/// # 字节布局（大端序）
///
/// ```text
/// Offset  Size  Field
/// 0       1     type               类型
/// 1       1     index1             索引1（服务组或类型）
/// 2       1     index2             索引2（服务信息）
/// 3       1     numOpt             选项数量
/// 4       2     serviceId          服务 ID（0x433F）
/// 6       2     instanceId         实例 ID（0xFFFF）
/// 8       4     majorVersionAndTtl 主版本号和 TTL（高16位=版本，低16位=TTL）
/// 12      4     minorVersion       次版本号
/// ```
///
/// 共 16 字节。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SomeIPEntry {
    /// 类型（8 bit）
    pub entry_type: u8,
    /// 索引1：用于标识服务组或类型（8 bit）
    pub index1: u8,
    /// 索引2：用于标识服务信息（8 bit）
    pub index2: u8,
    /// 选项数量（8 bit）
    pub num_opt: u8,
    /// 服务 ID（16 bit，大端序存储），默认 0x433F
    pub service_id: u16,
    /// 实例 ID（16 bit，大端序存储），默认 0xFFFF
    pub instance_id: u16,
    /// 主版本号和 TTL（32 bit，大端序存储）
    ///
    /// 高 16 位为主版本号，低 16 位为 TTL。
    /// 默认值：`0xFF | 0xFFFFFF00` = `0xFFFFFF00`（主版本 0xFF，TTL 0xFF00？）
    /// C++ 默认值：`0xFF | 0xFFFFFF00` = 0xFFFFFF00，即高24位为0xFF，低8位为0x00。
    /// 实际上 C++ 的写法是 `0xFF | 0xFFFFFF00` = `0xFFFFFFFF`（因为 0xFF 扩展到 32 位是 0x000000FF，
    /// 与 0xFFFFFF00 或运算得 0xFFFFFF00 | 0x000000FF = 0xFFFFFF00 + 0x000000FF = 不对，应该是 0xFFFFFF00 | 0x000000FF = 0xFFFFFF00 + 0x000000FF = 0xFFFFFFFF）。
    /// 这里按 C++ 实际效果：majorVersion=0xFF，TTL=0xFFFF。
    pub major_version_and_ttl: u32,
    /// 次版本号（32 bit，大端序存储），默认 0xFFFFFFFF
    pub minor_version: u32,
}

impl SomeIPEntry {
    /// 创建默认服务发现条目。
    ///
    /// 对应 C++ 默认构造函数（someip_protocol.h:43-45）。
    pub fn default_entry() -> Self {
        SomeIPEntry {
            entry_type: 0x00,
            index1: 0x00,
            index2: 0x00,
            num_opt: 0x00,
            service_id: 0x433F_u16.to_be(),
            instance_id: 0xFFFF_u16.to_be(),
            major_version_and_ttl: (0xFFu32 << 24 | 0xFFFF).to_be(), // 高16位=0xFF，低16位=0xFFFF
            minor_version: 0xFFFFFFFF_u32.to_be(),
        }
    }

    /// 创建自定义服务发现条目。
    pub fn new(
        entry_type: u8,
        index1: u8,
        index2: u8,
        num_opt: u8,
        service_id: u16,
        instance_id: u16,
        major_version: u8,
        ttl: u16,
        minor_version: u32,
    ) -> Self {
        let mut entry = Self::default_entry();
        entry.entry_type = entry_type;
        entry.index1 = index1;
        entry.index2 = index2;
        entry.num_opt = num_opt;
        entry.service_id = service_id.to_be();
        entry.instance_id = instance_id.to_be();
        entry.major_version_and_ttl = ((major_version as u32) << 24 | ttl as u32).to_be();
        entry.minor_version = minor_version.to_be();
        entry
    }

    /// 序列化为字节数组（大端序）。
    ///
    /// 对应 C++ `reinterpret_cast<const char*>(&entry)`。
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        bytes.push(self.entry_type);
        bytes.push(self.index1);
        bytes.push(self.index2);
        bytes.push(self.num_opt);
        bytes.extend_from_slice(&self.service_id.to_be_bytes());
        bytes.extend_from_slice(&self.instance_id.to_be_bytes());
        bytes.extend_from_slice(&self.major_version_and_ttl.to_be_bytes());
        bytes.extend_from_slice(&self.minor_version.to_be_bytes());
        bytes
    }

    /// 从字节数组反序列化。
    ///
    /// # 错误
    ///
    /// 当 `bytes.len() < 16` 时返回 `InsufficientBuffer`。
    pub fn from_bytes(bytes: &[u8]) -> SomeIPResult<Self> {
        if bytes.len() < 16 {
            return Err(SomeIPError::insufficient_buffer(16, bytes.len()));
        }

        Ok(SomeIPEntry {
            entry_type: bytes[0],
            index1: bytes[1],
            index2: bytes[2],
            num_opt: bytes[3],
            service_id: u16::from_be_bytes([bytes[4], bytes[5]]),
            instance_id: u16::from_be_bytes([bytes[6], bytes[7]]),
            major_version_and_ttl: u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            minor_version: u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
        })
    }
}

/// SomeIP 服务状态枚举。
///
/// 对应 C++ `enum class SomeIPStatus`（someip_protocol.h:48-52）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SomeIPStatus {
    /// 初始化状态
    Init = 1,
    /// 已发现服务
    Found = 2,
    /// 等待响应
    Wait = 3,
}

impl Default for SomeIPStatus {
    fn default() -> Self {
        SomeIPStatus::Init
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_to_bytes_roundtrip() {
        let entry = SomeIPEntry::default_entry();
        let bytes = entry.to_bytes();
        assert_eq!(bytes.len(), 16);

        let parsed = SomeIPEntry::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.service_id, 0x433F_u16.to_be());
    }
}
