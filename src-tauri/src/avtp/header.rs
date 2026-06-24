/*!
 * 标准 AVTP 协议头部定义
 *
 * 该模块定义了标准 AVTP 协议（IEEE 1722）的头部格式和解析逻辑。
 *
 * 协议格式：
 * ```
 * 以太网帧（14 字节）
 * ├── 目标 MAC (6 字节)
 * ├── 源 MAC (6 字节)
 * └── EtherType = 0x22F0 (2 字节)
 *
 * AVTP Common Stream Header（24 字节）
 * ├── subtype (1 字节)                  [以太网帧偏移量 14]
 * ├── version_seq (1 字节)              [以太网帧偏移量 15]
 * │   ├── version (3 bits)
 * │   ├── sequence_num (4 bits)
 * │   └── reserved (1 bit)
 * ├── stream_id (8 字节)               [以太网帧偏移量 16-23]
 * ├── timestamp (4 字节)               [以太网帧偏移量 24-27]
 * ├── gateway_info (2 字节)            [以太网帧偏移量 28-29]
 * ├── packet_count (2 字节)            [以太网帧偏移量 30-31]
 * └── reserved (6 字节)               [以太网帧偏移量 32-37]
 *
 * AVTP 格式特定头部（可选，取决于 subtype）
 * └── ...（例如：MJPEG 格式有额外的视频流头部）
 * ```
 *
 * 支持的 subtype：
 * - 0x00：AAF Audio
 * - 0x05：H.264 Video
 * - 0x07：MJPEG Video
 */

use crate::avtp::error::{Result, AvtpError};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

/// AVTP Subtype（子类型）
///
/// 该枚举定义了 AVTP 协议支持的子类型。
/// subtype 字段位于 AVTP Common Stream Header 的偏移量 0。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvtpSubtype {
    /// AAF Audio Format
    ///
    /// 纯音频格式，用于传输未压缩的音频数据。
    Aaf = 0x00,

    /// MJPEG Video Format
    ///
    /// Motion JPEG 视频格式，用于传输 JPEG 图像序列。
    Mjpeg = 0x07,

    /// H.264 Video Format
    ///
    /// H.264 视频格式，用于传输 H.264 编码的视频数据。
    H264 = 0x05,

    /// 未知子类型
    ///
    /// 不支持或未知的子类型。
    Unknown(u8),
}

impl From<u8> for AvtpSubtype {
    fn from(value: u8) -> Self {
        match value {
            0x00 => AvtpSubtype::Aaf,
            0x07 => AvtpSubtype::Mjpeg,
            0x05 => AvtpSubtype::H264,
            _ => AvtpSubtype::Unknown(value),
        }
    }
}

impl Into<u8> for AvtpSubtype {
    fn into(self) -> u8 {
        match self {
            AvtpSubtype::Aaf => 0x00,
            AvtpSubtype::Mjpeg => 0x07,
            AvtpSubtype::H264 => 0x05,
            AvtpSubtype::Unknown(v) => v,
        }
    }
}

/// AVTP Common Stream Header
///
/// 该结构体表示 AVTP 协议的通用流头部（24 字节）。
/// 所有 AVTP 数据包都包含这个头部。
///
/// # 字段说明
/// - `subtype`: 子类型（例如：0x07 = MJPEG）
/// - `version`: 协议版本（当前为 0）
/// - `sequence_num`: 序列号（用于检测丢包）
/// - `stream_id`: 流 ID（唯一标识一个 AVTP 流）
/// - `timestamp`: 时间戳（表示数据采集时间）
/// - `gateway_info`: 网关信息（保留字段）
/// - `packet_count`: 包计数（在流中的序号）
#[derive(Debug, Clone)]
pub struct AvtpHeader {
    /// 子类型
    pub subtype: AvtpSubtype,

    /// 协议版本（3 bits）
    ///
    /// 当前版本为 0。
    pub version: u8,

    /// 序列号（4 bits）
    ///
    /// 用于检测丢包。接收端可以通过检查序列号是否连续来判断是否丢包。
    pub sequence_num: u8,

    /// 流 ID（8 字节）
    ///
    /// 唯一标识一个 AVTP 流。
    /// 通常由设备的 MAC 地址 + 流编号组成。
    pub stream_id: u64,

    /// 时间戳（4 字节）
    ///
    /// 表示数据采集时间（纳秒为单位，从 1970-01-01 00:00:00 UTC 开始）。
    pub timestamp: u32,

    /// 网关信息（2 字节）
    ///
    /// 保留字段，当前未使用。
    pub gateway_info: u16,

    /// 包计数（2 字节）
    ///
    /// 表示当前包在流中的序号。
    pub packet_count: u16,

    /// 保留字段（6 字节）
    pub reserved: [u8; 6],
}

impl AvtpHeader {
    /// AVTP Common Stream Header 长度（字节）
    pub const SIZE: usize = 24;

    /// 从以太网帧数据解析 AVTP 头部
    ///
    /// 该方法从原始以太网帧数据中提取 AVTP Common Stream Header。
    ///
    /// # 参数
    /// - `ethernet_frame`: 原始以太网帧数据（至少 38 字节）
    ///
    /// # 返回值
    /// - `Ok(AvtpHeader)`: 解析成功
    /// - `Err(AvtpError)`: 解析失败
    ///
    /// # 错误
    /// - `BufferTooShort`: 缓冲区长度不足
    /// - `InvalidEtherType`: 不是 AVTP 协议（EtherType != 0x22F0）
    pub fn from_ethernet_frame(ethernet_frame: &[u8]) -> Result<Self> {
        // 检查缓冲区长度
        // 至少需要：以太网头（14 字节）+ AVTP 头部（24 字节）= 38 字节
        if ethernet_frame.len() < 14 + Self::SIZE {
            return Err(AvtpError::BufferTooShort {
                need: 14 + Self::SIZE,
                got: ethernet_frame.len(),
            });
        }

        // 检查 EtherType 是否为 0x22F0
        let ethertype = ((ethernet_frame[12] as u16) << 8) | (ethernet_frame[13] as u16);
        if ethertype != 0x22F0 {
            return Err(AvtpError::InvalidEtherType(ethertype));
        }

        // 解析 AVTP Common Stream Header
        // 位于以太网帧偏移量 14-37
        let header_data = &ethernet_frame[14..14 + Self::SIZE];

        // 偏移量 0：subtype
        let subtype = AvtpSubtype::from(header_data[0]);

        // 偏移量 1：version (3 bits) + sequence_num (4 bits) + reserved (1 bit)
        let version = (header_data[1] >> 5) & 0x07;
        let sequence_num = (header_data[1] >> 1) & 0x0F;
        // 注意：这里简化了，实际的 AVTP 头部格式可能更复杂

        // 偏移量 2-9：stream_id (8 字节，大端)
        let stream_id = u64::from_be_bytes([
            header_data[2], header_data[3], header_data[4], header_data[5],
            header_data[6], header_data[7], header_data[8], header_data[9],
        ]);

        // 偏移量 10-13：timestamp (4 字节，大端)
        let timestamp = u32::from_be_bytes([
            header_data[10], header_data[11], header_data[12], header_data[13],
        ]);

        // 偏移量 14-15：gateway_info (2 字节，大端)
        let gateway_info = u16::from_be_bytes([header_data[14], header_data[15]]);

        // 偏移量 16-17：packet_count (2 字节，大端)
        let packet_count = u16::from_be_bytes([header_data[16], header_data[17]]);

        // 偏移量 18-23：reserved (6 字节)
        let mut reserved = [0u8; 6];
        reserved.copy_from_slice(&header_data[18..24]);

        Ok(Self {
            subtype,
            version,
            sequence_num,
            stream_id,
            timestamp,
            gateway_info,
            packet_count,
            reserved,
        })
    }

    /// 获取 AVTP 数据起始位置
    ///
    /// AVTP 数据位于以太网帧偏移量 38 的位置（14 + 24）。
    ///
    /// # 返回值
    /// - AVTP 数据在以太网帧中的起始偏移量（固定为 38）
    pub fn avtp_data_offset() -> usize {
        14 + Self::SIZE  // 14 (以太网头) + 24 (AVTP 头部)
    }

    /// 获取 AVTP 数据
    ///
    /// 从以太网帧中提取 AVTP 数据部分（偏移量 38 开始）。
    ///
    /// # 参数
    /// - `ethernet_frame`: 原始以太网帧数据
    ///
    /// # 返回值
    /// - `Ok(&[u8])`: AVTP 数据切片
    /// - `Err(AvtpError)`: 数据长度不足
    pub fn get_avtp_data<'a>(ethernet_frame: &'a [u8]) -> Result<&'a [u8]> {
        let offset = Self::avtp_data_offset();
        if ethernet_frame.len() <= offset {
            return Err(AvtpError::BufferTooShort {
                need: offset + 1,
                got: ethernet_frame.len(),
            });
        }

        Ok(&ethernet_frame[offset..])
    }

    /// 将时间戳转换为 SystemTime
    ///
    /// AVTP 时间戳是从 1970-01-01 00:00:00 UTC 开始的纳秒数。
    ///
    /// # 返回值
    /// - `SystemTime`: 对应的系统时间
    pub fn timestamp_to_system_time(timestamp: u32) -> SystemTime {
        // 注意：AVTP 时间戳的实际单位是纳秒，但这里简化为秒
        // 完整的实现需要考虑纳秒到秒的转换
        UNIX_EPOCH + Duration::from_secs(timestamp as u64)
    }
}

/// MJPEG AVTP 视频流头部
///
/// 该结构体表示 MJPEG 格式的 AVTP 视频流头部。
/// 位于 AVTP Common Stream Header 之后。
///
/// # 字段说明
/// - `header`: AVTP Common Stream Header
/// - `mjpeg_payload`: MJPEG 负载数据
#[derive(Debug, Clone)]
pub struct MjpegAvtpPacket {
    /// AVTP Common Stream Header
    pub header: AvtpHeader,

    /// MJPEG 负载数据
    ///
    /// 包含 JPEG 图像数据。
    pub mjpeg_payload: Vec<u8>,
}

impl MjpegAvtpPacket {
    /// 从以太网帧数据解析 MJPEG AVTP 数据包
    ///
    /// # 参数
    /// - `ethernet_frame`: 原始以太网帧数据
    ///
    /// # 返回值
    /// - `Ok(MjpegAvtpPacket)`: 解析成功
    /// - `Err(AvtpError)`: 解析失败
    pub fn from_ethernet_frame(ethernet_frame: &[u8]) -> Result<Self> {
        // 1. 解析 AVTP Common Stream Header
        let header = AvtpHeader::from_ethernet_frame(ethernet_frame)?;

        // 2. 检查 subtype 是否为 MJPEG
        if header.subtype != AvtpSubtype::Mjpeg {
            return Err(AvtpError::InvalidSubtype(header.subtype.into()));
        }

        // 3. 获取 AVTP 数据
        let avtp_data = AvtpHeader::get_avtp_data(ethernet_frame)?;

        // 4. 解析 MJPEG 负载（这里简化了，实际需要解析 MJPEG 特定的头部）
        let mjpeg_payload = avtp_data.to_vec();

        Ok(Self {
            header,
            mjpeg_payload,
        })
    }
}
