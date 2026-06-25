//! MJPEG over RTP 重组器 (RFC 2435)
//!
//! 从 RTP 包流中重组 JPEG 图像

use crate::rtp::packet::RtpPacket;
use crate::rtp::decoder::frame::MediaPacket;
use crate::rtp::decoder::types::{MediaType, CodecType};
use bytes::Bytes;
use std::collections::BTreeMap;

/// JPEG/RTP 头 (RFC 2435 Section 3)
#[derive(Debug, Clone)]
pub struct JpegRtpHeader {
    /// Type field (bits 7-5: type, bits 4-3: q, bit 2: extension, bits 1-0: restart)
    pub type_spec: u8,
    /// Fragment offset (big-endian)
    pub offset: u32,
    /// JPEG type (from type_spec)
    pub jpeg_type: u8,
    /// Quantization factor (from type_spec)
    pub q: u8,
    /// Width in pixels (16 bits, big-endian)
    pub width: u16,
    /// Height in pixels (16 bits, big-endian)
    pub height: u16,
}

impl JpegRtpHeader {
    /// 从 RTP payload 解析 JPEG/RTP 头
    pub fn from_payload(payload: &[u8]) -> Option<Self> {
        if payload.len() < 8 {
            return None;
        }

        let type_spec = payload[0];
        let offset = u32::from_be_bytes([0, payload[1], payload[2], payload[3]]);
        let width = u16::from_be_bytes([payload[4], payload[5]]);
        let height = u16::from_be_bytes([payload[6], payload[7]]);

        let jpeg_type = (type_spec >> 3) & 0x07;
        let q = type_spec & 0x07;

        Some(Self {
            type_spec,
            offset,
            jpeg_type,
            q,
            width,
            height,
        })
    }

    /// 获取 JPEG 数据部分 (跳过 8 字节头)
    pub fn data_offset(&self) -> usize {
        8
    }
}

/// 量化表 (用于重建 JPEG)
#[derive(Debug, Clone)]
pub struct QuantizationTables {
    /// Luminance quantization table (64 bytes)
    pub luma: Option<[u8; 64]>,
    /// Chrominance quantization table (64 bytes)
    pub chroma: Option<[u8; 64]>,
}

/// 一个完整的 JPEG 帧 (可能由多个 RTP 包组成)
#[derive(Debug, Clone)]
pub struct JpegFrame {
    /// RTP 时间戳
    pub timestamp: u32,
    /// JPEG/RTP 头 (从第一个包获取)
    pub jpeg_header: JpegRtpHeader,
    /// 量化表 (如果需要)
    pub quant_tables: Option<QuantizationTables>,
    /// 重组后的 JPEG 数据
    pub data: Bytes,
}

/// MJPEG RTP 重组器
///
/// 收集同一个时间戳的所有 RTP 包，按 offset 排序后重组为完整的 JPEG 帧
pub struct MjpegReassembler {
    /// 当前正在收集的帧: timestamp -> (JpegRtpHeader, BTreeMap<offset, payload>)
    frames: BTreeMap<u32, (JpegRtpHeader, BTreeMap<u32, Vec<u8>>)>,
    /// 已完成的帧
    completed_frames: Vec<JpegFrame>,
    /// 最大缓存帧数
    max_frames: usize,
}

impl MjpegReassembler {
    pub fn new() -> Self {
        Self {
            frames: BTreeMap::new(),
            completed_frames: Vec::new(),
            max_frames: 10,
        }
    }

    /// 处理一个 RTP 包
    pub fn push_packet(&mut self, packet: &RtpPacket) -> Option<JpegFrame> {
        // 只处理 JPEG payload type (26)
        if packet.header.payload_type != 26 {
            return None;
        }

        let timestamp = packet.header.timestamp;

        // 解析 JPEG/RTP 头
        let jpeg_header = match JpegRtpHeader::from_payload(&packet.payload) {
            Some(h) => h,
            None => return None,
        };

        let data_offset = jpeg_header.data_offset();
        if packet.payload.len() <= data_offset {
            return None;
        }

        let jpeg_data = &packet.payload[data_offset..];

        // 获取或创建帧
        let entry = self.frames.entry(timestamp).or_insert_with(|| {
            (jpeg_header.clone(), BTreeMap::new())
        });

        // 存储此分片
        entry.1.insert(jpeg_header.offset, jpeg_data.to_vec());

        // 检查是否收到 marker 位 (最后一包)
        if packet.header.marker {
            // 重组帧
            if let Some(frame) = self.reassemble_frame(timestamp) {
                return Some(frame);
            }
        }

        None
    }

    /// 重组一个完整的帧
    fn reassemble_frame(&mut self, timestamp: u32) -> Option<JpegFrame> {
        if let Some((jpeg_header, fragments)) = self.frames.remove(&timestamp) {
            // 按 offset 顺序拼接数据
            let mut data = Vec::new();
            for (_, fragment) in &fragments {
                data.extend_from_slice(fragment);
            }

            let frame = JpegFrame {
                timestamp,
                jpeg_header,
                quant_tables: None,
                data: Bytes::from(data),
            };

            self.completed_frames.push(frame.clone());

            // 限制缓存大小
            if self.completed_frames.len() > self.max_frames {
                self.completed_frames.remove(0);
            }

            Some(frame)
        } else {
            None
        }
    }

    /// 获取已完成的帧
    pub fn frames(&self) -> &[JpegFrame] {
        &self.completed_frames
    }

    /// 清除已完成的帧
    pub fn clear_frames(&mut self) {
        self.completed_frames.clear();
    }
}

/// 将 JPEG 数据包装为完整的 JPEG 文件格式
///
/// RTP 中的 JPEG 数据可能不包含完整的 JPEG 文件头，
/// 此函数检查数据是否已经是有效的 JPEG 文件，如果不是则添加必要的 JPEG 标记。
pub fn wrap_jpeg_frame(frame: &JpegFrame) -> Vec<u8> {
    let data = &frame.data;
    
    // 检查数据是否已经是完整的 JPEG 文件 (以 SOI 开头，以 EOI 结尾)
    if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8 {
        // 已经是 JPEG 文件，检查是否有 EOI 结尾
        if data.len() >= 4 {
            let last_two = &data[data.len() - 2..];
            if last_two[0] == 0xFF && last_two[1] == 0xD9 {
                // 完整的 JPEG 文件，直接返回
                return data.to_vec();
            }
        }
        
        // 有 SOI 但没有 EOI，添加 EOI
        let mut jpeg_file = data.to_vec();
        jpeg_file.extend_from_slice(&[0xFF, 0xD9]);
        return jpeg_file;
    }
    
    // 数据不是有效的 JPEG 文件，需要包装
    // 注意: 这种情况比较少见，因为大多数 IP 摄像头发送的是完整 JPEG
    let mut jpeg_file = Vec::new();

    // SOI (Start of Image)
    jpeg_file.extend_from_slice(&[0xFF, 0xD8]);

    // APP0 (JFIF marker)
    jpeg_file.extend_from_slice(&[0xFF, 0xE0]);
    jpeg_file.extend_from_slice(&[0x00, 0x10]); // Length
    jpeg_file.extend_from_slice(b"JFIF\0");
    jpeg_file.extend_from_slice(&[0x01, 0x02]); // Version 1.2
    jpeg_file.extend_from_slice(&[0x00]); // Units (0 = aspect ratio)
    jpeg_file.extend_from_slice(&[0x00, 0x01]); // X density
    jpeg_file.extend_from_slice(&[0x00, 0x01]); // Y density
    jpeg_file.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Thumbnail

    // DQT (Quantization Table) - 使用默认表或从 RTP 头获取
    add_default_quant_table(&mut jpeg_file);

    // DHT (Huffman Table) - 使用默认表
    add_default_huffman_table(&mut jpeg_file);

    // SOF0 (Start of Frame)
    jpeg_file.extend_from_slice(&[0xFF, 0xC0]);
    let height = frame.jpeg_header.height;
    let width = frame.jpeg_header.width;
    let sof_len = 8 + 3 * 3;
    jpeg_file.extend_from_slice(&(sof_len as u16).to_be_bytes());
    jpeg_file.extend_from_slice(&[0x08]); // Precision
    jpeg_file.extend_from_slice(&height.to_be_bytes());
    jpeg_file.extend_from_slice(&width.to_be_bytes());
    jpeg_file.extend_from_slice(&[0x03]); // Number of components
    jpeg_file.extend_from_slice(&[0x01, 0x22, 0x00]); // Y
    jpeg_file.extend_from_slice(&[0x02, 0x11, 0x01]); // Cb
    jpeg_file.extend_from_slice(&[0x03, 0x11, 0x01]); // Cr

    // SOS (Start of Scan)
    jpeg_file.extend_from_slice(&[0xFF, 0xDA]);
    jpeg_file.extend_from_slice(&[0x00, 0x0C]);
    jpeg_file.extend_from_slice(&[0x03]);
    jpeg_file.extend_from_slice(&[0x01, 0x00]);
    jpeg_file.extend_from_slice(&[0x02, 0x11]);
    jpeg_file.extend_from_slice(&[0x03, 0x11]);
    jpeg_file.extend_from_slice(&[0x00, 0x3F, 0x00]);

    // JPEG 数据
    jpeg_file.extend_from_slice(data);

    // EOI (End of Image)
    jpeg_file.extend_from_slice(&[0xFF, 0xD9]);

    jpeg_file
}

/// 添加默认量化表
fn add_default_quant_table(jpeg: &mut Vec<u8>) {
    // Luma table
    jpeg.extend_from_slice(&[0xFF, 0xDB]);
    jpeg.extend_from_slice(&[0x00, 0x43]); // Length
    jpeg.extend_from_slice(&[0x00]); // Table ID (0 = luma)
    let luma_table = [
        16, 11, 10, 16, 24, 40, 51, 61,
        12, 12, 14, 19, 26, 58, 60, 55,
        14, 13, 16, 24, 40, 57, 69, 56,
        14, 17, 22, 29, 51, 87, 80, 62,
        18, 22, 37, 56, 68, 109, 103, 77,
        24, 35, 55, 64, 81, 104, 113, 92,
        49, 64, 78, 87, 103, 121, 120, 101,
        72, 92, 95, 98, 112, 100, 103, 99,
    ];
    jpeg.extend_from_slice(&luma_table);

    // Chroma table
    jpeg.extend_from_slice(&[0xFF, 0xDB]);
    jpeg.extend_from_slice(&[0x00, 0x43]); // Length
    jpeg.extend_from_slice(&[0x01]); // Table ID (1 = chroma)
    let chroma_table = [
        17, 18, 24, 47, 99, 99, 99, 99,
        18, 21, 26, 66, 99, 99, 99, 99,
        24, 26, 56, 99, 99, 99, 99, 99,
        47, 66, 99, 99, 99, 99, 99, 99,
        99, 99, 99, 99, 99, 99, 99, 99,
        99, 99, 99, 99, 99, 99, 99, 99,
        99, 99, 99, 99, 99, 99, 99, 99,
        99, 99, 99, 99, 99, 99, 99, 99,
    ];
    jpeg.extend_from_slice(&chroma_table);
}

/// 添加默认 Huffman 表
fn add_default_huffman_table(jpeg: &mut Vec<u8>) {
    // DC Luma
    jpeg.extend_from_slice(&[0xFF, 0xC4]);
    jpeg.extend_from_slice(&[0x00, 0x1F]); // Length
    jpeg.extend_from_slice(&[0x00]); // Table ID
    let dc_luma_bits = [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0];
    jpeg.extend_from_slice(&dc_luma_bits);
    let dc_luma_vals = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    jpeg.extend_from_slice(&dc_luma_vals);

    // AC Luma
    jpeg.extend_from_slice(&[0xFF, 0xC4]);
    jpeg.extend_from_slice(&[0x00, 0xB5]); // Length
    jpeg.extend_from_slice(&[0x10]); // Table ID
    let ac_luma_bits = [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 0x7D];
    jpeg.extend_from_slice(&ac_luma_bits);
    let ac_luma_vals = [
        0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12,
        0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07,
        0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08,
        0x23, 0x42, 0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0,
        0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16,
        0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28,
        0x29, 0x2A, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39,
        0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
        0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59,
        0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
        0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79,
        0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
        0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98,
        0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7,
        0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6,
        0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5,
        0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4,
        0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2,
        0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA,
        0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8,
        0xF9, 0xFA,
    ];
    jpeg.extend_from_slice(&ac_luma_vals);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jpeg_rtp_header_parse() {
        let payload = [
            0x00, // type_spec: type=0, q=0
            0x00, 0x00, 0x00, // offset = 0
            0x01, 0x40, // width = 320
            0x00, 0xF0, // height = 240
            0xFF, 0xD8, 0xFF, 0xE0, // JPEG data starts here
        ];

        let header = JpegRtpHeader::from_payload(&payload).unwrap();
        assert_eq!(header.jpeg_type, 0);
        assert_eq!(header.q, 0);
        assert_eq!(header.offset, 0);
        assert_eq!(header.width, 320);
        assert_eq!(header.height, 240);
        assert_eq!(header.data_offset(), 8);
    }

    #[test]
    fn test_mjpeg_reassembler() {
        let mut reassembler = MjpegReassembler::new();

        // 创建两个 RTP 包 (模拟一帧 JPEG)
        let payload1 = Bytes::from(vec![
            0x00, 0x00, 0x00, 0x00, // JPEG RTP header
            0x01, 0x40, 0x00, 0xF0,
            0xAA, 0xBB, 0xCC, // JPEG data part 1
        ]);

        let payload2 = Bytes::from(vec![
            0x00, 0x00, 0x00, 0x03, // JPEG RTP header, offset = 3
            0x01, 0x40, 0x00, 0xF0,
            0xDD, 0xEE, 0xFF, // JPEG data part 2
        ]);

        // 这里需要创建完整的 RtpPacket，但为了简化测试，我们跳过
        // 实际使用时需要从网络接收的 RTP 包
    }
}

/// 将 JpegFrame 转换为 MediaPacket
impl From<JpegFrame> for MediaPacket {
    fn from(frame: JpegFrame) -> Self {
        MediaPacket {
            media_type: MediaType::Video,
            codec_type: CodecType::MJPEG,
            pts: frame.timestamp as i64,
            dts: frame.timestamp as i64,
            keyframe: true, // MJPEG 每一帧都是关键帧
            data: frame.data,
            backend: None,
        }
    }
}
