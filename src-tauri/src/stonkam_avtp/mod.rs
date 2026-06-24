/*!
 * Stonkam 自定义 AVTP 协议解析模块
 *
 * 该模块提供了 Stonkam 车载视频监控设备使用的自定义协议（EtherType 0x0022）的完整解析功能。
 *
 * # 协议背景
 *
 * Stonkam 是一家车载视频监控设备制造商，其设备使用自定义的以太网协议传输视频流。
 * 该协议基于以太网层（Layer 2），使用 EtherType 0x0022 标识，直接传输 JPEG 图像数据。
 *
 * 与标准 AVTP（IEEE 1722，EtherType 0x22F0）不同，Stonkam 协议是厂商自定义的简化版本，
 * 具有以下特点：
 * - 无流 ID、时间戳、序列号等标准 AVTP 字段
 * - 使用帧起始/结束标志进行 JPEG 帧重组
 * - 图像参数（宽、高、质量）嵌入在 JPEG 数据流的前 12 字节中
 *
 * # 模块结构
 *
 * ```
 * stonkam_avtp/
 * ├── mod.rs          # 模块入口（本文件）
 * ├── error.rs        # 错误类型定义
 * ├── header.rs       # 协议头部定义和解析
 * └── parser.rs       # 数据包解析和 JPEG 帧重组
 * ```
 *
 * # 快速开始
 *
 * ## 示例 1：基本使用（与 pcap 模块集成）
 *
 * ```rust
 * use crate::pcap::capture::Capture;
 * use crate::stonkam_avtp::parser::StonkamAvtpParser;
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
 * let mut parser = StonkamAvtpParser::new();
 *
 * // 3. 接收并解析数据包
 * for pkt in rx {
 *     // 检查 EtherType
 *     if pkt.data.len() >= 14 {
 *         let ethertype = u16::from_be_bytes([pkt.data[12], pkt.data[13]]);
 *         if ethertype == 0x0022 {
 *             // 解析数据包
 *             match parser.parse_packet(&pkt.data, pkt.timestamp) {
 *                 Ok(Some(jpeg_data)) => {
 *                     println!("Received complete JPEG frame: {} bytes", jpeg_data.len());
 *                     // 解码并显示 JPEG 图像
 *                 }
 *                 Ok(None) => {
 *                     // 中间包，继续等待
 *                 }
 *                 Err(e) => {
 *                     eprintln!("Parse error: {}", e);
 *                 }
 *             }
 *         }
 *     }
 * }
 * ```
 *
 * ## 示例 2：独立使用（解析离线数据）
 *
 * ```rust
 * use crate::stonkam_avtp::header::StonkamAvtpHeader;
 *
 * // 从文件读取以太网帧数据
 * let ethernet_frame = std::fs::read("capture.pcap")?;
 *
 * // 解析协议头部
 * let header = StonkamAvtpHeader::from_ethernet_frame(&ethernet_frame)?;
 * println!("Frame start: {}, Frame end: {}", header.frame_start, header.frame_end);
 * println!("Payload length: {}", header.payload_len);
 *
 * // 解析 JPEG 嵌入式头部
 * let jpeg_data = StonkamAvtpHeader::get_jpeg_data(&ethernet_frame)?;
 * let embedded_header = StonkamAvtpHeader::parse_jpeg_embedded_header(jpeg_data)?;
 * println!("Image size: {}x{}", embedded_header.width, embedded_header.height);
 * println!("Quality factor: {}", embedded_header.qp);
 * ```
 *
 * # 与标准 AVTP 的区别
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
 * 1. **EtherType 检查**：原 C++ 代码只检查 `packet[12] == 0x22`（低字节），
 *    未检查 `packet[13] == 0x00`（高字节）。这可能导致误收其他使用 0x22xx EtherType 的协议。
 *    建议在应用层添加额外的过滤逻辑。
 *
 * 2. **协议头部冗余**：以太网帧偏移 16-33（14 字节）的用途未知，
 *    可能是预留字段或未使用的历史字段。
 *
 * 3. **无错误校验**：协议没有 CRC 或校验和字段，无法检测数据传输错误。
 *
 * 4. **无序列号**：无法检测丢包或乱序包（解析器通过帧标志位检测，但无法恢复）。
 *
 * # 参考资料
 *
 * - **Stonkam 官网**：https://www.stonkam.com/
 * - **标准 AVTP (IEEE 1722)**：https://standards.ieee.org/ieee/1722/6157/
 * - **EtherType 列表**：https://www.iana.org/assignments/ieee-802-numbers/ieee-802-numbers.xhtml
 * - **JPEG 标准 (ITU-T T.81)**：https://www.itu.int/rec/T-REC-T.81
 */

// 模块声明
pub mod error;
pub mod header;
pub mod parser;

// 公共类型重导出
// 这样用户可以直接通过 `use crate::stonkam_avtp::StonkamAvtpError` 导入
pub use error::{
    StonkamAvtpError,
    Result,
};

pub use header::{
    StonkamAvtpHeader,
    JpegEmbeddedHeader,
};

pub use parser::StonkamAvtpParser;

// 模块版本信息
/// 模块名称
pub const MODULE_NAME: &str = "stonkam_avtp";

/// 模块版本
pub const MODULE_VERSION: &str = "1.0.0";

/// 支持的协议 EtherType
pub const ETHER_TYPE: u16 = 0x0022;

/// 协议头部长度（字节）
pub const HEADER_SIZE: usize = 24;

/// JPEG 嵌入式头部长度（字节）
pub const JPEG_EMBEDDED_HEADER_SIZE: usize = 12;

/// 以太网帧头部长度（字节）
pub const ETHERNET_HEADER_SIZE: usize = 14;

/// 默认最大 JPEG 帧大小（5MB）
pub const DEFAULT_MAX_FRAME_SIZE: usize = 5 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, Duration};

    /// 测试：解析有效的 Stonkam 协议数据包
    #[test]
    fn test_parse_valid_packet() {
        // 构造一个模拟的以太网帧
        let mut frame = Vec::new();

        // 以太网头部（14 字节）
        frame.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);  // 目标 MAC
        frame.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]);  // 源 MAC
        frame.extend_from_slice(&[0x00, 0x22]);  // EtherType = 0x0022

        // 协议头部（24 字节）
        frame.push(0x00);  // 保留/版本
        frame.push(0x01);  // 帧起始标志（bit 0 = 1）
        frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);  // 保留字段 1
        frame.extend_from_slice(&[0x00; 14]);  // 保留字段 2
        frame.extend_from_slice(&[0x00, 0x64]);  // 负载长度 = 100 字节
        frame.push(0x00);  // 帧结束标志（bit 4 = 0）
        frame.push(0x00);  // 保留字段

        // JPEG 数据（100 字节，前 12 字节是嵌入式头部）
        // 嵌入式头部
        frame.extend_from_slice(&[0x00; 5]);  // 未知（5 字节）
        frame.push(0x50);  // 质量因子 QP = 80
        frame.push(0x28);  // 图像宽度 = 40 * 8 = 320 像素
        frame.push(0x20);  // 图像高度 = 32 * 8 = 256 像素
        frame.extend_from_slice(&[0x00, 0x00]);  // 重启间隔 = 0
        frame.extend_from_slice(&[0x00, 0x00]);  // 帧计数 = 0
        // JPEG 熵编码数据（88 字节）
        frame.extend_from_slice(&[0xFF, 0xD8]);  // JPEG 起始标记
        frame.extend_from_slice(&[0x00; 86]);  // 填充数据

        // 解析协议头部
        let header = StonkamAvtpHeader::from_ethernet_frame(&frame).unwrap();
        assert_eq!(header.frame_start, true);
        assert_eq!(header.frame_end, false);
        assert_eq!(header.payload_len, 100);

        // 解析 JPEG 嵌入式头部
        let jpeg_data = StonkamAvtpHeader::get_jpeg_data(&frame).unwrap();
        let embedded_header = StonkamAvtpHeader::parse_jpeg_embedded_header(jpeg_data).unwrap();
        assert_eq!(embedded_header.qp, 80);
        assert_eq!(embedded_header.width, 320);
        assert_eq!(embedded_header.height, 256);
    }

    /// 测试：无效的 EtherType
    #[test]
    fn test_invalid_ethertype() {
        let mut frame = Vec::new();

        // 以太网头部（14 字节）
        frame.extend_from_slice(&[0x00; 12]);
        frame.extend_from_slice(&[0x08, 0x00]);  // EtherType = 0x0800 (IPv4)

        // 协议数据
        frame.extend_from_slice(&[0x00; 24]);

        // 解析应该失败
        let result = StonkamAvtpHeader::from_ethernet_frame(&frame);
        assert!(result.is_err());
    }

    /// 测试：缓冲区长度不足
    #[test]
    fn test_buffer_too_short() {
        let frame = vec![0x00; 30];  // 只有 30 字节，不足 38 字节

        let result = StonkamAvtpHeader::from_ethernet_frame(&frame);
        assert!(result.is_err());
    }
}
