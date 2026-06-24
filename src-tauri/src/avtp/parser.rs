/*!
 * 标准 AVTP 协议解析器
 *
 * 该模块提供了标准 AVTP 协议（IEEE 1722）的完整解析功能。
 * 包括：
 * - 以太网帧解析
 * - AVTP Common Stream Header 解析
 * - 支持多种 subtype（AAF、MJPEG、H.264）
 * - 与 pcap 模块的集成
 *
 * 使用示例：
 * ```rust
 * use crate::avtp::parser::AvtpParser;
 *
 * let mut parser = AvtpParser::new();
 *
 * // 从 pcap 接收数据包
 * for pkt in packet_receiver {
 *     match parser.parse_packet(&pkt.data)? {
 *         Ok(Some(avtp_packet)) => {
 *             // 处理 AVTP 数据包
 *             println!("Received AVTP packet: subtype={:?}", avtp_packet.header.subtype);
 *         }
 *         Ok(None) => {
 *             // 不是 AVTP 协议，跳过
 *         }
 *         Err(e) => {
 *             eprintln!("Parse error: {}", e);
 *         }
 *     }
 * }
 * ```
 */

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::SystemTime;

use crate::avtp::error::{Result, AvtpError};
use crate::avtp::header::{AvtpHeader, AvtpSubtype, MjpegAvtpPacket};

/// 标准 AVTP 协议解析器
///
/// 该结构体用于解析标准 AVTP 协议的数据包。
///
/// # 工作原理
/// 1. 接收原始以太网帧（来自 pcap）
/// 2. 检查 EtherType 是否为 0x22F0
/// 3. 解析 AVTP Common Stream Header
/// 4. 根据 subtype 解析格式特定的数据
/// 5. 返回解析后的 AVTP 数据包
///
/// # 字段说明
/// - `stream_filters`: 流过滤器（只接收指定 stream_id 的数据包）
/// - `sequence_tracking`: 序列号跟踪（用于检测丢包）
/// - `stats`: 统计信息
pub struct AvtpParser {
    /// 流过滤器
    ///
    /// 如果设置了流过滤器，只接收 stream_id 在过滤器中的数据包。
    stream_filters: HashMap<u64, String>,  // stream_id -> stream_name

    /// 序列号跟踪
    ///
    /// 用于检测丢包。键是 stream_id，值是上一个序列号。
    sequence_tracking: HashMap<u64, u8>,

    /// 统计信息
    packet_count: AtomicU32,
    error_count: AtomicU32,
    dropped_packets: AtomicU32,
}

impl AvtpParser {
    /// 创建新的解析器实例
    ///
    /// # 返回值
    /// - `AvtpParser`: 新的解析器实例
    pub fn new() -> Self {
        Self {
            stream_filters: HashMap::new(),
            sequence_tracking: HashMap::new(),
            packet_count: AtomicU32::new(0),
            error_count: AtomicU32::new(0),
            dropped_packets: AtomicU32::new(0),
        }
    }

    /// 添加流过滤器
    ///
    /// 添加后，只接收指定 stream_id 的数据包。
    ///
    /// # 参数
    /// - `stream_id`: 流 ID
    /// - `stream_name`: 流名称（用于日志）
    pub fn add_stream_filter(&mut self, stream_id: u64, stream_name: String) {
        self.stream_filters.insert(stream_id, stream_name);
    }

    /// 移除流过滤器
    ///
    /// # 参数
    /// - `stream_id`: 流 ID
    pub fn remove_stream_filter(&mut self, stream_id: u64) {
        self.stream_filters.remove(&stream_id);
    }

    /// 清除所有流过滤器
    pub fn clear_stream_filters(&mut self) {
        self.stream_filters.clear();
    }

    /// 解析单个数据包
    ///
    /// 该方法接收一个原始以太网帧，解析其中的 AVTP 协议数据。
    ///
    /// # 参数
    /// - `ethernet_frame`: 原始以太网帧数据
    /// - `timestamp`: 数据包捕获时间戳
    ///
    /// # 返回值
    /// - `Ok(Some(AvtpHeader))`: 解析成功，返回 AVTP 头部
    /// - `Ok(None)`: 不是 AVTP 协议，跳过
    /// - `Err(AvtpError)`: 解析错误
    pub fn parse_packet(
        &mut self,
        ethernet_frame: &[u8],
        _timestamp: SystemTime,
    ) -> Result<Option<AvtpHeader>> {
        // 1. 检查缓冲区长度
        if ethernet_frame.len() < 14 {
            return Ok(None);  // 不是有效的以太网帧
        }

        // 2. 检查 EtherType 是否为 0x22F0
        let ethertype = ((ethernet_frame[12] as u16) << 8) | (ethernet_frame[13] as u16);
        if ethertype != 0x22F0 {
            return Ok(None);  // 不是 AVTP 协议
        }

        // 3. 解析 AVTP Common Stream Header
        let header = match AvtpHeader::from_ethernet_frame(ethernet_frame) {
            Ok(h) => h,
            Err(e) => {
                self.error_count.fetch_add(1, Ordering::SeqCst);
                return Err(e);
            }
        };

        // 4. 检查流过滤器
        if !self.stream_filters.is_empty() {
            if !self.stream_filters.contains_key(&header.stream_id) {
                return Ok(None);  // 不在过滤器中，跳过
            }
        }

        // 5. 检查序列号（检测丢包）
        if let Some(&last_seq) = self.sequence_tracking.get(&header.stream_id) {
            let expected_seq = (last_seq + 1) & 0x0F;  // 序列号是 4 bits，模 16
            if header.sequence_num != expected_seq {
                self.dropped_packets.fetch_add(1, Ordering::SeqCst);
                // 注意：这里只记录丢包，不返回错误
                eprintln!(
                    "Warning: Packet loss detected for stream {:016X}: expected seq={}, actual seq={}",
                    header.stream_id, expected_seq, header.sequence_num
                );
            }
        }

        // 6. 更新序列号跟踪
        self.sequence_tracking.insert(header.stream_id, header.sequence_num);

        // 7. 更新统计信息
        self.packet_count.fetch_add(1, Ordering::SeqCst);

        Ok(Some(header))
    }

    /// 解析 MJPEG AVTP 数据包
    ///
    /// 该方法专门解析 MJPEG 格式的 AVTP 数据包。
    ///
    /// # 参数
    /// - `ethernet_frame`: 原始以太网帧数据
    ///
    /// # 返回值
    /// - `Ok(MjpegAvtpPacket)`: 解析成功
    /// - `Err(AvtpError)`: 解析错误
    pub fn parse_mjpeg_packet(&self, ethernet_frame: &[u8]) -> Result<MjpegAvtpPacket> {
        MjpegAvtpPacket::from_ethernet_frame(ethernet_frame)
    }

    /// 重置解析器状态
    ///
    /// 清空序列号跟踪和统计信息。
    pub fn reset(&mut self) {
        self.sequence_tracking.clear();
        self.packet_count.store(0, Ordering::SeqCst);
        self.error_count.store(0, Ordering::SeqCst);
        self.dropped_packets.store(0, Ordering::SeqCst);
    }

    /// 获取统计信息
    ///
    /// # 返回值
    /// - `(u32, u32, u32)`: (数据包计数, 错误计数, 丢包计数)
    pub fn get_stats(&self) -> (u32, u32, u32) {
        (
            self.packet_count.load(Ordering::SeqCst),
            self.error_count.load(Ordering::SeqCst),
            self.dropped_packets.load(Ordering::SeqCst),
        )
    }
}

/// 辅助函数：解析以太网帧中的 EtherType
///
/// # 参数
/// - `ethernet_frame`: 原始以太网帧数据
///
/// # 返回值
/// - `Some(u16)`: EtherType 值
/// - `None`: 缓冲区长度不足
pub fn parse_ethertype(ethernet_frame: &[u8]) -> Option<u16> {
    if ethernet_frame.len() < 14 {
        return None;
    }

    Some(((ethernet_frame[12] as u16) << 8) | (ethernet_frame[13] as u16))
}

/// 辅助函数：检查是否为 Stonkam 自定义协议
///
/// # 参数
/// - `ethernet_frame`: 原始以太网帧数据
///
/// # 返回值
/// - `bool`: 是否为 Stonkam 协议（EtherType = 0x0022）
pub fn is_stonkam_protocol(ethernet_frame: &[u8]) -> bool {
    if let Some(ethertype) = parse_ethertype(ethernet_frame) {
        // 注意：原 C++ 代码只检查低字节
        (ethertype & 0xFF) == 0x22
    } else {
        false
    }
}

/// 辅助函数：检查是否为标准 AVTP 协议
///
/// # 参数
/// - `ethernet_frame`: 原始以太网帧数据
///
/// # 返回值
/// - `bool`: 是否为标准 AVTP 协议（EtherType = 0x22F0）
pub fn is_standard_avtp(ethernet_frame: &[u8]) -> bool {
    if let Some(ethertype) = parse_ethertype(ethernet_frame) {
        ethertype == 0x22F0
    } else {
        false
    }
}
