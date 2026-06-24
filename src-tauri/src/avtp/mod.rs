/*!
 * 标准 AVTP 协议解析模块
 *
 * 该模块提供了标准 AVTP 协议（IEEE 1722）的完整解析功能。
 *
 * # 协议背景
 *
 * AVTP（Audio Video Transport Protocol，音视频传输协议）是 IEEE 1722 标准定义的
 * 用于在网络上传输音视频数据的链路层协议。它基于以太网，使用 EtherType 0x22F0 标识。
 *
 * AVTP 支持多种格式：
 * - AAF（Audio Applications Format）：纯音频格式
 * - MJPEG（Motion JPEG）：运动 JPEG 视频格式
 * - H.264：H.264 视频格式
 *
 * # 模块结构
 *
 * ```
 * avtp/
 * ├── mod.rs          # 模块入口（本文件）
 * ├── error.rs        # 错误类型定义
 * ├── header.rs       # 协议头部定义和解析
 * └── parser.rs       # 数据包解析和流跟踪
 * ```
 *
 * # 快速开始
 *
 * ## 示例 1：基本使用（与 pcap 模块集成）
 *
 * ```rust
 * use crate::pcap::capture::Capture;
 * use crate::avtp::parser::AvtpParser;
 *
 * // 1. 启动 pcap 抓包
 * let (mut capture, rx) = Capture::start_with_channel(
 *     r"\Device\NPF_{GUID}",
 *     true,   // 混杂模式
 *     65536,  // snaplen
 *     1000,   // 超时 ms
 * )?;
 *
 * // 2. 创建解析器
 * let mut parser = AvtpParser::new();
 *
 * // 3. 添加流过滤器（可选）
 * parser.add_stream_filter(0x123456789ABC0001, "Camera1".to_string());
 *
 * // 4. 接收并解析数据包
 * for pkt in rx {
 *     match parser.parse_packet(&pkt.data, pkt.timestamp) {
 *         Ok(Some(avtp_header)) => {
 *             println!("Received AVTP packet: subtype={:?}", avtp_header.subtype);
 *             // 根据 subtype 处理数据
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
 *
 * ## 示例 2：解析 MJPEG AVTP 数据包
 *
 * ```rust
 * use crate::avtp::parser::{AvtpParser, is_standard_avtp};
 * use crate::avtp::header::MjpegAvtpPacket;
 *
 * let mut parser = AvtpParser::new();
 *
 * for pkt in packet_receiver {
 *     // 检查是否为标准 AVTP 协议
 *     if is_standard_avtp(&pkt.data) {
 *         // 解析 MJPEG AVTP 数据包
 *         match parser.parse_mjpeg_packet(&pkt.data) {
 *             Ok(mjpeg_packet) => {
 *                 println!("MJPEG payload size: {} bytes", mjpeg_packet.mjpeg_payload.len());
 *                 // 处理 MJPEG 数据
 *             }
 *             Err(e) => {
 *                 eprintln!("Parse error: {}", e);
 *             }
 *         }
 *     }
 * }
 * ```
 *
 * # AVTP 协议格式
 *
 * ```
 * 以太网帧（14 字节）
 * ├── 目标 MAC (6 字节)
 * ├── 源 MAC (6 字节)
 * └── EtherType = 0x22F0 (2 字节)
 *
 * AVTP Common Stream Header（24 字节）
 * ├── subtype (1 字节)                  [以太网帧偏移 14]
 * ├── version_seq (1 字节)              [以太网帧偏移 15]
 * │   ├── version (3 bits)
 * │   ├── sequence_num (4 bits)
 * │   └── reserved (1 bit)
 * ├── stream_id (8 字节)               [以太网帧偏移 16-23]
 * ├── timestamp (4 字节)               [以太网帧偏移 24-27]
 * ├── gateway_info (2 字节)            [以太网帧偏移 28-29]
 * ├── packet_count (2 字节)            [以太网帧偏移 30-31]
 * └── reserved (6 字节)               [以太网帧偏移 32-37]
 *
 * AVTP 格式特定头部（可选，取决于 subtype）
 * └── ...（例如：MJPEG 格式有额外的视频流头部）
 * ```
 *
 * # 与 Stonkam 自定义协议的区别
 *
 * | 对比项 | 标准 AVTP (IEEE 1722) | Stonkam 自定义协议 |
 * |---|---|---|
 * | EtherType | 0x22F0 | 0x0022 |
 * | 协议头部长度 | 24 字节（Common Stream Header） | 24 字节（自定义格式） |
 * | 流 ID 字段 | 有（8 字节） | 无 |
 * | 时间戳字段 | 有（4 字节） | 无 |
 * | 序列号字段 | 有（4 bits） | 无 |
 * | 帧标志位 | 无 | 有（起始/结束标志） |
 * | 嵌入式参数 | 无 | 有（JPEG 数据前 12 字节） |
 *
 * # 注意事项
 *
 * 1. **协议复杂度**：标准 AVTP 协议比 Stonkam 自定义协议复杂得多，
 *    支持多种格式、流管理、时间戳同步等高级功能。
 *
 * 2. **实现完整性**：本模块目前实现了 AVTP Common Stream Header 的解析，
 *    对于特定格式（如 MJPEG、H.264）的解析可能需要进一步扩展。
 *
 * 3. **时间戳处理**：AVTP 时间戳使用纳秒为单位，需要与系统时钟同步。
 *
 * 4. **序列号跟踪**：解析器会自动跟踪序列号，检测丢包。
 *
 * # 参考资料
 *
 * - **IEEE 1722-2016**：AVTP 标准文档
 * - **Wireshark AVTP 解析器**：`epan/dissectors/packet-ieee1722.c`
 * - **Linux AVTP 工具**：https://github.com/AVnu/avtp
 * - **EtherType 列表**：https://www.iana.org/assignments/ieee-802-numbers/ieee-802-numbers.xhtml
 */

// 模块声明
pub mod error;
pub mod header;
pub mod parser;

// 公共类型重导出
// 这样用户可以直接通过 `use crate::avtp::AvtpError` 导入
pub use error::{
    AvtpError,
    Result,
};

pub use header::{
    AvtpSubtype,
    AvtpHeader,
    MjpegAvtpPacket,
};

pub use parser::{
    AvtpParser,
    parse_ethertype,
    is_stonkam_protocol,
    is_standard_avtp,
};

// 模块版本信息
/// 模块名称
pub const MODULE_NAME: &str = "avtp";

/// 模块版本
pub const MODULE_VERSION: &str = "1.0.0";

/// 支持的协议 EtherType
pub const ETHER_TYPE: u16 = 0x22F0;

/// AVTP Common Stream Header 长度（字节）
pub const COMMON_HEADER_SIZE: usize = 24;

/// 以太网帧头部长度（字节）
pub const ETHERNET_HEADER_SIZE: usize = 14;

/// 默认最大 AVTP 数据包大小（64KB）
pub const DEFAULT_MAX_PACKET_SIZE: usize = 64 * 1024;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, Duration};

    /// 测试：解析有效的 AVTP 数据包
    #[test]
    fn test_parse_valid_avtp_packet() {
        // 构造一个模拟的以太网帧
        let mut frame = Vec::new();

        // 以太网头部（14 字节）
        frame.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);  // 目标 MAC
        frame.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]);  // 源 MAC
        frame.extend_from_slice(&[0x22, 0xF0]);  // EtherType = 0x22F0

        // AVTP Common Stream Header（24 字节）
        frame.push(0x07);  // subtype = 0x07 (MJPEG)
        frame.push(0x00);  // version = 0, sequence_num = 0
        frame.extend_from_slice(&[0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);  // stream_id
        frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);  // timestamp
        frame.extend_from_slice(&[0x00, 0x00]);  // gateway_info
        frame.extend_from_slice(&[0x00, 0x01]);  // packet_count = 1
        frame.extend_from_slice(&[0x00; 6]);  // reserved

        // AVTP 数据（模拟）
        frame.extend_from_slice(&[0xFF, 0xD8]);  // JPEG 起始标记

        // 解析 AVTP 头部
        let header = AvtpHeader::from_ethernet_frame(&frame).unwrap();
        assert_eq!(header.subtype, AvtpSubtype::Mjpeg);
        assert_eq!(header.version, 0);
        assert_eq!(header.sequence_num, 0);
        assert_eq!(header.stream_id, 0x123456789ABCDEF0);
        assert_eq!(header.packet_count, 1);
    }

    /// 测试：无效的 EtherType
    #[test]
    fn test_invalid_ethertype() {
        let mut frame = Vec::new();

        // 以太网头部（14 字节）
        frame.extend_from_slice(&[0x00; 12]);
        frame.extend_from_slice(&[0x08, 0x00]);  // EtherType = 0x0800 (IPv4)

        // AVTP 数据
        frame.extend_from_slice(&[0x00; 24]);

        // 解析应该返回 None（不是 AVTP 协议）
        let result = is_standard_avtp(&frame);
        assert_eq!(result, false);
    }

    /// 测试：缓冲区长度不足
    #[test]
    fn test_buffer_too_short() {
        let frame = vec![0x00; 30];  // 只有 30 字节，不足 38 字节

        let result = AvtpHeader::from_ethernet_frame(&frame);
        assert!(result.is_err());
    }

    /// 测试：辅助函数
    #[test]
    fn test_helper_functions() {
        let mut frame = Vec::new();
        frame.extend_from_slice(&[0x00; 12]);
        frame.extend_from_slice(&[0x22, 0xF0]);  // EtherType = 0x22F0

        // 测试 parse_ethertype
        let ethertype = parse_ethertype(&frame).unwrap();
        assert_eq!(ethertype, 0x22F0);

        // 测试 is_standard_avtp
        let is_avtp = is_standard_avtp(&frame);
        assert_eq!(is_avtp, true);

        // 测试 is_stonkam_protocol
        let is_stonkam = is_stonkam_protocol(&frame);
        assert_eq!(is_stonkam, false);
    }
}
